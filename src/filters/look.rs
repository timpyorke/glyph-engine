//! Parametric "camera look" pipeline used by the camera bodies.
//!
//! Order: layout remap -> chroma shift -> softness -> grade -> glow -> vignette -> grain -> frame border.
//! "Grade" (channel mix, color adjust, 3D LUT, film response, roll-off, fade, split tone) depends only on the
//! pixel's colour, so it is baked once per look into a cached 3D LUT. Everything per-pixel runs in
//! a single rayon pass over rows.

use crate::filters::color::{adjust_rgb, ColorParams};
use crate::filters::lut3d::Lut3d;
use crate::filters::stock::Stock;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Clone, Copy)]
#[allow(dead_code)] // Border, HalfFrame and Collage are not used by a body yet
pub enum Layout {
    Full,
    /// Uniform print border, as a fraction of the short side.
    Border { size: f32, color: [f32; 3] },
    /// Two copies of the centre crop side by side along the long axis, with a dark gutter.
    HalfFrame,
    /// 2x2 grid of the frame with a slightly different cast per tile.
    Collage,
    /// Black bars across the short axis so the picture is `ratio`:1 along the long axis (cinema scope).
    Letterbox { ratio: f32 },
}

#[derive(Clone, Copy)]
pub struct Look {
    pub color: ColorParams,
    pub lut: Option<&'static Lut3d>,
    /// Film response (see `stock.rs`), applied after colour adjust and LUT.
    pub stock: Option<Stock>,
    /// 3x3 row-major channel mixer applied before color adjust (rows are output R, G, B).
    pub mix: Option<[[f32; 3]; 3]>,
    /// Blend toward a blurred copy: 0 = sharp, 1 = fully soft.
    pub soft: f32,
    /// Additive bloom from bright areas of the blurred copy.
    pub glow: f32,
    /// Per-channel colour of the bloom; [1, 1, 1] is neutral, red-heavy reads as film halation.
    pub glow_tint: [f32; 3],
    /// Red/blue misregistration in pixels at a 480px short side (scaled with resolution).
    pub chroma_shift: f32,
    /// > 0 compresses highlights, < 0 pushes them toward clipping (blown CCD highlights).
    pub rolloff: f32,
    /// Black lift, 0..~0.2.
    pub fade: f32,
    pub shadow_tint: [f32; 3],
    pub highlight_tint: [f32; 3],
    pub vignette: f32,
    pub grain: f32,
    /// Grain cell size in pixels at a 480px short side (scaled with resolution).
    pub grain_size: f32,
    pub chroma_grain: f32,
    pub layout: Layout,
}

pub const NEUTRAL_COLOR: ColorParams = ColorParams {
    brightness: 0.0,
    contrast: 1.0,
    saturation: 1.0,
    temperature: 0.0,
    tint: 0.0,
};

pub const BASE: Look = Look {
    color: NEUTRAL_COLOR,
    lut: None,
    stock: None,
    mix: None,
    soft: 0.0,
    glow: 0.0,
    glow_tint: [1.0; 3],
    chroma_shift: 0.0,
    rolloff: 0.0,
    fade: 0.0,
    shadow_tint: [0.0; 3],
    highlight_tint: [0.0; 3],
    vignette: 0.0,
    grain: 0.0,
    grain_size: 1.0,
    chroma_grain: 0.0,
    layout: Layout::Full,
};

/// Per-tile casts for the collage layout (added after color adjust).
const COLLAGE_CASTS: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [0.05, 0.01, -0.04],
    [-0.03, 0.02, 0.05],
    [0.04, -0.03, 0.03],
];

#[inline]
fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// Uniform noise in [-1, 1].
#[inline]
fn noise(x: u32, y: u32, seed: u32) -> f32 {
    (hash(x, y, seed) & 0xffff) as f32 / 32767.5 - 1.0
}

