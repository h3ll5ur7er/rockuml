//! Commands every diagram with a skin understands (PlantUML's `CommonCommands`).

mod skin_block;
mod sprite;
mod unported;

use std::marker::PhantomData;
use std::sync::LazyLock;

use regex::Regex;

use super::chrome::Warning;
use super::scale::Scale;
use super::titled::TitledDiagram;
use crate::abel::DisplayPositioned;
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PatternCommand, SingleLine,
    SingleLineCommand,
};
use crate::creole::Display;
use crate::klimt::HorizontalAlignment;
use crate::klimt::VerticalAlignment;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::style::{SName, StyleParsingError};
use crate::text::LineLocation;

/// Titles, captions, legends, headers and footers (`CommonCommands.addTitleCommands`).
pub(super) fn add_title_commands<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    vec![
        single(labelled("title", "TITLE1", "TITLE2"), set_title),
        single(mainframe_pattern(), set_mainframe),
        single(labelled("caption", "DISPLAY1", "DISPLAY2"), set_caption),
        Box::new(Multiline::new(
            &plantuml_regex("^caption$"),
            &plantuml_regex("^end[%s]?caption$"),
            set_multiline_caption,
        )),
        Box::new(Multiline::new(
            &plantuml_regex("^title$"),
            &plantuml_regex("^end[%s]?title$"),
            set_multiline_title,
        )),
        Box::new(
            Multiline::new(
                &LEGEND_START,
                &plantuml_regex("^end[%s]?legend$"),
                set_multiline_legend,
            )
            .skipping_quote_lines(),
        ),
        single(labelled("legend", "LEGEND1", "LEGEND2"), set_legend),
        single(Ribbon::Footer.pattern(), |diagram, arg, location| {
            Ribbon::Footer.set_from_line(diagram, arg, location);
        }),
        Box::new(Multiline::new(
            Ribbon::Footer.block_start(),
            &plantuml_regex("^end[%s]?footer$"),
            |diagram, lines| Ribbon::Footer.set_from_block(diagram, lines),
        )),
        single(Ribbon::Header.pattern(), |diagram, arg, location| {
            Ribbon::Header.set_from_line(diagram, arg, location);
        }),
        Box::new(Multiline::new(
            Ribbon::Header.block_start(),
            &plantuml_regex("^end[%s]?header$"),
            |diagram, lines| Ribbon::Header.set_from_block(diagram, lines),
        )),
        namespace_separator(),
    ]
}

/// Blank lines, pragmas, skin parameters, sprites and styles (`CommonCommands.addCommonCommands2`).
pub(super) fn add_common_commands2<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    vec![
        single(blank_line_pattern(), |_, _, _| {}),
        single(pragma_pattern(), define_pragma),
        unported::assume_transparent(),
        single(skinparam_pattern(), set_skinparam),
        Box::new(skin_block::SkinParamBlock),
        unported::skin(),
        unported::minwidth(),
        unported::page(),
        unported::rotate(),
        sprite::multi_line(),
        sprite::single_line(),
        sprite::md5(),
        sprite::svg(),
        sprite::stdlib(),
        sprite::stdlib_svg(),
        sprite::svg_multi_line(),
        sprite::file(),
        unported::style_single_line_css(),
        Box::new(
            Multiline::new(
                &plantuml_regex(r"^\<style\>$"),
                &plantuml_regex(r"^[%s]*\</?style\>[%s]*$"),
                apply_style_sheet,
            )
            .skipping_quote_lines(),
        ),
        unported::style_import(),
    ]
}

/// `scale` in its forms (`CommonCommands.addCommonScaleCommands`).
pub(super) fn add_common_scale_commands<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    vec![
        scale(scale_pattern(), scale_factor),
        scale(sized("WIDTH", "HEIGHT", false), |arg| {
            Ok(Scale::WidthAndHeight(
                number(arg, "WIDTH")?,
                number(arg, "HEIGHT")?,
            ))
        }),
        scale(width_or_height_pattern(), scale_width_or_height),
        scale(capped("WIDTH", "width"), |arg| {
            Ok(Scale::MaxWidth(number(arg, "WIDTH")?))
        }),
        scale(capped("HEIGHT", "height"), |arg| {
            Ok(Scale::MaxHeight(number(arg, "HEIGHT")?))
        }),
        scale(sized("WIDTH", "HEIGHT", true), |arg| {
            Ok(Scale::MaxWidthAndHeight(
                number(arg, "WIDTH")?,
                number(arg, "HEIGHT")?,
            ))
        }),
    ]
}

