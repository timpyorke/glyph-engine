//! Print frames drawn over the finished image. IDs match `FrameType.kt`:
//! 0 none, 1 white, 2 Polaroid, 3 35mm, 4 Super 8, Instax prints as `style * 10 + size`.
//! Size 5 Mini, 6 Wide, 7 Square. Style 0 cream, 1 black, 2 rainbow, 3 confetti, 4 airmail, 5 gold, 6 silver sparkle.
//!
//! Frames are laid out upright; `rotation` is the clockwise turn (0/90/180/270) the buffer needs to
//! be upright, so a sideways sensor buffer still gets its chin and sprockets on the right edges.

const PRINT_WHITE: [u8; 3] = [245, 242, 235];
const FILM_BLACK: [u8; 3] = [12, 12, 12];
const SPROCKET: [u8; 3] = [235, 232, 220];

pub fn apply_frame(pixels: &mut [u8], w: usize, h: usize, id: i32, rotation: i32) {
    if w == 0 || h == 0 || pixels.len() < w * h * 4 || !(1..=4).contains(&id) && !is_instax(id) {
        return;
    }
    let turns = (rotation.rem_euclid(360) / 90) as usize;
    let pixels = &mut pixels[..w * h * 4];
    if turns == 0 {
        draw(pixels, w, h, id);
        return;
    }
    let (mut upright, uw, uh) = rotate_cw(pixels, w, h, turns);
    draw(&mut upright, uw, uh, id);
    let (back, bw, bh) = rotate_cw(&upright, uw, uh, (4 - turns) % 4);
    debug_assert_eq!((bw, bh), (w, h));
    pixels.copy_from_slice(&back);
}

fn rotate_cw(src: &[u8], w: usize, h: usize, turns: usize) -> (Vec<u8>, usize, usize) {
    let mut cur = src.to_vec();
    let (mut cw, mut ch) = (w, h);
    for _ in 0..turns % 4 {
        let mut next = vec![0u8; cur.len()];
        for y in 0..ch {
            for x in 0..cw {
                // (x, y) moves to (ch - 1 - y, x) in the rotated image of size ch x cw.
                let d = (x * ch + (ch - 1 - y)) * 4;
                let s = (y * cw + x) * 4;
                next[d..d + 4].copy_from_slice(&cur[s..s + 4]);
            }
        }
        cur = next;
        std::mem::swap(&mut cw, &mut ch);
    }
    (cur, cw, ch)
}

fn draw(p: &mut [u8], w: usize, h: usize, id: i32) {
    let short = w.min(h) as f32;
    match id {
        1 => {
            let b = (short * 0.04) as usize;
            border(p, w, h, b, b, b, b, PRINT_WHITE);
        }
        2 => {
            let side = (short * 0.05) as usize;
            border(p, w, h, side, side, side, (short * 0.2) as usize, PRINT_WHITE);
        }
        3 => {
            let band = (h as f32 * 0.11) as usize;
            border(p, w, h, 0, band, 0, band, FILM_BLACK);
            let hole_w = band as f32 * 0.5;
            let hole_h = band as f32 * 0.36;
            let pitch = band as f32 * 0.85;
            let mut x = pitch * 0.5;
            while x + hole_w < w as f32 {
                let inset = (band as f32 - hole_h) / 2.0;
                rounded_rect(p, w, h, x, inset, hole_w, hole_h, SPROCKET);
                rounded_rect(p, w, h, x, h as f32 - inset - hole_h, hole_w, hole_h, SPROCKET);
                x += pitch;
            }
        }
        4 => {
            let thin = (short * 0.02) as usize;
            let left = (short * 0.1) as usize;
            border(p, w, h, left, thin, thin, thin, FILM_BLACK);
            let hole_w = left as f32 * 0.45;
            let hole_h = left as f32 * 0.6;
            let pitch = hole_h * 1.9;
            let mut y = pitch * 0.4;
            while y + hole_h < h as f32 {
                rounded_rect(p, w, h, (left as f32 - hole_w) / 2.0, y, hole_w, hole_h, SPROCKET);
                y += pitch;
            }
        }
        _ if is_instax(id) => {
            let (wf, hf) = (w as f32, h as f32);
            let borders = match id % 10 {
                5 => [0.075 * wf, 0.075 * wf, 0.075 * wf, 0.30 * wf],
                6 => [0.045 * wf, 0.07 * hf, 0.045 * wf, 0.21 * hf],
                _ => [0.07 * wf, 0.07 * wf, 0.07 * wf, 0.26 * wf],
            };
            instax(p, w, h, borders, id / 10);
        }
        _ => {}
    }
}

fn is_instax(id: i32) -> bool {
    (5..=7).contains(&(id % 10)) && (5..=67).contains(&id)
}

