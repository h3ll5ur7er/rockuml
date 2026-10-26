//! EBNF grammars drawn as railroad diagrams, `@startebnf` (PlantUML's `ebnf` package). Its tiles also draw
//! regular expressions.

mod curves;
mod engine;
mod expression;
pub(super) mod tile;

use std::rc::Rc;

use engine::EbnfEngine;
use expression::{Symbol, Token};
use tile::{ETile, ETileWithCircles, TileStyle};

use super::builder::CommandFactory;
use super::common_commands::add_common_commands1;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{
    BlocLines, Command, CommandError, CommandResult, Multiline, PastTheEnd, PatternCommand,
    SingleLine, UBrexSingleLine,
};
use crate::creole::{CreoleMode, Display};
use crate::ftile::vcompact::FloatingNote;
use crate::klimt::blocks::{TextBlockMarged, TextBlockVertical};
use crate::klimt::font::{FontConfiguration, FontStyle, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::shape::{UShape, UText};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::{RegexResult, RegexTree, plantuml_regex};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::text::{LineLocation, StringLocated};
use crate::ubrex::UnicodeBracketedExpression;
use crate::ubrex::builder::UBrexPart;

/// What a diagram holds, in order: rules and notes (`TextBlockable`).
enum TextBlockable {
    Rule(EbnfExpression),
    Note(Display),
}

/// A rule's tokens and the comments written around its name (`EbnfExpression`).
struct EbnfExpression {
    tokens: Vec<Token>,
    comment_above: Option<String>,
    comment_below: Option<String>,
}

pub(super) struct PSystemEbnf {
    source: Rc<UmlSource>,
    titled: Titled,
    expressions: Vec<TextBlockable>,
}

/// Reads EBNF grammars (PlantUML's `PSystemEbnfFactory`).
pub(super) struct PSystemEbnfFactory;

impl CommandFactory for PSystemEbnfFactory {
    type Diagram = PSystemEbnf;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Ebnf;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> PSystemEbnf {
        PSystemEbnf {
            source: source.clone(),
            titled: Titled::new(SName::Ebnf, "EBNF", source),
            expressions: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<PSystemEbnf>>> {
        let mut commands = add_common_commands1();
        commands.extend([
            ubrex_command_comment(),
            command_comment_multilines2(),
            command_comment_multilines(),
            command_ebnf_single_line(),
            ubrex_command_ebnf_multilines(),
        ]);
        commands
    }
}

/// PlantUML's `UBrexCommandComment`: `(* comment *)` alone on its line.
fn ubrex_command_comment() -> Box<dyn Command<PSystemEbnf>> {
    Box::new(UBrexSingleLine::new(
        UnicodeBracketedExpression::build("〇*〴s (* 〶$COMMENT=〄>〘*) 〇*〴s 〒$〙"),
        |diagram: &mut PSystemEbnf, _: &LineLocation, matcher: &crate::ubrex::UMatcher| {
            let comment = matcher
                .find_values_by_key("COMMENT")
                .first()
                .copied()
                .unwrap_or_default();
            diagram.add_note(Display::with_newlines(crate::java::trim(comment)));
            Ok(())
        },
    ))
}

/// PlantUML's `CommandCommentMultilines2`: `(** … **)` over several lines.
fn command_comment_multilines2() -> Box<dyn Command<PSystemEbnf>> {
    Box::new(Multiline::new(
        &plantuml_regex(r"^[%s]*\(\*\*.*$"),
        &plantuml_regex(r"^.*\*\*\)[%s]*$"),
        |diagram: &mut PSystemEbnf, lines: &BlocLines| {
            diagram.add_note(lines.remove_few_chars(3).to_display());
            Ok(())
        },
    ))
}

/// PlantUML's `CommandCommentMultilines`: `(* … *)` over several lines.
fn command_comment_multilines() -> Box<dyn Command<PSystemEbnf>> {
    Box::new(Multiline::new(
        &plantuml_regex(r"^[%s]*\(\*.*$"),
        &plantuml_regex(r"^.*\*\)[%s]*$"),
        |diagram: &mut PSystemEbnf, lines: &BlocLines| {
            diagram.add_note(lines.remove_few_chars(2).to_display());
            Ok(())
        },
    ))
}

/// A comment `(* … *)` before or after a rule's name.
fn rule_comment(name: &'static str) -> RegexTree {
    RegexTree::optional(RegexTree::concat(vec![
        RegexTree::spaces_zero_or_more(),
        RegexTree::leaf(r"\(\*"),
        RegexTree::named(1, name, r"(.*[^%s].*)"),
        RegexTree::leaf(r"\*\)"),
        RegexTree::spaces_zero_or_more(),
    ]))
}

/// PlantUML's `CommandEBnfSingleLine`: `name = …;` on one line, with comments around the name.
fn command_ebnf_single_line() -> Box<dyn Command<PSystemEbnf>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            rule_comment("COMMENTA"),
            RegexTree::named(1, "ID", r"([%pLN_][-%pLN_]*)"),
            rule_comment("COMMENTB"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "EQUALS", "(=)"),
            RegexTree::spaces_zero_or_more(),
            RegexTree::named(1, "LINE", "(.*;)"),
            RegexTree::end(),
        ]),
        |diagram: &mut PSystemEbnf, location: &LineLocation, arg: &RegexResult| {
            let full = ["ID", "EQUALS", "LINE"]
                .map(|name| arg.get(name, 0).unwrap_or_default())
                .concat();
            diagram.add_bloc_lines(
                &BlocLines::single(StringLocated::new(full, location.clone())),
                arg.get("COMMENTA", 0),
                arg.get("COMMENTB", 0),
            )
        },
    )))
}