/// Hiding parts of entities (`CommonCommands.addCommonHides`).
pub(super) fn add_common_hides<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    vec![
        single(
            RegexTree::concat(vec![
                RegexTree::start(),
                RegexTree::named(1, "HIDE", r"(hide|show)"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"empty"),
                RegexTree::spaces_one_or_more(),
                RegexTree::leaf(r"description"),
                RegexTree::end(),
            ]),
            |diagram, arg, _| {
                let hide = arg
                    .get("HIDE", 0)
                    .is_some_and(|hide| hide.eq_ignore_ascii_case("hide"));
                diagram.set_hide_empty_description(hide);
            },
        ),
        super::class::hide_show_by_visibility(),
        super::class::hide_show_by_gender(),
    ]
}

/// The titles, then the second group, the scales and the hides (`CommonCommands.addCommonCommands1`).
pub(super) fn add_common_commands1<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    let mut commands = add_title_commands();
    commands.extend(add_common_commands2());
    commands.extend(add_common_scale_commands());
    commands.extend(add_common_hides());
    commands
}

/// PlantUML's `CommandNamespaceSeparator`: `set separator ::`, or `none` to keep names whole. Only diagrams of
/// entities have namespaces; the others ignore it.
fn namespace_separator<D: TitledDiagram + 'static>() -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"set"),
            RegexTree::spaces_one_or_more(),
            RegexTree::or(vec![
                RegexTree::leaf(r"separator"),
                RegexTree::leaf(r"namespaceseparator"),
            ]),
            RegexTree::spaces_one_or_more(),
            RegexTree::named(
                1,
                "SEPARATOR",
                r"((?:none|null)|[\\]{2}|::|[^%pLN%s_$#\\{}<>%g])",
            ),
            RegexTree::end(),
        ]),
        |diagram: &mut D, _: &LineLocation, arg: &RegexResult| {
            let separator = arg.get("SEPARATOR", 0).unwrap_or_default();
            let separator = (!separator.eq_ignore_ascii_case("none")
                && !separator.eq_ignore_ascii_case("null"))
            .then_some(separator);
            if let Some(cuca) = diagram.entity_diagram() {
                cuca.set_namespace_separator(separator);
            }
            Ok(())
        },
    )))
}

fn blank_line_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::spaces_zero_or_more(),
        RegexTree::end(),
    ])
}

type ApplyLine<D> = fn(&mut D, &RegexResult, &LineLocation);

/// A single-line command that cannot fail.
fn single<D: TitledDiagram + 'static>(
    pattern: RegexTree,
    apply: ApplyLine<D>,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(PatternCommand::new(
        pattern,
        move |diagram: &mut D, location: &LineLocation, arg: &RegexResult| {
            apply(diagram, arg, location);
            Ok(())
        },
    )))
}

/// A `scale` command and how it reads the scale.
struct ScaleCommand<D> {
    pattern: RegexTree,
    read: fn(&RegexResult) -> Result<Scale, CommandError>,
    diagram: PhantomData<D>,
}

fn scale<D: TitledDiagram + 'static>(
    pattern: RegexTree,
    read: fn(&RegexResult) -> Result<Scale, CommandError>,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(ScaleCommand {
        pattern,
        read,
        diagram: PhantomData,
    }))
}

impl<D: TitledDiagram> SingleLineCommand<D> for ScaleCommand<D> {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(&self, diagram: &mut D, _: &LineLocation, arg: &RegexResult) -> CommandResult {
        diagram.titled().set_scale((self.read)(arg)?);
        Ok(())
    }
}

const NUMBER: &str = "([0-9.]+)";

