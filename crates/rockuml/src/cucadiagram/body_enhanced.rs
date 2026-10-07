//! A body split into blocks by separator lines like `--`, `..`, `==` or `__`, each block's lines drawn as
//! members (PlantUML's `BodyEnhanced1` and `BodyEnhancedAbstract`).

use super::methods_or_fields_area::{BodyLine, MethodsOrFieldsArea};
use crate::color::Colors;
use crate::creole::{CreoleMode, Display};
use crate::java;
use crate::klimt::blocks::{TextBlockLineBefore, TextBlockMarged, TextBlockVertical, TitledSeparator};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, Style, StyleBuilder, ValueReading};

const MARGIN_X: f64 = 6.0;

pub(crate) fn is_block_separator(s: &str) -> bool {
    (s.starts_with("--") && s.ends_with("--"))
        || (s.starts_with("==") && s.ends_with("=="))
        || (s.starts_with("..") && s.ends_with("..") && s != "...")
        || (s.starts_with("__") && s.ends_with("__"))
}

/// The blocks of a body under a first separator line, stacked.
pub(crate) fn body_enhanced1(
    align: HorizontalAlignment,
    raw_body: &[BodyLine],
    skin: &SkinParam,
    style_builder: &StyleBuilder,
    style: &Style,
    colors: &Colors,
) -> Box<dyn TextBlock> {
    let title_config = style.font_configuration_with(colors);
    let thickness = style.value(PName::LineThickness).as_double();
    let build = |lines: &[BodyLine], separator: char, title: Option<Box<dyn TextBlock>>| {
        let area = MethodsOrFieldsArea::new(lines, skin, style_builder, style, colors, align);
        decorate(Box::new(area), separator, title, thickness)
    };
    let mut blocks: Vec<Box<dyn TextBlock>> = Vec::new();
    let mut separator = '_';
    let mut title = None;
    let mut lines = Vec::new();
    for line in raw_body {
        let s = line.as_str();
        if is_block_separator(&s) {
            blocks.push(build(&std::mem::take(&mut lines), separator, title.take()));
            separator = s.chars().next().expect("separators are not empty");
            title = get_title(&s, &title_config, skin);
        } else {
            lines.push(line.clone());
        }
    }
    blocks.push(build(&lines, separator, title));
    if blocks.len() == 1 {
        blocks.remove(0)
    } else {
        Box::new(TextBlockVertical::new(blocks, align))
    }
}

/// The title of a separator like `== Title ==`.
fn get_title(
    s: &str,
    title_config: &crate::klimt::font::FontConfiguration,
    skin: &SkinParam,
) -> Option<Box<dyn TextBlock>> {
    let length = s.chars().count();
    if length <= 4 {
        return None;
    }
    let inner: String = s.chars().skip(2).take(length - 4).collect();
    let display = Display::with_newlines(java::trim(&inner));
    Some(Box::new(display.create0(
        title_config,
        HorizontalAlignment::Left,
        skin,
        0.0,
        CreoleMode::Full,
    )))
}

/// A block after its separator, with room around it (`BodyEnhancedAbstract.decorate`).
fn decorate(
    block: Box<dyn TextBlock>,
    separator: char,
    title: Option<Box<dyn TextBlock>>,
    thickness: f64,
) -> Box<dyn TextBlock> {
    let margin = ClockwiseTopRightBottomLeft::top_right_bottom_left;
    let Some(title) = title else {
        return Box::new(TextBlockLineBefore {
            block: Box::new(TextBlockMarged::new(
                block,
                margin(4.0, MARGIN_X, 4.0, MARGIN_X),
            )),
            style: separator,
            title: None,
            thickness,
        });
    };
    Box::new(TitledSeparator {
        block,
        style: separator,
        title,
        thickness,
        margin_x: MARGIN_X,
    })
}