/// PlantUML's `UBrexCommandEbnfMultilines`: `name =` then lines up to one ending with `;`.
fn ubrex_command_ebnf_multilines() -> Box<dyn Command<PSystemEbnf>> {
    let start = UBrexPart::named(
        "LINE",
        UBrexPart::concat(vec![
            UBrexPart::leaf("「〴an_」"),
            UBrexPart::leaf("〇*「〴an_-」"),
            UBrexPart::space_zero_or_more(),
            UBrexPart::leaf("="),
            UBrexPart::leaf("〇*〴."),
        ]),
    );
    Box::new(Multiline::between_ubrex(
        start.build(),
        UnicodeBracketedExpression::build("〇+〘 〄>; 〙"),
        |diagram: &mut PSystemEbnf, lines: &BlocLines| diagram.add_bloc_lines(lines, None, None),
    ))
}

impl PSystemEbnf {
    fn add_bloc_lines(
        &mut self,
        lines: &BlocLines,
        comment_above: Option<&str>,
        comment_below: Option<&str>,
    ) -> CommandResult {
        let tokens = expression::tokenize(&mut lines.inspector())
            .map_err(|_| CommandError::thrown(PastTheEnd::EXCEPTION))?;
        if tokens.is_empty() {
            return Err(CommandError::new("Unparsable expression"));
        }
        self.expressions.push(TextBlockable::Rule(EbnfExpression {
            tokens,
            comment_above: comment_above.map(str::to_owned),
            comment_below: comment_below.map(str::to_owned),
        }));
        Ok(())
    }

    fn add_note(&mut self, note: Display) {
        self.expressions.push(TextBlockable::Note(note));
    }
}

/// The `ebnf` element style's font and colours, which tiles are drawn with (`ETile.getStyleSignature`).
pub(super) fn tile_style(skin_param: &SkinParam) -> TileStyle {
    let style = StyleSignature::of(&[SName::Root, SName::Element, SName::Ebnf])
        .get_merged_style(&skin_param.current_style_builder());
    TileStyle {
        font: style.font_configuration(),
        line_color: style.value(PName::LineColor).as_color(),
        background_color: style.value(PName::BackGroundColor).as_color(),
    }
}

/// `Syntax error!`, drawn where a rule cannot be (`EbnfEngine.syntaxError`).
fn syntax_error<'a>(font: &FontConfiguration, skin_param: &SkinParam) -> Box<dyn TextBlock + 'a> {
    Box::new(Display::create(["Syntax error!"]).create0(
        font,
        HorizontalAlignment::Left,
        skin_param,
        0.0,
        CreoleMode::Full,
    ))
}

fn note(text: &str, skin_param: &SkinParam) -> FloatingNote {
    FloatingNote::create(&Display::with_newlines(text), skin_param, SName::Ebnf)
}

