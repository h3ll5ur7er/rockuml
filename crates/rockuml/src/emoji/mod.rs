//! Twemoji pictures (<https://twemoji.twitter.com/>), drawn as vectors by creole's `<:name:>`.

mod color_resolver;
mod ugraphic_with_scale;

use std::collections::HashMap;
use std::io::Read;
use std::sync::{LazyLock, OnceLock};

pub(crate) use color_resolver::ColorResolver;
pub(crate) use ugraphic_with_scale::UGraphicWithScale;

use crate::color::HColor;
use crate::klimt::ugraphic::UGraphic;
use crate::svg_parser::SvgNanoParser;

/// The bundle `tools/bundle-emoji.sh` builds: one `<code>[;<shortcut>] <svg>` line per emoji. Builds without
/// the `emoji` feature know no emoji.
static TWEMOJI: LazyLock<String> = LazyLock::new(|| {
    let Some(compressed) = crate::assets::get("emoji/twemoji.br") else {
        return String::new();
    };
    let mut text = String::new();
    brotli_decompressor::Decompressor::new(compressed, 4096)
        .read_to_string(&mut text)
        .expect("the bundle is valid Brotli-compressed UTF-8");
    text
});

/// Each emoji, with its names: a code point (`1f600`) and maybe a shortcut (`grinning`).
static EMOJIS: LazyLock<Vec<(&'static str, Emoji)>> = LazyLock::new(|| {
    TWEMOJI
        .lines()
        .map(|line| {
            let (names, svg) = line.split_once(' ').expect("a name and a picture");
            let emoji = Emoji {
                svg,
                parser: OnceLock::new(),
            };
            (names, emoji)
        })
        .collect()
});

static ALL: LazyLock<HashMap<&'static str, &'static Emoji>> = LazyLock::new(|| {
    let mut all = HashMap::new();
    for (names, emoji) in EMOJIS.iter() {
        for name in names.split(';') {
            all.insert(name, emoji);
        }
    }
    all
});

pub(crate) struct Emoji {
    svg: &'static str,
    /// Read when the emoji is first drawn, as PlantUML does.
    parser: OnceLock<SvgNanoParser>,
}

impl Emoji {
    pub(crate) fn retrieve(name: &str) -> Option<&'static Self> {
        ALL.get(name.to_lowercase().as_str()).copied()
    }

    /// Draws the picture, 36 units square, at `scale`; a colour turns it into shades of that colour.
    pub(crate) fn draw_u(&self, ug: &UGraphic, scale: f64, color_for_monochrome: Option<&HColor>) {
        let parser = self.parser.get_or_init(|| SvgNanoParser::new(self.svg));
        parser.draw_u(
            ug,
            scale,
            color_for_monochrome.cloned(),
            color_for_monochrome.cloned(),
        );
    }
}

#[cfg(all(test, feature = "emoji"))]
mod tests {
    use super::*;

    #[test]
    fn emoji_are_found_by_code_and_by_shortcut_in_any_case() {
        assert_eq!(TWEMOJI.lines().count(), 1174);
        assert_eq!(
            Emoji::retrieve("SMILE").map(|emoji| emoji.svg),
            Emoji::retrieve("1f604").map(|emoji| emoji.svg)
        );
        assert!(Emoji::retrieve("1f3fb").is_some());
        assert!(Emoji::retrieve("no_such_emoji").is_none());
    }
}
