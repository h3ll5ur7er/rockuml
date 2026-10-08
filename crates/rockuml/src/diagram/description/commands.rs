//! The commands only usecase, component, deployment and archimate diagrams have (PlantUML's
//! `descdiagram.command` package and `CommandCreateDomain`).

use super::DescriptionDiagram;
use crate::abel::{EntityId, GroupType, LeafType, LinkArg};
use crate::color::{self, ColorType};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
    SingleLineCommand,
};
use crate::creole::Display;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::decoration::{LinkDecor, LinkType, WithLinkType};
use crate::diagram::cuca::EntityDiagram;
use crate::diagram::cuca_commands::labels::Labels;
use crate::diagram::cuca_commands::{
    ALL_TYPES, add_tags, char_encoding, colors, display_with_generic, exists_with_bad_type3,
    is_bare_name, unknown_symbol,
};
use crate::direction::Direction;
use crate::java;
use crate::klimt::url::Url;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::SkinParam;
use crate::skin::actor::ActorStyle;
use crate::stereo::{self, Stereotype};
use crate::text::{LineLocation, without_quotes_or_brackets};

/// What `[x]`, `(x)`, `:x:` and `"x"` may name an element as, at either end of a link.
const LINK_END: &str = r"([%pLN_.]+|[%g][^%g]+[%g]|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|(?!\[\*\])\[[^\[\]]+\]|\((?!\*\))[^)]+\)/?)";

/// The keys of the style an arrow may give in brackets, like `-[#red,dashed]->`
/// (`CommandLinkElement.LINE_STYLE`).
const LINE_STYLE: &str = r"(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*";

/// Line styles for parallel lines, separated by `;` (`LINE_STYLE_MULTIPLES`).
fn line_style_multiples() -> String {
    format!("{LINE_STYLE}(?:(?:;{LINE_STYLE})*)")
}

/// The style in brackets inside an arrow's body, as most link commands write it.
pub(in crate::diagram) fn arrow_style() -> String {
    format!(r"(?:\[({LINE_STYLE})\])?")
}

/// The style of an activity arrow, like `-[#red;#blue]->` (`STYLE_COLORS_MULTIPLES`).
pub(in crate::diagram) fn style_colors_multiples() -> String {
    format!(r"-\[({}*)\]->", line_style_multiples())
}

/// PlantUML's `CommandLinkElement`: `A --> B`, creating the ends it names as their notation says.
pub(super) fn link_element() -> Box<dyn Command<DescriptionDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "ENT1", LINK_END),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "FIRST_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "HEAD1", LinkDecor::get_regex_decors1()),
            RegexTree::named(1, "BODY1", r"([-=.~]+)"),
            RegexTree::named(
                1,
                "ARROW_STYLE1",
                format!(r"(?:\[({})\])?", line_style_multiples()),
            ),
            RegexTree::optional(RegexTree::named(
                1,
                "DIRECTION",
                r"(left|right|up|down|le?|ri?|up?|do?)(?=[-=.~0()\[])",
            )),
            RegexTree::optional(RegexTree::named(
                1,
                "INSIDE",
                r"(0|\(0\)|\(0|0\))(?=[-=.~])",
            )),
            RegexTree::named(1, "ARROW_STYLE2", arrow_style()),
            RegexTree::named(1, "BODY2", r"([-=.~]*)"),
            RegexTree::named(1, "HEAD2", LinkDecor::get_regex_decors2()),
            RegexTree::spaces_zero_or_more(),
            RegexTree::optional(RegexTree::named(1, "SECOND_LABEL", r"[%g]([^%g]+)[%g]")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "ENT2", LINK_END),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            stereo::optional_pattern("STEREOTYPE"),
            RegexTree::named(1, "LABEL_LINK", r"(?::[%s]*(.+))?"),
            RegexTree::end(),
        ]),
        execute_link_element,
    )))
}

