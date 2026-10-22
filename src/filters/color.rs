use crate::filters::stock::curve;

#[derive(Clone, Copy)]
pub struct ColorParams {
    pub brightness: f32,  // -1.0 to 1.0
    pub contrast: f32,    // 0.0 to 2.0 (1.0 = neutral)
    pub saturation: f32,  // 0.0 to 2.0 (1.0 = neutral)
    pub temperature: f32, // -1.0 to 1.0 (negative = cool, positive = warm)
    pub tint: f32,        // -1.0 to 1.0 (negative = magenta, positive = green)
}

/// Per-pixel color adjustment on normalized RGB. Output is not clamped.
#[inline]
pub fn adjust_rgb(rgb: [f32; 3], params: &ColorParams) -> [f32; 3] {
    let [mut r, mut g, mut b] = rgb;

    // Temperature (yellow-blue)
    if params.temperature > 0.0 {
        // Warm: boost red, reduce blue
        r += params.temperature * 0.1;
        b -= params.temperature * 0.1;
    } else {
        // Cool: boost blue, reduce red
        b -= params.temperature * 0.1;
        r += params.temperature * 0.1;
    }

    // Tint (green-magenta)
    if params.tint > 0.0 {
        // Green: boost green
        g += params.tint * 0.1;
    } else {
        // Magenta: boost red and blue
        r -= params.tint * 0.05;
        b -= params.tint * 0.05;
    }

    // Brightness
    r += params.brightness;
    g += params.brightness;
    b += params.brightness;

    // Contrast around the 0.5 midpoint. Raising it bends into black and white along an S-curve
    // instead of clipping, so no shadow or highlight levels are lost; lowering it stays linear,
    // which lifts black and dims white like a faded print.
    if params.contrast > 1.0 {
        r = curve(r, params.contrast, 0.5);
        g = curve(g, params.contrast, 0.5);
        b = curve(b, params.contrast, 0.5);
    } else {
        r = (r - 0.5) * params.contrast + 0.5;
        g = (g - 0.5) * params.contrast + 0.5;
        b = (b - 0.5) * params.contrast + 0.5;
    }

    // Saturation
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    r = luma + (r - luma) * params.saturation;
    g = luma + (g - luma) * params.saturation;
    b = luma + (b - luma) * params.saturation;

    [r, g, b]
}
