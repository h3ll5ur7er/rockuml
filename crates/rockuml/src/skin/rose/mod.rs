//! PlantUML's default look of sequence diagrams (`skin.rose`): one component per kind of element.

pub(crate) mod arrow;
pub(crate) mod life;
pub(crate) mod line;
pub(crate) mod participant;

use arrow::{ArrowParts, ComponentRoseArrow};

use super::SkinParam;
use super::arrow::{ArrowConfiguration, ArrowDirection};
use super::component::{ArrowComponent, TextualPart, component_text, with_margin};
use crate::creole::Display;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// A message's label: its number, if numbered, and its text.
pub(crate) struct MessageLabel<'a> {
    pub number: Option<&'a str>,
    pub display: &'a Display,
}

/// `Display.create0` for a message label: a number goes left of the text, both centred vertically.
fn message_text(label: &MessageLabel<'_>, style: &Style) -> Box<dyn TextBlock> {
    let font = style.font_configuration();
    let text = component_text(label.display, font.clone(), style);
    let Some(number) = label.number else {
        return text;
    };
    let number = component_text(&Display::create([number]), font, style);
    Box::new(TextBlockHorizontal {
        left: with_margin(
            number,
            ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 4.0, 0.0, 0.0),
        ),
        right: text,
    })
}

/// Two blocks side by side, centred vertically (`TextBlockUtils.mergeLR`).
struct TextBlockHorizontal {
    left: Box<dyn TextBlock>,
    right: Box<dyn TextBlock>,
}

impl TextBlock for TextBlockHorizontal {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let left = self.left.calculate_dimension(string_bounder);
        let right = self.right.calculate_dimension(string_bounder);
        XDimension2D::new(left.width + right.width, left.height.max(right.height))
    }

    fn draw_u(&self, ug: &UGraphic) {
        let total = self.calculate_dimension(ug.string_bounder());
        let mut x = 0.0;
        for block in [&self.left, &self.right] {
            let dimension = block.calculate_dimension(ug.string_bounder());
            block.draw_u(&ug.translated(x, (total.height - dimension.height) / 2.0));
            x += dimension.width;
        }
    }
}

/// `Rose.createComponentArrow`: where the label goes follows the arrow style's alignment, which can depend
/// on the arrow's direction.
pub(crate) fn create_component_arrow(
    style: &Style,
    configuration: &ArrowConfiguration,
    skin: &SkinParam,
    label: &MessageLabel<'_>,
) -> Box<dyn ArrowComponent> {
    let text = TextualPart::new(
        message_text(label, style),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 7.0, 1.0, 7.0),
    );
    let parts = ArrowParts::new(text, style.clone(), configuration);
    if configuration.arrow_direction() == ArrowDirection::SelfArrow {
        todo!("self messages")
    }
    let text_style = skin
        .merged_style(&StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::SequenceDiagram,
            SName::Arrow,
        ]))
        .expect("the skin styles arrows");
    let direction = configuration.arrow_direction();
    let reverse_define = configuration.is_reverse_define();
    let alignment = match text_style
        .value(PName::HorizontalAlignment)
        .as_string()
        .to_lowercase()
        .as_str()
    {
        "first" => {
            let leftward = direction == ArrowDirection::RightToLeftReverse;
            Some(if leftward == reverse_define {
                HorizontalAlignment::Left
            } else {
                HorizontalAlignment::Right
            })
        }
        "direction" => match direction {
            ArrowDirection::LeftToRightNormal => Some(HorizontalAlignment::Left),
            ArrowDirection::RightToLeftReverse => Some(HorizontalAlignment::Right),
            ArrowDirection::BothDirection => Some(HorizontalAlignment::Center),
            ArrowDirection::SelfArrow => text_style.horizontal_alignment(),
        },
        "reversedirection" => match direction {
            ArrowDirection::LeftToRightNormal => Some(HorizontalAlignment::Right),
            ArrowDirection::RightToLeftReverse => Some(HorizontalAlignment::Left),
            ArrowDirection::BothDirection => Some(HorizontalAlignment::Center),
            ArrowDirection::SelfArrow => text_style.horizontal_alignment(),
        },
        _ => text_style.horizontal_alignment(),
    };
    Box::new(ComponentRoseArrow::new(
        parts,
        alignment,
        !skin.strict_uml_style(),
        skin.response_message_below_arrow(),
    ))
}
