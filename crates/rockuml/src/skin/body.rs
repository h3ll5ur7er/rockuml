//! The text of notes: blocks separated by lines like `--`, `==`, `..` or `__`, optionally titled like
//! `== Title ==` (PlantUML's `BodyEnhanced2` and `TextBlockLineBefore`).

use super::component::creole_text;
use crate::creole::Display;
use crate::klimt::blocks::{TextBlockMarged, TextBlockVertical};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::stencil::UHorizontalLine;
use crate::klimt::ugraphic::UGraphic;
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
            );
            blocks.push(decorate(block, separator.take(), thickness));
            let title = title(line, &font);
            separator = Some((
                line.chars().next().expect("separators are not empty"),
                title,
            ));
        } else {
            lines.push(line.clone());
        }
    }
    let block = creole_text(&lines, font, alignment, max_width);
    blocks.push(decorate(block, separator, thickness));
    if blocks.len() == 1 {
        return blocks.remove(0);
    }
    Box::new(TextBlockVertical::new(blocks, alignment))
}

/// The title of a separator like `== Title ==`.
fn title(line: &str, font: &FontConfiguration) -> Option<Box<dyn TextBlock>> {
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
        }),
    }
}

/// A block under a separator with a title, which leaves room for half the title above and below the line.
struct TitledSeparator {
    block: Box<dyn TextBlock>,
    style: char,
    title: Box<dyn TextBlock>,
    thickness: f64,
}

impl TitledSeparator {
    fn layout(&self, string_bounder: &dyn StringBounder) -> impl TextBlock + '_ {
        let half_title = self.title.calculate_dimension(string_bounder).height / 2.0;
        let margin = ClockwiseTopRightBottomLeft::top_right_bottom_left;
        let raw = TextBlockLineBefore {
            block: Box::new(TextBlockMarged::new(
                &*self.block,
                margin(half_title, 6.0, 4.0, 0.0),
            )),
            style: self.style,
            title: Some(Box::new(&*self.title)),
            thickness: self.thickness,
        };
        TextBlockMarged::new(raw, margin(half_title, 0.0, 0.0, 0.0))
    }
}

impl TextBlock for TitledSeparator {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        self.layout(string_bounder)
            .calculate_dimension(string_bounder)
    }

    fn draw_u(&self, ug: &UGraphic) {
        self.layout(ug.string_bounder()).draw_u(ug);
    }
}

/// A block with a separator line across its top.
struct TextBlockLineBefore<'a> {
    block: Box<dyn TextBlock + 'a>,
    style: char,
    title: Option<Box<dyn TextBlock + 'a>>,
    thickness: f64,
}

impl TextBlockLineBefore<'_> {
    fn line(&self) -> UHorizontalLine<'_> {
        UHorizontalLine {
            style: self.style,
            title: self.title.as_deref(),
            default_thickness: self.thickness,
            skip: 1.0,
        }
    }
}

impl TextBlock for TextBlockLineBefore<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let dimension = self.block.calculate_dimension(string_bounder);
        match &self.title {
            None => dimension,
            Some(title) => {
                let title = title.calculate_dimension(string_bounder);
                XDimension2D::new(
                    dimension.width.max(title.width + 8.0),
                    dimension.height.max(title.height),
                )
            }
        }
    }

    fn draw_u(&self, ug: &UGraphic) {
        if self.title.is_none() {
            ug.draw_horizontal_line(&self.line());
        }
        self.block.draw_u(ug);
        if self.title.is_some() {
            ug.draw_horizontal_line(&self.line());
        }
    }
}
