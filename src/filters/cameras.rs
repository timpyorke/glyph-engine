//! Camera body looks, matching the bodies the app ships. IDs must match `FilterType.kt`.
//! Video 100..=108, digital 200 and 202..=204, Fujifilm X-T body 210, instant bodies 230..=234, 120 bodies 240..=244,
//! 135 bodies 250..=261.
//! The "Original" bodies map to 0 (passthrough).

use crate::filters::color::ColorParams;
use crate::filters::films::film_look;
use crate::filters::look::{Layout, Look, BASE};
use crate::filters::presets::VINTAGE_LUT;
use crate::filters::stock::{hue, hues, Stock, NEUTRAL};

const fn color(brightness: f32, contrast: f32, saturation: f32, temperature: f32, tint: f32) -> ColorParams {
    ColorParams { brightness, contrast, saturation, temperature, tint }
}

/// Colour response of film [id] in `films.rs`, for a body that is defined by the stock it shoots.
fn stock_of(film: i32) -> Option<Stock> {
    film_look(film).and_then(|f| f.stock)
}

/// 120 bodies: a bigger negative, so a loaded film renders with finer grain (see `filters::look_for`).
pub fn is_medium_format(id: i32) -> bool {
    (240..=244).contains(&id)
}

pub fn camera_look(id: i32) -> Option<Look> {
    let look = match id {
        // ---- Video ----
        // Super 8 — Canon 518 on Ektachrome 100D: warm, soft, coarse grain
        100 => Look {
            color: color(0.0, 1.0, 0.7, 0.4, 0.0),
            lut: Some(&*VINTAGE_LUT),
            soft: 0.3,
            fade: 0.1,
            vignette: 0.5,
            grain: 0.6,
            grain_size: 2.0,
            ..BASE
        },
        // Bolex H16 — spring-wound 16mm: sharper than Super 8, natural colour, medium grain
        101 => Look {
            color: color(0.0, 1.05, 0.95, 0.1, 0.0),
            soft: 0.1,
            rolloff: 0.25,
            fade: 0.05,
            vignette: 0.3,
            grain: 0.35,
            grain_size: 1.5,
            ..BASE
        },
        // 16mm on CineStill-style 800T — teal shadows, warm highlights, red halation round lights
        102 => Look {
            stock: stock_of(40),
            glow: 0.45,
            glow_tint: [1.6, 0.45, 0.3],
            vignette: 0.3,
            grain: 0.35,
            grain_size: 1.6,
            ..BASE
        },
        // Kodachrome 40 in a compact Super 8 — dense slide contrast, deep reds, warm yellows, olive
        // greens, deep cyan-leaning skies
        103 => Look {
            stock: Some(Stock {
                balance: [1.03, 1.0, 0.94],
                contrast: [1.25, 1.22, 1.2],
                saturation: 1.2,
                highlight_sat: 0.85,
                shadow_sat: 0.9,
                hues: hues([
                    hue(20.0, 35.0, 2.0, 1.15, -0.02),
                    hue(45.0, 30.0, 2.0, 1.05, 0.0),
                    hue(95.0, 35.0, -3.0, 1.1, 0.0),
                    hue(135.0, 40.0, -6.0, 0.95, 0.0),
                    hue(250.0, 45.0, -4.0, 1.05, -0.01),
                ]),
                black: [0.01, 0.01, 0.0],
                white: [1.0, 0.99, 0.95],
                ..NEUTRAL
            }),
            soft: 0.2,
            vignette: 0.45,
            grain: 0.45,
            grain_size: 1.8,
            ..BASE
        },
        // Anamorphic cinema — 2.39:1 letterbox, blue-tinted flare from highlights, filmic contrast
        104 => Look {
            color: color(0.0, 1.12, 0.95, -0.05, 0.0),
            rolloff: 0.3,
            shadow_tint: [-0.01, 0.005, 0.02],
            highlight_tint: [0.02, 0.01, -0.01],
            glow: 0.3,
            glow_tint: [0.4, 0.8, 1.6],
            vignette: 0.2,
            grain: 0.12,
            layout: Layout::Letterbox { ratio: 2.39 },
            ..BASE
        },
        // Hi8 camcorder — soft, slightly washed colour, blooming highlights, video noise
        105 => Look {
            color: color(0.03, 0.95, 0.85, 0.05, 0.0),
            soft: 0.3,
            glow: 0.25,
            chroma_shift: 1.0,
            fade: 0.06,
            rolloff: -0.05,
            grain: 0.2,
            chroma_grain: 0.3,
            ..BASE
        },
        // VHS-C — low sharpness, colour bleeding off edges, noisy shadows
        106 => Look {
            color: color(0.02, 1.05, 1.15, 0.05, 0.03),
            soft: 0.4,
            chroma_shift: 2.5,
            fade: 0.08,
            shadow_tint: [0.0, 0.01, 0.02],
            grain: 0.25,
            grain_size: 1.4,
            chroma_grain: 0.5,
            ..BASE
        },
        // MiniDV — crisp early-digital video: punchy contrast, clipped highlights, clean
        107 => Look {
            color: color(0.02, 1.2, 1.1, -0.03, 0.0),
            rolloff: -0.15,
            chroma_shift: 0.5,
            grain: 0.08,
            chroma_grain: 0.15,
            ..BASE
        },
        // NightShot — infrared night mode: green monochrome, bright, very noisy
        108 => Look {
            mix: Some([[0.1, 0.25, 0.05], [0.35, 0.7, 0.15], [0.1, 0.25, 0.05]]),
            color: color(0.12, 1.15, 1.0, 0.0, 0.0),
            rolloff: -0.1,
            soft: 0.15,
            glow: 0.3,
            vignette: 0.55,
            grain: 0.5,
            grain_size: 1.3,
            ..BASE
        },

        // ---- Photo ----
        // GR Digital — digital street compact: hard contrast, deep clean blacks, crisp noise-free
        // CCD rendering (no grain), slightly punchy colour
        200 => Look {
            color: color(0.03, 1.3, 1.08, 0.0, 0.0),
            fade: 0.0,
            vignette: 0.22,
            grain: 0.0,
            ..BASE
        },

        // Cyber-shot-style early-2000s CCD compact — blown highlights, cool vivid skies, hard flash,
        // over-sharpened edges (chroma shift), clean
        202 => Look {
            color: color(0.04, 1.15, 1.15, -0.15, 0.0),
            rolloff: -0.25,
            highlight_tint: [-0.01, 0.0, 0.015],
            chroma_shift: 0.5,
            vignette: 0.12,
            chroma_grain: 0.1,
            ..BASE
        },
        // IXUS-style CCD compact — vivid reds and blues, warm skin, cleaner than the CS
        203 => Look {
            color: color(0.03, 1.1, 1.2, 0.12, 0.0),
            rolloff: -0.15,
            highlight_tint: [0.015, 0.005, 0.0],
            vignette: 0.1,
            chroma_grain: 0.06,
            ..BASE
        },
        // 720nm infrared B&W — foliage glows white, blue sky goes near black
        204 => Look {
            mix: Some([[0.45, 0.95, -0.4]; 3]),
            color: color(0.02, 1.2, 1.0, 0.0, 0.0),
            rolloff: 0.2,
            glow: 0.25,
            vignette: 0.2,
            grain: 0.15,
            ..BASE
        },

        // ---- Fujifilm X-T ----
        // X-T body — clean mirrorless optics with a touch of corner falloff. Colour and grain come
        // from the Fujifilm film simulation loaded into it (`films.rs`, IDs 10..=20).
        210 => Look {
            vignette: 0.08,
            ..BASE
        },

        // ---- Instax ---- (the print border comes from the Instax frames, see `frames.rs`)
        // Instax Mini — bright, saturated, pink skin, slight direct-flash punch
        230 => Look {
            color: color(0.04, 1.0, 1.2, 0.1, -0.1),
            rolloff: 0.35,
            fade: 0.05,
            highlight_tint: [0.03, 0.005, 0.01],
            vignette: 0.2,
            grain: 0.12,
            ..BASE
        },
        // Instax Wide — wider print, softer contrast, gentle warmth
        231 => Look {
            color: color(0.03, 0.92, 1.05, 0.15, -0.04),
            soft: 0.08,
            rolloff: 0.45,
            fade: 0.07,
            vignette: 0.18,
            grain: 0.12,
            ..BASE
        },
        // Instax Square — pale, gentle colour, cool shadows
        232 => Look {
            color: color(0.03, 0.88, 0.95, 0.0, 0.0),
            rolloff: 0.4,
            fade: 0.09,
            shadow_tint: [-0.01, 0.01, 0.02],
            vignette: 0.15,
            grain: 0.1,
            ..BASE
        },
        // SX-70 — folding instant SLR: soft green-yellow cast, blooming highlights, low contrast
        233 => Look {
            color: color(0.03, 0.85, 0.85, 0.1, 0.06),
            rolloff: 0.45,
            fade: 0.1,
            shadow_tint: [0.0, 0.015, 0.0],
            highlight_tint: [0.02, 0.02, -0.02],
            soft: 0.12,
            glow: 0.2,
            vignette: 0.3,
            grain: 0.1,
            ..BASE
        },
        // 600 instant — cool blue cast, lifted blacks, muted instant-film colour
        234 => Look {
            color: color(0.02, 0.92, 0.85, -0.2, 0.0),
            rolloff: 0.35,
            fade: 0.12,
            shadow_tint: [-0.01, 0.0, 0.025],
            soft: 0.06,
            vignette: 0.25,
            grain: 0.1,
            ..BASE
        },

        // ---- 135 ---- Bodies without a colour are film bodies: colour and grain come from the
        // film loaded into them (`films.rs`), so they only carry optics.
        // Contax T2 — 38mm Sonnar: crisp, light falloff (default film Portra 400)
        250 => Look {
            vignette: 0.12,
            ..BASE
        },
        // Olympus mju-II — direct flash: hot foreground, dark falloff, slightly green cast
        251 => Look {
            color: color(0.04, 1.2, 1.05, -0.05, 0.03),
            rolloff: -0.1,
            shadow_tint: [-0.005, 0.01, 0.0],
            vignette: 0.45,
            grain: 0.25,
            ..BASE
        },
        // Ricoh GR1 — 28mm: very sharp, hard contrast, deep blacks, fine but visible grain
        252 => Look {
            color: color(0.0, 1.3, 1.0, 0.0, 0.0),
            vignette: 0.25,
            grain: 0.18,
            grain_size: 0.9,
            ..BASE
        },
        // Yashica T4 — 35mm Tessar: clean, slight softness (default film Fuji C200)
        253 => Look {
            soft: 0.02,
            vignette: 0.15,
            ..BASE
        },
        // Lomo LC-A — cross-processed slide: crushed contrast, cyan shadows, yellow highlights,
        // very dark corners
        254 => Look {
            color: color(0.02, 1.35, 1.35, 0.1, 0.0),
            rolloff: -0.05,
            shadow_tint: [-0.02, 0.02, 0.03],
            highlight_tint: [0.03, 0.03, -0.04],
            soft: 0.06,
            vignette: 0.75,
            grain: 0.22,
            ..BASE
        },
        // Canon Sure Shot — autofocus compact: neutral, mild softness (default film Ultramax 400)
        255 => Look {
            soft: 0.05,
            vignette: 0.2,
            ..BASE
        },
        // Leica M6 — 35mm Summicron: smooth, low falloff (default film Portra 400)
        256 => Look {
            vignette: 0.1,
            ..BASE
        },
        // Pentax K1000 — 50mm: honest SLR optics (default film ColorPlus 200)
        257 => Look {
            soft: 0.03,
            vignette: 0.18,
            ..BASE
        },
        // Aerochrome — false-colour infrared through a yellow filter: foliage turns red-magenta,
        // skies go deep blue
        258 => Look {
            mix: Some([[0.1, 0.9, 0.0], [0.8, 0.1, 0.1], [0.0, 0.2, 0.8]]),
            color: color(0.0, 1.15, 1.25, 0.0, 0.0),
            rolloff: 0.15,
            vignette: 0.2,
            grain: 0.25,
            ..BASE
        },
        // Kodak FunSaver — disposable flash, ISO 800: direct flash, warm, coarse grain
        259 => Look {
            color: color(0.05, 1.15, 1.1, 0.3, 0.0),
            rolloff: -0.15,
            soft: 0.1,
            vignette: 0.4,
            grain: 0.4,
            grain_size: 1.4,
            ..BASE
        },
        // Fujifilm QuickSnap — disposable flash, ISO 400: cool green cast, hard flash, high contrast
        260 => Look {
            color: color(0.0, 1.25, 1.05, -0.15, 0.06),
            shadow_tint: [-0.01, 0.015, 0.0],
            rolloff: -0.1,
            soft: 0.1,
            vignette: 0.4,
            grain: 0.3,
            grain_size: 1.3,
            ..BASE
        },
        // Big Mini — Konica 35mm compact: soft plastic lens, mild corner falloff (default film
        // Kodak Gold 200). Was 201 before the bodies were grouped by format.
        261 => Look {
            soft: 0.06,
            vignette: 0.25,
            ..BASE
        },

        // ---- 120 medium format ---- (a large negative: finer grain than the 135 bodies)
        // Pentax 67 — 105mm f/2.4 on saturated ISO 100 colour negative (the Ektar response): rich
        // colour, very fine grain, gentle falloff
        240 => Look {
            stock: stock_of(6),
            vignette: 0.15,
            grain: 0.05,
            grain_size: 0.8,
            ..BASE
        },
        // Hasselblad 500C/M — 80mm f/2.8 on pastel ISO 160 portrait negative (the Portra 160
        // response): soft roll-off, low contrast, warm highlights
        241 => Look {
            stock: stock_of(7),
            vignette: 0.12,
            grain: 0.06,
            grain_size: 0.8,
            ..BASE
        },
        // Rolleiflex TLR — 80mm f/2.8 on classic ISO 400 B&W: medium contrast, crisp grain,
        // shot through a yellow filter that darkens skies
        242 => Look {
            stock: Some(Stock { mono: Some([0.36, 0.52, 0.12]), contrast: [1.12; 3], black: [0.03; 3], ..NEUTRAL }),
            soft: 0.04,
            vignette: 0.2,
            grain: 0.18,
            grain_size: 1.1,
            ..BASE
        },
        // Holga — plastic meniscus lens on 120: heavy vignette, soft edges, muted colour
        243 => Look {
            color: color(0.02, 1.05, 0.85, 0.08, 0.0),
            soft: 0.25,
            chroma_shift: 0.6,
            fade: 0.06,
            vignette: 0.85,
            grain: 0.2,
            grain_size: 1.2,
            ..BASE
        },
        // Diana — plastic toy camera on 120: dreamy glow, saturated colour, dark corners
        244 => Look {
            color: color(0.03, 1.0, 1.3, 0.05, 0.05),
            soft: 0.2,
            glow: 0.3,
            fade: 0.05,
            vignette: 0.5,
            grain: 0.2,
            grain_size: 1.2,
            ..BASE
        },
        _ => return None,
    };
    Some(look)
}
