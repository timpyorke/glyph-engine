pub mod cameras;
pub mod color;
pub mod films;
pub mod look;
pub mod lut3d;
pub mod presets;
pub mod stock;

use look::{Look, BASE};

/// Added to a filter ID to switch the look's vignette off.
pub const NO_VIGNETTE: i32 = 1_000_000;

/// Splits the vignette flag off a filter ID: (ID without the flag, vignette enabled).
fn split_flags(filter_type: i32) -> (i32, bool) {
    if filter_type >= NO_VIGNETTE {
        (filter_type - NO_VIGNETTE, false)
    } else {
        (filter_type, true)
    }
}

/// Look for a filter ID, or None for passthrough.
/// 0=NONE, 1=VINTAGE, 2=FUJI, 3=KODAK, 4=MONO, 100+ = camera bodies (see `cameras.rs`).
/// IDs from 1000 up load film `id / 1000` (see `films.rs`) into body `id % 1000`.
/// Adding `NO_VIGNETTE` turns the vignette off, whatever the body or film.
pub fn look_for(filter_type: i32) -> Option<Look> {
    let (filter_type, vignette) = split_flags(filter_type);
    let mut look = look_without_flags(filter_type)?;
    if !vignette {
        look.vignette = 0.0;
    }
    Some(look)
}

/// Grain strength and cell size of a film on a 120 body, relative to 135.
const MEDIUM_FORMAT_GRAIN: f32 = 0.6;
const MEDIUM_FORMAT_GRAIN_SIZE: f32 = 0.75;

fn look_without_flags(filter_type: i32) -> Option<Look> {
    let (film, body) = films::split_id(filter_type);
    if film != 0 {
        let film = films::film_look(film)?;
        let medium_format = cameras::is_medium_format(body);
        let body = if body == 0 { BASE } else { body_look(body)? };
        let mut look = films::load_film(body, film);
        if medium_format {
            look.grain *= MEDIUM_FORMAT_GRAIN;
            look.grain_size *= MEDIUM_FORMAT_GRAIN_SIZE;
        }
        return Some(look);
    }
    body_look(body)
}

fn body_look(filter_type: i32) -> Option<Look> {
    if let Some(look) = cameras::camera_look(filter_type) {
        return Some(look);
    }
    let preset = match filter_type {
        1 => presets::vintage(),
        2 => presets::fuji(),
        3 => presets::kodak(),
        4 => presets::mono(),
        _ => return None,
    };
    Some(Look {
        color: preset.color_params,
        lut: Some(preset.lut3d),
        grain: preset.grain_intensity,
        ..BASE
    })
}

/// Cache key for a filter's baked grade. The grade doesn't depend on the vignette, and a film's grade
/// comes entirely from the film (`load_film` keeps only the body's optics), so every body loading a
/// film shares one LUT: the cache holds one per body and one per film, not one per pair.
fn grade_key(filter_type: i32) -> i32 {
    let (film, body) = films::split_id(split_flags(filter_type).0);
    if film != 0 {
        film * films::FILM_ID_STRIDE
    } else {
        body
    }
}

/// Apply a filter in place to tightly packed RGBA pixel data of `width` x `height`.
pub fn apply_filter(pixels: &mut [u8], width: usize, height: usize, filter_type: i32) {
    if let Some(look) = look_for(filter_type) {
        let grade = look::baked_grade(grade_key(filter_type), &look);
        look::apply_look(pixels, width, height, &look, grade, frame_seed(pixels));
    }
}

/// User-tuned custom filter: color adjust + grain, through the same pipeline as the presets.
/// The grade is baked per call since the params change; at 25^3 points that is well under 1 ms.
pub fn apply_custom(pixels: &mut [u8], width: usize, height: usize, color: color::ColorParams, grain: f32) {
    let look = Look { color, grain, ..BASE };
    let grade = look::bake(&look);
    look::apply_look(pixels, width, height, &look, &grade, frame_seed(pixels));
}

