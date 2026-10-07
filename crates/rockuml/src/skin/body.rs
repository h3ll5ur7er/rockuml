//! The text of notes: blocks separated by lines like `--`, `==`, `..` or `__`, optionally titled like
//! `== Title ==` (PlantUML's `BodyEnhanced2` and `TextBlockLineBefore`).

use super::component::creole_text;
use crate::creole::Display;
use crate::klimt::blocks::{
    TextBlockLineBefore, TextBlockMarged, TextBlockVertical, TitledSeparator,
};
use crate::klimt::font::FontConfiguration;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::sprite::SpriteContainer;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, Style, ValueReading};

fn is_block_separator(line: &str) -> bool {
    (line.starts_with("--") && line.ends_with("--"))
        || (line.starts_with("==") && line.ends_with("=="))
        || (line.starts_with("..") && line.ends_with("..") && line != "...")
        || (line.starts_with("__") && line.ends_with("__"))
}

pub(crate) fn enhanced_text(
    display: &Display,
    font: FontConfiguration,
    alignment: HorizontalAlignment,
    style: &Style,
    sprites: &dyn SpriteContainer,
) -> Box<dyn TextBlock> {
    let thickness = style.value(PName::LineThickness).as_double();
    let max_width = style.wrap_width();
    let mut blocks: Vec<Box<dyn TextBlock>> = Vec::new();
    let mut separator: Option<(char, Option<Box<dyn TextBlock>>)> = None;
    let mut lines: Vec<String> = Vec::new();
    for line in display.lines() {
        if is_block_separator(line) {
            let block = creole_text(
                &std::mem::take(&mut lines),
                font.clone(),
                alignment,
                max_width,
                sprites,
            );
            blocks.push(decorate(block, separator.take(), thickness));
            let title = title(line, &font, sprites);
            separator = Some((
                line.chars().next().expect("separators are not empty"),
                title,
            ));
        } else {
            lines.push(line.clone());
        }
    }
    let block = creole_text(&lines, font, alignment, max_width, sprites);
    blocks.push(decorate(block, separator, thickness));
    if blocks.len() == 1 {
        return blocks.remove(0);
    }
    Box::new(TextBlockVertical::new(blocks, alignment))
}

/// The title of a separator like `== Title ==`.
fn title(
    line: &str,
    font: &FontConfiguration,
    sprites: &dyn SpriteContainer,
) -> Option<Box<dyn TextBlock>> {
    if line.chars().count() <= 4 {
        return None;
    }
    let inner: String = line
        .chars()
        .skip(2)
        .take(line.chars().count() - 4)
        .collect();
    let display = Display::with_newlines(crate::java::trim(&inner));
    Some(creole_text(
        display.lines(),
        font.clone(),
        HorizontalAlignment::Left,
        0.0,
        sprites,
    ))
}

/// A block after its separator line, and the room the separator takes.
fn decorate(
    block: Box<dyn TextBlock>,
    separator: Option<(char, Option<Box<dyn TextBlock>>)>,
    thickness: f64,
) -> Box<dyn TextBlock> {
    let Some((style, title)) = separator else {
        return block;
    };
    match title {
        None => Box::new(TextBlockLineBefore {
            block: Box::new(TextBlockMarged::new(
                block,
                ClockwiseTopRightBottomLeft::top_right_bottom_left(4.0, 0.0, 4.0, 0.0),
            )),
            style,
            title: None,
            thickness,
        }),
        Some(title) => Box::new(TitledSeparator {
            block,
            style,
            title,
            thickness,
            margin_x: 0.0,
        }),
    }
}
