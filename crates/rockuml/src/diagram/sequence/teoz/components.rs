//! The components drawing each part of the diagram, with the styles PlantUML gives them (the
//! `createComponent` calls of PlantUML's teoz tiles).

use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{LiveColors, MessageCommon, ParticipantId, ParticipantType};
use crate::diagram::sequence::styles::{
    merged, merged_with_stereotype, message_style, participant_styles, sequence_signature,
    sequence_signature2,
};
use crate::skin::arrow::ArrowConfiguration;
use crate::skin::component::{ArrowComponent, Component, TextualPart, component_text};
use crate::skin::rose::life::{
    ComponentRoseActiveLine, ComponentRoseDelayLine, ComponentRoseDestroy,
};
use crate::skin::rose::line::ComponentRoseLine;
use crate::skin::rose::participant::ComponentRoseParticipant;
use crate::skin::rose::{self, MessageLabel};
use crate::style::{PName, SName, StyleBuilder, ValueReading};

/// The display of a participant as its boxes show it: underlined if the skin asks for it.
fn participant_display(diagram: &SequenceDiagram, participant: ParticipantId) -> Display {
    let display = &diagram.participant(participant).display;
    if diagram.skin().force_sequence_participant_underlined() {
        Display::create(display.lines().iter().map(|line| format!("<u>{line}")))
    } else {
        display.clone()
    }
}

/// A participant's head (`head`) or tail box.
pub(super) fn participant_component(
    diagram: &SequenceDiagram,
    participant: ParticipantId,
    head: bool,
) -> Box<dyn Component> {
    let model = diagram.participant(participant);
    let (style, _stereo) = participant_styles(model);
    let display = participant_display(diagram, participant);
    let _ = head;
    match model.kind {
        ParticipantType::Participant | ParticipantType::Collections => {
            let padding = style.padding();
            let text = TextualPart::new(
                component_text(&display, style.font_configuration(), &style),
                padding,
            );
            Box::new(ComponentRoseParticipant::new(
                text,
                style.symbol_context(&crate::color::Colors::default()),
                style.value(PName::MinimumWidth).as_double(),
                model.kind == ParticipantType::Collections,
                style.margin(),
            ))
        }
        other => todo!("{other:?} heads"),
    }
}

pub(super) fn lifeline(
    diagram: &SequenceDiagram,
    participant: ParticipantId,
) -> Box<dyn Component> {
    let model = diagram.participant(participant);
    let style = merged_with_stereotype(
        &diagram.style_builder(),
        &sequence_signature(SName::LifeLine),
        model.stereotype.as_ref().map(|(stereotype, _)| stereotype),
    );
    let display = participant_display(diagram, participant);
    let tooltip = display.lines().first().cloned().unwrap_or_default();
    Box::new(ComponentRoseLine::new(
        style.value(PName::LineColor).as_color(),
        style.stroke(),
        &tooltip,
    ))
}

pub(super) fn delay_line(
    diagram: &SequenceDiagram,
    participant: ParticipantId,
) -> Box<dyn Component> {
    let model = diagram.participant(participant);
    let style = merged_with_stereotype(
        &diagram.style_builder(),
        &sequence_signature2(SName::LifeLine, SName::Delay),
        model.stereotype.as_ref().map(|(stereotype, _)| stereotype),
    );
    Box::new(ComponentRoseDelayLine::new(
        style.value(PName::LineColor).as_color(),
        style.stroke(),
    ))
}

/// An activation box, in the colours of the activation if it has any.
pub(super) fn activation_box(
    diagram: &SequenceDiagram,
    participant: ParticipantId,
    style_builder: Option<&StyleBuilder>,
    colors: Option<&LiveColors>,
    close_up: bool,
    close_down: bool,
) -> Box<dyn Component> {
    let model = diagram.participant(participant);
    let current = diagram.style_builder();
    let style = merged_with_stereotype(
        style_builder.unwrap_or(&current),
        &sequence_signature(SName::ActivationBox),
        model.stereotype.as_ref().map(|(stereotype, _)| stereotype),
    )
    .eventually_override(
        PName::BackGroundColor,
        colors.and_then(|colors| colors.back.as_ref()),
    );
    Box::new(ComponentRoseActiveLine::new(
        style.symbol_context(&crate::color::Colors::default()),
        close_up,
        close_down,
    ))
}

pub(super) fn destroy(diagram: &SequenceDiagram) -> Box<dyn Component> {
    let style = merged(
        &diagram.style_builder(),
        &sequence_signature2(SName::LifeLine, SName::Destroy),
    );
    Box::new(ComponentRoseDestroy::new(
        style.value(PName::LineColor).as_color(),
        style.stroke(),
    ))
}

/// A message's arrow, with its number before the label.
pub(super) fn message_arrow(
    diagram: &SequenceDiagram,
    common: &MessageCommon,
    configuration: &ArrowConfiguration,
) -> Box<dyn ArrowComponent> {
    rose::create_component_arrow(
        &message_style(common),
        configuration,
        diagram.skin(),
        &MessageLabel {
            number: common.message_number.as_deref(),
            display: &common.label,
        },
    )
}
