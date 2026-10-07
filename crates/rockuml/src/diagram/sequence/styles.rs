//! The styles sequence elements are drawn with (the `getUsedStyles` of PlantUML's sequence classes).

use crate::color::HColor;
use crate::stereo::Stereotype;
use crate::style::{PName, SName, Style, StyleBuilder, StyleSignature};

use super::model::{
    GroupingStart, GroupingType, MessageCommon, Note, NoteStyle, Participant, ParticipantEnglober,
    ParticipantType,
};

/// `root, element, sequenceDiagram`: what every sequence element inherits.
pub(crate) fn sequence_signature_root() -> StyleSignature {
    StyleSignature::of(&[SName::Root, SName::Element, SName::SequenceDiagram])
}

pub(crate) fn sequence_signature(name: SName) -> StyleSignature {
    StyleSignature::of(&[SName::Root, SName::Element, SName::SequenceDiagram, name])
}

pub(crate) fn sequence_signature2(name: SName, child: SName) -> StyleSignature {
    sequence_signature(name).with_name(child)
}

pub(crate) fn merged(builder: &StyleBuilder, signature: &StyleSignature) -> Style {
    signature.get_merged_style(builder)
}

/// The style of an element with a stereotype: the stereotype's rules for each of its labels, merged
/// (`withTOBECHANGED` and `StyleSignatures.getMergedStyle`).
pub(crate) fn merged_with_stereotype(
    builder: &StyleBuilder,
    signature: &StyleSignature,
    stereotype: Option<&Stereotype>,
) -> Style {
    signature.get_merged_style_with(builder, stereotype)
}

fn participant_signature(kind: ParticipantType) -> StyleSignature {
    sequence_signature(match kind {
        ParticipantType::Participant => SName::Participant,
        ParticipantType::Actor => SName::Actor,
        ParticipantType::Boundary => SName::Boundary,
        ParticipantType::Control => SName::Control,
        ParticipantType::Entity => SName::Entity,
        ParticipantType::Queue => SName::Queue,
        ParticipantType::Database => SName::Database,
        ParticipantType::Collections => SName::Collections,
    })
}

/// A participant's style, and the style of its stereotype.
pub(crate) fn participant_styles(participant: &Participant) -> (Style, Style) {
    let mut signature = participant_signature(participant.kind);
    if participant.url.is_some() {
        signature = signature.with_name(SName::Clickable);
    }
    let stereotype = participant
        .stereotype
        .as_ref()
        .map(|(stereotype, _)| stereotype);
    let builder = &participant.style_builder;
    let style = merged_with_stereotype(builder, &signature, stereotype)
        .eventually_override_colors(&participant.colors);
    let stereo = signature.get_merged_style_for_stereotype_itself(builder, stereotype);
    let stereo = style.merge_with(&stereo);
    (style, stereo)
}

/// A message's style, in the arrow's own colour if it has one.
pub(crate) fn message_style(common: &MessageCommon) -> Style {
    merged_with_stereotype(
        &common.style_builder,
        &sequence_signature(SName::Arrow),
        common.stereotype.as_ref(),
    )
    .eventually_override(PName::LineColor, common.arrow_configuration.color())
}

pub(crate) fn note_style(note: &Note) -> Style {
    let signature = match note.style {
        NoteStyle::Hexagonal => sequence_signature2(SName::Note, SName::Hnote),
        NoteStyle::Box => sequence_signature2(SName::Note, SName::Rnote),
        NoteStyle::Normal => sequence_signature(SName::Note),
    };
    let signature = match &note.stereotype {
        Some(stereotype) => stereotype
            .style_names()
            .iter()
            .fold(signature, |signature, name| signature.with_stereotype(name)),
        None => signature,
    };
    merged(&note.style_builder, &signature).eventually_override_colors(&note.colors)
}

/// A box around participants, in its own colour if it has one (`Doll.getUsedStyles`).
pub(crate) fn englober_style(builder: &StyleBuilder, englober: &ParticipantEnglober) -> Style {
    merged_with_stereotype(
        builder,
        &sequence_signature(SName::Box),
        englober.stereotype.as_ref(),
    )
    .eventually_override(PName::BackGroundColor, englober.box_color.as_ref())
}

/// A group's frame style and its header's.
pub(crate) fn grouping_styles(
    builder: &StyleBuilder,
    kind: GroupingType,
    back_color_general: Option<&HColor>,
    back_color_element: Option<&HColor>,
) -> (Style, Style) {
    let (style, header) = if kind == GroupingType::StartPartition {
        let style = merged(builder, &sequence_signature(SName::Partition));
        let header = merged(
            builder,
            &sequence_signature2(SName::Partition, SName::Header),
        );
        (style, header)
    } else {
        let style = merged(builder, &sequence_signature(SName::Group));
        let flat = merged(builder, &sequence_signature(SName::GroupHeader));
        let nested = merged(builder, &sequence_signature2(SName::Group, SName::Header));
        (style.clone(), flat.merge_nested_child_over(&nested, &style))
    };
    (
        style.eventually_override(PName::BackGroundColor, back_color_general),
        header.eventually_override(PName::BackGroundColor, back_color_element),
    )
}

/// `GroupingStart.getUsedStyles`, whose frame also takes the start's own background.
pub(crate) fn grouping_start_styles(start: &GroupingStart) -> (Style, Style) {
    let (style, header) = grouping_styles(
        &start.style_builder,
        start.kind,
        start.back_color_general.as_ref(),
        start.back_color_element.as_ref(),
    );
    (
        style.eventually_override(PName::BackGroundColor, start.back_color_general.as_ref()),
        header,
    )
}

/// A reference's frame style and its tab's (`Reference.getUsedStyles`).
pub(crate) fn reference_styles(reference: &super::model::Reference) -> (Style, Style) {
    let builder = &reference.style_builder;
    let style = merged(builder, &sequence_signature(SName::Reference));
    let flat = merged(builder, &sequence_signature(SName::ReferenceHeader));
    let nested = merged(
        builder,
        &sequence_signature2(SName::Reference, SName::Header),
    );
    let header = flat
        .merge_nested_child_over(&nested, &style)
        .eventually_override(
            PName::BackGroundColor,
            reference.back_color_element.as_ref(),
        );
    (style, header)
}
