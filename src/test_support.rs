//! Helpers shared by the unit tests.

/// IDs of the enum variants declared in a Kotlin file under `filter/`, so the tests cover everything
/// the app can select. None when the engine is checked out on its own, outside the app repo.
pub(crate) fn kotlin_ids(file: &str) -> Option<Vec<i32>> {
    let path = format!("{}/../android/app/src/main/java/com/codenour/glyphcam/filter/{}", env!("CARGO_MANIFEST_DIR"), file);
    let source = std::fs::read_to_string(path).ok()?;
    let ids: Vec<i32> = source
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.trim().split_once('(')?;
            let is_variant = !name.is_empty()
                && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            if !is_variant {
                return None;
            }
            rest.split(',').next()?.trim().parse().ok()
        })
        .collect();
    assert!(!ids.is_empty(), "failed to parse IDs from {}", file);
    Some(ids)
}