fn merge_tb<'a>(
    top: Box<dyn TextBlock + 'a>,
    bottom: Box<dyn TextBlock + 'a>,
    alignment: HorizontalAlignment,
) -> Box<dyn TextBlock + 'a> {
    Box::new(TextBlockVertical::new(vec![top, bottom], alignment))
}

impl EbnfExpression {
    /// The rule's name above its track, or a syntax error (`getUDrawable`).
    fn get_u_drawable<'a>(&self, skin_param: &'a SkinParam) -> Box<dyn TextBlock + 'a> {
        let style = tile_style(skin_param);
        let font = style.font.clone();
        let (Some(name), Some(definition)) = (self.tokens.first(), self.tokens.get(1)) else {
            return syntax_error(&font, skin_param);
        };
        if definition.symbol != Symbol::Definition {
            return syntax_error(&font, skin_param);
        }
        let main = if self.tokens.len() > 2 {
            match expression::shunting_yard(&self.tokens[2..]) {
                Ok(full) if !full.is_empty() => {
                    match EbnfEngine::new(style.clone(), skin_param).run(&full) {
                        Ok(Some(main)) => Box::new(main) as Box<dyn TextBlock>,
                        // Tiles left over make the track a syntax error under the rule's name.
                        Ok(None) => syntax_error(&font, skin_param),
                        Err(_) => return syntax_error(&font, skin_param),
                    }
                }
                _ => return syntax_error(&font, skin_param),
            }
        } else {
            Box::new(ETileWithCircles::new(
                ETile::Empty,
                style.line_color.clone(),
            ))
        };
        let mut title_box: Box<dyn TextBlock + 'a> =
            Box::new(TitleBox::new(name.data.clone().unwrap_or_default(), &font));
        if let Some(comment) = &self.comment_above {
            title_box = merge_tb(
                Box::new(note(comment, skin_param)),
                title_box,
                HorizontalAlignment::Center,
            );
        }
        if let Some(comment) = &self.comment_below {
            title_box = merge_tb(
                title_box,
                Box::new(note(comment, skin_param)),
                HorizontalAlignment::Center,
            );
        }
        merge_tb(
            title_box,
            Box::new(TextBlockMarged::new(
                main,
                ClockwiseTopRightBottomLeft::top_right_bottom_left(10.0, 0.0, 15.0, 0.0),
            )),
            HorizontalAlignment::Left,
        )
    }
}

/// A rule's name in bold (`TitleBox`).
struct TitleBox {
    value: String,
    font: FontConfiguration,
}

impl TitleBox {
    fn new(value: String, font: &FontConfiguration) -> Self {
        Self {
            value,
            font: font.with_style(FontStyle::Bold),
        }
    }
}

impl TextBlock for TitleBox {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        string_bounder.calculate_dimension(&self.font.font(), &self.value)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let string_bounder = ug.string_bounder();
        let height = self.calculate_dimension(string_bounder).height;
        let descent = string_bounder.descent(&self.font.font(), &self.value);
        ug.translated(0.0, height - descent)
            .draw(&UShape::Text(UText::new(&self.value, self.font.clone())));
    }
}

impl AbstractDiagram for PSystemEbnf {}

impl TitledDiagram for PSystemEbnf {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for PSystemEbnf {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let skin_param = &self.titled.skin;
        let blocks: Vec<Box<dyn TextBlock + '_>> = self
            .expressions
            .iter()
            .map(|expression| match expression {
                TextBlockable::Rule(rule) => rule.get_u_drawable(skin_param),
                TextBlockable::Note(display) => Box::new(TextBlockMarged::new(
                    FloatingNote::create(display, skin_param, SName::Ebnf),
                    ClockwiseTopRightBottomLeft::top_right_bottom_left(5.0, 0.0, 15.0, 0.0),
                )) as Box<dyn TextBlock>,
            })
            .collect();
        let drawing = if blocks.is_empty() {
            syntax_error(&tile_style(skin_param).font, skin_param)
        } else {
            Box::new(TextBlockVertical::new(blocks, HorizontalAlignment::Left))
        };
        Ok(self.titled.add_chrome(drawing, string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}
