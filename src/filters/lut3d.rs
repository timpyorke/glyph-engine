use rayon::prelude::*;

/// Largest accepted LUT edge length.
const MAX_SIZE: usize = 65;

#[derive(Debug)]
pub struct Lut3d {
    size: usize,
    data: Vec<[f32; 3]>,
}

impl Lut3d {
    /// Parse a .cube string into a Lut3d.
    pub fn from_cube_str(cube: &str) -> Option<Self> {
        let mut size: Option<usize> = None;
        let mut values = Vec::new();

        for line in cube.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0] {
                "LUT_3D_SIZE" => {
                    if tokens.len() >= 2 {
                        size = tokens[1].parse::<usize>().ok();
                    }
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    // Domain lines are informational; we ignore them and always normalize to [0,1].
                }
                _ => {
                    if tokens.len() >= 3 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            tokens[0].parse::<f32>(),
                            tokens[1].parse::<f32>(),
                            tokens[2].parse::<f32>(),
                        ) {
                            values.push([r, g, b]);
                        }
                    }
                }
            }
        }

        let size = size?;
        // sample_tetra needs a 2x2x2 cell at minimum; the cap keeps a hostile size from overflowing
        // `size^3` or reserving gigabytes. Real .cube files are 17, 33 or 65.
        if !(2..=MAX_SIZE).contains(&size) || values.len() != size * size * size {
            return None;
        }

        Some(Lut3d { size, data: values })
    }

    /// Identity LUT for the requested cube size.
    pub fn identity(size: usize) -> Self {
        let mut values = Vec::with_capacity(size * size * size);
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    values.push([
                        r as f32 / (size as f32 - 1.0),
                        g as f32 / (size as f32 - 1.0),
                        b as f32 / (size as f32 - 1.0),
                    ]);
                }
            }
        }
        Lut3d { size, data: values }
    }

    /// Builds a LUT by evaluating [f] at every grid point (r fastest, matching .cube order).
    pub fn from_fn(size: usize, f: impl Fn([f32; 3]) -> [f32; 3] + Sync) -> Self {
        let step = 1.0 / (size as f32 - 1.0);
        let data = (0..size * size * size)
            .into_par_iter()
            .map(|i| {
                let r = i % size;
                let g = (i / size) % size;
                let b = i / (size * size);
                f([r as f32 * step, g as f32 * step, b as f32 * step])
            })
            .collect();
        Lut3d { size, data }
    }

    fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    }

    fn get(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        let index = (b * self.size + g) * self.size + r;
        self.data[index]
    }

    /// Sample with tetrahedral interpolation: 4 lattice reads instead of trilinear's 8, which is
    /// what makes the per-pixel grade cheap enough for live preview. Inputs must be in [0, 1].
    #[inline]
    pub fn sample_tetra(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let n = self.size;
        let scale = (n - 1) as f32;
        let rf = r.clamp(0.0, 1.0) * scale;
        let gf = g.clamp(0.0, 1.0) * scale;
        let bf = b.clamp(0.0, 1.0) * scale;
        // Non-negative, so truncation is floor. Clamp keeps the +1 neighbour in range at 1.0.
        let r0 = (rf as usize).min(n - 2);
        let g0 = (gf as usize).min(n - 2);
        let b0 = (bf as usize).min(n - 2);
        let (fr, fg, fb) = (rf - r0 as f32, gf - g0 as f32, bf - b0 as f32);

        let dr = 1;
        let dg = n;
        let db = n * n;
        let base = b0 * db + g0 * dg + r0;
        let d = &self.data;
        let c000 = d[base];
        let c111 = d[base + dr + dg + db];

        // Walk from c000 to c111 through the two corners of the tetrahedron containing the point.
        let (w1, w2, w3, c1, c2) = if fr > fg {
            if fg > fb {
                (fr, fg, fb, d[base + dr], d[base + dr + dg])
            } else if fr > fb {
                (fr, fb, fg, d[base + dr], d[base + dr + db])
            } else {
                (fb, fr, fg, d[base + db], d[base + dr + db])
            }
        } else if fb > fg {
            (fb, fg, fr, d[base + db], d[base + dg + db])
        } else if fb > fr {
            (fg, fb, fr, d[base + dg], d[base + dg + db])
        } else {
            (fg, fr, fb, d[base + dg], d[base + dr + dg])
        };

        let mut out = [0.0f32; 3];
        for i in 0..3 {
            out[i] = c000[i] + w1 * (c1[i] - c000[i]) + w2 * (c2[i] - c1[i]) + w3 * (c111[i] - c2[i]);
        }
        out
    }

    /// Sample the LUT with trilinear interpolation.
    pub fn sample(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        if self.size == 0 {
            return [r, g, b];
        }

        let scale = (self.size - 1) as f32;
        let rf = (r.clamp(0.0, 1.0) * scale).clamp(0.0, scale);
        let gf = (g.clamp(0.0, 1.0) * scale).clamp(0.0, scale);
        let bf = (b.clamp(0.0, 1.0) * scale).clamp(0.0, scale);

        let r0 = rf.floor() as usize;
        let g0 = gf.floor() as usize;
        let b0 = bf.floor() as usize;
        let r1 = (r0 + 1).min(self.size - 1);
        let g1 = (g0 + 1).min(self.size - 1);
        let b1 = (b0 + 1).min(self.size - 1);

        let tr = rf - r0 as f32;
        let tg = gf - g0 as f32;
        let tb = bf - b0 as f32;

        let c000 = self.get(r0, g0, b0);
        let c100 = self.get(r1, g0, b0);
        let c010 = self.get(r0, g1, b0);
        let c110 = self.get(r1, g1, b0);
        let c001 = self.get(r0, g0, b1);
        let c101 = self.get(r1, g0, b1);
        let c011 = self.get(r0, g1, b1);
        let c111 = self.get(r1, g1, b1);

        let c00 = Lut3d::lerp(c000, c100, tr);
        let c01 = Lut3d::lerp(c001, c101, tr);
        let c10 = Lut3d::lerp(c010, c110, tr);
        let c11 = Lut3d::lerp(c011, c111, tr);

        let c0 = Lut3d::lerp(c00, c10, tg);
        let c1 = Lut3d::lerp(c01, c11, tg);

        Lut3d::lerp(c0, c1, tb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_degenerate_and_oversized_cubes() {
        assert!(Lut3d::from_cube_str("LUT_3D_SIZE 1\n0 0 0\n").is_none());
        assert!(Lut3d::from_cube_str("LUT_3D_SIZE 0\n").is_none());
        assert!(Lut3d::from_cube_str("LUT_3D_SIZE 99999999999\n").is_none());
        assert!(Lut3d::from_cube_str("LUT_3D_SIZE 66\n").is_none());
    }

    #[test]
    fn accepts_a_minimal_cube() {
        let cube = "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
        let lut = Lut3d::from_cube_str(cube).expect("2x2x2 cube parses");
        let s = lut.sample_tetra(0.25, 0.5, 0.75);
        assert!((s[0] - 0.25).abs() < 1e-5 && (s[1] - 0.5).abs() < 1e-5 && (s[2] - 0.75).abs() < 1e-5);
    }
}
