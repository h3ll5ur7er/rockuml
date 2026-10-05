//! Resources bundled with PlantUML: the standard library and the themes.

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// `path` is relative to the assets directory with `/` separators, e.g. `themes/puml-theme-amiga.puml`.
pub(crate) fn get(path: &str) -> Option<&'static [u8]> {
    FILES
        .binary_search_by(|(name, _)| (*name).cmp(path))
        .ok()
        .map(|index| FILES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_files_are_found_by_relative_path() {
        assert!(get("themes/puml-theme-cerulean.puml").is_some());
        assert!(get("stdlib/c4/puml.spm").is_some());
        assert!(get("stdlib/c4/missing.spm").is_none());
    }
}