fn execute_link_element(
    diagram: &mut DescriptionDiagram,
    location: &LineLocation,
    arg: &RegexResult,
) -> CommandResult {
    let ent1 = arg.get("ENT1", 0).unwrap_or_default();
    let ent2 = arg.get("ENT2", 0).unwrap_or_default();
    let queue = format!(
        "{}{}",
        arg.get("BODY1", 0).unwrap_or_default(),
        arg.get("BODY2", 0).unwrap_or_default()
    );
    let link_type = link_type(arg, &queue);
    let direction = Direction::of_queue(arg.get("DIRECTION", 0).unwrap_or(&queue));
    let length = if matches!(direction, Direction::Left | Direction::Right) {
        1
    } else {
        queue.len()
    };
    let labels = Labels::new(arg);
    let link_colors = colors(arg, ColorType::Line)?;
    let (ent1_clean, ent2_clean) = (
        DescriptionDiagram::clean_id(ent1),
        DescriptionDiagram::clean_id(ent2),
    );
    let (cl1, cl2) = match (
        diagram.cuca.is_group(ent1_clean),
        diagram.cuca.is_group(ent2_clean),
    ) {
        (true, true) => (
            diagram.cuca.get_group(ent1_clean).expect("a group"),
            diagram.cuca.get_group(ent2_clean).expect("a group"),
        ),
        _ => (
            get_dummy(location, diagram, ent1)?,
            get_dummy(location, diagram, ent2)?,
        ),
    };
    let cuca = &mut diagram.cuca;
    let link_arg = LinkArg::build_managing(
        labels.get_label_link().map(Display::with_newlines),
        i32::try_from(length).expect("an arrow fits a line"),
        cuca.skin().class_attribute_icon_size() > 0,
    )
    .with_quantifier(labels.get_first_label(), labels.get_second_label());
    let mut link = cuca.new_link(Some(location), cl1, cl2, link_type, link_arg);
    if matches!(direction, Direction::Left | Direction::Up) {
        link = cuca.get_inv(link);
    }
    let link_mut = cuca.link_mut(link);
    link_mut.link_arrow = labels.get_link_arrow();
    link_mut.set_colors(link_colors);
    link_mut.apply_style(arg.get_lazzy("ARROW_STYLE", 0));
    if let Some(stereotype) = arg.get("STEREOTYPE", 0) {
        link_mut.stereotype = Some(Stereotype::new(stereotype));
    }
    cuca.add_link(link);
    Ok(())
}

/// The decorations at both ends, and the line `.`, `~` or `=` in the body asks for.
fn link_type(arg: &RegexResult, queue: &str) -> LinkType {
    let head = |key| {
        arg.get(key, 0)
            .map(|head| java::trim(head).to_lowercase().replace('_', ""))
    };
    let decor1 = LinkDecor::lookup_decors1(head("HEAD1").as_deref());
    let decor2 = LinkDecor::lookup_decors2(head("HEAD2").as_deref());
    let mut result = LinkType::new(decor2, decor1);
    if queue.contains('.') {
        result = result.go_dashed();
    } else if queue.contains('~') {
        result = result.go_dotted();
    } else if queue.contains('=') {
        result = result.go_bold();
    }
    match arg.get("INSIDE", 0) {
        Some("0") => result.with_middle_circle(),
        Some("0)") => result.with_middle_circle_circled1(),
        Some("(0") => result.with_middle_circle_circled2(),
        Some("(0)") => result.with_middle_circle_circled(),
        _ => result,
    }
}

/// The entity a link end names, created as its notation says if new: `()x` an interface, `(x)` a use case,
/// `:x:` an actor, `[x]` a component, a trailing `/` the business variant; a plain name stays unknown
/// until the diagram is complete.
fn get_dummy(
    location: &LineLocation,
    diagram: &mut DescriptionDiagram,
    ident: &str,
) -> Result<EntityId, CommandError> {
    if ident.starts_with("()") {
        let ident = DescriptionDiagram::clean_id(ident);
        let cuca = &mut diagram.cuca;
        let quark = cuca.quark_in_context(true, ident)?;
        if let Some(existing) = cuca.quark(quark).get_data() {
            return Ok(existing);
        }
        let display = Display::with_newlines(cuca.quark(quark).get_name());
        return Ok(create_leaf(
            diagram,
            location,
            quark,
            display,
            LeafType::Description,
            Some(USymbols::INTERFACE),
        ));
    }
    let code_char = if ident.chars().count() > 2 {
        ident.chars().next()
    } else {
        None
    };
    let end_with_slash = ident.ends_with('/');
    let ident = DescriptionDiagram::clean_id(ident);
    let cuca = &mut diagram.cuca;
    let quark = cuca.quark_in_context(true, ident)?;
    if let Some(existing) = cuca.quark(quark).get_data() {
        return Ok(existing);
    }
    let display = Display::with_newlines(cuca.quark(quark).get_name());
    let (leaf_type, usymbol) = match code_char {
        Some('(') if end_with_slash => {
            (LeafType::UsecaseBusiness, Some(USymbols::USECASE_BUSINESS))
        }
        Some('(') => (LeafType::Usecase, Some(USymbols::USECASE)),
        Some(':') if end_with_slash => (
            LeafType::Description,
            Some(ActorStyle::StickmanBusiness.to_u_symbol()),
        ),
        Some(':') => (
            LeafType::Description,
            Some(cuca.skin().actor_style().to_u_symbol()),
        ),
        Some('[') => (
            LeafType::Description,
            Some(cuca.skin().component_style().to_u_symbol()),
        ),
        _ => (LeafType::StillUnknown, None),
    };
    Ok(create_leaf(
        diagram, location, quark, display, leaf_type, usymbol,
    ))
}