/// Instax print: cream paper with a soft top-to-bottom tone and fine fibre noise, a deep chin with a
/// faint gloss streak, and the photo window edged by a thin inner shadow plus a pale developing
/// fringe, like a real instant print. [b] is the border thickness (left, top, right, bottom).
fn instax(p: &mut [u8], w: usize, h: usize, b: [f32; 4], style: i32) {
    let (l, t, r, bt) = (b[0], b[1], b[2], b[3]);
    let (x0, y0, x1, y1) = (l, t, w as f32 - r, h as f32 - bt);
    let fringe = (w.min(h) as f32 * 0.012).max(1.5);
    let shadow = fringe * 2.2;
    for y in 0..h {
        for x in 0..w {
            let o = (y * w + x) * 4;
            let (fx, fyy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = fx >= x0 && fx < x1 && fyy >= y0 && fyy < y1;
            if inside {
                // Distance to the nearest window edge (inside).
                let d = (fx - x0).min(x1 - fx).min(fyy - y0).min(y1 - fyy);
                if d < shadow {
                    let k = 1.0 - d / shadow;
                    let dark = 1.0 - 0.28 * k * k;
                    for i in 0..3 {
                        p[o + i] = (p[o + i] as f32 * dark) as u8;
                    }
                }
                if d < fringe {
                    // Pale, slightly warm bleed where the emulsion thins at the edge.
                    let k = (1.0 - d / fringe) * 0.22;
                    let paper = fringe_tone(style);
                    for i in 0..3 {
                        p[o + i] = (p[o + i] as f32 * (1.0 - k) + paper[i] * k) as u8;
                    }
                }
                continue;
            }
            let n = (hash(x as u32, y as u32) % 5) as f32 - 2.0;
            let mut c = paper(style, x, y, w, h, b, n);
            if fyy >= y1 {
                // Gloss streak across the chin.
                let u = (fx / w as f32 + (fyy - y1) / bt.max(1.0) * 0.5).fract();
                let g = (1.0 - ((u - 0.5).abs() * 6.0).min(1.0)) * 6.0;
                for v in c.iter_mut() {
                    *v += g;
                }
            }
            for i in 0..3 {
                p[o + i] = c[i].clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Colour the photo edge bleeds toward, per paper style.
fn fringe_tone(style: i32) -> [f32; 3] {
    match style {
        1 => [48.0, 48.0, 52.0],
        5 => [248.0, 228.0, 170.0],
        6 => [232.0, 236.0, 242.0],
        _ => [250.0, 244.0, 232.0],
    }
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h6 = (h.fract() + 1.0).fract() * 6.0;
    let f = h6 - h6.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let c = match h6 as i32 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    };
    [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0]
}

/// Paper colour at a border pixel for each print style. [n] is shared fibre noise.
fn paper(style: i32, x: usize, y: usize, w: usize, h: usize, b: [f32; 4], n: f32) -> [f32; 3] {
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    let short = w.min(h) as f32;
    let diag = (fx + fy) / (w + h) as f32;
    let tone = 1.0 - 0.04 * (y as f32 / h as f32);
    let cream = [244.0 * tone + n, 240.0 * tone + n, 228.0 * tone + n];
    match style {
        // Black frame
        1 => [24.0 + n, 24.0 + n, 27.0 + n],
        // Rainbow: hue sweeps across the whole print on the diagonal
        2 => {
            let c = hsv(diag * 1.1, 0.55, 0.97);
            [c[0] + n, c[1] + n, c[2] + n]
        }
        // Confetti: cream paper scattered with small coloured dots and flecks
        3 => {
            let cell = (short * 0.05).max(4.0);
            let (cx, cy) = ((fx / cell).floor(), (fy / cell).floor());
            let hv = hash(cx as u32, cy as u32);
            if hv % 3 == 0 {
                let jx = ((hv >> 4) % 100) as f32 / 100.0;
                let jy = ((hv >> 11) % 100) as f32 / 100.0;
                let (dx, dy) = (fx - (cx + 0.2 + 0.6 * jx) * cell, fy - (cy + 0.2 + 0.6 * jy) * cell);
                let r = cell * (0.12 + 0.1 * ((hv >> 18) % 4) as f32 / 4.0);
                if dx * dx + dy * dy < r * r {
                    return hsv(((hv >> 20) % 12) as f32 / 12.0, 0.65, 0.95);
                }
            }
            cream
        }
        // Airmail: red and blue diagonal stripes in a ring along the outer edge
        4 => {
            let edge = fx.min(fy).min(w as f32 - fx).min(h as f32 - fy);
            let ring = b[0].min(b[1]).min(b[2]) * 0.5;
            if edge < ring {
                let stripe = (short * 0.03).max(3.0);
                match (((fx + fy) / stripe) as i32).rem_euclid(4) {
                    0 => return [214.0 + n, 52.0 + n, 56.0 + n],
                    2 => return [48.0 + n, 94.0 + n, 178.0 + n],
                    _ => {}
                }
            }
            [cream[0] + 6.0, cream[1] + 6.0, cream[2] + 6.0]
        }
        // Gold: brushed metallic sweep
        5 => {
            let m = 0.5 + 0.5 * (diag * 14.0).sin();
            let k = 0.35 + 0.65 * m;
            [196.0 + 56.0 * k + n, 156.0 + 72.0 * k + n, 70.0 + 80.0 * k + n]
        }
        // Silver sparkle: cool metallic base with glitter flecks
        6 => {
            let m = 0.5 + 0.5 * (diag * 9.0).sin();
            let mut v = 188.0 + 40.0 * m + n;
            if hash(x as u32 ^ 0x51, y as u32) % 55 == 0 {
                v += 60.0;
            }
            [v, v + 3.0, v + 8.0]
        }
        _ => cream,
    }
}

fn hash(x: u32, y: u32) -> u32 {
    let mut v = x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca6b);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2c1b_3c6d);
    v ^ (v >> 12)
}

/// Paints solid bands of the given thickness over the edges (left, top, right, bottom).
fn border(p: &mut [u8], w: usize, h: usize, left: usize, top: usize, right: usize, bottom: usize, color: [u8; 3]) {
    for y in 0..h {
        for x in 0..w {
            if x < left || y < top || x >= w.saturating_sub(right) || y >= h.saturating_sub(bottom) {
                let o = (y * w + x) * 4;
                p[o..o + 3].copy_from_slice(&color);
            }
        }
    }
}

fn rounded_rect(p: &mut [u8], w: usize, h: usize, x: f32, y: f32, rw: f32, rh: f32, color: [u8; 3]) {
    let r = rw.min(rh) * 0.25;
    let (x0, y0) = (x.max(0.0) as usize, y.max(0.0) as usize);
    let (x1, y1) = (((x + rw).ceil() as usize).min(w), ((y + rh).ceil() as usize).min(h));
    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
            let dx = (r - fx).max(fx - (rw - r)).max(0.0);
            let dy = (r - fy).max(fy - (rh - r)).max(0.0);
            if dx * dx + dy * dy <= r * r {
                let o = (py * w + px) * 4;
                p[o..o + 3].copy_from_slice(&color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: usize, h: usize) -> Vec<u8> {
        [100u8, 100, 100, 255].repeat(w * h)
    }

    #[test]
    fn every_frame_paints_the_edges_and_keeps_the_centre() {
        let ids: Vec<i32> = match crate::test_support::kotlin_ids("FrameType.kt") {
            Some(ids) => ids.into_iter().filter(|&id| id > 0).collect(),
            None => (1..=4).chain((0..=6).flat_map(|st| (5..=7).map(move |sz| st * 10 + sz))).collect(),
        };
        for id in ids {
            let mut p = grey(120, 90);
            apply_frame(&mut p, 120, 90, id, 0);
            assert_ne!(&p[..3], &[100, 100, 100], "frame {} should cover the corner", id);
            let c = (45 * 120 + 60) * 4;
            assert_eq!(&p[c..c + 3], &[100, 100, 100], "frame {} must keep the centre", id);
            assert!(p.chunks(4).all(|c| c[3] == 255));
        }
    }

    #[test]
    fn polaroid_chin_follows_rotation() {
        // Buffer is 90x120 (portrait) but needs a 90 deg clockwise turn to be upright (landscape
        // 120x90), so the upright bottom (the chin) is the buffer's right edge.
        let (w, h) = (90, 120);
        let mut p = grey(w, h);
        apply_frame(&mut p, w, h, 2, 90);
        let chin_col = |x: usize| (60 * w + x) * 4; // mid-height row
        let left_painted = (0..18).filter(|&x| p[chin_col(x)] > 200).count();
        let right_painted = (w - 18..w).filter(|&x| p[chin_col(x)] > 200).count();
        assert!(right_painted > left_painted, "chin should sit on the right: {} vs {}", right_painted, left_painted);
    }

    #[test]
    fn rotation_roundtrip_preserves_unframed_pixels() {
        let (w, h) = (40, 30);
        let mut a = grey(w, h);
        for (i, px) in a.chunks_mut(4).enumerate() {
            px[0] = (i % 251) as u8;
        }
        let (r, rw, rh) = rotate_cw(&a, w, h, 1);
        let (back, bw, bh) = rotate_cw(&r, rw, rh, 3);
        assert_eq!((bw, bh), (w, h));
        assert_eq!(back, a);
    }

    #[test]
    fn none_unknown_and_short_buffers_are_safe() {
        let original = grey(32, 24);
        for id in [0, 9, -1] {
            let mut p = original.clone();
            apply_frame(&mut p, 32, 24, id, 0);
            assert_eq!(p, original);
        }
        let mut short = vec![1u8; 16];
        apply_frame(&mut short, 64, 48, 3, 90);
    }
}