#[inline]
fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Grid size for baked grades. 25 keeps the error within ~5/255 (only where contrast clips
/// saturated colours) while staying small enough (~190 KB) to sit in cache during the per-pixel pass.
const BAKED_LUT_SIZE: usize = 25;

static BAKED: Lazy<Mutex<HashMap<i32, &'static Lut3d>>> = Lazy::new(|| Mutex::new(HashMap::new()));

impl Look {
    /// Colour-only part of the look, evaluated directly. Input and output are normalized RGB.
    pub fn grade(&self, mut c: [f32; 3]) -> [f32; 3] {
        if let Some(m) = &self.mix {
            c = [
                m[0][0] * c[0] + m[0][1] * c[1] + m[0][2] * c[2],
                m[1][0] * c[0] + m[1][1] * c[1] + m[1][2] * c[2],
                m[2][0] * c[0] + m[2][1] * c[1] + m[2][2] * c[2],
            ];
        }

        c = adjust_rgb(c, &self.color);

        if let Some(lut) = self.lut {
            c = lut.sample(c[0], c[1], c[2]);
        }

        if let Some(stock) = &self.stock {
            c = stock.render(c);
        }

        for v in c.iter_mut() {
            let x = v.clamp(0.0, 1.0);
            *v = if self.rolloff > 0.0 {
                x * (1.0 + self.rolloff) / (1.0 + self.rolloff * x)
            } else {
                x - self.rolloff * x * x
            };
            *v = self.fade + *v * (1.0 - self.fade);
        }

        let l = luma(c).clamp(0.0, 1.0);
        let sh = (1.0 - l) * (1.0 - l);
        let hi = l * l;
        for i in 0..3 {
            c[i] = (c[i] + self.shadow_tint[i] * sh + self.highlight_tint[i] * hi).clamp(0.0, 1.0);
        }
        c
    }
}

/// Bakes [look]'s colour grade without caching it.
pub fn bake(look: &Look) -> Lut3d {
    Lut3d::from_fn(BAKED_LUT_SIZE, |c| look.grade(c))
}

/// Baked grade for filter [id], built on first use and kept for the life of the process.
pub fn baked_grade(id: i32, look: &Look) -> &'static Lut3d {
    if let Some(&lut) = BAKED.lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
        return lut;
    }
    // Bake without the lock: other filters stay available meanwhile, and the rayon bake can never
    // wait on a lock its own thread holds. Two threads racing on one ID both bake; the first wins.
    let baked = bake(look);
    let mut cache = BAKED.lock().unwrap_or_else(|e| e.into_inner());
    cache.entry(id).or_insert_with(|| Box::leak(Box::new(baked)))
}

/// Box-downsampled copy used for softness and glow, sampled back with bilinear filtering.
struct LowRes {
    w: usize,
    h: usize,
    factor: f32,
    data: Vec<[f32; 3]>,
}