fn number(arg: &RegexResult, name: &str) -> Result<f64, CommandError> {
    arg.get(name, 0)
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| CommandError::new("Invalid number"))
}

/// `scale 1.5` or `scale 3/2`.
fn scale_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("scale"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "SCALE", NUMBER),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf("/"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "DIV", NUMBER),
        ])),
        RegexTree::end(),
    ])
}

fn scale_factor(arg: &RegexResult) -> Result<Scale, CommandError> {
    let zero = || CommandError::new("Scale cannot be zero");
    let mut factor = number(arg, "SCALE")?;
    if factor == 0.0 {
        return Err(zero());
    }
    if arg.get("DIV", 0).is_some() {
        let divisor = number(arg, "DIV")?;
        if divisor == 0.0 {
            return Err(zero());
        }
        factor /= divisor;
    }
    Ok(Scale::Factor(factor))
}

/// `scale 800*600`, or with `max` before the size, `scale max 800x600`.
fn sized(width: &'static str, height: &'static str, max: bool) -> RegexTree {
    let mut parts = vec![
        RegexTree::start(),
        RegexTree::leaf("scale"),
        RegexTree::spaces_one_or_more(),
    ];
    if max {
        parts.extend([RegexTree::leaf("max"), RegexTree::spaces_one_or_more()]);
    }
    parts.extend([
        RegexTree::named(1, width, NUMBER),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf("[*x]"),
        RegexTree::spaces_zero_or_more(),
        RegexTree::named(1, height, NUMBER),
        RegexTree::end(),
    ]);
    RegexTree::concat(parts)
}

/// `scale 800 width` or `scale 600 height`.
fn width_or_height_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("scale"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "VALUE", NUMBER),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "WIDTH", "(width|height)"),
        RegexTree::end(),
    ])
}

fn scale_width_or_height(arg: &RegexResult) -> Result<Scale, CommandError> {
    let size = number(arg, "VALUE")?;
    let is_width = arg
        .get("WIDTH", 0)
        .is_some_and(|dimension| dimension.eq_ignore_ascii_case("width"));
    Ok(if is_width {
        Scale::Width(size)
    } else {
        Scale::Height(size)
    })
}

/// `scale max 800 width` or `scale max 600 height`.
fn capped(name: &'static str, dimension: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("scale"),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf("max"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, name, NUMBER),
        RegexTree::spaces_one_or_more(),
        RegexTree::leaf(dimension),
        RegexTree::end(),
    ])
}

/// `keyword text`, `keyword: text` or `keyword "text"`, the text in group `quoted` or `plain`.
fn labelled(keyword: &'static str, quoted: &'static str, plain: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf(keyword),
        RegexTree::leaf("(?:[%s]*:[%s]*|[%s]+)"),
        quoted_or_plain(quoted, plain),
        RegexTree::end(),
    ])
}

/// A label in double quotes (group `quoted`), or one with at least a letter, digit, `_` or `.` (group `plain`).
fn quoted_or_plain(quoted: &'static str, plain: &'static str) -> RegexTree {
    RegexTree::or(vec![
        RegexTree::named(1, quoted, "[%g](.*)[%g]"),
        RegexTree::named(1, plain, "(.*[%pLN_.].*)"),
    ])
}

fn label(arg: &RegexResult, prefix: &str) -> Display {
    Display::with_newlines(arg.get_lazzy(prefix, 0).unwrap_or_default())
}

fn skinparam_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::named(1, "TYPE", "(skinparam|skinparamlocked)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", r"([\w.]*(?:\<\<[^<>]*\>\>)?[\w.]*)"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "VALUE", "([^{}]*)"),
        RegexTree::end(),
    ])
}

fn pragma_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("!pragma"),
        RegexTree::spaces_one_or_more(),
        RegexTree::named(1, "NAME", "([A-Za-z_][A-Za-z_0-9]*)"),
        RegexTree::optional(RegexTree::concat(vec![
            RegexTree::spaces_one_or_more(),
            RegexTree::named(1, "VALUE", "(.*)"),
        ])),
        RegexTree::end(),
    ])
}

