//! PlantUML's default look of sequence diagrams (`skin.rose`): one component per kind of element.

pub(crate) mod actor;
pub(crate) mod arrow;
pub(crate) mod englober;
pub(crate) mod grouping;
pub(crate) mod life;
pub(crate) mod line;
pub(crate) mod note;
pub(crate) mod participant;
pub(crate) mod queue;
pub(crate) mod reference;
pub(crate) mod self_arrow;
pub(crate) mod separators;

use arrow::{ArrowParts, ComponentRoseArrow};
use note::{ComponentRoseNote, ComponentRoseNoteBox, ComponentRoseNoteHexagonal};
use self_arrow::ComponentRoseSelfArrow;

use super::SkinParam;
use super::arrow::{ArrowConfiguration, ArrowDirection};
use super::body::enhanced_text;
use super::component::{
    ArrowComponent, Component, TextBlockEmpty, TextualPart, component_text, creole_text,
    with_margin,
};
use crate::color::Colors;
use crate::creole::Display;
use crate::klimt::blocks::TextBlockHorizontal;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::style::{PName, SName, Style, StyleSignature, ValueReading};

/// A message's label: its number, if numbered, and its text.
pub(crate) struct MessageLabel<'a> {
    pub number: Option<&'a str>,
    pub display: &'a Display,
}

/// `Display.create0` for a message label: a number goes left of the text, both centred vertically.
fn message_text(
    label: &MessageLabel<'_>,
    style: &Style,
    max_width: f64,
    skin: &SkinParam,
) -> Box<dyn TextBlock> {
    let font = style.font_configuration();
    let Some(number) = label.number else {
        return component_text(label.display, font, style, max_width, skin);
    };
    // Only a whole label of one empty line takes no room; the text after a number is always creole.
    let alignment = label
        .display
        .natural_alignment()
        .or_else(|| style.horizontal_alignment())
        .unwrap_or_default();
    let number = creole_text(
        &[number.to_owned()],
        font.clone(),
        alignment,
        max_width,
        skin,
    );
    Box::new(TextBlockHorizontal {
        left: with_margin(
            number,
            ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 4.0, 0.0, 0.0),
        ),
        right: creole_text(label.display.lines(), font, alignment, max_width, skin),
    })
}

/// The label and style every message arrow shares. The style's wrap width wins over `maxMessageSize`.
fn arrow_parts(
    style: &Style,
    configuration: &ArrowConfiguration,
    skin: &SkinParam,
    label: &MessageLabel<'_>,
) -> ArrowParts {
    let max_width = match style.wrap_width() {
        0.0 => skin.max_message_size(),
        style_width => style_width,
    };
    let text = TextualPart::new(
        message_text(label, style, max_width, skin),
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
        arrow_parts(style, configuration, skin, label),
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
    let parts = arrow_parts(style, configuration, skin, label);
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
    colors: &Colors,
    over_several: bool,
) -> Box<dyn Component> {
    let fashion = style.symbol_context(colors);
    let font = style.font_configuration();
    let alignment = style.horizontal_alignment().unwrap_or_default();
    let padding = ClockwiseTopRightBottomLeft::top_right_bottom_left;
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
            let text_padding = if text_alignment == HorizontalAlignment::Center {
                padding(5.0, 15.0, 5.0, 15.0)
            } else {
                padding(5.0, 15.0, 5.0, 6.0)
            };
            let text = if display.is_single_empty_line() {
                Box::new(TextBlockEmpty::default()) as Box<dyn TextBlock>
            } else {
                enhanced_text(display, font, alignment, style, skin)
            };
            Box::new(ComponentRoseNote::new(
                TextualPart::new(text, text_padding),
                fashion,
                position,
            ))
        }
        NoteShape::Hexagonal => Box::new(ComponentRoseNoteHexagonal::new(
            TextualPart::new(
                component_text(display, font, style, style.wrap_width(), skin),
                padding(4.0, 12.0, 4.0, 12.0),
            ),
            fashion,
        )),
        NoteShape::Box => Box::new(ComponentRoseNoteBox::new(
            TextualPart::new(
                component_text(display, font, style, style.wrap_width(), skin),
                padding(4.0, 4.0, 4.0, 4.0),
            ),
            fashion,
        )),
    }
}
