//! Commands every diagram with a skin understands (PlantUML's `CommonCommands`).

use std::marker::PhantomData;
use std::sync::LazyLock;

use regex::Regex;

use super::scale::Scale;
use super::titled::{Positioned, TitledDiagram, VerticalAlignment};
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, SingleLine, SingleLineCommand,
};
use crate::creole::Display;
use crate::klimt::HorizontalAlignment;
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::style::{SName, StyleParsingError};
use crate::text::LineLocation;

/// The common commands, in PlantUML's order: skin parameters and styles, then titles and the like.
pub fn common_commands<D: TitledDiagram + 'static>() -> Vec<Box<dyn Command<D>>> {
    vec![
        single(skinparam_pattern(), set_skinparam),
        Box::new(
            Multiline::new(
                plantuml_regex(r"^\<style\>$"),
                plantuml_regex(r"^[%s]*\</?style\>[%s]*$"),
                apply_style_sheet,
            )
            .skipping_quote_lines(),
        ),
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
        single(labelled("title", "TITLE1", "TITLE2"), set_title),
        single(labelled("caption", "DISPLAY1", "DISPLAY2"), set_caption),
        Box::new(Multiline::new(
            plantuml_regex("^caption$"),
            plantuml_regex("^end[%s]?caption$"),
            set_multiline_caption,
        )),
        Box::new(Multiline::new(
            plantuml_regex("^title$"),
            plantuml_regex("^end[%s]?title$"),
            set_multiline_title,
        )),
        Box::new(
            Multiline::new(
                LEGEND_START.clone(),
                plantuml_regex("^end[%s]?legend$"),
                set_multiline_legend,
            )
            .skipping_quote_lines(),
        ),
        single(labelled("legend", "LEGEND1", "LEGEND2"), set_legend),
        single(ribbon_pattern("footer"), set_footer),
        Box::new(Multiline::new(
            ribbon_block_start("footer").clone(),
            plantuml_regex("^end[%s]?footer$"),
            set_multiline_footer,
        )),
        single(ribbon_pattern("header"), set_header),
        Box::new(Multiline::new(
            ribbon_block_start("header").clone(),
            plantuml_regex("^end[%s]?header$"),
            set_multiline_header,
        )),
    ]
}

type ApplyLine<D> = fn(&mut D, &RegexResult, &LineLocation);

/// A single-line command made of a pattern and what to do with what it matched.
struct Single<D> {
    pattern: RegexTree,
    apply: ApplyLine<D>,
}

fn single<D: TitledDiagram + 'static>(
    pattern: RegexTree,
    apply: ApplyLine<D>,
) -> Box<dyn Command<D>> {
    Box::new(SingleLine(Single { pattern, apply }))
}

impl<D: TitledDiagram> SingleLineCommand<D> for Single<D> {
    fn pattern(&self) -> &RegexTree {
        &self.pattern
    }

    fn execute_arg(
        &self,
        diagram: &mut D,
        location: &LineLocation,
        arg: &RegexResult,
    ) -> CommandResult {
        (self.apply)(diagram, arg, location);
        Ok(())
    }
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
        RegexTree::or(vec![
            RegexTree::named(1, quoted, "[%g](.*)[%g]"),
            RegexTree::named(1, plain, "(.*[%pLN_.].*)"),
        ]),
        RegexTree::end(),
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

fn set_skinparam<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, _: &LineLocation) {
    let name = arg.get("NAME", 0).unwrap_or_default().to_lowercase();
    let value = arg.get("VALUE", 0).unwrap_or_default();
    diagram.titled().skin.set_param(&name, value);
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
            StyleParsingError::Unexpected => CommandError::new("Error in style definition"),
        })
}

fn set_title<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    diagram.titled().set_title(label(arg, "TITLE"), location);
}

fn set_caption<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    diagram
        .titled()
        .set_caption(label(arg, "DISPLAY"), location);
}

