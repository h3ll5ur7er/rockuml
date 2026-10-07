//! Notes on their own, on entities and on links, tips on members, and constraints on links (PlantUML's
//! `command.note` package).

use std::sync::LazyLock;

use regex::Regex;

use crate::abel::{CucaNote, EntityId, LeafType, LinkArg, Position};
use crate::color::{self, ColorType, Colors, HColor};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, ParserPass, PatternCommand,
    SingleLine,
};
use crate::creole::Display;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::cuca::{CucaDiagram, EntityDiagram};
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::plasma::QuarkId;
use crate::stereo::{self, Stereotag, Stereotype};
use crate::text::{LineLocation, unquoted};

static END_NOTE: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^[%s]*end[%s]?note$"));
static END_NOTE_ON_ENTITY: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex("^[%s]*(end[%s]?note)$"));
static END_WITH_BRACKET: LazyLock<Regex> = LazyLock::new(|| plantuml_regex(r"^(\})$"));
/// Notes on links end without leading spaces.
pub(in crate::diagram) static END_NOTE_ON_LINK: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex("^end[%s]?note$"));

/// A single-line command that runs in `pass`.
fn single_line<D: 'static>(
    pattern: RegexTree,
    pass: ParserPass,
    apply: impl Fn(&mut D, &LineLocation, &RegexResult) -> CommandResult + 'static,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(
        PatternCommand::new(pattern, apply).in_passes(pass.alone()),
    ))
}

/// A block from a line `start` matches to one `end` matches, run in `pass` on the first line's groups and
/// the lines in between as a label.
fn multi_line<D: 'static>(
    start: impl Fn() -> RegexTree,
    end: &Regex,
    pass: ParserPass,
    apply: impl Fn(&mut D, Option<&LineLocation>, &RegexResult, Display) -> CommandResult + 'static,
) -> Box<dyn Command<D>> {
    let first_line = start();
    Box::new(
        Multiline::starting_with_owned(start(), end, move |diagram: &mut D, lines: &BlocLines| {
            let first = lines.first().expect("a block has its first line").trimmed();
            let arg = first_line
                .matcher(first.text())
                .expect("the block's first line matched");
            let body = lines.sub_extract(1, 1).without_empty_columns();
            let location = body.first().map(|line| line.location().clone());
            apply(diagram, location.as_ref(), &arg, body.to_display())
        })
        .in_passes(pass.alone()),
    )
}

/// The colours a `COLOR` specification gives, the main one painting the background
/// (`ColorParser.simpleColor(ColorType.BACK).getColor`).
fn colors(arg: &RegexResult) -> Result<Colors, CommandError> {
    arg.get("COLOR", 0)
        .map(|data| Colors::parse(data, ColorType::Back).map_err(|_| CommandError::bad_color()))
        .transpose()
        .map(Option::unwrap_or_default)
}

/// `$tag1 $tag2` on an entity (`CommandCreateClassMultilines.addTags`).
fn add_tags(cuca: &mut CucaDiagram, entity: EntityId, tags: Option<&str>) {
    for tag in tags.into_iter().flat_map(|tags| tags.split(' ')) {
        if let Some(name) = tag.strip_prefix('$') {
            cuca.entity_mut(entity).add_stereotag(Stereotag {
                name: name.to_owned(),
            });
        }
    }
}

fn note_head(single_line: bool) -> Vec<RegexTree> {
    let mut pattern = vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
    ];
    if single_line {
        pattern.extend([
            RegexTree::leaf("[%g]"),
            RegexTree::named(1, "DISPLAY", "([^%g]+)"),
            RegexTree::leaf("[%g]"),
            RegexTree::spaces_one_or_more(),
        ]);
    }
    pattern.extend([
        RegexTree::leaf("as"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "CODE", "([%pLN_.]+)"),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS"),
        stereo::optional_pattern("STEREO"),
        color::optional_pattern("COLOR"),
        RegexTree::end(),
    ]);
    pattern
}

/// `note "text" as N1` (`CommandFactoryNote.createSingleLine`).
pub(in crate::diagram) fn note<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    single_line(
        RegexTree::concat(note_head(true)),
        ParserPass::One,
        |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            let display = Display::with_newlines(arg.get("DISPLAY", 0).unwrap_or_default());
            create_note(diagram.cuca(), Some(location), arg, display)
        },
    )
}

