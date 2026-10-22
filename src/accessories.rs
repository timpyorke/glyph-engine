//! Lens accessories applied to RGBA pixels before the camera look. IDs match `AccessoryType.kt`:
//! 0 none, 1 ND, 2 Fisheye, 3 Prism, 4 Flash C, 5 Star, 6 Light leak.

use rayon::prelude::*;

pub fn apply_accessory(pixels: &mut [u8], w: usize, h: usize, id: i32) {
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    let pixels = &mut pixels[..w * h * 4];
    match id {
        1 => nd(pixels),
        2 => fisheye(pixels, w, h, 0.4, 0.45),
        3 => prism(pixels, w, h),
        4 => flash(pixels, w, h),
        5 => star(pixels, w, h),
        6 => light_leak(pixels, w, h),
        _ => {}
    }
}

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// About 1.5 stops darker, done in linear light so colours keep their balance.
fn nd(pixels: &mut [u8]) {
    let gain = 2f32.powf(-1.5);
    let lut: Vec<u8> = (0..256)
        .map(|v| to_u8(linear_to_srgb(srgb_to_linear(v as f32 / 255.0) * gain)))
        .collect();
    pixels.par_chunks_mut(4).for_each(|p| {
        for c in &mut p[..3] {
            *c = lut[*c as usize];
        }
    });
}

fn sample(src: &[u8], w: usize, h: usize, x: f32, y: f32) -> [f32; 3] {
    let x = x.clamp(0.0, w as f32 - 1.0);
    let y = y.clamp(0.0, h as f32 - 1.0);
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let px = |xx: usize, yy: usize, i: usize| src[(yy * w + xx) * 4 + i] as f32;
    let mut out = [0.0; 3];
    for i in 0..3 {
        let top = px(x0, y0, i) * (1.0 - fx) + px(x1, y0, i) * fx;
        let bot = px(x0, y1, i) * (1.0 - fx) + px(x1, y1, i) * fx;
        out[i] = (top * (1.0 - fy) + bot * fy) / 255.0;
    }
    out
}

/// Barrel distortion that fills the frame, with darkening toward the corners. [k] is the bulge, [dark] the corner falloff.
fn fisheye(pixels: &mut [u8], w: usize, h: usize, k: f32, dark: f32) {
    let src = pixels.to_vec();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let radius = (cx * cx + cy * cy).sqrt();
    pixels.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let nx = (x as f32 - cx) / radius;
            let ny = (y as f32 - cy) / radius;
            let r2 = nx * nx + ny * ny;
            // Divided by the corner's factor so the frame corners map to themselves: every output
            // pixel then samples inside the source and the picture fills the frame (no black rim).
            let f = (1.0 + k * r2) / (1.0 + k);
            let (sx, sy) = (cx + nx * f * radius, cy + ny * f * radius);
            let o = x * 4;
            let c = sample(&src, w, h, sx, sy);
            let v = 1.0 - dark * r2.min(1.0) * r2.min(1.0);
            for i in 0..3 {
                row[o + i] = to_u8(c[i] * v);
            }
        }
    });
}

/// Two tinted ghost copies of the scene, offset either side of the original.
fn prism(pixels: &mut [u8], w: usize, h: usize) {
    let src = pixels.to_vec();
    let shift = (w.min(h) as f32 * 0.03).max(1.0);
    let ghosts = [(shift, shift * 0.4, [1.0, 0.35, 0.8]), (-shift, -shift * 0.4, [0.35, 0.8, 1.0])];
    pixels.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let o = x * 4;
            let mut c = [row[o] as f32 / 255.0, row[o + 1] as f32 / 255.0, row[o + 2] as f32 / 255.0];
            for (dx, dy, tint) in ghosts {
                let g = sample(&src, w, h, x as f32 + dx, y as f32 + dy);
                for i in 0..3 {
                    // Screen blend keeps the ghosts luminous without washing out darks.
                    let add = g[i] * tint[i] * 0.3;
                    c[i] = 1.0 - (1.0 - c[i]) * (1.0 - add);
                }
            }
            for i in 0..3 {
                row[o + i] = to_u8(c[i]);
            }
        }
    });
}

/// On-camera compact flash: hot centre, quick falloff, slightly lifted contrast.
fn flash(pixels: &mut [u8], w: usize, h: usize) {
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let radius = (cx * cx + cy * cy).sqrt();
    pixels.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let nx = (x as f32 - cx) / radius;
            let ny = (y as f32 - cy) / radius;
            let d2 = nx * nx + ny * ny;
            let gain = 0.55 + 0.85 * (-d2 * 7.0).exp();
            let o = x * 4;
            for i in 0..3 {
                let v = row[o + i] as f32 / 255.0 * gain;
                // Flash blows highlights toward white.
                let v = if v > 0.8 { 0.8 + (v - 0.8) * 1.4 } else { v };
                row[o + i] = to_u8(v);
            }
        }
    });
}

