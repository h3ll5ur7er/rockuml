//! The `OpenIconic` icon set (<https://useiconic.com/open>), drawn inline by creole's `<&name>`.

mod svg_path;

use std::sync::LazyLock;

use regex::Regex;
pub(crate) use svg_path::SvgPath;

use crate::color::HColor;
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{UTranslate, XDimension2D};
use crate::klimt::shape::UShape;
use crate::klimt::ugraphic::{UGraphic, UStroke};
use crate::pattern::java_regex;

pub(crate) struct OpenIconic {
    svg_path: SvgPath,
    width: f64,
    height: f64,
}

impl OpenIconic {
    pub(crate) fn retrieve(name: &str) -> Option<Self> {
        let svg = crate::assets::get(&format!("openiconic/{name}.svg"))?;
        Some(Self::new(
            std::str::from_utf8(svg).expect("the icons are ASCII"),
        ))
    }

    /// Icon files are a `<svg>` line with the size and a `<path>` with an optional `transform`.
    fn new(svg: &str) -> Self {
        let raw_data: Vec<&str> = svg.lines().collect();
        let translate = raw_data
            .iter()
            .rfind(|line| line.contains("transform=\""))
            .map_or(UTranslate::default(), |line| get_translate(line));
        let path_line = raw_data
            .iter()
            .rfind(|line| line.contains("<path"))
            .expect("every icon has a path");
        let header = raw_data[0];
        Self {
            svg_path: SvgPath::new(first_quoted(path_line), translate),
            width: get_number(header, "width"),
            height: get_number(header, "height"),
        }
    }

    pub(crate) fn as_text_block(&self, color: HColor, factor: f64) -> OpenIconicBlock<'_> {
        OpenIconicBlock {
            icon: self,
            color,
            factor,
        }
    }
}

/// Only non-negative offsets are read: `translate(-1)` moves nothing in PlantUML.
fn get_translate(line: &str) -> UTranslate {
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| java_regex(r"translate\((\d+)\s*(\d*)\)", false));
    PATTERN
        .captures(line)
        .map_or(UTranslate::default(), |captures| {
            let offset = |group: &str| group.parse().unwrap_or_default();
            UTranslate::new(offset(&captures[1]), offset(&captures[2]))
        })
}

/// The value of the first quoted attribute after `name`.
fn get_number(line: &str, name: &str) -> f64 {
    let after_name = &line[line.find(name).expect("the svg line has a size")..];
    first_quoted(after_name)
        .parse()
        .expect("icon sizes are integers")
}

fn first_quoted(line: &str) -> &str {
    line.split('"').nth(1).expect("a quoted value")
}

pub(crate) struct OpenIconicBlock<'a> {
    icon: &'a OpenIconic,
    color: HColor,
    factor: f64,
}

impl TextBlock for OpenIconicBlock<'_> {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.icon.width * self.factor,
            self.icon.height * self.factor,
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let ug = ug
            .with_color(self.color.clone())
            .with_backcolor(self.color.clone())
            .with_stroke(UStroke::with_thickness(0.0));
        ug.draw(&UShape::Path(self.icon.svg_path.to_upath(self.factor)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_found_by_name() {
        let heart = OpenIconic::retrieve("heart").expect("a bundled icon");
        assert_eq!((heart.width, heart.height), (8.0, 8.0));
        assert!(OpenIconic::retrieve("no-such-icon").is_none());
    }

    #[test]
    fn every_bundled_icon_parses() {
        let names = crate::assets::FILES
            .iter()
            .filter_map(|(path, _)| path.strip_prefix("openiconic/")?.strip_suffix(".svg"));
        for name in names {
            let icon = OpenIconic::retrieve(name).expect("a bundled icon");
            assert!(!icon.svg_path.to_upath(1.0).is_empty(), "{name}");
        }
    }

    #[test]
    fn a_negative_translation_is_ignored_like_in_plantuml() {
        assert_eq!(
            get_translate(r#"transform="translate(-1)""#),
            UTranslate::default()
        );
        assert_eq!(
            get_translate(r#"transform="translate(0 2)""#),
            UTranslate::new(0.0, 2.0)
        );
    }
}