fn set_legend<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    let legend = Positioned {
        display: label(arg, "LEGEND"),
        alignment: HorizontalAlignment::Center,
        location: location.clone(),
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
    let legend = Positioned {
        display: legend,
        alignment: horizontal
            .as_deref()
            .and_then(HorizontalAlignment::from_name)
            .unwrap_or(HorizontalAlignment::Center),
        location: first_location(&lines).clone(),
    };
    let vertical = match vertical.as_deref() {
        Some(top) if top.eq_ignore_ascii_case("top") => VerticalAlignment::Top,
        _ => VerticalAlignment::Bottom,
    };
    diagram.titled().set_legend(legend, vertical);
    Ok(())
}

/// `[left|right|center] header text`, or the same for footers.
fn ribbon_pattern(keyword: &'static str) -> RegexTree {
    RegexTree::concat(vec![
        RegexTree::start(),
        RegexTree::optional(RegexTree::named(1, "POSITION", "(left|right|center)")),
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(keyword),
        RegexTree::or(vec![
            RegexTree::concat(vec![
                RegexTree::spaces_zero_or_more(),
                RegexTree::leaf(":"),
                RegexTree::spaces_zero_or_more(),
            ]),
            RegexTree::spaces_one_or_more(),
        ]),
        RegexTree::or(vec![
            RegexTree::named(1, "LABEL1", "[%g](.*)[%g]"),
            RegexTree::named(1, "LABEL2", "(.*[%pLN_.].*)"),
        ]),
        RegexTree::end(),
    ])
}

fn ribbon_block_start(keyword: &str) -> &'static Regex {
    static HEADER: LazyLock<Regex> =
        LazyLock::new(|| plantuml_regex("^(?:(left|right|center)?[%s]*)header$"));
    static FOOTER: LazyLock<Regex> =
        LazyLock::new(|| plantuml_regex("^(?:(left|right|center)?[%s]*)footer$"));
    if keyword == "header" {
        &HEADER
    } else {
        &FOOTER
    }
}

/// The alignment the command gives, otherwise the one the part's style gives.
fn ribbon_alignment<D: TitledDiagram>(
    diagram: &mut D,
    given: Option<&str>,
    part: SName,
) -> HorizontalAlignment {
    match given.and_then(HorizontalAlignment::from_name) {
        Some(alignment) => alignment,
        None => diagram.titled().default_alignment(part),
    }
}

fn set_header<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    let header = ribbon(diagram, arg, location, SName::Header);
    diagram.titled().set_header(header);
}

fn set_footer<D: TitledDiagram>(diagram: &mut D, arg: &RegexResult, location: &LineLocation) {
    let footer = ribbon(diagram, arg, location, SName::Footer);
    diagram.titled().set_footer(footer);
}

fn ribbon<D: TitledDiagram>(
    diagram: &mut D,
    arg: &RegexResult,
    location: &LineLocation,
    part: SName,
) -> Positioned {
    Positioned {
        display: label(arg, "LABEL"),
        alignment: ribbon_alignment(diagram, arg.get("POSITION", 0), part),
        location: location.clone(),
    }
}

/// A header or footer block, aligned as written before its keyword or else as its style says.
fn ribbon_block<D: TitledDiagram>(
    diagram: &mut D,
    lines: &BlocLines,
    keyword: &str,
    part: SName,
) -> Result<Positioned, CommandError> {
    let lines = lines.trimmed();
    let first = lines.first().expect("the start line");
    let given = ribbon_block_start(keyword)
        .captures(first.text())
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_owned()));
    let display = lines.sub_extract(1, 1).to_display();
    if display.lines().is_empty() {
        return Err(CommandError::new(format!("Empty {keyword}")));
    }
    Ok(Positioned {
        display,
        alignment: ribbon_alignment(diagram, given.as_deref(), part),
        location: first.location().clone(),
    })
}

fn set_multiline_header<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let header = ribbon_block(diagram, lines, "header", SName::Header)?;
    diagram.titled().set_header(header);
    Ok(())
}

fn set_multiline_footer<D: TitledDiagram>(diagram: &mut D, lines: &BlocLines) -> CommandResult {
    let footer = ribbon_block(diagram, lines, "footer", SName::Footer)?;
    diagram.titled().set_footer(footer);
    Ok(())
}