fn define_pragma<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, _: &LineLocation) {
    let name = arg.get("NAME", 0).unwrap_or_default();
    diagram.titled().pragma.define(name, arg.get("VALUE", 0));
}

fn set_skinparam<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, _: &LineLocation) {
    let name = arg.get("NAME", 0).unwrap_or_default();
    let value = arg.get("VALUE", 0).unwrap_or_default();
    if let Some(warning) = deprecation_warning(name) {
        diagram.titled().add_warning(Warning(warning.to_owned()));
    }
    diagram.titled().skin.set_param(name, value);
}

/// The warning for a deprecated skin parameter. As in PlantUML, `skinparam` blocks do not warn.
fn deprecation_warning(name: &str) -> Option<&'static str> {
    [
        (
            "handwritten",
            "Please use '!option handwritten true' to enable handwritten ",
        ),
        (
            "ParticipantPadding",
            "Please use CSS style instead of skinparam ParticipantPadding",
        ),
        (
            "padding",
            "Please use CSS style instead of skinparam padding",
        ),
    ]
    .into_iter()
    .find(|(deprecated, _)| deprecated.eq_ignore_ascii_case(name))
    .map(|(_, warning)| warning)
}

fn apply_style_sheet<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let body = lines.sub_extract(1, 1);
    let texts: Vec<&str> = body.iter().map(crate::text::StringLocated::text).collect();
    diagram
        .titled()
        .skin
        .apply_style_sheet(&texts)
        .map_err(|error| match error {
            StyleParsingError::Invalid(message) => {
                CommandError::new(format!("Error in style definition: {message}"))
            }
            // PlantUML reports this as a crash; rockuml reports the style as faulty.
            StyleParsingError::Unexpected => CommandError::new("Error in style definition"),
        })
}

fn set_title<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    diagram.titled().set_title(label(arg, "TITLE"), location);
}

/// `mainframe text` or `mainframe: text`.
fn mainframe_pattern() -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::leaf("mainframe"),
        RegexTree::leaf("(?:[%s]*:[%s]*|[%s]+)"),
        RegexTree::named(1, "LABEL", "(.*[%pLN_.].*)"),
        RegexTree::end(),
    ])
}

fn set_mainframe<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, _: &LineLocation) {
    let label = Display::with_newlines(arg.get("LABEL", 0).unwrap_or_default());
    diagram.titled().set_mainframe(label);
}

fn set_caption<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    diagram
        .titled()
        .set_caption(label(arg, "DISPLAY"), location);
}

/// Unlike the other one-line commands, PlantUML does not remember where a one-line legend was written.
fn set_legend<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, _: &LineLocation) {
    let legend = DisplayPositioned {
        display: label(arg, "LEGEND"),
        alignment: HorizontalAlignment::Center,
        location: None,
    };
    diagram
        .titled()
        .set_legend(legend, VerticalAlignment::Bottom);
}

/// The lines between a block's first and last line, without their common indentation.
fn block_body(lines: &BlocLines) -> Option<Display> {
    let display = lines.sub_extract(1, 1).without_empty_columns().to_display();
    (!display.lines().is_empty()).then(|| display.replace_backslash_t())
}

fn set_multiline_title<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let title = block_body(lines).ok_or_else(|| CommandError::new("No title defined"))?;
    diagram.titled().set_title(title, first_location(lines));
    Ok(())
}

fn set_multiline_caption<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let caption = block_body(lines).ok_or_else(|| CommandError::new("No caption defined"))?;
    diagram.titled().set_caption(caption, first_location(lines));
    Ok(())
}

fn first_location(lines: &BlocLines) -> &LineLocation {
    lines.first().expect("a block has a start line").location()
}

/// `legend [top|bottom] [left|right|center]`.
static LEGEND_START: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex("^legend(?:[%s]+(top|bottom))?(?:[%s]+(left|right|center))?$"));