/// `reallyCreateLeaf` with a symbol.
fn create_leaf(
    diagram: &mut DescriptionDiagram,
    location: &LineLocation,
    quark: crate::plasma::QuarkId,
    display: Display,
    leaf_type: LeafType,
    usymbol: Option<USymbol>,
) -> EntityId {
    let entity = diagram
        .cuca
        .really_create_leaf(Some(location), quark, display, leaf_type);
    diagram.cuca.entity_mut(entity).usymbol = usymbol;
    entity
}

const CODE_CORE: &str =
    r"[%pLN_.]+|\(\)[%s]*[%pLN_.]+|\(\)[%s]*[%g][^%g]+[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]";
const DISPLAY_CORE: &str = r"[%g].+?[%g]|:[^:]+:/?|\([^()]+\)/?|\[[^\[\]]+\]";

fn code() -> String {
    format!("({CODE_CORE})")
}

fn code_with_quote() -> String {
    format!("({CODE_CORE}|[%g].+?[%g])")
}

fn display() -> String {
    format!("({DISPLAY_CORE})")
}

fn display_without_quote() -> String {
    format!("({DISPLAY_CORE}|[%pLN_.]+)")
}

/// The four ways to give an element's code and display, with the stereotype pattern each may carry.
fn code_and_display(stereotype: fn(&'static str) -> RegexTree) -> RegexTree {
    RegexTree::or(vec![
        RegexTree::named(1, "CODE1", code_with_quote()),
        RegexTree::concat(vec![
            RegexTree::named(1, "DISPLAY2", display()),
            stereotype("STEREOTYPE2"),
            RegexTree::leaf("as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE2", code()),
        ]),
        RegexTree::concat(vec![
            RegexTree::named(1, "CODE3", code()),
            stereotype("STEREOTYPE3"),
            RegexTree::leaf("as"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "DISPLAY3", display()),
        ]),
        RegexTree::concat(vec![
            RegexTree::named(1, "DISPLAY4", display_without_quote()),
            stereotype("STEREOTYPE4"),
            RegexTree::leaf("as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE4", code()),
        ]),
    ])
}

/// PlantUML's `CommandCreateElementFull`: `node "Name" as N <<stereo>> #color`, `[Component]`, `(Use case)`,
/// `:Actor:`...
struct CreateElementFull {
    pattern: RegexTree,
}

pub(super) fn create_element_full() -> Box<dyn Command<DescriptionDiagram>> {
    Box::new(SingleLine(CreateElementFull {
        pattern: RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "SYMBOL", format!(r"(?:({ALL_TYPES}|\(\))[%s]+)?")),
            color::optional_pattern("COLOR2"),
            RegexTree::spaces_zero_or_more(),
            code_and_display(stereo::optional_pattern),
            RegexTree::spaces_zero_or_more(),
            stereo::tags_pattern("TAGS1"),
            stereo::optional_pattern("STEREOTYPE"),
            stereo::tags_pattern("TAGS2"),
            RegexTree::spaces_zero_or_more(),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::end(),
        ]),
    }))
}

impl SingleLineCommand<DescriptionDiagram> for CreateElementFull {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn is_forbidden(&self, line: &str) -> bool {
        is_bare_name(line)
    }

