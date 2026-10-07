//! The components drawing each part of the diagram, with the styles PlantUML gives them (the
//! `createComponent` calls of PlantUML's teoz tiles).

use crate::color::Colors;
use crate::creole::Display;
use crate::diagram::sequence::SequenceDiagram;
use crate::diagram::sequence::model::{
    GroupingLeaf, GroupingStart, Labelled, LiveColors, MessageCommon, Note, NoteStyle,
    ParticipantId, ParticipantType, Reference,
};
use crate::diagram::sequence::styles::{
    englober_style, grouping_start_styles, grouping_styles, merged, merged_with_stereotype,
    message_style, note_style, participant_styles, reference_styles, sequence_signature,
    sequence_signature2,
};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::arrow::ArrowConfiguration;
use crate::skin::component::{
    ArrowComponent, Component, TextBlockEmpty, TextualPart, component_text, creole_text,
    stereotyped_component_text,
};
use crate::skin::rose::actor::ComponentRoseActor;
use crate::skin::rose::englober::ComponentRoseEnglober;
use crate::skin::rose::grouping::{ComponentRoseGroupingElse, ComponentRoseGroupingHeader};
use crate::skin::rose::life::{
    ComponentRoseActiveLine, ComponentRoseDelayLine, ComponentRoseDestroy,
};
use crate::skin::rose::line::ComponentRoseLine;
use crate::skin::rose::participant::ComponentRoseParticipant;
use crate::skin::rose::queue::ComponentRoseQueue;
use crate::skin::rose::reference::ComponentRoseReference;
use crate::skin::rose::separators::{
    ComponentRoseDelayText, ComponentRoseDivider, ComponentRoseNewpage,
};
use crate::skin::rose::{self, MessageLabel, NoteShape};
use crate::skin::symbol::{Boundary, Control, EntityDomain, SmallDatabase, SmallQueue};
use crate::style::{PName, SName, StyleBuilder, ValueReading};

/// The display of a participant as its boxes show it: underlined if the skin asks for it, with its
/// stereotype.
fn participant_display(diagram: &SequenceDiagram, participant: ParticipantId) -> Display {
    let model = diagram.participant(participant);
    let display = if diagram.skin().force_sequence_participant_underlined() {
        Display::create(
            model
                .display
                .lines()
                .iter()
                .map(|line| format!("<u>{line}")),
        )
    } else {
        model.display.clone()
    };
    match &model.stereotype {
        Some((stereotype, top)) => display.with_stereotype(stereotype.clone(), *top),
        None => display,
    }
}

/// A participant's head (`head`) or tail, whose name goes below or above a symbol.
pub(super) fn participant_component(
    diagram: &SequenceDiagram,
    participant: ParticipantId,
    head: bool,
) -> Box<dyn Component> {
    let model = diagram.participant(participant);
    let (style, stereo) = participant_styles(model);
    let display = participant_display(diagram, participant);
    let text_block =
        stereotyped_component_text(&display, style.font_configuration(), &style, &stereo, 0.0);
    let fashion = style.symbol_context(&Colors::default());
    match model.kind {
        ParticipantType::Participant | ParticipantType::Collections => {
            Box::new(ComponentRoseParticipant::new(
                TextualPart::new(text_block, style.padding()),
                fashion,
                style.value(PName::MinimumWidth).as_double(),
                model.kind == ParticipantType::Collections,
                style.margin(),
            ))
        }
        ParticipantType::Queue => Box::new(ComponentRoseQueue::new(Box::new(SmallQueue::new(
            text_block, fashion,
        )))),
        ParticipantType::Actor => with_symbol(
            text_block,
            diagram.skin().actor_style().text_block(fashion),
            head,
        ),
        ParticipantType::Boundary => {
            with_symbol(text_block, Box::new(Boundary::new(fashion)), head)
        }
        ParticipantType::Control => with_symbol(text_block, Box::new(Control::new(fashion)), head),
        ParticipantType::Entity => {
            with_symbol(text_block, Box::new(EntityDomain::new(fashion)), head)
        }
        ParticipantType::Database => {
            let room = Box::new(TextBlockEmpty {
                dimension: XDimension2D::new(16.0, 17.0),
            });
            with_symbol(
                text_block,
                Box::new(SmallDatabase::new(room, fashion)),
                head,
            )
        }
    }
}

