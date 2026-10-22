use rayon::prelude::*;

/// BT.601 full-range YUV_420_888 -> RGBA. `strides` is (y_row, uv_row, uv_pixel).
/// Returns false (leaving `out` untouched) when any buffer is too small for `w` x `h`.
pub fn yuv_to_rgba(
    y: &[u8],
    u: &[u8],
    v: &[u8],
    w: usize,
    h: usize,
    strides: (usize, usize, usize),
    out: &mut [u8],
) -> bool {
    let (y_stride, uv_stride, uv_ps) = strides;
    if w == 0 || h == 0 || out.len() < w * h * 4 {
        return false;
    }
    let y_needed = (h - 1) * y_stride + w;
    let uv_needed = ((h - 1) >> 1) * uv_stride + ((w - 1) >> 1) * uv_ps + 1;
    if y.len() < y_needed || u.len() < uv_needed || v.len() < uv_needed {
        return false;
    }

    out[..w * h * 4]
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(row, row_pixels)| {
            let y_row = &y[row * y_stride..row * y_stride + w];
            let uv_row_offset = (row >> 1) * uv_stride;

            for (col, px) in row_pixels.chunks_exact_mut(4).enumerate() {
                let y_val = y_row[col] as i32;
                let uv = uv_row_offset + (col >> 1) * uv_ps;
                let u_val = u[uv] as i32 - 128;
                let v_val = v[uv] as i32 - 128;

                // BT.601 full-range integer math
                px[0] = (y_val + ((v_val * 1404) >> 10)).clamp(0, 255) as u8;
                px[1] = (y_val - ((u_val * 346 + v_val * 715) >> 10)).clamp(0, 255) as u8;
                px[2] = (y_val + ((u_val * 1774) >> 10)).clamp(0, 255) as u8;
                px[3] = 255;
            }
        });
    true
}

/// `flip` bit: mirror columns (left-right in sensor space).
pub const FLIP_X: u32 = 1;
/// `flip` bit: mirror rows (top-bottom in sensor space).
pub const FLIP_Y: u32 = 2;

/// RGBA -> tightly packed NV12 (BT.601 limited range, U then V), mirrored by [FLIP_X] / [FLIP_Y]
/// bits in `flip`. Returns false if a buffer is too small.
pub fn rgba_to_nv12(rgba: &[u8], w: usize, h: usize, out: &mut [u8], flip: u32) -> bool {
    let y_size = w * h;
    let uv_size = (w / 2) * (h / 2) * 2;
    if w == 0 || h == 0 || rgba.len() < y_size * 4 || out.len() < y_size + uv_size {
        return false;
    }
    let (y_plane, uv_plane) = out[..y_size + uv_size].split_at_mut(y_size);

    let (fx, fy) = (flip & FLIP_X != 0, flip & FLIP_Y != 0);
    let src_col = |c: usize| if fx { w - 1 - c } else { c };
    let src_row = |r: usize| if fy { h - 1 - r } else { r };

    y_plane.par_chunks_mut(w).enumerate().for_each(|(row, ys)| {
        let sr = src_row(row);
        let src = &rgba[sr * w * 4..(sr + 1) * w * 4];
        for (col, y) in ys.iter_mut().enumerate() {
            let p = src_col(col) * 4;
            let (r, g, b) = (src[p] as i32, src[p + 1] as i32, src[p + 2] as i32);
            *y = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16) as u8;
        }
    });

    // Chroma subsampled 2x2, taken from the top-left pixel of each (mirrored) block.
    uv_plane.par_chunks_mut((w / 2) * 2).enumerate().for_each(|(uv_row, uvs)| {
        let src = &rgba[src_row(uv_row * 2) * w * 4..];
        for (i, pair) in uvs.chunks_exact_mut(2).enumerate() {
            let p = src_col(i * 2) * 4;
            let (r, g, b) = (src[p] as i32, src[p + 1] as i32, src[p + 2] as i32);
            pair[0] = (((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128) as u8;
            pair[1] = (((112 * r - 94 * g - 18 * b + 128) >> 8) + 128) as u8;
        }
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nv12_matches_reference_values() {
        // White, black, red, blue in a 2x2 block plus a second block.
        let px: [[u8; 4]; 2] = [[255, 255, 255, 255], [0, 0, 0, 255]];
        let (w, h) = (4, 2);
        let mut rgba = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&px[x / 2]);
            }
        }
        let mut out = vec![0u8; w * h + w * h / 2];
        assert!(rgba_to_nv12(&rgba, w, h, &mut out, 0));
        assert_eq!(out[0], 235); // white luma
        assert_eq!(out[2], 16); // black luma
        assert_eq!(&out[8..12], &[128, 128, 128, 128]); // neutral chroma
    }

    #[test]
    fn nv12_flip_mirrors_luma() {
        // 4x2 frame: columns get brighter left to right, the bottom row is black.
        let (w, h) = (4, 2);
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = if y == 0 { (x * 80) as u8 } else { 0 };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let size = w * h + w * h / 2;
        let (mut plain, mut fx, mut fy) = (vec![0u8; size], vec![0u8; size], vec![0u8; size]);
        assert!(rgba_to_nv12(&rgba, w, h, &mut plain, 0));
        assert!(rgba_to_nv12(&rgba, w, h, &mut fx, FLIP_X));
        assert!(rgba_to_nv12(&rgba, w, h, &mut fy, FLIP_Y));
        let row = |p: &[u8], r: usize| p[r * w..(r + 1) * w].to_vec();
        let mut reversed = row(&plain, 0);
        reversed.reverse();
        assert_eq!(row(&fx, 0), reversed, "FLIP_X reverses each row");
        assert_eq!(row(&fy, 0), row(&plain, 1), "FLIP_Y swaps top and bottom rows");
        assert_eq!(row(&fy, 1), row(&plain, 0));
    }

    #[test]
    fn yuv_rejects_short_planes() {
        let mut out = vec![0u8; 16];
        assert!(!yuv_to_rgba(&[0; 3], &[0; 1], &[0; 1], 2, 2, (2, 2, 2), &mut out));
    }

    #[test]
    fn yuv_grey_stays_grey() {
        let y = [128u8; 4];
        let uv = [128u8; 2];
        let mut out = vec![0u8; 16];
        assert!(yuv_to_rgba(&y, &uv, &uv, 2, 2, (2, 2, 2), &mut out));
        assert!(out.chunks(4).all(|p| p == [128, 128, 128, 255]));
    }
}