/// `note as N1` ... `end note` (`CommandFactoryNote.createMultiLine`).
pub(in crate::diagram) fn note_multi_line<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    multi_line(
        || RegexTree::concat(note_head(false)),
        &END_NOTE,
        ParserPass::One,
        |diagram: &mut D, location, arg, display| {
            create_note(diagram.cuca(), location, arg, display)
        },
    )
}

/// A note of its own, which links can then reach by its code (`CommandFactoryNote.executeInternal`).
fn create_note(
    cuca: &mut CucaDiagram,
    location: Option<&LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandResult {
    let id_short = arg.get("CODE", 0).unwrap_or_default();
    let quark = cuca.quark_in_context(false, CucaDiagram::clean_id(id_short));
    if cuca.quark(quark).get_data().is_some() {
        return Err(CommandError::new(format!(
            "Note already created: {}",
            cuca.quark(quark).get_name()
        )));
    }
    let entity = cuca.really_create_leaf(location, quark, display, LeafType::Note);
    let back = arg
        .get("COLOR", 0)
        .map(|color| {
            HColor::parse(color)
                .ok()
                .flatten()
                .ok_or_else(CommandError::bad_color)
        })
        .transpose()?;
    let note = cuca.entity_mut(entity);
    note.colors = note.colors.with(ColorType::Back, back);
    if let Some(stereotype) = arg.get("STEREO", 0) {
        note.stereotype = Some(Stereotype::new(stereotype));
    }
    add_tags(cuca, entity, arg.get("TAGS", 0));
    Ok(())
}

/// What notes on entities start with: the position and the entity named by `code`, or the last entity.
fn note_on_entity_head(code: RegexTree) -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", "(right|left|top|bottom)"),
        RegexTree::or(vec![
            RegexTree::concat(vec![
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf("of"),
                RegexTree::spaces_one_or_more(),
                code,
            ]),
            RegexTree::leaf(""),
        ]),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS1"),
        stereo::optional_pattern("STEREO"),
        stereo::tags_pattern("TAGS2"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
    ]
}

/// `note left of A : text`, run in `pass` (`CommandFactoryNoteOnEntity.createSingleLine`). The single line
/// ignores its link.
pub(in crate::diagram) fn note_on_entity<D: EntityDiagram + 'static>(
    code: fn() -> RegexTree,
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    let mut pattern = note_on_entity_head(code());
    pattern.extend([
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "NOTE", "(.*)"),
        RegexTree::end(),
    ]);
    single_line(
        RegexTree::concat(pattern),
        pass,
        |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            let display = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
            add_note_on_entity(diagram.cuca(), Some(location), arg, None, display)
        },
    )
}

/// `note left of A` ... `end note`, or `note left of A {` ... `}` `with_bracket`, run in `pass`
/// (`CommandFactoryNoteOnEntity.createMultiLine`).
pub(in crate::diagram) fn note_on_entity_multi_line<D: EntityDiagram + 'static>(
    code: fn() -> RegexTree,
    pass: ParserPass,
    with_bracket: bool,
) -> Box<dyn Command<D>> {
    multi_line(
        move || block_start(note_on_entity_head(code()), with_bracket),
        block_end(with_bracket),
        pass,
        |diagram: &mut D, location, arg, display| {
            let url = arg.get("URL", 0).and_then(Url::parse);
            add_note_on_entity(diagram.cuca(), location, arg, url, display)
        },
    )
}

/// The first line of a block on an entity, which may open with `{`.
fn block_start(mut head: Vec<RegexTree>, with_bracket: bool) -> RegexTree {
    if with_bracket {
        head.extend([RegexTree::spaces_zero_or_more(), RegexTree::leaf(r"\{")]);
    }
    head.push(RegexTree::end());
    RegexTree::concat(head)
}

/// The last line of a block on an entity: `}` if it opened with `{`, else `end note`.
fn block_end(with_bracket: bool) -> &'static Regex {
    if with_bracket {
        &END_WITH_BRACKET
    } else {
        &END_NOTE_ON_ENTITY
    }
}

