//! The commands of state diagrams, in the order `StateDiagramFactory` tries them.

use super::StateDiagram;
use crate::abel::{EntityId, GroupType, LeafType, LinkArg};
use crate::color::{self, ColorType, Colors, HColor, NoSuchColor};
use crate::command::unported::NotPortedCommands;
use crate::command::{
    Command, CommandError, CommandResult, ParserPass, SingleLine, SingleLineCommand,
};
use crate::creole::Display;
use crate::decoration::symbol::USymbols;
use crate::decoration::{LinkDecor, LinkType};
use crate::diagram::cuca::EntityDiagram;
use crate::direction::Direction;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree};
use crate::stereo::{self, Stereogroup, Stereotag, Stereotype};
use crate::text::LineLocation;

const ALL_PASSES: &[ParserPass] = &[ParserPass::One, ParserPass::Two, ParserPass::Three];

type Apply = fn(&mut StateDiagram, &LineLocation, &RegexResult) -> CommandResult;

/// A single-line state command, run in some of the passes.
struct StateCommand {
    pattern: RegexTree,
    passes: &'static [ParserPass],
    apply: Apply,
}

impl SingleLineCommand<StateDiagram> for StateCommand {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut StateDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        (self.apply)(diagram, location, arg)
    }

    fn is_eligible_for(&self, pass: ParserPass) -> bool {
        self.passes.contains(&pass)
    }
}

fn command(
    pattern: RegexTree,
    passes: &'static [ParserPass],
    apply: Apply,
) -> Box<dyn Command<StateDiagram>> {
    Box::new(SingleLine(StateCommand {
        pattern,
        passes,
        apply,
    }))
}

fn bad_color(_: NoSuchColor) -> CommandError {
    CommandError::bad_color()
}

/// `##[dashed]blue`: a line style and colour.
fn line_color_pattern() -> RegexTree {
    RegexTree::optional(RegexTree::named(
        2,
        "LINECOLOR",
        r"##(?:\[(dotted|dashed|bold)\])?(\w+)?",
    ))
}

/// The background colour, and the line colour and style `##` sets.
fn colors(arg: &RegexResult) -> Result<Colors, CommandError> {
    let mut colors = arg
        .get("COLOR", 0)
        .map(|color| Colors::parse(color, ColorType::Back))
        .transpose()
        .map_err(bad_color)?
        .unwrap_or_default();
    if let Some(line) = arg.get("LINECOLOR", 1) {
        let color = HColor::parse(line)
            .ok()
            .flatten()
            .ok_or_else(CommandError::bad_color)?;
        colors = colors.with(ColorType::Line, Some(color));
    }
    if let Some(style) = arg.get("LINECOLOR", 0) {
        colors = colors.add_legacy_stroke(style);
    }
    Ok(colors)
}

/// `$tag1 $tag2` (`CommandCreateClassMultilines.addTags`).
fn add_tags(diagram: &mut StateDiagram, entity: EntityId, tags: Option<&str>) {
    for tag in tags.into_iter().flat_map(|tags| tags.split(' ')) {
        if let Some(name) = tag.strip_prefix('$') {
            diagram.cuca.entity_mut(entity).add_stereotag(Stereotag {
                name: name.to_owned(),
            });
        }
    }
}

/// PlantUML's `CommandCreateState`: `state Name`, with a display, stereotypes, colours and a first
/// description line.
pub(super) fn create_state() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"state"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]+)[%g]"),
                ]),
                RegexTree::concat(vec![
                    RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]+)[%g]"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::leaf(r"as"),
                    RegexTree::spaces_one_or_more(),
                    RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
                ]),
                RegexTree::named(1, "CODE3", r"([%pLN_.]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            Stereogroup::optional_pattern(),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            line_color_pattern(),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::leaf(r":"),
                RegexTree::spaces_zero_or_more(),
                RegexTree::named(1, "ADDFIELD", r"(.*)"),
            ])),
            RegexTree::end(),
        ]),
        ALL_PASSES,
        execute_create_state,
    )
}

