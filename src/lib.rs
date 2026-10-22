mod accessories;
mod ffi;
mod filters;
mod frames;
#[cfg(test)]
mod test_support;
mod yuv;

use filters::color::ColorParams;
use jni::errors::Result as JniResult;
use jni::objects::{JByteArray, JByteBuffer, JClass};
use jni::sys::{jboolean, jbyteArray, jfloat, jint, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs a JNI body so nothing can unwind into the JVM (which would abort the whole process).
/// A JNI error or a Rust panic becomes a pending `RuntimeException` and `default` is returned.
fn guarded<T>(
    env: &mut JNIEnv,
    default: T,
    body: impl FnOnce(&mut JNIEnv) -> JniResult<T>,
) -> T {
    let message = match catch_unwind(AssertUnwindSafe(|| body(env))) {
        Ok(Ok(value)) => return value,
        Ok(Err(e)) => e.to_string(),
        Err(_) => "panic in glyph-engine".to_string(),
    };
    // A failed JNI call may already have left an exception pending; don't throw over it.
    if !env.exception_check().unwrap_or(true) {
        let _ = env.throw_new("java/lang/RuntimeException", message);
    }
    default
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_helloFromRust(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        Ok(env.new_string("Hello from Rust 🦀")?.into_raw())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyFilter(
    mut env: JNIEnv,
    _class: JClass,
    pixels: JByteArray,
    width: jint,
    height: jint,
    filter_type: jint,
) -> jbyteArray {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        let mut rgba = env.convert_byte_array(&pixels)?;
        filters::apply_filter(&mut rgba, dim(width), dim(height), filter_type);
        Ok(env.byte_array_from_slice(&rgba)?.into_raw())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyCustomFilter(
    mut env: JNIEnv,
    _class: JClass,
    pixels: JByteArray,
    width: jint,
    height: jint,
    brightness: jfloat,
    contrast: jfloat,
    saturation: jfloat,
    temperature: jfloat,
    tint: jfloat,
    grain_intensity: jfloat,
) -> jbyteArray {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        let mut rgba = env.convert_byte_array(&pixels)?;
        let params = ColorParams { brightness, contrast, saturation, temperature, tint };
        filters::apply_custom(&mut rgba, dim(width), dim(height), params, grain_intensity);
        Ok(env.byte_array_from_slice(&rgba)?.into_raw())
    })
}

/// Zero-copy variant of `applyFilter` for a direct RGBA ByteBuffer, filtered in place.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyFilterInPlace(
    mut env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
    width: jint,
    height: jint,
    filter_type: jint,
) {
    guarded(&mut env, (), |env| {
        if let Some(rgba) = direct_slice(env, &buffer) {
            filters::apply_filter(rgba, dim(width), dim(height), filter_type);
        }
        Ok(())
    })
}

/// Zero-copy variant of `applyCustomFilter` for a direct RGBA ByteBuffer, filtered in place.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyCustomFilterInPlace(
    mut env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
    width: jint,
    height: jint,
    brightness: jfloat,
    contrast: jfloat,
    saturation: jfloat,
    temperature: jfloat,
    tint: jfloat,
    grain_intensity: jfloat,
) {
    guarded(&mut env, (), |env| {
        if let Some(rgba) = direct_slice(env, &buffer) {
            let params = ColorParams { brightness, contrast, saturation, temperature, tint };
            filters::apply_custom(rgba, dim(width), dim(height), params, grain_intensity);
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_rgbaToNv12(
    mut env: JNIEnv,
    _class: JClass,
    rgba: JByteArray,
    width: jint,
    height: jint,
    flip: jint,
) -> jbyteArray {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        let rgba_input = env.convert_byte_array(&rgba)?;
        let (w, h) = (dim(width), dim(height));
        let mut out = vec![0u8; w * h + (w / 2) * (h / 2) * 2];
        yuv::rgba_to_nv12(&rgba_input, w, h, &mut out, flip as u32);
        Ok(env.byte_array_from_slice(&out)?.into_raw())
    })
}

/// Zero-copy RGBA -> NV12, e.g. straight into a MediaCodec input buffer, mirrored by the
/// `yuv::FLIP_X` / `FLIP_Y` bits in `flip`. Returns the bytes written, or 0 if a buffer is not
/// direct or too small.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_rgbaToNv12Into(
    mut env: JNIEnv,
    _class: JClass,
    rgba: JByteBuffer,
    width: jint,
    height: jint,
    out: JByteBuffer,
    flip: jint,
) -> jint {
    guarded(&mut env, 0, |env| {
        let (Some(rgba), Some(out)) = (direct_ref(env, &rgba), direct_slice(env, &out)) else {
            return Ok(0);
        };
        let (w, h) = (dim(width), dim(height));
        Ok(if yuv::rgba_to_nv12(rgba, w, h, out, flip as u32) {
            (w * h + (w / 2) * (h / 2) * 2) as jint
        } else {
            0
        })
    })
}

/// Converts camera YUV_420_888 planes (direct ByteBuffers, as CameraX hands them out) into a
/// direct RGBA ByteBuffer without any intermediate copies. Returns false if a buffer is not direct
/// or is too small for the given dimensions.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_yuvToRgbaInto(
    mut env: JNIEnv,
    _class: JClass,
    y: JByteBuffer,
    u: JByteBuffer,
    v: JByteBuffer,
    width: jint,
    height: jint,
    y_row_stride: jint,
    uv_row_stride: jint,
    uv_pixel_stride: jint,
    out: JByteBuffer,
) -> jboolean {
    guarded(&mut env, JNI_FALSE, |env| {
        // U and V are read-only views: on interleaved (NV21/NV12-style) cameras they overlap in
        // memory, so they must never be `&mut`.
        let (Some(y), Some(u), Some(v), Some(out)) = (
            direct_ref(env, &y),
            direct_ref(env, &u),
            direct_ref(env, &v),
            direct_slice(env, &out),
        ) else {
            return Ok(JNI_FALSE);
        };
        let strides = (dim(y_row_stride), dim(uv_row_stride), dim(uv_pixel_stride));
        Ok(if yuv::yuv_to_rgba(y, u, v, dim(width), dim(height), strides, yuv::Matrix::Bt601, out) {
            JNI_TRUE
        } else {
            JNI_FALSE
        })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyAccessory(
    mut env: JNIEnv,
    _class: JClass,
    pixels: JByteArray,
    width: jint,
    height: jint,
    accessory: jint,
) -> jbyteArray {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        let mut rgba = env.convert_byte_array(&pixels)?;
        accessories::apply_accessory(&mut rgba, dim(width), dim(height), accessory);
        Ok(env.byte_array_from_slice(&rgba)?.into_raw())
    })
}

/// Zero-copy variant of `applyAccessory` for a direct RGBA ByteBuffer, processed in place.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyAccessoryInPlace(
    mut env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
    width: jint,
    height: jint,
    accessory: jint,
) {
    guarded(&mut env, (), |env| {
        if let Some(rgba) = direct_slice(env, &buffer) {
            accessories::apply_accessory(rgba, dim(width), dim(height), accessory);
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyFrame(
    mut env: JNIEnv,
    _class: JClass,
    pixels: JByteArray,
    width: jint,
    height: jint,
    frame: jint,
    rotation: jint,
) -> jbyteArray {
    guarded(&mut env, std::ptr::null_mut(), |env| {
        let mut rgba = env.convert_byte_array(&pixels)?;
        frames::apply_frame(&mut rgba, dim(width), dim(height), frame, rotation);
        Ok(env.byte_array_from_slice(&rgba)?.into_raw())
    })
}

/// Zero-copy variant of `applyFrame` for a direct RGBA ByteBuffer, drawn in place.
#[no_mangle]
pub extern "system" fn Java_com_codenour_glyphcam_native_RustBridge_applyFrameInPlace(
    mut env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
    width: jint,
    height: jint,
    frame: jint,
    rotation: jint,
) {
    guarded(&mut env, (), |env| {
        if let Some(rgba) = direct_slice(env, &buffer) {
            frames::apply_frame(rgba, dim(width), dim(height), frame, rotation);
        }
        Ok(())
    })
}

fn dim(v: jint) -> usize {
    v.max(0) as usize
}

/// Address and length of a direct ByteBuffer, or None if it is not direct.
fn direct_parts(env: &JNIEnv, buffer: &JByteBuffer) -> Option<(*mut u8, usize)> {
    let ptr = env.get_direct_buffer_address(buffer).ok()?;
    let len = env.get_direct_buffer_capacity(buffer).ok()?;
    if ptr.is_null() {
        return None;
    }
    Some((ptr, len))
}

/// Borrows a direct ByteBuffer's memory for writing. The Java side must not touch the buffer during the call.
fn direct_slice<'a>(env: &JNIEnv, buffer: &JByteBuffer) -> Option<&'a mut [u8]> {
    let (ptr, len) = direct_parts(env, buffer)?;
    // SAFETY: the JVM guarantees `len` bytes at `ptr` for a direct buffer, and the buffer stays
    // reachable (it is a call argument) for the duration of this native call.
    Some(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
}

/// Borrows a direct ByteBuffer's memory read-only. Use this for inputs: camera planes can overlap
/// each other, and overlapping shared borrows are fine where overlapping `&mut` would be UB.
fn direct_ref<'a>(env: &JNIEnv, buffer: &JByteBuffer) -> Option<&'a [u8]> {
    let (ptr, len) = direct_parts(env, buffer)?;
    // SAFETY: as for `direct_slice`; nothing writes through the buffer during the call.
    Some(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) })
}