fn set_multiline_legend<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let lines = lines.trim_smart(1);
    let first = lines.first().expect("the start line").trimmed();
    let captures = LEGEND_START.captures(first.text()).map(|captures| {
        (
            captures.get(1).map(|m| m.as_str().to_owned()),
            captures.get(2).map(|m| m.as_str().to_owned()),
        )
    });
    let (vertical, horizontal) = captures.unwrap_or_default();
    let legend = block_body(&lines).ok_or_else(|| CommandError::new("No legend defined"))?;
    let legend = DisplayPositioned {
        display: legend,
        alignment: horizontal
            .as_deref()
            .and_then(HorizontalAlignment::from_name)
            .unwrap_or(HorizontalAlignment::Center),
        location: Some(first_location(&lines).clone()),
    };
    let vertical = match vertical.as_deref() {
        Some(top) if top.eq_ignore_ascii_case("top") => VerticalAlignment::Top,
        _ => VerticalAlignment::Bottom,
    };
    diagram.titled().set_legend(legend, vertical);
    Ok(())
}

/// Headers and footers.
#[derive(Clone, Copy)]
enum Ribbon {
    Header,
    Footer,
}

impl Ribbon {
    fn keyword(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }

    fn style(self) -> SName {
        match self {
            Self::Header => SName::Header,
            Self::Footer => SName::Footer,
        }
    }

    /// `[left|right|center] header text`, or the same for footers.
    fn pattern(self) -> RegexTree {
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::optional(RegexTree::named(1, "POSITION", "(left|right|center)")),
            RegexTree::spaces_zero_or_more(),
            RegexTree::leaf(self.keyword()),
            RegexTree::or(vec![
                RegexTree::concat(vec![
                    RegexTree::spaces_zero_or_more(),
                    RegexTree::leaf(":"),
                    RegexTree::spaces_zero_or_more(),
                ]),
                RegexTree::spaces_one_or_more(),
            ]),
            quoted_or_plain("LABEL1", "LABEL2"),
            RegexTree::end(),
        ])
    }

    /// `[left|right|center] header` on its own, starting a block.
    fn block_start(self) -> &'static Regex {
        static HEADER: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex("^(?:(left|right|center)?[%s]*)header$"));
        static FOOTER: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex("^(?:(left|right|center)?[%s]*)footer$"));
        match self {
            Self::Header => &HEADER,
            Self::Footer => &FOOTER,
        }
    }

    fn set<D: TitledDiagram>(self, diagram: &mut D, positioned: DisplayPositioned) {
        match self {
            Self::Header => diagram.titled().set_header(positioned),
            Self::Footer => diagram.titled().set_footer(positioned),
        }
    }

    /// The alignment the command gives, otherwise the one the ribbon's style gives.
    fn alignment<D: TitledDiagram>(
        self,
        diagram: &mut D,
        given: Option<&str>,
    ) -> HorizontalAlignment {
        match given.and_then(HorizontalAlignment::from_name) {
            Some(alignment) => alignment,
            None => diagram.titled().default_alignment(self.style()),
        }
    }

    fn set_from_line<D: TitledDiagram>(
        self,
        diagram: &mut D,
        arg: &RegexResult,
        location: &LineLocation,
    ) {
        let positioned = DisplayPositioned {
            display: label(arg, "LABEL"),
            alignment: self.alignment(diagram, arg.get("POSITION", 0)),
            location: Some(location.clone()),
        };
        self.set(diagram, positioned);
    }

    fn set_from_block<D: TitledDiagram>(self, diagram: &mut D, lines: &BlocLines) -> CommandResult {
        let lines = lines.trimmed();
        let first = lines.first().expect("a block has a start line");
        let given = self
            .block_start()
            .captures(first.text())
            .and_then(|captures| captures.get(1).map(|m| m.as_str().to_owned()));
        let display = lines.sub_extract(1, 1).to_display();
        if display.lines().is_empty() {
            return Err(CommandError::new(format!("Empty {}", self.keyword())));
        }
        let positioned = DisplayPositioned {
            display,
            alignment: self.alignment(diagram, given.as_deref()),
            location: Some(first.location().clone()),
        };
        self.set(diagram, positioned);
        Ok(())
    }
}