fn execute_create_state(
    diagram: &mut StateDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let id_short = arg.get_lazzy("CODE", 0).unwrap_or_default();
    let quark = diagram
        .cuca
        .quark_in_context(true, StateDiagram::clean_id(id_short))?;
    let name = diagram.cuca.quark(quark).get_name().to_owned();
    let display = arg.get_lazzy("DISPLAY", 0).unwrap_or(&name).to_owned();
    let stereogroup = Stereogroup::build(arg.get("STEREOGROUP", 0));
    let leaf_type = stereogroup.get_leaf_type().unwrap_or(LeafType::State);
    if !diagram.check_concurrent_state_ok(quark)? {
        return Err(CommandError::new(format!(
            "The state {name} has been created in a concurrent state : it cannot be used here."
        )));
    }
    let ent = match diagram.cuca.quark(quark).get_data() {
        Some(existing) => {
            diagram.cuca.set_last_entity(Some(existing));
            existing
        }
        None => diagram.cuca.really_create_leaf(
            Some(location),
            quark,
            Display::with_newlines(&display),
            leaf_type,
        ),
    };
    diagram.ensure_parent_state(location, quark);
    if diagram.current_pass != ParserPass::One {
        return Ok(());
    }
    let colors = colors(arg)?.merge_with(&stereogroup.get_inner_colors().map_err(bad_color)?);
    let entity = diagram.cuca.entity_mut(ent);
    entity.display = Display::with_newlines(&display);
    entity.stereotype = stereogroup.build_stereotype();
    if let Some(url) = arg.get("URL", 0).and_then(Url::parse) {
        entity.url = Some(url);
    }
    entity.colors = colors;
    if let Some(field) = arg.get("ADDFIELD", 0) {
        entity.bodier.add_field_or_method(field);
    }
    add_tags(diagram, ent, arg.get_lazzy("TAGS", 0));
    let cuca = &diagram.cuca;
    let entity = cuca.entity(ent);
    let in_root = entity
        .get_parent_container(cuca)
        .is_some_and(|parent| cuca.entity(parent).is_root());
    if in_root && !entity.get_entity_position().is_normal() {
        return Err(CommandError::new("You cannot use this stereotype here"));
    }
    Ok(())
}

/// The end of a transition: a state, `[*]`, a history `[H]` or `[H*]`, or a bar `==name==`.
fn state_pattern(name: &'static str) -> RegexTree {
    RegexTree::named(
        1,
        name,
        r"([%pLN_.:]+|[%pLN_.:]+\[H\*?\]|\[\*\]|\[H\*?\]|(?:==+)(?:[%pLN_.:]+)(?:==+))",
    )
}

fn arrow_style_pattern(name: &'static str) -> RegexTree {
    RegexTree::named(
        1,
        name,
        r"(?:\[((?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?",
    )
}

/// What follows the arrow: a stereotype and a label.
fn link_tail() -> [RegexTree; 4] {
    [
        RegexTree::spaces_zero_or_more(),
        stereo::optional_pattern("STEREOTYPE"),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LABEL", r"(.+)"),
        ])),
        RegexTree::end(),
    ]
}