    fn execute_arg(
        &self,
        diagram: &mut DescriptionDiagram,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        let mut code_raw = arg.get_lazzy("CODE", 0).unwrap_or_default();
        let mut display_raw = arg.get_lazzy("DISPLAY", 0);
        let code_char = char_encoding(Some(code_raw));
        let code_display = char_encoding(display_raw);
        let symbol_arg = arg.get("SYMBOL", 0);
        let business = symbol_arg.is_some_and(|symbol| symbol.ends_with('/'));
        let symbol = if let Some(interface) = code_raw.strip_prefix("()") {
            code_raw = without_quotes_or_brackets(java::trim(interface));
            Some("interface")
        } else if code_char == Some('(') || code_display == Some('(') {
            Some(business_variant(
                business,
                &mut display_raw,
                &mut code_raw,
                ")/",
                "usecase/",
                "usecase",
            ))
        } else if code_char == Some(':') || code_display == Some(':') {
            Some(business_variant(
                business,
                &mut display_raw,
                &mut code_raw,
                ":/",
                "actor/",
                "actor",
            ))
        } else if code_char == Some('[') || code_display == Some('[') {
            Some("component")
        } else {
            symbol_arg
        };
        let (leaf_type, usymbol) = leaf_type_and_symbol(symbol, diagram.cuca.skin())?;
        let code = DescriptionDiagram::clean_id(code_raw).to_owned();
        let cuca = &mut diagram.cuca;
        let quark = cuca.quark_in_context(false, &code)?;
        let name = cuca.quark(quark).get_name().to_owned();
        if cuca.is_group_quark(quark) {
            return Err(already_defined(&name));
        }
        let display =
            Display::with_newlines(without_quotes_or_brackets(display_raw.unwrap_or(&name)));
        if cuca
            .quark(quark)
            .get_data()
            .is_some_and(|other| exists_with_bad_type3(cuca.entity(other), leaf_type, usymbol))
        {
            return Err(already_defined(&name));
        }
        if matches!(leaf_type, LeafType::Portin | LeafType::Portout)
            && cuca.entity(cuca.get_current_group()).is_root()
        {
            return Err(CommandError::new(
                "Port can only be used inside an element and not at root level",
            ));
        }
        let entity = match cuca.quark(quark).get_data() {
            Some(existing) => existing,
            None => create_leaf(
                diagram,
                location,
                quark,
                display.clone(),
                leaf_type,
                usymbol,
            ),
        };
        let colors = colors(arg, ColorType::Back)?;
        let entity = diagram.cuca.entity_mut(entity);
        entity.display = display;
        if let Some(stereotype) = arg.get_lazzy("STEREOTYPE", 0) {
            entity.stereotype = Some(Stereotype::with_spot(stereotype)?);
        }
        add_tags(entity, arg.get_lazzy("TAGS", 0));
        if let Some(url) = arg.get("URL", 0).and_then(Url::parse) {
            entity.url = Some(url);
        }
        entity.colors = colors;
        Ok(())
    }
}

/// The type and symbol of the element a keyword like `node`, or the notation standing for one, declares;
/// no keyword means an actor.
fn leaf_type_and_symbol(
    symbol: Option<&str>,
    skin: &SkinParam,
) -> Result<(LeafType, Option<USymbol>), CommandError> {
    Ok(match symbol.map(str::to_ascii_lowercase).as_deref() {
        None => (
            LeafType::Description,
            Some(skin.actor_style().to_u_symbol()),
        ),
        Some("portin" | "port") => (LeafType::Portin, None),
        Some("portout") => (LeafType::Portout, None),
        Some("usecase") => (LeafType::Usecase, None),
        Some("usecase/") => (LeafType::UsecaseBusiness, None),
        Some("circle") => (LeafType::Circle, None),
        Some(other) => (
            LeafType::Description,
            Some(
                USymbols::from_string_skin_param(other, skin)
                    .ok_or_else(|| unknown_symbol(other))?,
            ),
        ),
    })
}

/// Which of a use case or actor notation's two symbols the line declares; a trailing `/` on the display
/// or code asks for the business one and is dropped.
fn business_variant<'a>(
    business_keyword: bool,
    display_raw: &mut Option<&str>,
    code_raw: &mut &str,
    business_end: &str,
    business: &'a str,
    plain: &'a str,
) -> &'a str {
    if business_keyword {
        return business;
    }
    if let Some(display) = display_raw.filter(|display| display.ends_with(business_end)) {
        *display_raw = Some(&display[..display.len() - 1]);
        return business;
    }
    if code_raw.ends_with(business_end) {
        *code_raw = &code_raw[..code_raw.len() - 1];
        return business;
    }
    plain
}

