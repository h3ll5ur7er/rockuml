//! Resources bundled with PlantUML: the standard library, themes, skins, icons, emoji, and the fonts.

use std::io::Read;
use std::sync::OnceLock;

struct Asset {
    /// Relative to the assets directory with `/` separators.
    name: &'static str,
    embedded: &'static [u8],
    is_deflated: bool,
    inflated: OnceLock<Vec<u8>>,
}

impl Asset {
    const fn stored(name: &'static str, contents: &'static [u8]) -> Self {
        Self {
            name,
            embedded: contents,
            is_deflated: false,
            inflated: OnceLock::new(),
        }
    }

    const fn deflated(name: &'static str, deflated: &'static [u8]) -> Self {
        Self {
            name,
            embedded: deflated,
            is_deflated: true,
            inflated: OnceLock::new(),
        }
    }

    fn contents(&'static self) -> &'static [u8] {
        if !self.is_deflated {
            return self.embedded;
        }
        self.inflated.get_or_init(|| {
            let mut contents = Vec::new();
            flate2::read::DeflateDecoder::new(self.embedded)
                .read_to_end(&mut contents)
                .expect("the build script deflated the asset");
            contents
        })
    }
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// `path` is relative to the assets directory with `/` separators, e.g. `themes/puml-theme-amiga.puml`.
pub(crate) fn get(path: &str) -> Option<&'static [u8]> {
    FILES
        .binary_search_by(|asset| asset.name.cmp(path))
        .ok()
        .map(|index| FILES[index].contents())
}

/// The paths of all bundled files, sorted.
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    FILES.iter().map(|asset| asset.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_files_are_found_by_relative_path() {
        assert!(get("themes/puml-theme-cerulean.puml").is_some());
        assert!(get("themes/puml-theme-missing.puml").is_none());
    }

    #[test]
    fn the_stdlib_is_bundled_with_its_feature_only() {
        assert_eq!(
            get("stdlib/c4/puml.spm").is_some(),
            cfg!(feature = "stdlib")
        );
    }

    #[test]
    fn deflated_files_read_as_they_were() {
        assert_eq!(
            get("themes/puml-theme-cerulean.puml"),
            Some(&include_bytes!("../assets/themes/puml-theme-cerulean.puml")[..])
        );
        assert_eq!(
            get("fonts/LiberationMono-Bold.ttf"),
            Some(&include_bytes!("../assets/fonts/LiberationMono-Bold.ttf")[..])
        );
    }
}