/// PlantUML's `CommandLinkState`: `A --> B : label`.
pub(super) fn link_state() -> Box<dyn Command<StateDiagram>> {
    let mut parts = vec![
        RegexTree::start(),
        state_pattern("ENT1"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::concat(vec![
            RegexTree::named(1, "ARROW_CROSS_START", r"(x)?"),
            RegexTree::named(1, "ARROW_BODY1", r"(-+)"),
            arrow_style_pattern("ARROW_STYLE1"),
            RegexTree::named(
                1,
                "ARROW_DIRECTION",
                r"(left|right|up|down|le?|ri?|up?|do?)?",
            ),
            arrow_style_pattern("ARROW_STYLE2"),
            RegexTree::named(1, "ARROW_BODY2", r"(-*)"),
            RegexTree::leaf(r"\>"),
            RegexTree::named(1, "ARROW_CIRCLE_END", r"(o[%s]+)?"),
        ]),
        RegexTree::spaces_zero_or_more(),
        state_pattern("ENT2"),
    ];
    parts.extend(link_tail());
    command(RegexTree::concat(parts), &[ParserPass::Two], |d, l, a| {
        execute_link(d, l, a, None)
    })
}

/// PlantUML's `CommandLinkStateReverse`: `B <-- A : label`.
pub(super) fn link_state_reverse() -> Box<dyn Command<StateDiagram>> {
    let mut parts = vec![
        RegexTree::start(),
        state_pattern("ENT2"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::concat(vec![
            RegexTree::named(1, "ARROW_CIRCLE_END", r"(o[%s]+)?"),
            RegexTree::leaf(r"\<"),
            RegexTree::named(1, "ARROW_BODY2", r"(-*)"),
            arrow_style_pattern("ARROW_STYLE2"),
            RegexTree::named(
                1,
                "ARROW_DIRECTION",
                r"(left|right|up|down|le?|ri?|up?|do?)?",
            ),
            arrow_style_pattern("ARROW_STYLE1"),
            RegexTree::named(1, "ARROW_BODY1", r"(-+)"),
            RegexTree::named(1, "ARROW_CROSS_START", r"(x)?"),
        ]),
        RegexTree::spaces_zero_or_more(),
        state_pattern("ENT1"),
    ];
    parts.extend(link_tail());
    command(RegexTree::concat(parts), &[ParserPass::Two], |d, l, a| {
        execute_link(d, l, a, Some(Direction::Left))
    })
}

fn execute_link(
    diagram: &mut StateDiagram,
    location: &LineLocation,
    arg: &RegexResult,
    default_direction: Option<Direction>,
) -> CommandResult {
    let ent1 = arg.get("ENT1", 0).unwrap_or_default();
    let ent2 = arg.get("ENT2", 0).unwrap_or_default();
    let cannot_be_used =
        |ent: &str| CommandError::new(format!("The state {ent} cannot be used here."));
    let cl1 = if ent1.starts_with("[*]") {
        Some(diagram.get_start(location)?)
    } else {
        get_entity(diagram, location, ent1)?
    }
    .ok_or_else(|| cannot_be_used(ent1))?;
    let cl2 = if ent2.starts_with("[*]") {
        Some(diagram.get_end(location)?)
    } else {
        get_entity(diagram, location, ent2)?
    }
    .ok_or_else(|| cannot_be_used(ent2))?;

    let direction = arg
        .get("ARROW_DIRECTION", 0)
        .map(Direction::of_queue)
        .or(default_direction);
    let sideways = matches!(direction, Some(Direction::Left | Direction::Right));
    let length = if sideways {
        1
    } else {
        let body = |name| arg.get(name, 0).unwrap_or_default().len();
        body("ARROW_BODY1") + body("ARROW_BODY2")
    };
    let link_type = LinkType::new(
        if arg.get("ARROW_CIRCLE_END", 0).is_some() {
            LinkDecor::ArrowAndCircle
        } else {
            LinkDecor::Arrow
        },
        if arg.get("ARROW_CROSS_START", 0).is_some() {
            LinkDecor::CircleCross
        } else {
            LinkDecor::None
        },
    );
    let label = arg.get("LABEL", 0);
    if label.is_some_and(|label| !label.trim().is_empty()) && use_node_style(diagram, arg) {
        // PlantUML names the node it draws the label in after the current time, so no output could match.
        diagram.command_not_ported("EntityImageTransitionLabel");
        return Ok(());
    }
    let label = label.map(Display::with_newlines);
    let cuca = &mut diagram.cuca;
    let link_arg = LinkArg::build_managing(
        label,
        i32::try_from(length).unwrap_or(i32::MAX),
        cuca.skin().class_attribute_icon_size() > 0,
    );
    let mut link = cuca.new_link(Some(location), cl1, cl2, link_type, link_arg);
    if matches!(direction, Some(Direction::Left | Direction::Up)) {
        link = cuca.get_inv(link);
    }
    cuca.link_mut(link)
        .apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    if let Some(stereotype) = arg.get("STEREOTYPE", 0) {
        cuca.link_mut(link).stereotype = Some(Stereotype::new(stereotype));
    }
    cuca.add_link(link);
    Ok(())
}

/// Whether the label goes in a node of its own, as `-[node]->` or the skin asks (`shouldUseNodeStyle`).
fn use_node_style(diagram: &StateDiagram, arg: &RegexResult) -> bool {
    arg.get_lazzy("ARROW_STYLE", 0)
        .is_some_and(|style| style.to_lowercase().contains("node"))
        || diagram
            .cuca
            .skin()
            .value("statediagramedgelabelstyle")
            .is_some_and(|style| style.eq_ignore_ascii_case("node"))
}

/// The state `code` names in a transition, created if needed; `None` when it may not be used here.
fn get_entity(
    diagram: &mut StateDiagram,
    location: &LineLocation,
    code: &str,
) -> Result<Option<EntityId>, CommandError> {
    if code.eq_ignore_ascii_case("[H]") {
        return diagram.get_historical(location).map(Some);
    }
    if let Some(state) = code.strip_suffix("[H]") {
        return diagram
            .get_history_of(location, state, "*historical*", LeafType::PseudoState)
            .map(Some);
    }
    if code.eq_ignore_ascii_case("[H*]") {
        return diagram.get_deep_history(location).map(Some);
    }
    if let Some(state) = code.strip_suffix("[H*]") {
        return diagram
            .get_history_of(location, state, "*deephistory*", LeafType::DeepHistory)
            .map(Some);
    }
    if code.starts_with('=') && code.ends_with('=') {
        let quark = diagram
            .cuca
            .quark_in_context(true, StateDiagram::clean_id(code.trim_matches('=')))?;
        let display = Display::with_newlines(diagram.cuca.quark(quark).get_name());
        return Ok(Some(diagram.leaf_named(
            location,
            quark,
            display,
            LeafType::SynchroBar,
        )));
    }
    let current = diagram.cuca.get_current_group();
    if diagram.cuca.entity(current).get_name(&diagram.cuca) == code {
        return Ok(Some(current));
    }
    let quark = diagram
        .cuca
        .quark_in_context(true, StateDiagram::clean_id(code))?;
    if !diagram.check_concurrent_state_ok(quark)? {
        return Ok(None);
    }
    if let Some(existing) = diagram.cuca.quark(quark).get_data() {
        return Ok(Some(existing));
    }
    let has_parent_entity = diagram
        .cuca
        .quark(quark)
        .get_parent()
        .is_some_and(|parent| diagram.cuca.quark(parent).get_data().is_some());
    if !has_parent_entity {
        return Ok(None);
    }
    let display = Display::with_newlines(diagram.cuca.quark(quark).get_name());
    Ok(Some(diagram.cuca.really_create_leaf(
        Some(location),
        quark,
        display,
        LeafType::State,
    )))
}

/// `CODE1 as "display"` or `"display" as CODE2` or `CODE2`, then what ends a group's first line.
fn group_name() -> RegexTree {
    RegexTree::or(vec![
        RegexTree::concat(vec![
            RegexTree::named(1, "CODE1", r"([%pLN_.]+)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "DISPLAY1", r"[%g]([^%g]+)[%g]"),
        ]),
        RegexTree::concat(vec![
            RegexTree::optional(RegexTree::concat(vec![
                RegexTree::named(1, "DISPLAY2", r"[%g]([^%g]+)[%g]"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"as"),
                RegexTree::spaces_one_or_more(),
            ])),
            RegexTree::named(1, "CODE2", r"([%pLN_.]+)"),
        ]),
    ])
}

fn not_null<'a>(arg: &'a RegexResult, v1: &str, v2: &str) -> Option<&'a str> {
    arg.get(v1, 0).or_else(|| arg.get(v2, 0))
}

/// PlantUML's `CommandCreatePackageState`: `state Name {` opens a composite state.
pub(super) fn create_package_state() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"state"),
            RegexTree::spaces_one_or_more(),
            group_name(),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            Stereogroup::optional_pattern(),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            line_color_pattern(),
            RegexTree::leaf(r"(?:[%s]*\{|[%s]+begin)"),
            RegexTree::end(),
        ]),
        ALL_PASSES,
        execute_create_package_state,
    )
}

