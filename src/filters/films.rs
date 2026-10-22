//! Film stocks loadable into camera bodies. Which bodies accept which films is decided by the app
//! (`FilmType.kt` groups, per-camera lists in `FiltersScreen.kt`); the engine can combine any pair.
//! IDs must match `FilmType.kt`: colour-negative stocks 1..=9, Fujifilm simulations 10..=20,
//! Ricoh GR modes 30.., cine, slide and B&W stocks 40..=44.
//!
//! A film owns the colour response and the grain: its `Stock` (balance, characteristic curve, hue
//! response, print black and white, see `stock.rs`) plus grain. The body keeps the optics: softness,
//! glow, chroma shift, vignette and layout. A body + film pair is addressed with one filter ID,
//! `film * 1000 + body` (body 0 is a bare film on the untouched image), see `filters::look_for`.
//!
//! The stocks are tuned against how each one is known to render: datasheet contrast and
//! sensitivity, plus the hue habits of scans and prints (warm Kodak yellows and olive greens,
//! cool Fuji teal greens and magenta reds, Portra's muted pastels, slide film's dense blacks).

use crate::filters::look::{Look, BASE};
use crate::filters::stock::{hue, hues, Stock, NEUTRAL};

/// Multiplier that packs a film ID above a body ID in a filter ID.
pub const FILM_ID_STRIDE: i32 = 1000;

const fn film(stock: Stock, grain: f32, grain_size: f32) -> Look {
    Look { stock: Some(stock), grain, grain_size, ..BASE }
}