/// Six-point star streaks from bright highlights (three lines through each source).
fn star(pixels: &mut [u8], w: usize, h: usize) {
    let short = w.min(h) as f32;
    let step = ((short / 160.0) as usize).max(1);
    let len = (short * 0.14) as i32;
    let mut glow = vec![0f32; w * h * 3];
    let dirs: [(f32, f32); 6] = (0..6)
        .map(|i| {
            let a = std::f32::consts::PI / 3.0 * i as f32 - std::f32::consts::FRAC_PI_2;
            (a.cos(), a.sin())
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    for y in (0..h).step_by(step) {
        for x in (0..w).step_by(step) {
            let o = (y * w + x) * 4;
            let c = [pixels[o] as f32 / 255.0, pixels[o + 1] as f32 / 255.0, pixels[o + 2] as f32 / 255.0];
            let l = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
            if l < 0.88 {
                continue;
            }
            let power = (l - 0.88) / 0.12;
            for (dx, dy) in dirs {
                for t in 1..len {
                    let (px, py) = ((x as f32 + dx * t as f32) as i32, (y as f32 + dy * t as f32) as i32);
                    if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                        break;
                    }
                    let fall = 1.0 - t as f32 / len as f32;
                    let k = fall * fall * power * 0.035 * step as f32;
                    let g = (py as usize * w + px as usize) * 3;
                    for i in 0..3 {
                        glow[g + i] += c[i] * k;
                    }
                }
            }
        }
    }
    pixels.par_chunks_mut(4).zip(glow.par_chunks(3)).for_each(|(p, g)| {
        for i in 0..3 {
            let c = p[i] as f32 / 255.0;
            p[i] = to_u8(1.0 - (1.0 - c) * (1.0 - g[i].min(1.0)));
        }
    });
}

/// Light getting past a worn camera back: a red-orange glow bleeding in from the left edge with a hot
/// core and faint streaks, plus a smaller amber wash in the top-right corner. Screen-blended so darks stay put.
fn light_leak(pixels: &mut [u8], w: usize, h: usize) {
    let aspect = h as f32 / w as f32;
    pixels.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let v = y as f32 / h as f32;
        for x in 0..w {
            let u = x as f32 / w as f32;
            // Edge leak, tilted so the boundary isn't a straight vertical line.
            let d = (u + 0.22 * (v - 0.5)).max(0.0);
            let streak = 0.8 + 0.2 * (v * 9.0 + 1.3).sin();
            let edge = (-d * 4.2).exp() * streak;
            // Corner wash, measured in square units so it stays round on wide frames.
            let (cx, cy) = (1.0 - u, v * aspect);
            let corner = (-(cx * cx + cy * cy).sqrt() * 4.5).exp() * 0.6;
            let core = ((edge - 0.55) / 0.45).clamp(0.0, 1.0);
            let red = [1.0, 0.3 + 0.55 * core, 0.06 + 0.4 * core];
            let amber = [1.0, 0.72, 0.22];
            let o = x * 4;
            for i in 0..3 {
                let add = (edge * 0.85 * red[i] + corner * amber[i]).min(1.0);
                let c = row[o + i] as f32 / 255.0;
                row[o + i] = to_u8(1.0 - (1.0 - c) * (1.0 - add));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(w: usize, h: usize) -> Vec<u8> {
        let mut p = Vec::new();
        for y in 0..h {
            for x in 0..w {
                // A bright spot in the middle so the star filter has a source.
                let hot = (x as i32 - w as i32 / 2).abs() < 3 && (y as i32 - h as i32 / 2).abs() < 3;
                let v = if hot { 255 } else { (60 + x * 100 / w) as u8 };
                p.extend_from_slice(&[v, v, (v / 2).max(40), 255]);
            }
        }
        p
    }

    /// Accessory IDs the app can select (everything but NONE); the engine's own range when standalone.
    fn accessory_ids() -> Vec<i32> {
        match crate::test_support::kotlin_ids("AccessoryType.kt") {
            Some(ids) => ids.into_iter().filter(|&id| id > 0).collect(),
            None => (1..=6).collect(),
        }
    }

    #[test]
    fn every_accessory_changes_pixels_and_keeps_alpha() {
        for id in accessory_ids() {
            let original = scene(96, 72);
            let mut p = original.clone();
            apply_accessory(&mut p, 96, 72, id);
            assert_ne!(p, original, "accessory {} should modify pixels", id);
            assert!(p.chunks(4).all(|c| c[3] == 255), "accessory {} must keep alpha", id);
        }
    }

    #[test]
    fn none_and_unknown_are_passthrough() {
        let original = scene(32, 24);
        for id in [0, 99, -1] {
            let mut p = original.clone();
            apply_accessory(&mut p, 32, 24, id);
            assert_eq!(p, original);
        }
    }

    #[test]
    fn nd_is_about_one_and_a_half_stops() {
        let mut p = vec![200, 200, 200, 255];
        apply_accessory(&mut p, 1, 1, 1);
        let ratio = srgb_to_linear(p[0] as f32 / 255.0) / srgb_to_linear(200.0 / 255.0);
        assert!((ratio - 2f32.powf(-1.5)).abs() < 0.02, "ratio {}", ratio);
    }

    #[test]
    fn short_buffer_does_not_panic() {
        let mut p = vec![10u8; 16];
        for id in accessory_ids() {
            apply_accessory(&mut p, 64, 48, id);
        }
    }
}