fn already_defined(name: &str) -> CommandError {
    CommandError::new(format!("This element ({name}) is already defined"))
}

/// The stereotype of an archimate element: one word naming its icon (`StereotypePattern.optionalArchimate`).
fn optional_archimate(name: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::optional(RegexTree::named(1, name, r"(\<\<[-\w]+?\>\>)")),
        RegexTree::spaces_zero_or_more(),
    ])
}

/// The archimate icon a stereotype like `<<business-actor>>` names, as a stereotype drawing it.
fn archimate_stereotype(arg: &RegexResult) -> Result<Option<Stereotype>, CommandError> {
    arg.get_lazzy("STEREOTYPE", 0)
        .map(|stereotype| {
            let icon = stereotype
                .strip_prefix("<<")
                .and_then(|icon| icon.strip_suffix(">>"))
                .unwrap_or(stereotype);
            Ok(Stereotype::with_spot(&format!("<<$archimate/{icon}>>"))?)
        })
        .transpose()
}

/// PlantUML's `CommandArchimate`: `archimate #Business "Name" as N <<business-actor>>`.
pub(super) fn archimate() -> Box<dyn Command<DescriptionDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "SYMBOL", "archimate"),
            RegexTree::spaces_one_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_one_or_more(),
            code_and_display(optional_archimate),
            optional_archimate("STEREOTYPE"),
            RegexTree::end(),
        ]),
        |diagram: &mut DescriptionDiagram, location: &LineLocation, arg: &RegexResult| {
            let code = DescriptionDiagram::clean_id(arg.get_lazzy("CODE", 0).unwrap_or_default())
                .to_owned();
            let quark = diagram.cuca.quark_in_context(true, &code)?;
            let display = Display::with_newlines(arg.get_lazzy("DISPLAY", 0).map_or(
                diagram.cuca.quark(quark).get_name(),
                without_quotes_or_brackets,
            ));
            let entity = match diagram.cuca.quark(quark).get_data() {
                Some(existing) => existing,
                None => create_leaf(
                    diagram,
                    location,
                    quark,
                    display.clone(),
                    LeafType::Description,
                    Some(USymbols::ARCHIMATE),
                ),
            };
            let stereotype = archimate_stereotype(arg)?;
            let colors = colors(arg, ColorType::Back)?;
            let entity = diagram.cuca.entity_mut(entity);
            entity.display = display;
            if stereotype.is_some() {
                entity.stereotype = stereotype;
            }
            entity.colors = colors;
            Ok(())
        },
    )))
}

/// PlantUML's `CommandArchimateMultilines`: `archimate #Business A [` with a description up to `]`.
pub(super) fn archimate_multilines() -> Box<dyn Command<DescriptionDiagram>> {
    fn start() -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf("archimate"),
            RegexTree::spaces_one_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([%pLN_.]+)"),
            optional_archimate("STEREOTYPE"),
            Url::optional_pattern(),
            RegexTree::spaces_zero_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\["),
            RegexTree::named(1, "DESC", "(.*)"),
            RegexTree::end(),
        ])
    }
    let start_pattern = start();
    Box::new(
        Multiline::starting_with_owned(
            start(),
            &plantuml_regex(r"^(.*)\]$"),
            move |diagram: &mut DescriptionDiagram, lines: &BlocLines| {
                let location = lines.first().map(|first| first.location().clone());
                let lines = lines.trimmed();
                let first = lines.first().expect("a block has a first line");
                let head = start_pattern
                    .matcher(first.text())
                    .expect("the first line matched");
                let code =
                    DescriptionDiagram::clean_id(head.get_lazzy("CODE", 0).unwrap_or_default())
                        .to_owned();
                let cuca = &mut diagram.cuca;
                let quark = cuca.quark_in_context(false, &code)?;
                if cuca.quark(quark).get_data().is_some() {
                    return Err(CommandError::new(format!(
                        "Already exists {}",
                        cuca.quark(quark).get_name()
                    )));
                }
                let stereotype = archimate_stereotype(&head)?;
                let display = Display::with_newlines(cuca.quark(quark).get_name());
                let entity = create_leaf(
                    diagram,
                    location.as_ref().expect("blocks have a location"),
                    quark,
                    display,
                    LeafType::Description,
                    Some(USymbols::RECTANGLE),
                );
                let colors = colors(&head, ColorType::Back)?;
                let entity = diagram.cuca.entity_mut(entity);
                entity.display = lines.sub_extract(1, 1).to_display();
                if stereotype.is_some() {
                    entity.stereotype = stereotype;
                }
                entity.colors = colors;
                Ok(())
            },
        )
        .skipping_quote_lines(),
    )
}