/// A note leaf `GMN<n>` linked to the entity by a dashed line without decorations, on the side the note
/// asks for (`CommandFactoryNoteOnEntity.executeInternal`).
fn add_note_on_entity(
    cuca: &mut CucaDiagram,
    location: Option<&LineLocation>,
    arg: &RegexResult,
    url: Option<Url>,
    display: Display,
) -> CommandResult {
    let target = match arg.get("CODE", 0) {
        None => cuca
            .get_last_entity()
            .ok_or_else(|| CommandError::new("Nothing to note to"))?,
        Some(code) => {
            let id_short = CucaDiagram::clean_id(code);
            let quark = cuca.quark_in_context(true, id_short);
            cuca.quark(quark)
                .get_data()
                .ok_or_else(|| CommandError::new(format!("Not known: {id_short}")))?
        }
    };
    let position = Position::from_string(arg.get("POSITION", 0).unwrap_or_default())
        .expect("the pattern only matches positions");
    let colors = colors(arg)?;
    let tmp = cuca.get_unique_sequence("GMN");
    let quark = cuca.quark_in_context(true, &tmp);
    let note = cuca.really_create_leaf(location, quark, display, LeafType::Note);
    let entity = cuca.entity_mut(note);
    if let Some(stereotype) = arg.get("STEREO", 0) {
        entity.stereotype = Some(Stereotype::new(stereotype));
    }
    entity.colors = colors;
    entity.url = url;
    add_tags(cuca, note, arg.get_lazzy("TAGS", 0));
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).go_dashed();
    let (entity1, entity2, length) = match position {
        Position::Right => (target, note, 1),
        Position::Left => (note, target, 1),
        Position::Bottom => (target, note, 2),
        Position::Top => (note, target, 2),
    };
    let link = cuca.new_link(
        location,
        entity1,
        entity2,
        link_type,
        LinkArg::no_display(length),
    );
    if length == 1 {
        cuca.link_mut(link).horizontal_solitary = true;
    }
    cuca.add_link(link);
    Ok(())
}

/// What notes on links start with.
fn note_on_link_head() -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", "(right|left|top|bottom)?"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::counted(1, "(on|of)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("link"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
    ]
}

/// The pattern of `note on link : text`.
pub(in crate::diagram) fn note_on_link_pattern() -> RegexTree {
    let mut pattern = note_on_link_head();
    pattern.extend([
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(":"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, "NOTE", "(.*)"),
        RegexTree::end(),
    ]);
    RegexTree::concat(pattern)
}

/// The pattern of the first line of `note on link` ... `end note`.
pub(in crate::diagram) fn note_on_link_multi_line_pattern() -> RegexTree {
    let mut pattern = note_on_link_head();
    pattern.push(RegexTree::end());
    RegexTree::concat(pattern)
}

/// `note on link : text`, run in `pass` (`CommandFactoryNoteOnLink.createSingleLine`).
pub(in crate::diagram) fn note_on_link<D: EntityDiagram + 'static>(
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    single_line(
        note_on_link_pattern(),
        pass,
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let display = Display::with_newlines(arg.get("NOTE", 0).unwrap_or_default());
            add_note_on_link(diagram.cuca(), arg, display)
        },
    )
}

/// `note on link` ... `end note`, run in `pass` (`CommandFactoryNoteOnLink.createMultiLine`).
pub(in crate::diagram) fn note_on_link_multi_line<D: EntityDiagram + 'static>(
    pass: ParserPass,
) -> Box<dyn Command<D>> {
    multi_line(
        note_on_link_multi_line_pattern,
        &END_NOTE_ON_LINK,
        pass,
        |diagram: &mut D, _, arg, display| {
            if display.lines().is_empty() {
                return Err(CommandError::new("No note defined"));
            }
            add_note_on_link(diagram.cuca(), arg, display)
        },
    )
}

/// The note goes on the latest link between entities that are not notes, below it unless told otherwise.
fn add_note_on_link(cuca: &mut CucaDiagram, arg: &RegexResult, display: Display) -> CommandResult {
    let link = cuca
        .get_last_link()
        .ok_or_else(|| CommandError::new("No link defined"))?;
    let position = arg
        .get("POSITION", 0)
        .and_then(Position::from_string)
        .unwrap_or(Position::Bottom);
    let colors = colors(arg)?;
    cuca.link_mut(link).note = Some(CucaNote::build(display, position, colors));
    Ok(())
}

