//! PlantUML's default look of sequence diagrams (`skin.rose`): one component per kind of element.

pub(crate) mod actor;
pub(crate) mod arrow;
pub(crate) mod life;
pub(crate) mod line;
pub(crate) mod note;
pub(crate) mod participant;
pub(crate) mod queue;
pub(crate) mod self_arrow;

use arrow::{ArrowParts, ComponentRoseArrow};
use self_arrow::ComponentRoseSelfArrow;

use super::SkinParam;
use super::arrow::{ArrowConfiguration, ArrowDirection};
use super::component::{ArrowComponent, TextualPart, component_text, creole_text, with_margin};
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
    let Some(number) = label.number else {
        return component_text(label.display, font, style);
    };
    // Only a whole label of one empty line takes no room; the text after a number is always creole.
    let alignment = label
        .display
        .natural_alignment()
        .or_else(|| style.horizontal_alignment())
        .unwrap_or_default();
    let number = creole_text(&[number.to_owned()], font.clone(), alignment);
    Box::new(TextBlockHorizontal {
        left: with_margin(
            number,
            ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 4.0, 0.0, 0.0),
        ),
        right: creole_text(label.display.lines(), font, alignment),
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

/// The label and style every message arrow shares.
fn arrow_parts(
    style: &Style,
    configuration: &ArrowConfiguration,
    label: &MessageLabel<'_>,
) -> ArrowParts {
    let text = TextualPart::new(
        message_text(label, style),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 7.0, 1.0, 7.0),
    );
    ArrowParts::new(text, style.clone(), configuration)
}

/// `Rose.createComponentArrow` for a message to the sender itself.
pub(crate) fn create_component_self_arrow(
    style: &Style,
    configuration: &ArrowConfiguration,
    skin: &SkinParam,
    label: &MessageLabel<'_>,
) -> ComponentRoseSelfArrow {
    ComponentRoseSelfArrow::new(
        arrow_parts(style, configuration, label),
        !skin.strict_uml_style(),
    )
}

/// `Rose.createComponentArrow`: where the label goes follows the arrow style's alignment, which can depend
/// on the arrow's direction.
pub(crate) fn create_component_arrow(
    style: &Style,
    configuration: &ArrowConfiguration,
    skin: &SkinParam,
    label: &MessageLabel<'_>,
) -> Box<dyn ArrowComponent> {
    if configuration.arrow_direction() == ArrowDirection::SelfArrow {
        return Box::new(create_component_self_arrow(
            style,
            configuration,
            skin,
            label,
        ));
    }
    let parts = arrow_parts(style, configuration, label);
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

/// The three shapes of notes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteShape {
    Folded,
    Hexagonal,
    Box,
}

/// `Rose.createComponentNote`. Notes over several participants centre their text unless an alignment is
/// set; `over_several` is only known for notes not attached to a message.
pub(crate) fn create_component_note(
    style: &Style,
    shape: NoteShape,
    skin: &SkinParam,
    display: &Display,
    colors: &crate::color::Colors,
    over_several: bool,
) -> Box<dyn super::component::Component> {
    use note::{ComponentRoseNote, ComponentRoseNoteBox, ComponentRoseNoteHexagonal};
    let fashion = style.symbol_context(colors);
    let font = style.font_configuration();
    let alignment = style.horizontal_alignment().unwrap_or_default();
    let margin = ClockwiseTopRightBottomLeft::top_right_bottom_left;
    match shape {
        NoteShape::Folded => {
            let (text_alignment, position) = if over_several {
                let text_alignment = skin.note_text_alignment(HorizontalAlignment::Left);
                let position =
                    if text_alignment == skin.note_text_alignment(HorizontalAlignment::Center) {
                        text_alignment
                    } else {
                        HorizontalAlignment::Center
                    };
                (text_alignment, position)
            } else {
                let text_alignment = skin.note_text_alignment(HorizontalAlignment::Left);
                (text_alignment, text_alignment)
            };
            let padding = if text_alignment == HorizontalAlignment::Center {
                margin(5.0, 15.0, 5.0, 15.0)
            } else {
                margin(5.0, 15.0, 5.0, 6.0)
            };
            let text = if is_single_empty_line(display) {
                Box::new(super::component::TextBlockEmpty {
                    dimension: XDimension2D::default(),
                }) as Box<dyn TextBlock>
            } else {
                super::body::enhanced_text(display, font, alignment, style)
            };
            Box::new(ComponentRoseNote::new(
                TextualPart::new(text, padding),
                fashion,
                Some(position),
            ))
        }
        NoteShape::Hexagonal => Box::new(ComponentRoseNoteHexagonal::new(
            TextualPart::new(
                component_text(display, font, style),
                margin(4.0, 12.0, 4.0, 12.0),
            ),
            fashion,
        )),
        NoteShape::Box => Box::new(ComponentRoseNoteBox::new(
            TextualPart::new(
                component_text(display, font, style),
                margin(4.0, 4.0, 4.0, 4.0),
            ),
            fashion,
        )),
    }
}

fn is_single_empty_line(display: &Display) -> bool {
    display.lines().len() == 1 && display.lines()[0].is_empty()
}
