//! C ABI for the iOS app (declared in `include/glyph_engine.h`). Mirrors the JNI entry points in
//! `lib.rs`: every function borrows caller-owned buffers for the duration of the call, returns
//! false on a null pointer or a buffer too small for the given size, and never unwinds into C.

use crate::filters::color::ColorParams;
use crate::{accessories, filters, frames, yuv};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::slice;

fn guarded(body: impl FnOnce() -> bool) -> bool {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(false)
}

fn dim(v: i32) -> usize {
    v.max(0) as usize
}

/// SAFETY: the caller guarantees `len` writable bytes at `ptr` that nothing else touches during the call.
unsafe fn buffer<'a>(ptr: *mut u8, len: usize) -> Option<&'a mut [u8]> {
    (!ptr.is_null()).then(|| slice::from_raw_parts_mut(ptr, len))
}

/// SAFETY: the caller guarantees `len` readable bytes at `ptr` for the duration of the call.
unsafe fn input<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    (!ptr.is_null()).then(|| slice::from_raw_parts(ptr, len))
}

fn fits(pixels: &[u8], w: i32, h: i32) -> bool {
    w > 0 && h > 0 && pixels.len() >= dim(w) * dim(h) * 4
}

/// Applies camera look / film `filter` to tightly packed RGBA in place.
#[no_mangle]
pub unsafe extern "C" fn glyph_apply_filter(pixels: *mut u8, len: usize, w: i32, h: i32, filter: i32) -> bool {
    guarded(|| match buffer(pixels, len) {
        Some(rgba) if fits(rgba, w, h) => {
            filters::apply_filter(rgba, dim(w), dim(h), filter);
            true
        }
        _ => false,
    })
}

/// User-tuned colour adjust + grain, in place.
#[no_mangle]
pub unsafe extern "C" fn glyph_apply_custom(
    pixels: *mut u8,
    len: usize,
    w: i32,
    h: i32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    grain: f32,
) -> bool {
    guarded(|| match buffer(pixels, len) {
        Some(rgba) if fits(rgba, w, h) => {
            let params = ColorParams { brightness, contrast, saturation, temperature, tint };
            filters::apply_custom(rgba, dim(w), dim(h), params, grain);
            true
        }
        _ => false,
    })
}

/// Lens accessory (IDs match `AccessoryType`), in place.
#[no_mangle]
pub unsafe extern "C" fn glyph_apply_accessory(pixels: *mut u8, len: usize, w: i32, h: i32, accessory: i32) -> bool {
    guarded(|| match buffer(pixels, len) {
        Some(rgba) if fits(rgba, w, h) => {
            accessories::apply_accessory(rgba, dim(w), dim(h), accessory);
            true
        }
        _ => false,
    })
}

/// Print frame (IDs match `FrameType`) drawn in place; `rotation` turns the buffer upright.
#[no_mangle]
pub unsafe extern "C" fn glyph_apply_frame(
    pixels: *mut u8,
    len: usize,
    w: i32,
    h: i32,
    frame: i32,
    rotation: i32,
) -> bool {
    guarded(|| match buffer(pixels, len) {
        Some(rgba) if fits(rgba, w, h) => {
            frames::apply_frame(rgba, dim(w), dim(h), frame, rotation);
            true
        }
        _ => false,
    })
}

/// Full-range YUV 4:2:0 planes -> RGBA. For a bi-planar NV12 camera buffer pass the CbCr plane as
/// `u` and the same plane + 1 as `v`, with `uv_pixel_stride` 2.
#[no_mangle]
pub unsafe extern "C" fn glyph_yuv_to_rgba(
    y: *const u8,
    y_len: usize,
    u: *const u8,
    u_len: usize,
    v: *const u8,
    v_len: usize,
    w: i32,
    h: i32,
    y_row_stride: i32,
    uv_row_stride: i32,
    uv_pixel_stride: i32,
    out: *mut u8,
    out_len: usize,
) -> bool {
    guarded(|| {
        let (Some(y), Some(u), Some(v), Some(out)) =
            (input(y, y_len), input(u, u_len), input(v, v_len), buffer(out, out_len))
        else {
            return false;
        };
        let strides = (dim(y_row_stride), dim(uv_row_stride), dim(uv_pixel_stride));
        yuv::yuv_to_rgba(y, u, v, dim(w), dim(h), strides, out)
    })
}

/// RGBA -> tightly packed NV12 (BT.601 limited range), mirrored by the `GLYPH_FLIP_*` bits.
#[no_mangle]
pub unsafe extern "C" fn glyph_rgba_to_nv12(
    rgba: *const u8,
    rgba_len: usize,
    w: i32,
    h: i32,
    out: *mut u8,
    out_len: usize,
    flip: u32,
) -> bool {
    guarded(|| match (input(rgba, rgba_len), buffer(out, out_len)) {
        (Some(rgba), Some(out)) => yuv::rgba_to_nv12(rgba, dim(w), dim(h), out, flip),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_null_and_short_buffers() {
        let mut px = vec![128u8; 4 * 4 * 4];
        unsafe {
            assert!(!glyph_apply_filter(std::ptr::null_mut(), 64, 4, 4, 101));
            assert!(!glyph_apply_filter(px.as_mut_ptr(), px.len(), 8, 8, 101));
            assert!(!glyph_apply_frame(px.as_mut_ptr(), px.len(), 0, 4, 1, 0));
            assert!(glyph_apply_filter(px.as_mut_ptr(), px.len(), 4, 4, 101));
            assert!(glyph_apply_accessory(px.as_mut_ptr(), px.len(), 4, 4, 1));
        }
    }

    #[test]
    fn converts_interleaved_nv12() {
        let (w, h) = (4usize, 2usize);
        let y = vec![200u8; w * h];
        let uv = vec![128u8; w]; // one chroma row, Cb Cr interleaved
        let mut out = vec![0u8; w * h * 4];
        let ok = unsafe {
            glyph_yuv_to_rgba(
                y.as_ptr(), y.len(), uv.as_ptr(), uv.len(), uv.as_ptr().add(1), uv.len() - 1,
                w as i32, h as i32, w as i32, w as i32, 2, out.as_mut_ptr(), out.len(),
            )
        };
        assert!(ok);
        assert_eq!(&out[..4], &[200, 200, 200, 255]);
    }
}
