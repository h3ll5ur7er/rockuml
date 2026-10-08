//! One object or array of a JSON or YAML document as a table: keys on the left, values on the right, nested
//! containers left blank for the arrows to their own tables (PlantUML's `TextBlockJson`).

use std::rc::Rc;

use super::highlighted::Highlighted;
use crate::creole::{CreoleMode, Display};
use crate::json::JsonValue;
use crate::klimt::blocks::TextBlockMarged;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, Style, StyleBuilder, StyleSignature, ValueReading};

const MIN_WIDTH: f64 = 30.0;
const MIN_HEIGHT: f64 = 15.0;

struct Line {
    b1: Box<dyn TextBlock>,
    b2: Option<Box<dyn TextBlock>>,
    highlighted: Option<Highlighted>,
}

impl Line {
    fn get_height_of_row(&self, string_bounder: &dyn StringBounder) -> f64 {
        let height = self.b1.calculate_dimension(string_bounder).height;
        match &self.b2 {
            Some(b2) => height.max(b2.calculate_dimension(string_bounder).height),
            None => height,
        }
    }
}

pub(super) struct TextBlockJson<'a> {
    lines: Vec<Line>,
    skin_param: &'a SkinParam,
    root: &'a JsonValue,
    style_builder: Rc<StyleBuilder>,
    diagram_type: SName,
}