fn execute_create_package_state(
    diagram: &mut StateDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let id_short = not_null(arg, "CODE1", "CODE2").unwrap_or_default();
    let quark = diagram.cuca.quark_in_context(true, id_short)?;
    let display = not_null(arg, "DISPLAY1", "DISPLAY2");
    let shown = display.unwrap_or_else(|| diagram.cuca.quark(quark).get_name());
    let shown = Display::with_newlines(shown);
    diagram
        .cuca
        .goto_group(Some(location), quark, shown, GroupType::State);
    let stereogroup = Stereogroup::build(arg.get("STEREOGROUP", 0));
    let colors = colors(arg)?.merge_with(&stereogroup.get_inner_colors().map_err(bad_color)?);
    let group = diagram.cuca.get_current_group();
    let entity = diagram.cuca.entity_mut(group);
    if let Some(display) = display {
        entity.display = Display::with_newlines(display);
    }
    entity.stereotype = stereogroup.build_stereotype();
    if let Some(url) = arg.get("URL", 0).and_then(Url::parse) {
        entity.url = Some(url);
    }
    entity.colors = colors;
    add_tags(diagram, group, arg.get_lazzy("TAGS", 0));
    Ok(())
}

/// PlantUML's `CommandCreatePackage2`: `frame Name {` draws a frame around states.
pub(super) fn create_package2() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"frame"),
            RegexTree::spaces_one_or_more(),
            group_name(),
            stereo::optional_pattern("STEREOTYPE"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            line_color_pattern(),
            RegexTree::leaf(r"(?:[%s]*\{|[%s]+begin)"),
            RegexTree::end(),
        ]),
        ALL_PASSES,
        execute_create_package2,
    )
}