/// Look of film [id] on its own, or None for an unknown film.
pub fn film_look(id: i32) -> Option<Look> {
    let look = match id {
        // Kodak Gold 200 — golden warmth: rich yellows and oranges, olive greens, creamy highlights
        1 => film(
            Stock {
                balance: [1.06, 1.0, 0.88],
                exposure: 0.1,
                contrast: [1.08, 1.06, 1.0],
                saturation: 1.08,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 35.0, 4.0, 1.1, 0.0),
                    hue(45.0, 30.0, 2.0, 1.05, 0.01),
                    hue(80.0, 45.0, -4.0, 1.15, 0.0),
                    hue(135.0, 40.0, -10.0, 0.9, 0.0),
                    hue(255.0, 45.0, -3.0, 0.92, 0.01),
                ]),
                black: [0.04, 0.03, 0.025],
                white: [1.0, 0.985, 0.95],
                ..NEUTRAL
            },
            0.22,
            1.2,
        ),
        // Kodak ColorPlus 200 — warmer, flatter budget Gold: orange cast, muddy greens, lifted blacks
        2 => film(
            Stock {
                balance: [1.07, 1.0, 0.88],
                exposure: 0.05,
                contrast: [1.05, 1.02, 0.95],
                saturation: 0.98,
                highlight_sat: 0.75,
                shadow_sat: 0.8,
                hues: hues([
                    hue(20.0, 35.0, 5.0, 1.0, 0.0),
                    hue(60.0, 45.0, -2.0, 1.1, 0.0),
                    hue(135.0, 45.0, -12.0, 0.82, -0.01),
                    hue(255.0, 45.0, -4.0, 0.88, 0.01),
                ]),
                black: [0.06, 0.05, 0.04],
                white: [0.99, 0.97, 0.92],
                ..NEUTRAL
            },
            0.3,
            1.4,
        ),
        // Kodak UltraMax 400 — punchy consumer stock: warm, saturated reds and yellows, firm contrast
        3 => film(
            Stock {
                balance: [1.04, 1.0, 0.92],
                contrast: [1.15, 1.13, 1.1],
                saturation: 1.15,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 35.0, 3.0, 1.1, 0.0),
                    hue(90.0, 40.0, -3.0, 1.1, 0.0),
                    hue(135.0, 40.0, -8.0, 0.95, 0.0),
                    hue(265.0, 45.0, -4.0, 1.05, 0.0),
                ]),
                black: [0.025, 0.02, 0.02],
                white: [1.0, 0.99, 0.95],
                ..NEUTRAL
            },
            0.32,
            1.5,
        ),
        // Kodak Portra 400 — rated over box speed: airy, muted pastels, smooth warm skin, olive
        // greens, cyan-leaning skies, long highlight roll-off
        4 => film(
            Stock {
                balance: [1.03, 1.0, 0.95],
                exposure: 0.2,
                contrast: [0.95, 0.95, 0.92],
                saturation: 0.95,
                highlight_sat: 0.7,
                shadow_sat: 0.8,
                hues: hues([
                    hue(20.0, 30.0, 3.0, 1.0, 0.0),
                    hue(45.0, 35.0, 2.0, 0.92, 0.01),
                    hue(135.0, 45.0, -10.0, 0.8, 0.0),
                    hue(155.0, 40.0, -6.0, 0.85, 0.0),
                    hue(255.0, 45.0, -8.0, 0.8, 0.02),
                    hue(275.0, 40.0, -5.0, 0.85, 0.0),
                ]),
                black: [0.05, 0.05, 0.05],
                white: [1.0, 0.985, 0.96],
                ..NEUTRAL
            },
            0.2,
            1.2,
        ),
        // Fujicolor C200 — cool consumer Fuji: teal greens, pinkish reds, green-tinged shadows
        5 => film(
            Stock {
                balance: [0.97, 1.0, 1.03],
                contrast: [1.08, 1.04, 1.08],
                saturation: 1.05,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 30.0, -5.0, 1.0, 0.0),
                    hue(45.0, 30.0, -3.0, 0.95, 0.0),
                    hue(135.0, 40.0, 8.0, 1.1, 0.0),
                    hue(150.0, 40.0, 6.0, 1.1, 0.0),
                    hue(250.0, 40.0, -6.0, 1.05, 0.0),
                ]),
                black: [0.02, 0.035, 0.035],
                white: [0.98, 1.0, 0.995],
                ..NEUTRAL
            },
            0.24,
            1.3,
        ),
        // Kodak Ektar 100 — the most saturated negative: intense reds, deep blues, ruddy skin, fine grain
        6 => film(
            Stock {
                contrast: [1.18, 1.15, 1.15],
                saturation: 1.25,
                highlight_sat: 0.85,
                shadow_sat: 0.9,
                hues: hues([
                    hue(20.0, 35.0, 2.0, 1.15, -0.01),
                    hue(45.0, 30.0, -4.0, 1.1, 0.0),
                    hue(135.0, 40.0, -3.0, 1.05, 0.0),
                    hue(250.0, 40.0, 0.0, 1.1, -0.02),
                    hue(265.0, 45.0, 3.0, 1.15, -0.02),
                ]),
                black: [0.015, 0.015, 0.02],
                white: [1.0, 1.0, 0.99],
                ..NEUTRAL
            },
            0.1,
            0.9,
        ),
        // Kodak Portra 160 — the flattest, palest Portra: cooler than 400, very fine grain
        7 => film(
            Stock {
                balance: [1.01, 1.0, 0.98],
                exposure: 0.2,
                contrast: [0.9, 0.9, 0.88],
                saturation: 0.9,
                highlight_sat: 0.7,
                shadow_sat: 0.8,
                hues: hues([
                    hue(45.0, 35.0, 1.0, 0.92, 0.01),
                    hue(135.0, 45.0, -8.0, 0.8, 0.0),
                    hue(155.0, 40.0, -5.0, 0.85, 0.0),
                    hue(255.0, 45.0, -6.0, 0.85, 0.02),
                    hue(275.0, 40.0, -4.0, 0.85, 0.0),
                ]),
                black: [0.06, 0.06, 0.065],
                white: [0.99, 0.98, 0.96],
                ..NEUTRAL
            },
            0.12,
            1.0,
        ),
        // Kodak Portra 800 — warmer, punchier Portra with coarser grain for low light
        8 => film(
            Stock {
                balance: [1.05, 1.0, 0.92],
                exposure: 0.1,
                contrast: [1.02, 1.0, 0.95],
                saturation: 1.0,
                highlight_sat: 0.72,
                shadow_sat: 0.8,
                hues: hues([
                    hue(20.0, 30.0, 3.0, 1.05, 0.0),
                    hue(45.0, 35.0, 3.0, 0.98, 0.01),
                    hue(135.0, 45.0, -10.0, 0.85, 0.0),
                    hue(255.0, 45.0, -8.0, 0.85, 0.02),
                ]),
                black: [0.05, 0.045, 0.04],
                white: [1.0, 0.98, 0.94],
                ..NEUTRAL
            },
            0.3,
            1.4,
        ),
        // Fujicolor Superia 400 — crisp consumer Fuji: vivid teal greens, magenta reds, green shadows
        9 => film(
            Stock {
                balance: [0.98, 1.01, 1.0],
                contrast: [1.1, 1.05, 1.1],
                saturation: 1.1,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 30.0, -6.0, 1.05, 0.0),
                    hue(45.0, 30.0, -2.0, 0.95, 0.0),
                    hue(135.0, 40.0, 10.0, 1.1, 0.0),
                    hue(150.0, 40.0, 8.0, 1.1, 0.0),
                    hue(250.0, 40.0, -8.0, 1.05, 0.0),
                ]),
                black: [0.02, 0.04, 0.035],
                white: [0.99, 1.0, 0.98],
                ..NEUTRAL
            },
            0.28,
            1.4,
        ),

        // ---- Fujifilm film simulations (X-T only) ---- digital: clean blacks and whites
        // Provia — the standard: natural colour, moderate contrast
        10 => film(
            Stock {
                contrast: [1.05; 3],
                saturation: 1.05,
                highlight_sat: 0.9,
                shadow_sat: 0.95,
                hues: hues([hue(20.0, 30.0, 0.0, 1.05, 0.0), hue(260.0, 40.0, -2.0, 1.05, 0.0)]),
                ..NEUTRAL
            },
            0.08,
            1.0,
        ),
        // Velvia — vivid slide: dense contrast, deep blue skies, lush greens, magenta-leaning reds
        11 => film(
            Stock {
                balance: [1.0, 1.0, 1.02],
                contrast: [1.25; 3],
                saturation: 1.35,
                highlight_sat: 0.9,
                shadow_sat: 0.95,
                hues: hues([
                    hue(15.0, 35.0, -4.0, 1.1, -0.01),
                    hue(95.0, 35.0, -2.0, 1.05, 0.0),
                    hue(130.0, 40.0, 2.0, 1.1, -0.02),
                    hue(145.0, 45.0, 4.0, 1.15, -0.02),
                    hue(260.0, 45.0, 4.0, 1.15, -0.015),
                    hue(345.0, 35.0, 0.0, 1.15, 0.0),
                ]),
                black: [0.0, 0.0, 0.005],
                ..NEUTRAL
            },
            0.1,
            1.0,
        ),
        // Astia — soft contrast and gentle skin, but rich blues and greens
        12 => film(
            Stock {
                contrast: [0.92; 3],
                saturation: 1.05,
                highlight_sat: 0.85,
                hues: hues([
                    hue(45.0, 35.0, 0.0, 0.9, 0.015),
                    hue(145.0, 45.0, 0.0, 1.08, 0.0),
                    hue(255.0, 45.0, 0.0, 1.1, 0.0),
                ]),
                black: [0.02; 3],
                ..NEUTRAL
            },
            0.08,
            1.0,
        ),
        // Classic Chrome — documentary slide: muted, firm shadows, cyan-grey skies, olive greens,
        // brick reds
        13 => film(
            Stock {
                contrast: [1.17, 1.15, 1.12],
                pivot: 0.5,
                saturation: 0.8,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 35.0, 4.0, 0.9, -0.01),
                    hue(45.0, 30.0, 0.0, 0.95, 0.0),
                    hue(95.0, 35.0, -3.0, 0.85, 0.0),
                    hue(145.0, 45.0, -6.0, 0.75, 0.0),
                    hue(255.0, 50.0, -10.0, 0.7, -0.02),
                ]),
                black: [0.02, 0.025, 0.035],
                ..NEUTRAL
            },
            0.2,
            1.0,
        ),
        // Pro Neg Hi — studio portrait negative with extra punch
        14 => film(
            Stock {
                contrast: [1.12; 3],
                saturation: 1.05,
                highlight_sat: 0.85,
                hues: hues([hue(45.0, 35.0, 0.0, 0.95, 0.01), hue(255.0, 45.0, 0.0, 1.05, 0.0)]),
                ..NEUTRAL
            },
            0.1,
            1.0,
        ),
        // Classic Neg — consumer-negative look: hard contrast, teal greens and shadows, magenta reds,
        // cyan skies, yellowed highlights
        15 => film(
            Stock {
                balance: [1.0, 1.0, 0.97],
                contrast: [1.22, 1.18, 1.08],
                saturation: 0.92,
                highlight_sat: 0.75,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 30.0, -6.0, 1.1, -0.01),
                    hue(45.0, 30.0, -4.0, 0.95, 0.0),
                    hue(95.0, 40.0, -4.0, 0.95, 0.0),
                    hue(130.0, 45.0, 12.0, 0.95, 0.0),
                    hue(150.0, 40.0, 10.0, 0.95, 0.0),
                    hue(255.0, 45.0, -12.0, 0.85, 0.0),
                ]),
                black: [0.0, 0.02, 0.025],
                white: [1.0, 0.99, 0.95],
                ..NEUTRAL
            },
            0.25,
            1.0,
        ),
        // Nostalgic Neg — 70s colour print: amber highlights, deep reds, soft shoulder, teal skies
        16 => film(
            Stock {
                balance: [1.04, 1.0, 0.94],
                contrast: [1.05, 1.02, 0.94],
                saturation: 0.95,
                highlight_sat: 0.85,
                shadow_sat: 0.95,
                hues: hues([
                    hue(20.0, 35.0, 2.0, 1.1, -0.01),
                    hue(45.0, 30.0, 3.0, 1.0, 0.01),
                    hue(90.0, 40.0, -4.0, 1.05, 0.0),
                    hue(145.0, 40.0, -6.0, 0.85, 0.0),
                    hue(255.0, 45.0, -8.0, 0.85, 0.0),
                ]),
                black: [0.02; 3],
                white: [1.0, 0.975, 0.93],
                ..NEUTRAL
            },
            0.15,
            1.0,
        ),
        // Reala Ace — faithful colour with crisp contrast
        17 => film(
            Stock {
                contrast: [1.08; 3],
                saturation: 1.02,
                hues: hues([hue(45.0, 30.0, 0.0, 0.97, 0.0)]),
                ..NEUTRAL
            },
            0.06,
            1.0,
        ),
        // Acros — fine-grained B&W: deep blacks, rich midtone separation
        18 => film(Stock { mono: Some([0.3, 0.5, 0.2]), contrast: [1.22; 3], ..NEUTRAL }, 0.3, 1.0),
        // Eterna — cinema negative: flat, muted, teal-leaning greens, skin kept alive
        19 => film(
            Stock {
                contrast: [0.88, 0.88, 0.86],
                saturation: 0.65,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(45.0, 35.0, 0.0, 1.2, 0.0),
                    hue(145.0, 45.0, 6.0, 0.85, 0.0),
                    hue(250.0, 45.0, -6.0, 0.9, 0.0),
                ]),
                black: [0.04, 0.05, 0.055],
                white: [0.97, 0.97, 0.96],
                ..NEUTRAL
            },
            0.12,
            1.0,
        ),
        // Eterna Bleach Bypass — retained silver: high contrast, little colour, coarse grain
        20 => film(
            Stock { contrast: [1.3; 3], saturation: 0.45, highlight_sat: 0.6, black: [0.01; 3], ..NEUTRAL },
            0.4,
            1.5,
        ),

        // ---- Ricoh GR modes (GR Digital only) ----
        // High-contrast B&W — the GR's street black & white: luminance B&W, hard contrast, deep
        // blacks, clean and grain-free
        30 => film(
            Stock { mono: Some([0.2126, 0.7152, 0.0722]), contrast: [1.6; 3], exposure: 0.05, ..NEUTRAL },
            0.0,
            1.0,
        ),

        // ---- Cine, slide and B&W stocks ----
        // CineStill 800T — tungsten cine stock in daylight: blue cast, teal shadows, warm highlights,
        // strong reds. No halation here: glow belongs to the body's optics.
        40 => film(
            Stock {
                balance: [0.86, 0.95, 1.12],
                contrast: [1.08, 1.05, 0.95],
                saturation: 1.05,
                highlight_sat: 0.8,
                shadow_sat: 0.85,
                hues: hues([
                    hue(20.0, 30.0, 0.0, 1.1, 0.0),
                    hue(45.0, 35.0, 4.0, 1.05, 0.01),
                    hue(240.0, 45.0, -6.0, 1.0, 0.0),
                ]),
                black: [0.02, 0.035, 0.05],
                white: [0.98, 0.98, 1.0],
                ..NEUTRAL
            },
            0.3,
            1.4,
        ),
        // B&W stocks: panchromatic film is more blue-sensitive than the eye, so skies print lighter
        // than a plain luminance conversion.
        // Kodak Tri-X 400 — classic B&W: punchy contrast, gritty grain
        41 => film(
            Stock { mono: Some([0.27, 0.5, 0.23]), contrast: [1.35; 3], black: [0.01; 3], ..NEUTRAL },
            0.38,
            1.4,
        ),
        // Ilford HP5 Plus — wide latitude: softer contrast, open shadows
        42 => film(
            Stock {
                mono: Some([0.25, 0.5, 0.25]),
                exposure: 0.1,
                contrast: [1.08; 3],
                black: [0.035; 3],
                ..NEUTRAL
            },
            0.32,
            1.3,
        ),
        // Kodak T-Max 100 — modern tabular grain: smooth tones, very fine grain
        43 => film(Stock { mono: Some([0.28, 0.52, 0.2]), contrast: [1.18; 3], ..NEUTRAL }, 0.1, 0.9),
        // Kodak Ektachrome E100 — slide: clean, contrasty, neutral-cool, deep blues, dense blacks
        44 => film(
            Stock {
                balance: [0.98, 1.0, 1.03],
                contrast: [1.22, 1.22, 1.24],
                saturation: 1.15,
                highlight_sat: 0.9,
                shadow_sat: 0.9,
                hues: hues([
                    hue(20.0, 30.0, 0.0, 1.05, 0.0),
                    hue(45.0, 30.0, 0.0, 0.98, 0.0),
                    hue(145.0, 45.0, 2.0, 1.05, 0.0),
                    hue(260.0, 45.0, 2.0, 1.15, -0.02),
                ]),
                black: [0.0, 0.0, 0.01],
                ..NEUTRAL
            },
            0.08,
            0.9,
        ),
        _ => return None,
    };
    Some(look)
}

/// Loads [film] into [body]: the film's colour and grain replace the body's, the optics stay.
pub fn load_film(body: Look, film: Look) -> Look {
    Look {
        color: film.color,
        lut: film.lut,
        stock: film.stock,
        mix: film.mix,
        rolloff: film.rolloff,
        fade: film.fade,
        shadow_tint: film.shadow_tint,
        highlight_tint: film.highlight_tint,
        grain: film.grain,
        grain_size: film.grain_size,
        chroma_grain: film.chroma_grain,
        ..body
    }
}

/// Splits a filter ID into (film, body); film 0 means the ID is a plain body.
pub fn split_id(filter_type: i32) -> (i32, i32) {
    if filter_type >= FILM_ID_STRIDE {
        (filter_type / FILM_ID_STRIDE, filter_type % FILM_ID_STRIDE)
    } else {
        (0, filter_type)
    }
}