/// PlantUML's `CommandArchimatePackage`: `archimate #Business "Name" as N <<business-actor>> {`.
pub(super) fn archimate_package() -> Box<dyn Command<DescriptionDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(0, "SYMBOL", "archimate"),
            RegexTree::spaces_one_or_more(),
            color::optional_pattern("COLOR"),
            RegexTree::spaces_one_or_more(),
            code_and_display(optional_archimate),
            optional_archimate("STEREOTYPE"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(r"\{"),
            RegexTree::end(),
        ]),
        |diagram: &mut DescriptionDiagram, location: &LineLocation, arg: &RegexResult| {
            let code = DescriptionDiagram::clean_id(arg.get_lazzy("CODE", 0).unwrap_or_default())
                .to_owned();
            let cuca = &mut diagram.cuca;
            let quark = cuca.quark_in_context(true, &code)?;
            let display = Display::with_newlines(
                arg.get_lazzy("DISPLAY", 0)
                    .map_or(cuca.quark(quark).get_name(), without_quotes_or_brackets),
            );
            cuca.goto_group(Some(location), quark, display.clone(), GroupType::Package);
            let stereotype = archimate_stereotype(arg)?;
            let colors = colors(arg, ColorType::Back)?;
            let group = cuca.get_current_group();
            let entity = cuca.entity_mut(group);
            entity.usymbol = Some(USymbols::ARCHIMATE);
            entity.display = display;
            if stereotype.is_some() {
                entity.stereotype = stereotype;
            }
            entity.colors = colors;
            Ok(())
        },
    )))
}

/// PlantUML's `CommandCreateDomain`: `domain "Name" as D` or `requirement "Name" as R`, a block with `{`.
pub(super) fn create_domain() -> Box<dyn Command<DescriptionDiagram>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::named(1, "TYPE", r"(requirement|domain)"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(2, "DISPLAY", display_with_generic()),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(r"as"),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "CODE", r"([a-zA-Z0-9]+)"),
            stereo::optional_pattern("STEREO"),
            RegexTree::named(1, "GROUP", r"(\{)?"),
            RegexTree::end(),
        ]),
        |diagram: &mut DescriptionDiagram, location: &LineLocation, arg: &RegexResult| {
            let type_string = arg.get("TYPE", 0).unwrap_or_default();
            let display_string = arg.get_lazzy("DISPLAY", 0).unwrap_or_default();
            let code_string = arg.get_lazzy("CODE", 0).unwrap_or(display_string);
            let stereotype = arg.get("STEREO", 0);
            let domain = type_string.eq_ignore_ascii_case("domain");
            let (group_type, leaf_type) = if domain {
                (GroupType::Domain, LeafType::Domain)
            } else {
                (GroupType::Requirement, LeafType::Requirement)
            };
            let code = DescriptionDiagram::clean_id(code_string).to_owned();
            let cuca = &mut diagram.cuca;
            let quark = cuca.quark_in_context(true, &code)?;
            if cuca.quark(quark).get_data().is_some() {
                return Err(CommandError::new(format!(
                    "Object already exists : {code_string}"
                )));
            }
            let display = Display::with_newlines(display_string);
            // No symbol is named `domain` or `requirement`: their images draw them.
            let entity = if arg.get("GROUP", 0).is_some() {
                cuca.goto_group(Some(location), quark, display, group_type);
                cuca.get_current_group()
            } else {
                cuca.really_create_leaf(Some(location), quark, display, leaf_type)
            };
            if let Some(stereotype) = stereotype {
                diagram.cuca.entity_mut(entity).stereotype =
                    Some(Stereotype::with_spot(stereotype)?);
            }
            Ok(())
        },
    )))
}
