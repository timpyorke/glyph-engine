//! Film response: how a stock turns scene colour into print colour, modelled on how film works
//! rather than on slider moves.
//!
//! Order: hue-selective shifts and saturation (OKLCh) -> colour balance and exposure (linear gains)
//! -> B&W spectral sensitivity -> per-channel characteristic curve -> print black and white.
//! Added chroma eases into the gamut edge instead of clipping, the way dyes saturate.
//!
//! The curve keeps both ends: contrast steepens the middle and rolls smoothly into black and white
//! (toe and shoulder), so no levels are crushed. Giving the channels different contrast is what
//! crosses the colour over between shadows and highlights, as mismatched dye layers do.

/// Hue-selective tweak centred on an OKLCh hue. Reference hues: red 20, skin 45, orange 60,
/// yellow 95, foliage 130, green 145, teal 180, cyan 215, sky 255, blue 270, purple 315, magenta 345.
#[derive(Clone, Copy)]
pub struct Hue {
    /// Centre hue in degrees.
    pub hue: f32,
    /// Half-width in degrees; the effect fades out smoothly over this distance.
    pub width: f32,
    /// Hue rotation in degrees (positive turns red -> yellow -> green -> blue).
    pub shift: f32,
    /// Chroma multiplier.
    pub chroma: f32,
    /// OKLab lightness offset at full chroma.
    pub lightness: f32,
}

pub const fn hue(hue: f32, width: f32, shift: f32, chroma: f32, lightness: f32) -> Hue {
    Hue { hue, width, shift, chroma, lightness }
}

const NO_HUE: Hue = hue(0.0, 1.0, 0.0, 1.0, 0.0);

#[derive(Clone, Copy)]
pub struct Stock {
    /// Linear-light channel gains: the stock's colour balance in daylight. Black stays black.
    pub balance: [f32; 3],
    /// Exposure in stops, with a soft shoulder so brightening never clips.
    pub exposure: f32,
    /// B&W spectral sensitivity as linear-light R, G, B weights summing to 1; None for colour film.
    pub mono: Option<[f32; 3]>,
    /// Per-channel slope of the characteristic curve at the pivot (1 = unchanged).
    pub contrast: [f32; 3],
    /// Display value the curve pivots on (mid grey is ~0.46).
    pub pivot: f32,
    /// Overall chroma multiplier.
    pub saturation: f32,
    /// Chroma left in the brightest highlights and deepest shadows (film loses colour at both ends).
    pub highlight_sat: f32,
    pub shadow_sat: f32,
    pub hues: [Hue; 6],
    /// Print black (Dmax) and white (base / paper white) per channel, display values.
    pub black: [f32; 3],
    pub white: [f32; 3],
}

pub const NEUTRAL: Stock = Stock {
    balance: [1.0; 3],
    exposure: 0.0,
    mono: None,
    contrast: [1.0; 3],
    pivot: 0.46,
    saturation: 1.0,
    highlight_sat: 1.0,
    shadow_sat: 1.0,
    hues: [NO_HUE; 6],
    black: [0.0; 3],
    white: [1.0; 3],
};

/// Builds the hue list from up to six tweaks; the rest stay neutral.
pub const fn hues<const N: usize>(list: [Hue; N]) -> [Hue; 6] {
    let mut out = [NO_HUE; 6];
    let mut i = 0;
    while i < N {
        out[i] = list[i];
        i += 1;
    }
    out
}