impl LowRes {
    fn build(src: &[u8], w: usize, h: usize) -> Self {
        let f = (w.min(h) / 48).max(2);
        let lw = (w / f).max(1);
        let lh = (h / f).max(1);
        let mut data = vec![[0.0f32; 3]; lw * lh];
        data.par_chunks_mut(lw).enumerate().for_each(|(ly, row)| {
            for (lx, out) in row.iter_mut().enumerate() {
                let mut acc = [0.0f32; 3];
                let mut n = 0.0f32;
                // Sparse 4x4 sampling of each cell keeps this cheap at photo resolution.
                let step = (f / 4).max(1);
                let mut y = ly * f;
                while y < ((ly + 1) * f).min(h) {
                    let mut x = lx * f;
                    while x < ((lx + 1) * f).min(w) {
                        let i = (y * w + x) * 4;
                        acc[0] += src[i] as f32;
                        acc[1] += src[i + 1] as f32;
                        acc[2] += src[i + 2] as f32;
                        n += 1.0;
                        x += step;
                    }
                    y += step;
                }
                let k = 1.0 / (n.max(1.0) * 255.0);
                *out = [acc[0] * k, acc[1] * k, acc[2] * k];
            }
        });
        // One 3x3 box pass to widen the kernel.
        let mut blurred = data.clone();
        blurred.par_chunks_mut(lw).enumerate().for_each(|(y, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let mut acc = [0.0f32; 3];
                let mut n = 0.0;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let sx = x as i32 + dx;
                        let sy = y as i32 + dy;
                        if sx < 0 || sy < 0 || sx >= lw as i32 || sy >= lh as i32 {
                            continue;
                        }
                        let c = data[sy as usize * lw + sx as usize];
                        acc[0] += c[0];
                        acc[1] += c[1];
                        acc[2] += c[2];
                        n += 1.0;
                    }
                }
                *out = [acc[0] / n, acc[1] / n, acc[2] / n];
            }
        });
        LowRes { w: lw, h: lh, factor: f as f32, data: blurred }
    }

    #[inline]
    fn sample(&self, x: usize, y: usize) -> [f32; 3] {
        let fx = ((x as f32 + 0.5) / self.factor - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = ((y as f32 + 0.5) / self.factor - 0.5).clamp(0.0, (self.h - 1) as f32);
        let x0 = fx as usize;
        let y0 = fy as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let y1 = (y0 + 1).min(self.h - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let a = self.data[y0 * self.w + x0];
        let b = self.data[y0 * self.w + x1];
        let c = self.data[y1 * self.w + x0];
        let d = self.data[y1 * self.w + x1];
        let mut out = [0.0; 3];
        for i in 0..3 {
            let top = a[i] + (b[i] - a[i]) * tx;
            let bot = c[i] + (d[i] - c[i]) * tx;
            out[i] = top + (bot - top) * ty;
        }
        out
    }
}

/// Rearranges the frame for half-frame / collage layouts. Returns None for layouts that don't move pixels.
fn remap(src: &[u8], w: usize, h: usize, layout: Layout) -> Option<Vec<u8>> {
    match layout {
        Layout::HalfFrame => {
            let mut out = vec![0u8; src.len()];
            let horizontal = w >= h;
            let long = if horizontal { w } else { h };
            let half = long / 2;
            let quarter = long / 4;
            let gutter = (long / 80).max(2);
            out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
                for x in 0..w {
                    let pos = if horizontal { x } else { y };
                    let o = x * 4;
                    if pos.abs_diff(half) < gutter / 2 + 1 {
                        row[o..o + 4].copy_from_slice(&[12, 12, 12, 255]);
                        continue;
                    }
                    // Each half shows the centre half of the original frame.
                    let p = quarter + pos % half;
                    let (sx, sy) = if horizontal { (p.min(w - 1), y) } else { (x, p.min(h - 1)) };
                    let i = (sy * w + sx) * 4;
                    row[o..o + 4].copy_from_slice(&src[i..i + 4]);
                }
            });
            Some(out)
        }
        Layout::Collage => {
            let mut out = vec![0u8; src.len()];
            let hw = (w / 2).max(1);
            let hh = (h / 2).max(1);
            let gutter = (w.min(h) / 120).max(1);
            out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
                for x in 0..w {
                    let o = x * 4;
                    if x.abs_diff(hw) <= gutter || y.abs_diff(hh) <= gutter {
                        row[o..o + 4].copy_from_slice(&[245, 243, 238, 255]);
                        continue;
                    }
                    let sx = ((x % hw) * 2).min(w - 1);
                    let sy = ((y % hh) * 2).min(h - 1);
                    let i = (sy * w + sx) * 4;
                    row[o..o + 4].copy_from_slice(&src[i..i + 4]);
                }
            });
            Some(out)
        }
        _ => None,
    }
}

