//! Help diagrams: `help keywords`, `help types`, `help colors`, `help fonts` and `help themes` list what the
//! language knows (PlantUML's `help` package).

mod language_descriptor;

use std::rc::Rc;

use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{Command, ParserPass, PatternCommand, SingleLine};
use crate::creole::{CreoleMode, CreoleParser, SheetBlock1};
use crate::klimt::font::{FontConfiguration, StringBounder, UFont};
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::pattern::{RegexResult, RegexTree};
use crate::style::SName;
use crate::text::LineLocation;

/// Reads help requests (PlantUML's `HelpFactory`).
pub(super) struct HelpFactory;

impl CommandFactory for HelpFactory {
    type Diagram = Help;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Help;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> Help {
        Help {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "HELP", source),
            lines: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<Help>>> {
        vec![
            command_help_color(),
            command_help_font(),
            command_help_keyword(),
            command_help_type(),
            command_help_theme(),
        ]
    }
}

/// PlantUML's `CommandHelpColor`: the colour names.
fn command_help_color() -> Box<dyn Command<Help>> {
    help_command(
        "colors?",
        ["<b>Help on colors", " The possible colors are :"],
        crate::color::names,
    )
}

/// PlantUML's `CommandHelpFont`: the font families. PlantUML lists those of the machine, rockuml those it
/// always has.
fn command_help_font() -> Box<dyn Command<Help>> {
    help_command(
        "fonts?",
        ["<b>Help on font", " The possible font on your system are :"],
        crate::fonts::FontRegistry::embedded_family_names,
    )
}

/// PlantUML's `CommandHelpKeyword`.
fn command_help_keyword() -> Box<dyn Command<Help>> {
    help_command(
        "keywords?",
        ["<b>Help on keywords", " The possible keywords are :"],
        || language_descriptor::get_keyword().into_iter().collect(),
    )
}

/// PlantUML's `CommandHelpType`: the types of element.
fn command_help_type() -> Box<dyn Command<Help>> {
    help_command(
        "types?",
        ["<b>Help on types", " The possible types are :"],
        || language_descriptor::get_type().into_iter().collect(),
    )
}

/// PlantUML's `CommandHelpTheme`: the bundled themes.
fn command_help_theme() -> Box<dyn Command<Help>> {
    help_command(
        "themes?",
        ["<b>Help on themes", " The possible themes are :"],
        crate::assets::theme_names,
    )
}

/// `help` and a topic, which adds the heading, a blank line, the introduction and a bullet per name.
fn help_command(
    topic: &'static str,
    [heading, introduction]: [&'static str; 2],
    names: fn() -> Vec<String>,
) -> Box<dyn Command<Help>> {
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(vec![
            RegexTree::start(),
            RegexTree::leaf(r"help"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf(topic),
            RegexTree::end(),
        ]),
        move |diagram: &mut Help, _: &LineLocation, _: &RegexResult| {
            diagram
                .lines
                .extend([heading, " ", introduction].map(str::to_owned));
            diagram
                .lines
                .extend(names().into_iter().map(|name| format!("* {name}")));
            Ok(())
        },
    )))
}

/// The help's lines, drawn as creole (PlantUML's `Help`).
pub(super) struct Help {
    source: Rc<UmlSource>,
    titled: Titled,
    lines: Vec<String>,
}

impl AbstractDiagram for Help {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for Help {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for Help {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let font = FontConfiguration::black_blue_true(UFont::serif(16));
        let sheet = CreoleParser::with_mode(
            font,
            HorizontalAlignment::Left,
            CreoleMode::Full,
            &self.titled.skin,
        )
        .create_sheet(&self.lines);
        let sheet = SheetBlock1::new(sheet, ClockwiseTopRightBottomLeft::none());
        Ok(self.titled.add_chrome(Box::new(sheet), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::none())
    }
}
