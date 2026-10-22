// C ABI of glyph-engine (src/ffi.rs). Every call filters caller-owned buffers in place and
// returns false on a null pointer or a buffer too small for the given size.
#ifndef GLYPH_ENGINE_H
#define GLYPH_ENGINE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define GLYPH_FLIP_X 1u
#define GLYPH_FLIP_Y 2u

bool glyph_apply_filter(uint8_t *pixels, size_t len, int32_t w, int32_t h, int32_t filter);

bool glyph_apply_custom(uint8_t *pixels, size_t len, int32_t w, int32_t h,
                        float brightness, float contrast, float saturation,
                        float temperature, float tint, float grain);

bool glyph_apply_accessory(uint8_t *pixels, size_t len, int32_t w, int32_t h, int32_t accessory);

bool glyph_apply_frame(uint8_t *pixels, size_t len, int32_t w, int32_t h,
                       int32_t frame, int32_t rotation);

bool glyph_yuv_to_rgba(const uint8_t *y, size_t y_len,
                       const uint8_t *u, size_t u_len,
                       const uint8_t *v, size_t v_len,
                       int32_t w, int32_t h,
                       int32_t y_row_stride, int32_t uv_row_stride, int32_t uv_pixel_stride,
                       uint8_t *out, size_t out_len);

bool glyph_rgba_to_nv12(const uint8_t *rgba, size_t rgba_len, int32_t w, int32_t h,
                        uint8_t *out, size_t out_len, uint32_t flip);

#endif