impl<'a> TextBlockJson<'a> {
    pub(super) fn new(
        skin_param: &'a SkinParam,
        diagram_type: SName,
        root: &'a JsonValue,
        all_highlighteds: &[Highlighted],
    ) -> Self {
        let mut result = Self {
            lines: Vec::new(),
            skin_param,
            root,
            style_builder: skin_param.current_style_builder(),
            diagram_type,
        };
        match root {
            JsonValue::Object(object) => {
                for (key, value) in object.members() {
                    let highlighted = is_highlighted(key, all_highlighteds);
                    let block1 =
                        result.get_text_block(&result.get_style_to_use(true, highlighted), key);
                    let block2 = result.get_text_block(
                        &result.get_style_to_use(false, highlighted),
                        &get_short_string(value),
                    );
                    result.lines.push(Line {
                        b1: block1,
                        b2: Some(block2),
                        highlighted: highlighted.cloned(),
                    });
                }
            }
            JsonValue::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    let highlighted = is_highlighted(&i.to_string(), all_highlighteds);
                    let block2 = result.get_text_block(
                        &result.get_style_to_use(false, highlighted),
                        &get_short_string(value),
                    );
                    result.lines.push(Line {
                        b1: block2,
                        b2: None,
                        highlighted: highlighted.cloned(),
                    });
                }
            }
            _ => {}
        }
        result
    }

    fn get_style_to_use(&self, header: bool, highlighted: Option<&Highlighted>) -> Style {
        let mut names = vec![SName::Root, SName::Element, self.diagram_type];
        if header {
            names.push(SName::Header);
        }
        names.push(SName::Node);
        if highlighted.is_some() {
            names.push(SName::Highlight);
        }
        StyleSignature::of(&names).get_merged_style_with(
            &self.style_builder,
            highlighted.and_then(Highlighted::stereotype),
        )
    }

    /// The nested containers, in order, `None` for each line holding a plain value.
    pub(super) fn children(&self) -> Vec<Option<&'a JsonValue>> {
        let container = |value: &'a JsonValue| {
            matches!(value, JsonValue::Object(_) | JsonValue::Array(_)).then_some(value)
        };
        match self.root {
            JsonValue::Object(object) => object
                .members()
                .map(|(_, value)| container(value))
                .collect(),
            JsonValue::Array(values) => values.iter().map(container).collect(),
            _ => Vec::new(),
        }
    }

    /// Each line's key: an object's member names, an array's indexes.
    pub(super) fn keys(&self) -> Vec<String> {
        match self.root {
            JsonValue::Object(object) => object.names().map(str::to_owned).collect(),
            JsonValue::Array(values) => (0..values.len()).map(|i| i.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn get_width_col_a(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.lines
            .iter()
            .map(|line| line.b1.calculate_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    pub(super) fn get_width_col_b(&self, string_bounder: &dyn StringBounder) -> f64 {
        self.lines
            .iter()
            .filter_map(|line| line.b2.as_ref())
            .map(|b2| b2.calculate_dimension(string_bounder).width)
            .fold(0.0, f64::max)
    }

    fn get_total_height(&self, string_bounder: &dyn StringBounder) -> f64 {
        let height: f64 = self
            .lines
            .iter()
            .map(|line| line.get_height_of_row(string_bounder))
            .sum();
        if height == 0.0 { MIN_HEIGHT } else { height }
    }

    pub(super) fn get_all_heights(&self, string_bounder: &dyn StringBounder) -> Vec<f64> {
        self.lines
            .iter()
            .map(|line| line.get_height_of_row(string_bounder))
            .collect()
    }

    fn get_text_block(&self, style: &Style, key: &str) -> Box<dyn TextBlock> {
        let text = Display::with_newlines(key).create0(
            &style.font_configuration(),
            style
                .horizontal_alignment()
                .unwrap_or(HorizontalAlignment::Left),
            self.skin_param,
            style.wrap_width(),
            CreoleMode::NoCreole,
        );
        Box::new(TextBlockMarged::new(
            text,
            ClockwiseTopRightBottomLeft::margin1_margin2(2.0, 5.0),
        ))
    }

    fn node_style(&self) -> Style {
        StyleSignature::of(&[SName::Root, SName::Element, self.diagram_type, SName::Node])
            .get_merged_style(&self.style_builder)
    }
}

/// How a value shows in its cell.
fn get_short_string(value: &JsonValue) -> String {
    match value {
        JsonValue::String(text) => text.clone(),
        JsonValue::Null => "<U+2400>".to_owned(),
        JsonValue::Number(written) => written.clone(),
        JsonValue::Bool(true) => "<U+2611> true".to_owned(),
        JsonValue::Bool(false) => "<U+2610> false".to_owned(),
        JsonValue::Array(_) | JsonValue::Object(_) => "   ".to_owned(),
    }
}

fn is_highlighted<'h>(key: &str, highlighted: &'h [Highlighted]) -> Option<&'h Highlighted> {
    highlighted.iter().find(|tmp| tmp.is_key_highlight(key))
}

/// The style's line colour and stroke (`Style.applyStrokeAndLineColor`).
pub(super) fn apply_stroke_and_line_color(style: &Style, ug: &UGraphic) -> UGraphic {
    ug.apply(style.value(PName::LineColor).as_color())
        .apply(style.stroke())
}

impl TextBlock for TextBlockJson<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        XDimension2D::new(
            self.get_width_col_a(string_bounder) + self.get_width_col_b(string_bounder),
            self.get_total_height(string_bounder),
        )
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let full_dim = self.calculate_dimension(string_bounder);
        let mut true_width = full_dim.width;
        let width_col_a = self.get_width_col_a(string_bounder);
        let width_col_b = self.get_width_col_b(string_bounder);
        let style_node = self.node_style();
        let ug_node = apply_stroke_and_line_color(&style_node, ug);
        let mut y: f64 = self
            .lines
            .iter()
            .map(|line| line.get_height_of_row(string_bounder))
            .sum();
        if y == 0.0 {
            y = MIN_HEIGHT;
        }
        if true_width == 0.0 {
            true_width = MIN_WIDTH;
        }
        let round = style_node.value(PName::RoundCorner).as_double();
        let full_node_rectangle = UShape::Rectangle(URectangle::new(true_width, y).rounded(round));
        let back_color = style_node.value(PName::BackGroundColor).as_color();
        ug_node
            .with_backcolor(back_color.clone())
            .apply(back_color)
            .draw(&full_node_rectangle);
        let style_separator = style_node
            .signature()
            .with_name(SName::Separator)
            .get_merged_style(&self.skin_param.current_style_builder());
        let ug_separator = apply_stroke_and_line_color(&style_separator, ug);
        let horizontal_alignment = style_node
            .horizontal_alignment()
            .unwrap_or(HorizontalAlignment::Left);
        let mut y = 0.0;
        for line in &self.lines {
            let ug_line = ug_separator.translated(0.0, y);
            let height_of_row = line.get_height_of_row(string_bounder);
            if let Some(highlighted) = &line.highlighted {
                let back = UShape::Rectangle(
                    URectangle::new(true_width - 2.0, height_of_row).rounded(4.0),
                );
                let cell_back_color = StyleSignature::of(&[
                    SName::Root,
                    SName::Element,
                    self.diagram_type,
                    SName::Node,
                    SName::Highlight,
                ])
                .get_merged_style_with(&self.style_builder, highlighted.stereotype())
                .value(PName::BackGroundColor)
                .as_color();
                ug_line
                    .apply(cell_back_color.clone())
                    .with_backcolor(cell_back_color)
                    .translated(1.5, 0.0)
                    .draw(&back);
            }
            if y > 0.0 {
                ug_line.draw(&UShape::Line {
                    dx: true_width,
                    dy: 0.0,
                });
            }
            horizontal_alignment.draw(&ug_line, line.b1.as_ref(), 0.0, 0.0, width_col_a);
            if let Some(b2) = &line.b2 {
                let ug_line_col_b = ug_line.translated(width_col_a, 0.0);
                horizontal_alignment.draw(&ug_line_col_b, b2.as_ref(), 0.0, 0.0, width_col_b);
                ug_line_col_b.draw(&UShape::Line {
                    dx: 0.0,
                    dy: height_of_row,
                });
            }
            y += height_of_row;
        }
        ug_node.draw(&full_node_rectangle);
    }
}