/// Frame-varying seed based on pixel content so grain dances between frames.
fn frame_seed(pixels: &[u8]) -> u32 {
    if pixels.len() >= 4 {
        u32::from_le_bytes([pixels[0], pixels[1], pixels[2], pixels[3]])
    } else {
        0x12345678
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_pixels(count: usize) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(count * 4);
        for i in 0..count {
            let v = (i % 256) as u8;
            pixels.push(v); // R
            pixels.push(v); // G
            pixels.push(v); // B
            pixels.push(255); // A
        }
        pixels
    }

    #[test]
    fn test_none_passthrough() {
        let original = make_test_pixels(100);
        let mut pixels = original.clone();
        apply_filter(&mut pixels, 10, 10, 0);
        assert_eq!(pixels, original, "NONE filter should not modify pixels");
    }

    #[test]
    fn test_vintage_modifies_pixels() {
        let original = make_test_pixels(100);
        let mut pixels = original.clone();
        apply_filter(&mut pixels, 10, 10, 1);
        assert_ne!(pixels, original, "VINTAGE filter should modify pixels");
    }

    #[test]
    fn test_fuji_modifies_pixels() {
        let original = make_test_pixels(100);
        let mut pixels = original.clone();
        apply_filter(&mut pixels, 10, 10, 2);
        assert_ne!(pixels, original, "FUJI filter should modify pixels");
    }

    #[test]
    fn test_kodak_modifies_pixels() {
        let original = make_test_pixels(100);
        let mut pixels = original.clone();
        apply_filter(&mut pixels, 10, 10, 3);
        assert_ne!(pixels, original, "KODAK filter should modify pixels");
    }

    #[test]
    fn test_mono_produces_greyscale() {
        let mut pixels = vec![
            200, 100, 50, 255, // colored pixel
            50, 150, 200, 255, // another colored pixel
        ];
        apply_filter(&mut pixels, 2, 1, 4);
        // After MONO (saturation=0), R≈G≈B for each pixel (grain adds noise, so check approximate)
        let diff1 = (pixels[0] as i32 - pixels[1] as i32).unsigned_abs();
        let diff2 = (pixels[1] as i32 - pixels[2] as i32).unsigned_abs();
        // Allow some tolerance due to grain noise
        assert!(diff1 <= 50, "MONO R-G diff {} too large", diff1);
        assert!(diff2 <= 50, "MONO G-B diff {} too large", diff2);
    }

    use crate::test_support::kotlin_ids;

    fn kotlin_filter_ids() -> Option<Vec<i32>> {
        kotlin_ids("FilterType.kt")
    }

    /// Camera bodies from `FilterType.kt`, or every body the engine defines when standalone.
    fn camera_ids() -> Vec<i32> {
        match kotlin_filter_ids() {
            Some(ids) => ids.into_iter().filter(|&id| id >= 100).collect(),
            None => (100..1000).filter(|&id| cameras::camera_look(id).is_some()).collect(),
        }
    }

    #[test]
    fn test_every_kotlin_filter_resolves() {
        let Some(ids) = kotlin_filter_ids() else { return };
        for id in ids {
            match id {
                -1 => {} // CUSTOM goes through apply_custom
                0 => assert!(look_for(0).is_none(), "NONE must be passthrough"),
                _ => assert!(look_for(id).is_some(), "FilterType id {} has no look in the engine", id),
            }
        }
    }

    #[test]
    fn test_every_kotlin_film_resolves_and_loads_into_every_body() {
        let Some(films) = kotlin_ids("FilmType.kt") else { return };
        let bodies = camera_ids();
        for film in films.into_iter().filter(|&f| f > 0) {
            assert!(films::film_look(film).is_some(), "FilmType id {} has no film in the engine", film);
            for body in bodies.iter().copied().chain([0, 1]) {
                let id = film * films::FILM_ID_STRIDE + body;
                let look = look_for(id).unwrap_or_else(|| panic!("film {} on body {} has no look", film, body));
                let original = gradient(32, 32);
                let mut pixels = original.clone();
                apply_filter(&mut pixels, 32, 32, id);
                assert_ne!(pixels, original, "film {} on body {} should modify pixels", film, body);
                // The body keeps its optics when a film is loaded.
                if let Some(base) = look_for(body) {
                    assert_eq!(look.vignette, base.vignette, "film {} must not change body {} vignette", film, body);
                    assert_eq!(look.soft, base.soft, "film {} must not change body {} softness", film, body);
                }
            }
        }
    }

    #[test]
    fn test_no_vignette_flag_removes_only_the_vignette() {
        for id in [261, 210, 1000 + 261, 4000 + 210] {
            let on = look_for(id).unwrap();
            let off = look_for(NO_VIGNETTE + id).unwrap();
            assert!(on.vignette > 0.0, "{} should have a vignette", id);
            assert_eq!(off.vignette, 0.0);
            assert_eq!(off.color.contrast, on.color.contrast);
            assert_eq!(off.grain, on.grain);
            assert_eq!(off.soft, on.soft);
        }
        assert!(look_for(NO_VIGNETTE).is_none(), "flag alone must stay a passthrough");
        // A 200 px frame of flat grey: the corners match the centre only without the vignette.
        let flat = vec![128u8; 200 * 200 * 4];
        let (mut on, mut off) = (flat.clone(), flat.clone());
        apply_filter(&mut on, 200, 200, 261);
        apply_filter(&mut off, 200, 200, NO_VIGNETTE + 261);
        let luma = |p: &[u8], x: usize, y: usize| p[(y * 200 + x) * 4] as i32;
        assert!(luma(&on, 100, 100) - luma(&on, 0, 0) > 4, "vignette should darken the corner");
        assert!((luma(&off, 100, 100) - luma(&off, 0, 0)).abs() <= 4, "no vignette: corner matches centre");
    }

    #[test]
    fn test_film_grade_is_shared_by_every_body() {
        // grade_key relies on this: a film's colour must not depend on the body it is loaded into.
        let bodies = camera_ids();
        let samples = [[0.1, 0.2, 0.3], [0.8, 0.4, 0.1], [0.5, 0.5, 0.5], [0.95, 0.9, 0.2]];
        for film in (1..100).filter(|&f| films::film_look(f).is_some()) {
            let key = grade_key(film * films::FILM_ID_STRIDE + 261 + NO_VIGNETTE);
            let reference = look_for(film * films::FILM_ID_STRIDE).unwrap();
            for body in bodies.iter().copied().chain([1]) {
                let id = film * films::FILM_ID_STRIDE + body;
                assert_eq!(grade_key(id), key, "film {} on body {} must share its grade", film, body);
                let look = look_for(id).unwrap();
                for c in samples {
                    assert_eq!(look.grade(c), reference.grade(c), "film {} grade changes on body {}", film, body);
                }
            }
        }
        assert_eq!(grade_key(261 + NO_VIGNETTE), 261);
    }

    #[test]
    fn test_film_replaces_body_colour() {
        let body = look_for(261).unwrap();
        let on_gold = look_for(1 * films::FILM_ID_STRIDE + 261).unwrap();
        let on_portra = look_for(4 * films::FILM_ID_STRIDE + 261).unwrap();
        let balance = |l: &Look| l.stock.map(|s| s.balance);
        assert_ne!(balance(&on_gold), balance(&on_portra));
        assert!(body.stock.is_none() && on_gold.stock.is_some(), "the film brings its own response");
        assert_eq!(on_gold.vignette, body.vignette);
        assert!(look_for(99 * films::FILM_ID_STRIDE + 261).is_none(), "unknown film must not resolve");
        assert!(look_for(films::FILM_ID_STRIDE + 999).is_none(), "unknown body must not resolve");
    }

    #[test]
    fn test_scope_letterboxes_along_the_long_axis() {
        // Landscape 240x160: picture band 240 / 2.39 ≈ 100 rows, so ~30 black rows top and bottom.
        let mut land = vec![200u8; 240 * 160 * 4];
        apply_filter(&mut land, 240, 160, 104);
        let px = |p: &[u8], w: usize, x: usize, y: usize| p[(y * w + x) * 4];
        assert_eq!(px(&land, 240, 120, 2), 0, "top bar should be black");
        assert_eq!(px(&land, 240, 120, 157), 0, "bottom bar should be black");
        assert!(px(&land, 240, 120, 80) > 0, "centre should keep the picture");
        // Portrait 160x240: bars move to the left and right.
        let mut port = vec![200u8; 160 * 240 * 4];
        apply_filter(&mut port, 160, 240, 104);
        assert_eq!(px(&port, 160, 2, 120), 0, "left bar should be black");
        assert!(px(&port, 160, 80, 2) > 0, "top edge stays picture in portrait");
    }

    #[test]
    fn test_film_on_120_has_finer_grain() {
        for film in [1, 4, 41] {
            let on_135 = look_for(film * films::FILM_ID_STRIDE + 261).unwrap();
            let on_120 = look_for(film * films::FILM_ID_STRIDE + 240).unwrap();
            let body = look_for(240).unwrap();
            assert!(on_120.grain < on_135.grain, "film {} should be finer on 120", film);
            assert!(on_120.grain_size < on_135.grain_size, "film {} grain cells should be smaller on 120", film);
            assert_eq!(on_120.vignette, body.vignette);
            assert_eq!(on_120.soft, body.soft);
        }
    }

    #[test]
    fn test_no_engine_look_missing_from_kotlin() {
        let Some(ids) = kotlin_filter_ids() else { return };
        for id in 1..1000 {
            if look_for(id).is_some() {
                assert!(ids.contains(&id), "engine look {} is not exposed in FilterType.kt", id);
            }
        }
    }

    fn gradient(w: usize, h: usize) -> Vec<u8> {
        let mut p = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                p.extend_from_slice(&[(x * 255 / w) as u8, (y * 255 / h) as u8, 128, 255]);
            }
        }
        p
    }

    #[test]
    fn test_every_camera_has_a_look_that_changes_pixels() {
        for id in camera_ids() {
            assert!(cameras::camera_look(id).is_some(), "camera {} has no look", id);
            let original = gradient(64, 48);
            let mut pixels = original.clone();
            apply_filter(&mut pixels, 64, 48, id);
            assert_ne!(pixels, original, "camera {} should modify pixels", id);
            assert!(pixels.chunks(4).all(|p| p[3] == 255), "camera {} must keep alpha", id);
        }
    }

    #[test]
    fn test_baked_grade_matches_direct_math() {
        let films = (1..100).filter(|&f| films::film_look(f).is_some()).map(|f| f * films::FILM_ID_STRIDE + 261);
        for id in camera_ids().into_iter().chain(1..=4).chain(films) {
            let look = look_for(id).unwrap();
            let baked = look::baked_grade(id, &look);
            let mut worst = 0.0f32;
            for r in (0..=255).step_by(15) {
                for g in (0..=255).step_by(15) {
                    for b in (0..=255).step_by(15) {
                        let c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
                        let d = look.grade(c);
                        let s = baked.sample_tetra(c[0], c[1], c[2]);
                        for i in 0..3 {
                            worst = worst.max((d[i] - s[i]).abs());
                        }
                    }
                }
            }
            // Error concentrates where contrast clips saturated colours; it stays below the grain.
            // Error concentrates at the gamut edge, in display primaries no film reaches; it stays
            // below the grain.
            assert!(worst * 255.0 <= 7.0, "filter {} baked grade off by {:.1}/255", id, worst * 255.0);
        }
    }

    #[test]
    fn test_tetrahedral_is_exact_on_identity() {
        let lut = lut3d::Lut3d::identity(17);
        for &(r, g, b) in &[(0.0, 0.0, 0.0), (1.0, 1.0, 1.0), (0.3, 0.7, 0.1), (0.9, 0.2, 0.55), (0.5, 0.5, 0.5)] {
            let s = lut.sample_tetra(r, g, b);
            assert!((s[0] - r).abs() < 1e-5 && (s[1] - g).abs() < 1e-5 && (s[2] - b).abs() < 1e-5, "{:?}", s);
        }
    }

    #[test]
    fn test_camera_tolerates_short_buffer() {
        // Mismatched dimensions must not panic.
        let mut pixels = vec![10u8; 16];
        apply_filter(&mut pixels, 64, 48, 100);
    }

    #[test]
    fn test_gr_high_contrast_bw_is_monochrome_and_keeps_the_lens() {
        let mut pixels = gradient(32, 32);
        apply_filter(&mut pixels, 32, 32, 30 * films::FILM_ID_STRIDE + 200);
        for p in pixels.chunks(4) {
            assert!((p[0] as i32 - p[2] as i32).abs() <= 2, "B&W mode should be greyscale, got {:?}", p);
        }
        let look = look_for(30 * films::FILM_ID_STRIDE + 200).unwrap();
        assert_eq!(look.vignette, look_for(200).unwrap().vignette);
    }

    #[test]
    fn test_grd_keeps_colour_with_hard_contrast() {
        // Saturated red must stay red; mid grey must still be pushed apart from its neighbours.
        let mut pixels = vec![200, 40, 40, 255, 100, 100, 100, 255, 156, 156, 156, 255];
        apply_filter(&mut pixels, 3, 1, 200);
        assert!(pixels[0] as i32 - pixels[2] as i32 > 60, "GRD should keep colour, got {:?}", &pixels[..3]);
        let (dark, light) = (pixels[4] as i32, pixels[8] as i32);
        assert!(light - dark > 56 + 8, "GRD should add contrast: {} -> {}", dark, light);
    }

    #[test]
    fn test_alpha_preserved() {
        let mut pixels = vec![128, 64, 32, 200]; // alpha = 200
        apply_filter(&mut pixels, 1, 1, 1);
        assert_eq!(pixels[3], 200, "Alpha channel should be preserved");
    }
}