fn tip_head() -> Vec<RegexTree> {
    vec![
        RegexTree::start(),
        RegexTree::leaf("note"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "POSITION", "(right|left)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("of"),
        RegexTree::spaces_one_or_more(),
        // `NameAndCodeParser.codeWithMemberForClass`.
        RegexTree::named(
            2,
            "CODE",
            "([^%s{}%g<>:]+|[%g][^%g]+[%g])::([%g][^%g]+[%g]|[^%s]+)",
        ),
        RegexTree::spaces_zero_or_more(),
        stereo::tags_pattern("TAGS1"),
        stereo::optional_pattern("STEREO"),
        stereo::tags_pattern("TAGS2"),
        RegexTree::spaces_zero_or_more(),
        color::optional_pattern("COLOR"),
        RegexTree::spaces_zero_or_more(),
        Url::optional_pattern(),
    ]
}

/// `note right of A::member` ... `end note`, or `note right of A::member {` ... `}` `with_bracket`
/// (`CommandFactoryTipOnEntity.createMultiLine`). Its tags and link are ignored.
pub(in crate::diagram) fn tip_on_entity_multi_line<D: EntityDiagram + 'static>(
    with_bracket: bool,
) -> Box<dyn Command<D>> {
    multi_line(
        move || block_start(tip_head(), with_bracket),
        block_end(with_bracket),
        ParserPass::One,
        |diagram: &mut D, location, arg, display| add_tip(diagram.cuca(), location, arg, display),
    )
}

/// Tips on one side of an entity share a `TIPS` leaf, linked to it by an invisible link; each member keeps
/// its own tip (`CommandFactoryTipOnEntity.executeInternal`).
fn add_tip(
    cuca: &mut CucaDiagram,
    location: Option<&LineLocation>,
    arg: &RegexResult,
    display: Display,
) -> CommandResult {
    let id_short = arg.get("CODE", 0).unwrap_or_default();
    let member = unquoted(arg.get("CODE", 1).unwrap_or_default()).to_owned();
    let quark = cuca.quark_in_context(true, id_short);
    let target = cuca
        .quark(quark)
        .get_data()
        .ok_or_else(|| CommandError::new("Nothing to note to"))?;
    let position = Position::from_string(arg.get("POSITION", 0).unwrap_or_default())
        .expect("the pattern only matches positions");
    let tmp = format!("{id_short}$$${}", position.name());
    let ident_tip = cuca.quark_in_context(true, unquoted(&tmp));
    let tips = if let Some(tips) = cuca.quark(ident_tip).get_data() {
        tips
    } else {
        create_tips(cuca, location, ident_tip, target, position)
    };
    let colors = colors(arg)?;
    let stereotype = arg.get("STEREO", 0).map(Stereotype::new);
    cuca.entity_mut(tips)
        .put_tip(member, display, colors, stereotype);
    Ok(())
}

/// The `TIPS` leaf for one side of `target`, linked to it invisibly.
fn create_tips(
    cuca: &mut CucaDiagram,
    location: Option<&LineLocation>,
    ident_tip: QuarkId,
    target: EntityId,
    position: Position,
) -> EntityId {
    let tips = cuca.really_create_leaf(
        location,
        ident_tip,
        Display::with_newlines(""),
        LeafType::Tips,
    );
    let link_type = LinkType::new(LinkDecor::None, LinkDecor::None).get_invisible();
    let (entity1, entity2) = if position == Position::Right {
        (target, tips)
    } else {
        (tips, target)
    };
    let link = cuca.new_link(
        location,
        entity1,
        entity2,
        link_type,
        LinkArg::no_display(1),
    );
    cuca.add_link(link);
    tips
}

/// `constraint on links : {xor}` (`CommandConstraintOnLinks`). Only PlantUML's Graphviz layout draws the
/// constraint, so it needs nothing but the two links.
pub(in crate::diagram) fn constraint_on_links<D: EntityDiagram + 'static>() -> Box<dyn Command<D>> {
    single_line(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("constraint"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("on"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("links"),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "NOTE", "(.*)"),
            RegexTree::end(),
        ]),
        ParserPass::One,
        |diagram: &mut D, _: &LineLocation, _: &RegexResult| {
            diagram
                .cuca()
                .get_two_last_links()
                .map(|_| ())
                .ok_or_else(|| CommandError::new("Cannot put constraint on two last links"))
        },
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod image_tests;