/// Applies [look] in place. [grade] is the look's baked colour grade (see [baked_grade]).
pub fn apply_look(pixels: &mut [u8], w: usize, h: usize, look: &Look, grade: &Lut3d, seed: u32) {
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return;
    }
    let pixels = &mut pixels[..w * h * 4];
    // Effects tuned at a 480px short side; scale pixel-sized parameters to the real frame.
    let scale = (w.min(h) as f32 / 480.0).max(0.25);

    let remapped = remap(pixels, w, h, look.layout);
    if let Some(r) = &remapped {
        pixels.copy_from_slice(r);
    }

    let shift = (look.chroma_shift * scale).round() as i32;
    let source: Option<Vec<u8>> = if shift != 0 { Some(pixels.to_vec()) } else { None };
    let low = if look.soft > 0.0 || look.glow > 0.0 { Some(LowRes::build(pixels, w, h)) } else { None };

    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let inv_diag = 1.0 / (cx * cx + cy * cy).sqrt();
    let grain_cell = (look.grain_size * scale).max(1.0);
    let border = match look.layout {
        Layout::Border { size, color } => Some(((w.min(h) as f32 * size) as usize, color)),
        _ => None,
    };
    // Letterbox bars run along the long axis; the picture band is long / ratio tall.
    let letterbox = match look.layout {
        Layout::Letterbox { ratio } if ratio > 0.0 => {
            let band = (w.max(h) as f32 / ratio) as usize;
            Some(((w.min(h).saturating_sub(band)) / 2, w >= h))
        }
        _ => None,
    };

    pixels.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let o = x * 4;

            if let Some((b, color)) = border {
                if x < b || y < b || x >= w - b || y >= h - b {
                    row[o] = (color[0] * 255.0) as u8;
                    row[o + 1] = (color[1] * 255.0) as u8;
                    row[o + 2] = (color[2] * 255.0) as u8;
                    continue;
                }
            }

            if let Some((bar, horizontal)) = letterbox {
                let (pos, short) = if horizontal { (y, h) } else { (x, w) };
                if pos < bar || pos >= short - bar {
                    row[o] = 0;
                    row[o + 1] = 0;
                    row[o + 2] = 0;
                    continue;
                }
            }

            let mut c = match &source {
                Some(src) => {
                    // Diagonal misregistration reads the same in any sensor orientation.
                    let rx = (x as i32 + shift).clamp(0, w as i32 - 1) as usize;
                    let ry = (y as i32 + shift).clamp(0, h as i32 - 1) as usize;
                    let bx = (x as i32 - shift).clamp(0, w as i32 - 1) as usize;
                    let by = (y as i32 - shift).clamp(0, h as i32 - 1) as usize;
                    [
                        src[(ry * w + rx) * 4] as f32 / 255.0,
                        src[(y * w + x) * 4 + 1] as f32 / 255.0,
                        src[(by * w + bx) * 4 + 2] as f32 / 255.0,
                    ]
                }
                None => [row[o] as f32 / 255.0, row[o + 1] as f32 / 255.0, row[o + 2] as f32 / 255.0],
            };

            let blur = low.as_ref().map(|low| low.sample(x, y));
            if let Some(blur) = blur {
                if look.soft > 0.0 {
                    for i in 0..3 {
                        c[i] += (blur[i] - c[i]) * look.soft;
                    }
                }
            }

            c = grade.sample_tetra(c[0], c[1], c[2]);

            if let Layout::Collage = look.layout {
                let tile = (x >= w / 2) as usize + 2 * (y >= h / 2) as usize;
                for i in 0..3 {
                    c[i] += COLLAGE_CASTS[tile][i];
                }
            }

            if let Some(blur) = blur {
                if look.glow > 0.0 {
                    // Bloom is light added on top of the graded image.
                    let bright = ((luma(blur) - 0.5) / 0.5).max(0.0);
                    let k = bright * bright * look.glow;
                    for i in 0..3 {
                        c[i] += blur[i] * k * look.glow_tint[i];
                    }
                }
            }

            let l = luma(c).clamp(0.0, 1.0);

            if look.vignette > 0.0 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d = (dx * dx + dy * dy).sqrt() * inv_diag; // 0 centre, 1 corner
                let t = ((d - 0.35) / 0.65).clamp(0.0, 1.0);
                let v = 1.0 - look.vignette * t * t * (3.0 - 2.0 * t);
                for ch in c.iter_mut() {
                    *ch *= v;
                }
            }

            if look.grain > 0.0 || look.chroma_grain > 0.0 {
                let gx = (x as f32 / grain_cell) as u32;
                let gy = (y as f32 / grain_cell) as u32;
                // Grain is strongest in the midtones, like real film.
                let mid = 1.0 - (2.0 * l - 1.0).powi(2) * 0.6;
                let n = noise(gx, gy, seed) * look.grain * 0.16 * mid;
                for i in 0..3 {
                    let cn = if look.chroma_grain > 0.0 {
                        noise(gx, gy, seed ^ (0x9e37_79b9u32.wrapping_mul(i as u32 + 1))) * look.chroma_grain * 0.06
                    } else {
                        0.0
                    };
                    c[i] += n + cn;
                }
            }

            row[o] = (c[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            row[o + 1] = (c[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            row[o + 2] = (c[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    });
}