fn execute_create_package2(
    diagram: &mut StateDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let id_short = not_null(arg, "CODE1", "CODE2").unwrap_or_default();
    let quark = diagram
        .cuca
        .quark_in_context(true, StateDiagram::clean_id(id_short))?;
    let display = not_null(arg, "DISPLAY1", "DISPLAY2")
        .unwrap_or_else(|| diagram.cuca.quark(quark).get_name());
    let display = Display::with_newlines(display);
    diagram
        .cuca
        .goto_group(Some(location), quark, display, GroupType::Package);
    let colors = colors(arg)?;
    let group = diagram.cuca.get_current_group();
    let entity = diagram.cuca.entity_mut(group);
    entity.usymbol = Some(USymbols::FRAME);
    if let Some(stereotype) = arg.get("STEREOTYPE", 0) {
        entity.stereotype = Some(Stereotype::new(stereotype));
    }
    if let Some(url) = arg.get("URL", 0).and_then(Url::parse) {
        entity.url = Some(url);
    }
    entity.colors = colors;
    Ok(())
}

/// PlantUML's `CommandEndState`: `}` or `end state` closes a composite state.
pub(super) fn end_state() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::counted(1, r"(end[%s]?state|\})"),
            RegexTree::end(),
        ]),
        ALL_PASSES,
        |diagram, _, _| {
            let current = diagram.cuca.get_current_group();
            if diagram.cuca.entity(current).is_root() {
                return Err(CommandError::new("No inner state defined"));
            }
            diagram.end_group()?;
            Ok(())
        },
    )
}

/// PlantUML's `CommandAddField`: `State : text` adds a line to the state's description.
pub(super) fn add_field() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::or(vec![
                RegexTree::named(1, "CODE3", r"([%pLN_.]+)"),
                RegexTree::named(1, "CODE4", r"[%g]([^%g]+)[%g]"),
            ]),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r":"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "FIELD", r"(.*)"),
            RegexTree::end(),
        ]),
        ParserPass::One.alone(),
        |diagram, location, arg| {
            let code = arg.get_lazzy("CODE", 0).unwrap_or_default();
            let current = diagram.cuca.get_current_group();
            let quark = if diagram.cuca.entity(current).get_name(&diagram.cuca) == code {
                diagram.cuca.entity(current).get_quark()
            } else {
                diagram
                    .cuca
                    .quark_in_context(true, StateDiagram::clean_id(code))?
            };
            let display = Display::with_newlines(diagram.cuca.quark(quark).get_name());
            let entity = diagram.leaf_named(location, quark, display, LeafType::State);
            let field = arg.get("FIELD", 0).unwrap_or_default();
            diagram
                .cuca
                .entity_mut(entity)
                .bodier
                .add_field_or_method(field);
            Ok(())
        },
    )
}

/// PlantUML's `CommandConcurrentState`: `--` or `||` starts the next concurrent region.
pub(super) fn concurrent_state() -> Box<dyn Command<StateDiagram>> {
    command(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(--+|\|\|+)"),
            RegexTree::end(),
        ]),
        ALL_PASSES,
        |diagram, location, arg| {
            let direction = arg
                .get("TYPE", 0)
                .and_then(|kind| kind.chars().next())
                .unwrap_or('-');
            diagram.concurrent_state(location, direction)
        },
    )
}