#[inline]
fn to_linear(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

#[inline]
fn to_display(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn linear_to_oklab(c: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * c[0] + 0.536_332_55 * c[1] + 0.051_445_99 * c[2]).cbrt();
    let m = (0.211_903_5 * c[0] + 0.680_699_5 * c[1] + 0.107_396_96 * c[2]).cbrt();
    let s = (0.088_302_46 * c[0] + 0.281_718_85 * c[1] + 0.629_978_7 * c[2]).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn oklab_to_linear(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0] + 0.396_337_78 * lab[1] + 0.215_803_76 * lab[2];
    let m = lab[0] - 0.105_561_346 * lab[1] - 0.063_854_17 * lab[2];
    let s = lab[0] - 0.089_484_18 * lab[1] - 1.291_485_5 * lab[2];
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

/// Characteristic curve through (0, 0), (pivot, pivot) and (1, 1) with slope [k] at the pivot.
/// Above 1, power curves on each side meet with matching slope, so contrast bends smoothly into
/// black and white. Below 1 the flattening fades out toward the ends instead, keeping their slope
/// finite (a power curve would shoot up vertically there, which a baked LUT can't follow).
#[inline]
pub(crate) fn curve(x: f32, k: f32, p: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if k >= 1.0 {
        if x <= p {
            p * (x / p).powf(k)
        } else {
            1.0 - (1.0 - p) * ((1.0 - x) / (1.0 - p)).powf(k)
        }
    } else {
        let u = if x <= p { x / p } else { (1.0 - x) / (1.0 - p) };
        x + (k - 1.0) * (x - p) * u * (2.0 - u)
    }
}

/// Largest OKLab chroma at lightness [l] and hue [h] (degrees) that stays inside the RGB cube.
fn max_chroma(l: f32, h: f32) -> f32 {
    let (sin, cos) = h.to_radians().sin_cos();
    let (mut lo, mut hi) = (0.0, 0.5);
    for _ in 0..14 {
        let mid = 0.5 * (lo + hi);
        let v = oklab_to_linear([l, mid * cos, mid * sin]);
        if v.iter().all(|&x| (0.0..=1.0).contains(&x)) { lo = mid } else { hi = mid }
    }
    lo
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Stock {
    /// Grades one normalized display-RGB colour.
    pub fn render(&self, c: [f32; 3]) -> [f32; 3] {
        let c = if self.mono.is_none() { self.shape_colour(c) } else { c };
        let mut lin = [to_linear(c[0]), to_linear(c[1]), to_linear(c[2])];

        let g = 2f32.powf(self.exposure);
        for i in 0..3 {
            let gain = self.balance[i] * g;
            // A gain above 1 rolls into white (Reinhard) instead of clipping; below 1 it just scales.
            lin[i] = if gain > 1.0 { lin[i] * gain / (1.0 + (gain - 1.0) * lin[i]) } else { lin[i] * gain };
        }
        if let Some(w) = self.mono {
            let y = w[0] * lin[0] + w[1] * lin[1] + w[2] * lin[2];
            lin = [y; 3];
        }

        let mut d = [0.0f32; 3];
        for i in 0..3 {
            let v = curve(to_display(lin[i]), self.contrast[i], self.pivot);
            d[i] = self.black[i] + v * (self.white[i] - self.black[i]);
        }
        d
    }

    /// Hue-selective rotation and lightness in OKLCh, then saturation (overall and per hue). A
    /// rotated colour that leaves the gamut eases back toward its edge instead of clipping into a
    /// flat patch.
    ///
    /// Works on display values taken as if linear: hue spacing stays close to true OKLab, and
    /// everything stays smooth in the display domain the grade is baked in (true linear light is
    /// steep near black, which a 25-point LUT can't follow in saturated blues and reds).
    fn shape_colour(&self, d: [f32; 3]) -> [f32; 3] {
        let [l, a, b] = linear_to_oklab(d);
        let mut chroma = (a * a + b * b).sqrt();
        let mut out = d;
        let mut scale = 1.0;
        if chroma > 1e-5 {
            let h = b.atan2(a).to_degrees();
            // Near-greys have no reliable hue: fade the hue tweaks in with chroma.
            let colourful = smoothstep(0.0, 0.05, chroma);
            let mut shift = 0.0;
            let mut lift = 0.0;
            let mut reach = 0.0f32;
            for t in &self.hues {
                let dist = ((h - t.hue + 540.0) % 360.0 - 180.0).abs();
                if dist >= t.width {
                    continue;
                }
                let w = 0.5 + 0.5 * (std::f32::consts::PI * dist / t.width).cos();
                shift += t.shift * w * colourful;
                scale *= 1.0 + (t.chroma - 1.0) * w;
                lift += t.lightness * w * smoothstep(0.0, 0.12, chroma);
                reach = reach.max(w);
            }
            if reach > 0.0 {
                // Colours already at the gamut edge (display primaries, beyond any film) have no
                // room to turn: taper the hue tweaks off as they get there.
                let room = 1.0 - smoothstep(0.5, 1.0, chroma / max_chroma(l, h).max(1e-6));
                let h = h + shift * room;
                let l = (l + lift * room).clamp(0.0, 1.0);
                let lo = max_chroma(l, h);
                let (sin, cos) = h.to_radians().sin_cos();
                let to_rgb = |c: f32| oklab_to_linear([l, c * cos, c * sin]);
                // Ease in from below the edge so no colour lands exactly on it: a hard landing is
                // a kink that shows as a flat patch and that a baked LUT can't follow.
                let start = lo * (1.0 - 0.15 * reach);
                if chroma > start {
                    let room = (lo - start).max(1e-6);
                    chroma = start + room * ((chroma - start) / room).tanh();
                }
                out = to_rgb(chroma);
            }
        }

        let ends = 1.0 + (self.highlight_sat - 1.0) * smoothstep(0.7, 1.0, l)
            + (self.shadow_sat - 1.0) * smoothstep(0.45, 0.1, l);
        let s = self.saturation * scale * ends.max(0.0);
        if s > 1.0 {
            // Like adding dye: the weaker channels thin out under the strongest one, so colours get
            // richer and deeper instead of glowing brighter.
            let m = out[0].max(out[1]).max(out[2]);
            if m > 0.0 {
                for v in out.iter_mut() {
                    *v = m * (v.max(0.0) / m).powf(s);
                }
            }
        } else {
            let y = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
            for v in out.iter_mut() {
                *v = y + (*v - y) * s;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_stock_is_identity() {
        for &c in &[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.8, 0.3, 0.1], [0.2, 0.6, 0.9], [0.46, 0.46, 0.46]] {
            let o = NEUTRAL.render(c);
            for i in 0..3 {
                assert!((o[i] - c[i]).abs() < 2e-3, "{:?} -> {:?}", c, o);
            }
        }
    }

    #[test]
    fn contrast_keeps_every_level() {
        let s = Stock { contrast: [1.4; 3], ..NEUTRAL };
        let mut last = -1.0;
        for i in 0..=255 {
            let v = s.render([i as f32 / 255.0; 3])[0];
            assert!(v > last, "level {} crushed: {} after {}", i, v, last);
            last = v;
        }
    }

    #[test]
    fn low_contrast_keeps_the_ends_and_order() {
        let s = Stock { contrast: [0.85; 3], ..NEUTRAL };
        assert!(s.render([0.0; 3])[0].abs() < 1e-6 && (s.render([1.0; 3])[0] - 1.0).abs() < 1e-6);
        let mut last = -1.0;
        for i in 0..=255 {
            let v = s.render([i as f32 / 255.0; 3])[0];
            assert!(v > last, "level {} out of order", i);
            last = v;
        }
    }

    #[test]
    fn balance_leaves_black_neutral() {
        let s = Stock { balance: [1.2, 1.0, 0.8], ..NEUTRAL };
        assert_eq!(s.render([0.0; 3]), [0.0; 3]);
    }
}