#[cfg(test)]
mod bench {
    use super::look::{self, Look, BASE};

    fn frame(w: usize, h: usize) -> Vec<u8> {
        let mut p = Vec::with_capacity(w * h * 4);
        let mut s = 1u32;
        for y in 0..h {
            for x in 0..w {
                s ^= s << 13; s ^= s >> 17; s ^= s << 5;
                // Smooth scene plus a little sensor noise, like a real camera frame.
                let n = (s & 7) as usize;
                p.extend_from_slice(&[(x * 247 / w + n) as u8, (y * 247 / h + n) as u8, ((x + y) * 120 / (w + h) + 60 + n) as u8, 255]);
            }
        }
        p
    }

    fn time(label: &str, w: usize, h: usize, base: &[u8], f: impl Fn(&mut [u8])) {
        let mut best = std::time::Duration::MAX;
        for _ in 0..8 {
            let mut p = base.to_vec();
            let t = std::time::Instant::now();
            f(&mut p);
            best = best.min(t.elapsed());
        }
        println!("BENCH {:>4}x{:<4} {:<14} {:?}", w, h, label, best);
    }

    /// Writes every film's grade (and every camera body's own) as a 33^3 .cube into `$GLYPH_CUBE_DIR`,
    /// to check the looks in a grading tool: `GLYPH_CUBE_DIR=out cargo test dump_cubes -- --ignored`.
    #[test]
    #[ignore]
    fn dump_cubes() {
        let dir = std::env::var("GLYPH_CUBE_DIR").unwrap_or_else(|_| "cubes".into());
        std::fs::create_dir_all(&dir).unwrap();
        let ids = (1..100).map(|f| f * super::films::FILM_ID_STRIDE).chain(100..1000);
        for id in ids {
            let Some(look) = super::look_for(id) else { continue };
            let mut out = String::from("LUT_3D_SIZE 33\n");
            for i in 0..33 * 33 * 33 {
                // Red fastest, as .cube expects.
                let c = [(i % 33) as f32 / 32.0, (i / 33 % 33) as f32 / 32.0, (i / 1089) as f32 / 32.0];
                let c = look.grade(c);
                out.push_str(&format!("{:.6} {:.6} {:.6}\n", c[0], c[1], c[2]));
            }
            std::fs::write(format!("{}/{}.cube", dir, id), out).unwrap();
        }
    }

    #[test]
    #[ignore]
    fn bench_cameras() {
        let (w, h) = (1440, 1080);
        let base = frame(w, h);
        for id in [100, 200, 211, 219, 230] {
            time(&format!("id {}", id), w, h, &base, |p| super::apply_filter(p, w, h, id));
        }
        // Stage breakdown on Kino-like settings.
        let grade_only = Look { color: super::look_for(219).unwrap().color, ..BASE };
        let g = look::bake(&grade_only);
        time("grade", w, h, &base, |p| look::apply_look(p, w, h, &grade_only, &g, 1));
        let vig = Look { vignette: 0.3, ..grade_only };
        time("+vignette", w, h, &base, |p| look::apply_look(p, w, h, &vig, &g, 1));
        let grain = Look { grain: 0.3, ..vig };
        time("+grain", w, h, &base, |p| look::apply_look(p, w, h, &grain, &g, 1));
        let chroma = Look { chroma_grain: 0.3, ..grain };
        time("+chroma grain", w, h, &base, |p| look::apply_look(p, w, h, &chroma, &g, 1));
        let soft = Look { soft: 0.3, ..chroma };
        time("+soft", w, h, &base, |p| look::apply_look(p, w, h, &soft, &g, 1));
    }
}

