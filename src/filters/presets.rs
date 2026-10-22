use crate::filters::color::ColorParams;
use crate::filters::lut3d::Lut3d;
use once_cell::sync::Lazy;

const LUT_SIZE: usize = 17;
const VINTAGE_CUBE: &str = include_str!("lut_data/vintage.cube");
const FUJI_CUBE: &str = include_str!("lut_data/fuji.cube");
const KODAK_CUBE: &str = include_str!("lut_data/kodak.cube");
const MONO_CUBE: &str = include_str!("lut_data/mono.cube");

pub struct FilterPreset {
    pub color_params: ColorParams,
    pub lut3d: &'static Lut3d,
    pub grain_intensity: f32,
}

fn load_or_identity(cube_data: &str) -> Lut3d {
    Lut3d::from_cube_str(cube_data).unwrap_or_else(|| Lut3d::identity(LUT_SIZE))
}

pub(crate) static VINTAGE_LUT: Lazy<Lut3d> = Lazy::new(|| load_or_identity(VINTAGE_CUBE));
pub(crate) static FUJI_LUT: Lazy<Lut3d> = Lazy::new(|| load_or_identity(FUJI_CUBE));
pub(crate) static KODAK_LUT: Lazy<Lut3d> = Lazy::new(|| load_or_identity(KODAK_CUBE));
pub(crate) static MONO_LUT: Lazy<Lut3d> = Lazy::new(|| load_or_identity(MONO_CUBE));

/// VINTAGE(1) — warm sepia tones, faded look, moderate grain
pub fn vintage() -> FilterPreset {
    FilterPreset {
        color_params: ColorParams {
            brightness: 0.02,
            contrast: 0.9,
            saturation: 0.75,
            temperature: 0.3,
            tint: 0.0,
        },
        lut3d: &*VINTAGE_LUT,
        grain_intensity: 0.4,
    }
}

/// FUJI(2) — muted greens, soft contrast, subtle grain
pub fn fuji() -> FilterPreset {
    FilterPreset {
        color_params: ColorParams {
            brightness: 0.0,
            contrast: 1.05,
            saturation: 0.85,
            temperature: -0.1,
            tint: 0.0,
        },
        lut3d: &*FUJI_LUT,
        grain_intensity: 0.2,
    }
}

/// KODAK(3) — warm, rich contrast, visible grain
pub fn kodak() -> FilterPreset {
    FilterPreset {
        color_params: ColorParams {
            brightness: 0.01,
            contrast: 1.15,
            saturation: 1.1,
            temperature: 0.2,
            tint: 0.0,
        },
        lut3d: &*KODAK_LUT,
        grain_intensity: 0.35,
    }
}

/// MONO(4) — black and white, high contrast, strong grain
pub fn mono() -> FilterPreset {
    FilterPreset {
        color_params: ColorParams {
            brightness: 0.0,
            contrast: 1.2,
            saturation: 0.0,
            temperature: 0.0,
            tint: 0.0,
        },
        lut3d: &*MONO_LUT,
        grain_intensity: 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_cubes_parse() {
        // load_or_identity hides a broken file behind an identity LUT, so check each one here.
        for (name, cube) in [("vintage", VINTAGE_CUBE), ("fuji", FUJI_CUBE), ("kodak", KODAK_CUBE), ("mono", MONO_CUBE)] {
            assert!(Lut3d::from_cube_str(cube).is_some(), "{}.cube does not parse", name);
        }
    }
}
