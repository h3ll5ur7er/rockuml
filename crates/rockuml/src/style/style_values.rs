//! What drawing code reads from a style: fonts, strokes, spacing and colours (the reading half of
//! PlantUML's `Style`).

use super::{PName, Style, Value, ValueReading};
use crate::color::{ColorType, Colors, HColor};
use crate::klimt::HorizontalAlignment;
use crate::klimt::fashion::Fashion;
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::ugraphic::UStroke;

impl Style {
    /// `getUFont`: the font style sets the face, a separate font weight only the weight.
    pub(crate) fn ufont(&self) -> UFont {
        let size = match self.value(PName::FontSize).as_int_or_minus_one() {
            -1 => 14,
            size => size,
        };
        let mut face = self.value(PName::FontStyle).as_font_face();
        let weight = self.value(PName::FontWeight).as_font_face();
        if weight.weight != 400 {
            face = UFontFace {
                weight: weight.weight,
                ..face
            };
        }
        UFont::new(&self.value(PName::FontName).as_string(), face, size)
    }

    pub(crate) fn font_configuration(&self) -> FontConfiguration {
        self.font_configuration_with(&Colors::default())
    }

    /// The font configuration, with the text colour an element sets for itself.
    pub(crate) fn font_configuration_with(&self, colors: &Colors) -> FontConfiguration {
        let color = colors
            .get(ColorType::Text)
            .cloned()
            .unwrap_or_else(|| self.value(PName::FontColor).as_color());
        FontConfiguration::new(self.ufont(), color, 8).with_hyperlink_style(
            self.value(PName::HyperLinkColor).as_color(),
            self.stroke_of(
                PName::HyperlinkUnderlineThickness,
                PName::HyperlinkUnderlineStyle,
            ),
        )
    }

    /// `getStroke`: the line thickness, dashed by a `visible-space` line style.
    pub(crate) fn stroke(&self) -> UStroke {
        self.stroke_of(PName::LineThickness, PName::LineStyle)
    }

    /// `getStroke(Colors)`: the line style an element sets for itself wins.
    pub(crate) fn stroke_with(&self, colors: &Colors) -> UStroke {
        colors
            .get_specific_line_stroke()
            .unwrap_or_else(|| self.stroke())
    }

    fn stroke_of(&self, thickness: PName, line_style: PName) -> UStroke {
        let thickness = self.value(thickness).as_double();
        let dash = self.value(line_style).as_string();
        let mut lengths = dash
            .split(['-', ';', ','])
            .filter(|part| !part.is_empty())
            .map(|part| crate::java::trim(part).parse::<f64>());
        match lengths.next() {
            Some(Ok(visible)) => {
                let space = match lengths.next() {
                    Some(Ok(space)) => space,
                    Some(Err(_)) => return UStroke::with_thickness(thickness),
                    None => visible,
                };
                UStroke {
                    dash_visible: visible,
                    dash_space: space,
                    thickness,
                }
            }
            _ => UStroke::with_thickness(thickness),
        }
    }

    pub(crate) fn padding(&self) -> ClockwiseTopRightBottomLeft {
        self.spacing(PName::Padding)
    }

    pub(crate) fn margin(&self) -> ClockwiseTopRightBottomLeft {
        self.spacing(PName::Margin)
    }

    /// `ClockwiseTopRightBottomLeft.read`: one to four whole numbers, CSS-style; anything else is none.
    fn spacing(&self, property: PName) -> ClockwiseTopRightBottomLeft {
        let text = self.value(property).as_string();
        if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit() || c == ' ') {
            return ClockwiseTopRightBottomLeft::none();
        }
        let numbers: Result<Vec<f64>, _> = text
            .split(' ')
            .filter(|part| !part.is_empty())
            .map(|part| part.parse::<i32>().map(f64::from))
            .collect();
        let Ok(numbers) = numbers else {
            return ClockwiseTopRightBottomLeft::none();
        };
        let (top, right, bottom, left) = match numbers.as_slice() {
            [all] => (*all, *all, *all, *all),
            [vertical, horizontal] => (*vertical, *horizontal, *vertical, *horizontal),
            [top, horizontal, bottom] => (*top, *horizontal, *bottom, *horizontal),
            [top, right, bottom, left] => (*top, *right, *bottom, *left),
            _ => return ClockwiseTopRightBottomLeft::none(),
        };
        ClockwiseTopRightBottomLeft {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The alignment written in the style; nothing for an unknown name.
    pub(crate) fn horizontal_alignment(&self) -> Option<HorizontalAlignment> {
        self.value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
    }

    /// `wrapWidth`: how wide text may grow before it wraps, or 0 to never wrap.
    pub(crate) fn wrap_width(&self) -> f64 {
        max_width(&self.value(PName::MaximumWidth).as_string())
    }

    /// `getSymbolContext`: the element's own colours win over the style's.
    pub(crate) fn symbol_context(&self, colors: &Colors) -> Fashion {
        let back_color = colors
            .get(ColorType::Back)
            .cloned()
            .unwrap_or_else(|| self.value(PName::BackGroundColor).as_color());
        let fore_color = colors
            .get(ColorType::Line)
            .cloned()
            .unwrap_or_else(|| self.value(PName::LineColor).as_color());
        Fashion::new(back_color, fore_color)
            .with_stroke(self.stroke())
            .with_corner(
                self.value(PName::RoundCorner).as_double(),
                self.value(PName::DiagonalCorner).as_double(),
            )
    }

    /// The style with a colour set by the diagram element, which keeps the property's priority.
    #[must_use]
    pub(crate) fn eventually_override(&self, name: PName, color: Option<&HColor>) -> Style {
        let Some(color) = color else {
            return self.clone();
        };
        let priority = self.value(name).map_or(0, Value::priority);
        self.with_value(
            name,
            Value::Color {
                color: color.clone(),
                priority,
            },
        )
    }

    /// The style with the background, line and text colours an element sets for itself.
    #[must_use]
    pub(crate) fn eventually_override_colors(&self, colors: &Colors) -> Style {
        self.eventually_override(PName::BackGroundColor, colors.get(ColorType::Back))
            .eventually_override(PName::LineColor, colors.get(ColorType::Line))
            .eventually_override(PName::FontColor, colors.get(ColorType::Text))
    }
}

/// `LineBreakStrategy.getMaxWidth`: a whole number, or 0.
pub(crate) fn max_width(value: &str) -> f64 {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return 0.0;
    }
    value.parse().unwrap_or(0.0)
}