fn with_symbol(
    text_block: Box<dyn TextBlock>,
    symbol: Box<dyn TextBlock>,
    head: bool,
) -> Box<dyn Component> {
    let text = TextualPart::new(
        text_block,
        ClockwiseTopRightBottomLeft::top_right_bottom_left(0.0, 3.0, 0.0, 3.0),
    );
    Box::new(ComponentRoseActor::new(text, symbol, head))
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
    Box::new(ComponentRoseLine::new(
        style.value(PName::LineColor).as_color(),
        style.stroke(),
        &participant_display(diagram, participant).tooltip_text(),
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
        style.symbol_context(&Colors::default()),
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

/// The box around the participants of an englober, with its title.
pub(super) fn englober(diagram: &SequenceDiagram, englober: usize) -> ComponentRoseEnglober {
    let model = diagram.englober(englober);
    let style = englober_style(&diagram.style_builder(), model);
    let text = TextualPart::new(
        component_text(&model.title, style.font_configuration(), &style, 0.0),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 3.0, 1.0, 3.0),
    );
    ComponentRoseEnglober::new(
        text,
        style.symbol_context(&Colors::default()),
        f64::from(style.value(PName::RoundCorner).as_int()),
    )
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

/// The unlabelled copy of a message's arrow that goes to each further receiver.
pub(super) fn multicast_arrow(
    diagram: &SequenceDiagram,
    common: &MessageCommon,
    configuration: &ArrowConfiguration,
) -> Box<dyn ArrowComponent> {
    rose::create_component_arrow(
        &message_style(common),
        configuration,
        diagram.skin(),
        &MessageLabel {
            number: None,
            display: &Display::default(),
        },
    )
}

/// A note, in the shape its style asks for unless `folded` forces the folded corner, as some placements
/// do in PlantUML. Only notes not attached to a message know whether they are over several participants.
pub(super) fn note(
    diagram: &SequenceDiagram,
    note: &Note,
    folded: bool,
    over_several: bool,
) -> Box<dyn Component> {
    let shape = match note.style {
        _ if folded => NoteShape::Folded,
        NoteStyle::Normal => NoteShape::Folded,
        NoteStyle::Hexagonal => NoteShape::Hexagonal,
        NoteStyle::Box => NoteShape::Box,
    };
    rose::create_component_note(
        &note_style(note),
        shape,
        diagram.skin(),
        &note.display,
        &note.colors,
        over_several,
    )
}

pub(super) fn divider(divider: &Labelled) -> Box<dyn Component> {
    let style = merged(
        &divider.style_builder,
        &sequence_signature(SName::Separator),
    );
    let display = &divider.display;
    let empty = display.lines().first().is_none_or(String::is_empty);
    Box::new(ComponentRoseDivider::new(
        TextualPart::new(
            component_text(display, style.font_configuration(), &style, 0.0),
            ClockwiseTopRightBottomLeft::same(4.0),
        ),
        style.value(PName::BackGroundColor).as_color(),
        style.value(PName::LineColor).as_color(),
        style.stroke(),
        f64::from(style.value(PName::RoundCorner).as_int()),
        empty,
    ))
}

pub(super) fn delay_text(delay: &Labelled) -> Box<dyn Component> {
    let style = merged(
        &delay.style_builder,
        &sequence_signature2(SName::LifeLine, SName::Delay),
    );
    Box::new(ComponentRoseDelayText::new(TextualPart::new(
        component_text(&delay.display, style.font_configuration(), &style, 0.0),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(4.0, 0.0, 4.0, 0.0),
    )))
}

pub(super) fn newpage(style_builder: &StyleBuilder) -> Box<dyn Component> {
    let style = merged(style_builder, &sequence_signature(SName::Newpage));
    Box::new(ComponentRoseNewpage::new(
        style.value(PName::LineColor).as_color(),
        style.stroke(),
    ))
}

/// A reference frame: `ref` in the tab, the text below.
pub(super) fn reference(reference: &Reference) -> Box<dyn Component> {
    let (style, header_style) = reference_styles(reference);
    let header = creole_text(
        &["ref".to_owned()],
        header_style.font_configuration(),
        HorizontalAlignment::Left,
        0.0,
    );
    Box::new(ComponentRoseReference::new(
        TextualPart::new(
            component_text(&reference.display, style.font_configuration(), &style, 0.0),
            ClockwiseTopRightBottomLeft::same(4.0),
        ),
        header,
        style.symbol_context(&Colors::default()),
        header_style.symbol_context(&Colors::default()),
        style.horizontal_alignment(),
    ))
}

/// A group's frame with its title tab (`GROUPING_HEADER_TEOZ`): a plain `group` shows only its comment.
pub(super) fn grouping_header(start: &GroupingStart) -> Box<dyn Component> {
    let (style, header) = grouping_start_styles(start);
    let (title, comment) = if start.title == "group" {
        (start.comment.as_deref(), None)
    } else {
        (Some(start.title.as_str()), start.comment.as_deref())
    };
    let title = Display::with_newlines(title.unwrap_or_default());
    let text = TextualPart::new(
        component_text(&title, header.font_configuration(), &header, 0.0),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 30.0, 1.0, 15.0),
    );
    let comment = comment.map(|comment| {
        let display = Display::with_newlines(&format!("[{comment}]"));
        creole_text(
            display.lines(),
            style.font_configuration(),
            display
                .natural_alignment()
                .unwrap_or(HorizontalAlignment::Left),
            0.0,
        )
    });
    Box::new(ComponentRoseGroupingHeader::new(
        text,
        comment,
        style.symbol_context(&Colors::default()),
        header.symbol_context(&Colors::default()),
        f64::from(style.value(PName::RoundCorner).as_int()),
    ))
}

/// The dashed line where an `else` starts (`GROUPING_ELSE_TEOZ`).
pub(super) fn grouping_else(leaf: &GroupingLeaf) -> Box<dyn Component> {
    // Only the line and the text are drawn, so the background colours the leaf overrides do not matter.
    let (style, _) = grouping_styles(&leaf.style_builder, leaf.kind, None, None);
    let label = leaf
        .comment
        .as_ref()
        .map(|comment| format!("[{comment}]"))
        .unwrap_or_default();
    let text = TextualPart::new(
        component_text(
            &Display::with_newlines(&label),
            style.font_configuration(),
            &style,
            0.0,
        ),
        ClockwiseTopRightBottomLeft::top_right_bottom_left(1.0, 5.0, 1.0, 5.0),
    );
    Box::new(ComponentRoseGroupingElse::new(
        text,
        style.value(PName::LineColor).as_color(),
    ))
}
