//! BPM diagrams, `@startbpm`: events in a row from a start circle, merges, branches, gotos and resumes laid
//! out on a grid (PlantUML's `bpm` package and `BpmDiagramFactory`). PlantUML does not document them, and
//! draws them as the sketch they are: the grid's lines, red stubs and blue connectors. The factory has no
//! common commands, so `title`, `skinparam` and `<style>` are syntax errors, and the elements are drawn
//! with a skin of their own.

mod element;
mod grid;
mod grid_array;

use std::rc::Rc;

use element::{BpmElement, BpmElementType};
use grid::{Coord, Grid};
use grid_array::GridArray;

use super::builder::CommandFactory;
use super::diagram_type::DiagramType;
use super::titled::{Titled, TitledDiagram};
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::command::factory::AbstractDiagram;
use crate::command::{
    Command, CommandError, CommandResult, ParserPass, PatternCommand, SingleLine,
};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::ClockwiseTopRightBottomLeft;
use crate::pattern::{RegexResult, RegexTree};
use crate::skin::SkinParam;
use crate::style::SName;
use crate::text::LineLocation;

/// Reads BPM diagrams (PlantUML's `BpmDiagramFactory`).
pub(super) struct BpmDiagramFactory;

impl CommandFactory for BpmDiagramFactory {
    type Diagram = BpmDiagram;

    const DIAGRAM_TYPE: DiagramType = DiagramType::Bpm;

    fn create_empty_diagram(source: &Rc<UmlSource>) -> BpmDiagram {
        BpmDiagram {
            source: source.clone(),
            titled: Titled::new(SName::ActivityDiagram, "BPM", source),
            events: Vec::new(),
            branches: Vec::new(),
        }
    }

    fn init_commands_list() -> Vec<Box<dyn Command<BpmDiagram>>> {
        vec![
            command_docked_event(),
            command_merge(),
            command_resume(),
            command_goto(),
            command_new_branch(),
            command_else_branch(),
            command_end_branch(),
        ]
    }
}

type Execute = fn(&mut BpmDiagram, &LineLocation, &RegexResult) -> CommandResult;

fn command(parts: Vec<RegexTree>, execute: Execute) -> Box<dyn Command<BpmDiagram>> {
    let mut pattern = vec![RegexTree::start()];
    pattern.extend(parts);
    pattern.push(RegexTree::end());
    Box::new(SingleLine(PatternCommand::new(
        RegexTree::concat(pattern),
        execute,
    )))
}

fn id() -> RegexTree {
    RegexTree::named(1, "ID", r"([%pLN_.@]+)")
}

fn get_id(arg: &RegexResult) -> String {
    arg.get("ID", 0).unwrap_or_default().to_owned()
}

/// PlantUML's `CommandDockedEvent`: `:label;`.
fn command_docked_event() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![
            RegexTree::leaf(":"),
            RegexTree::named(1, "LABEL", "(.*)"),
            RegexTree::leaf(";"),
        ],
        |diagram, _, arg| {
            let label = arg.get("LABEL", 0).unwrap_or_default().to_owned();
            diagram.add_event(BpmEvent::Add(BpmElement::new(
                None,
                BpmElementType::DockedEvent,
                Some(label),
            )));
            Ok(())
        },
    )
}

/// PlantUML's `CommandMerge`: `id<+>` or `id:<+>`.
fn command_merge() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![id(), RegexTree::leaf(":?"), RegexTree::leaf(r"<\+>")],
        |diagram, _, arg| {
            diagram.add_event(BpmEvent::Add(BpmElement::new(
                Some(get_id(arg)),
                BpmElementType::Merge,
                None,
            )));
            Ok(())
        },
    )
}

/// PlantUML's `CommandResume`: `resume id`.
fn command_resume() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![
            RegexTree::leaf("resume"),
            RegexTree::spaces_one_or_more(),
            id(),
        ],
        |diagram, _, arg| {
            diagram.add_event(BpmEvent::Resume(get_id(arg)));
            Ok(())
        },
    )
}

/// PlantUML's `CommandGoto`: `goto id`.
fn command_goto() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![
            RegexTree::leaf("goto"),
            RegexTree::spaces_one_or_more(),
            id(),
        ],
        |diagram, _, arg| {
            diagram.add_event(BpmEvent::Goto(get_id(arg)));
            Ok(())
        },
    )
}

/// PlantUML's `CommandNewBranch`: `new branch`.
fn command_new_branch() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![
            RegexTree::leaf("new"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("branch"),
        ],
        |diagram, _, _| {
            diagram.new_branch();
            Ok(())
        },
    )
}

/// PlantUML's `CommandElseBranch`: `else`.
fn command_else_branch() -> Box<dyn Command<BpmDiagram>> {
    command(vec![RegexTree::leaf("else")], |diagram, _, _| {
        diagram.else_branch()
    })
}

/// PlantUML's `CommandEndBranch`: `end branch`.
fn command_end_branch() -> Box<dyn Command<BpmDiagram>> {
    command(
        vec![
            RegexTree::leaf("end"),
            RegexTree::spaces_one_or_more(),
            RegexTree::leaf("branch"),
        ],
        |diagram, _, _| diagram.end_branch(),
    )
}

/// What a line asks of the grid (`BpmEvent`, `BpmEventAdd`, `BpmEventGoto`, `BpmEventResume`).
enum BpmEvent {
    /// An element in a new column after the current one, connected from the last.
    Add(BpmElement),
    /// Goes on from the named element, on a new line below it.
    Resume(String),
    /// Connects the last element to the named one, and goes on from there.
    Goto(String),
}

/// An open `new branch`: a merge at its entry, another at its exit (`BpmBranch`).
struct BpmBranch {
    uid: usize,
    counter: usize,
}

/// What an `else` or `end branch` without an open branch throws, which PlantUML reports as a syntax error.
const NO_SUCH_ELEMENT: &str = "java.util.NoSuchElementException";

impl BpmBranch {
    fn get_entry_id(&self) -> String {
        format!("$branchA{}", self.uid)
    }

    fn get_exit_id(&self) -> String {
        format!("$branchB{}", self.uid)
    }

    fn get_entry_element(&self) -> BpmElement {
        BpmElement::new(Some(self.get_entry_id()), BpmElementType::Merge, None)
    }

    fn get_else_element(&self) -> BpmElement {
        BpmElement::new(Some(self.get_exit_id()), BpmElementType::Merge, None)
    }

    fn get_resume_entry_event(&self) -> BpmEvent {
        BpmEvent::Resume(self.get_entry_id())
    }

    fn get_go_to_end_event(&self) -> BpmEvent {
        BpmEvent::Goto(self.get_exit_id())
    }
}

/// A `@startbpm` diagram (PlantUML's `BpmDiagram`).
pub(super) struct BpmDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    events: Vec<BpmEvent>,
    branches: Vec<BpmBranch>,
}

impl BpmDiagram {
    fn add_event(&mut self, event: BpmEvent) {
        self.events.push(event);
    }

    fn new_branch(&mut self) {
        let branch = BpmBranch {
            uid: self.events.len(),
            counter: 1,
        };
        let entry = branch.get_entry_element();
        self.branches.push(branch);
        self.add_event(BpmEvent::Add(entry));
    }

    /// The first `else` closes the first way with the exit merge, the others go to it; each starts a way
    /// below the entry.
    fn else_branch(&mut self) -> CommandResult {
        let branch = self
            .branches
            .last_mut()
            .ok_or_else(|| CommandError::thrown(NO_SUCH_ELEMENT))?;
        branch.counter += 1;
        let close = if branch.counter == 2 {
            BpmEvent::Add(branch.get_else_element())
        } else {
            branch.get_go_to_end_event()
        };
        let resume = branch.get_resume_entry_event();
        self.add_event(close);
        self.add_event(resume);
        Ok(())
    }

    fn end_branch(&mut self) -> CommandResult {
        let branch = self
            .branches
            .pop()
            .ok_or_else(|| CommandError::thrown(NO_SUCH_ELEMENT))?;
        self.add_event(branch.get_go_to_end_event());
        Ok(())
    }

    fn create_grid(&self) -> Result<Grid, NotYetPorted> {
        let mut grid = Grid::new();
        let mut current = grid.get_root();
        let mut last = current;
        grid.set_element(current, BpmElement::new(None, BpmElementType::Start, None));
        for event in &self.events {
            match event {
                BpmEvent::Add(element) => {
                    let col = grid.insert_col_after(current.col);
                    current = Coord::new(current.line, col);
                    grid.set_element(current, element.clone());
                    grid.add_connection(last, current);
                    last = current;
                }
                BpmEvent::Resume(id) => {
                    last = grid.get_by_id(id).ok_or(grid::CRASH)?;
                    current = Coord::new(grid.insert_line_after(last.line), last.col);
                }
                BpmEvent::Goto(id) => {
                    let src = last;
                    last = grid.get_by_id(id).ok_or(grid::CRASH)?;
                    current = Coord::new(grid.insert_line_after(last.line), last.col);
                    grid.add_connection(src, last);
                }
            }
        }
        grid.add_connections()?;
        Ok(grid)
    }
}

impl AbstractDiagram for BpmDiagram {
    fn starting_pass(&mut self, _pass: ParserPass) {}
}

impl TitledDiagram for BpmDiagram {
    fn titled(&mut self) -> &mut Titled {
        &mut self.titled
    }
}

impl Diagram for BpmDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        let mut grid = self.create_grid()?;
        grid.clean()?;
        let skin_param = Rc::new(SkinParam::new(&self.source));
        let drawing = GridArray::new(grid.into_array(), &skin_param);
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::diagram::{ImageFormat, export};
    use crate::fonts::FontRegistry;
    use crate::host::IsolatedHost;
    use crate::preproc::{PreprocessorEnvironment, Source};

    fn export_debug(body: &str) -> Result<Vec<u8>, NotYetPorted> {
        let text = format!("@startbpm\n{body}\n@endbpm\n");
        let source = Source {
            text: &text,
            description: "t",
            directory: std::path::PathBuf::new(),
            environment: PreprocessorEnvironment::default(),
        };
        let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
        let diagram = crate::diagram::create(&block, &IsolatedHost)?;
        export(
            diagram.as_ref(),
            0,
            ImageFormat::Debug,
            &Arc::new(FontRegistry::default()),
            &IsolatedHost,
        )
    }

    #[test]
    fn a_goto_from_another_line_is_drawn() {
        assert!(export_debug(":A;\nm<+>\n:B;\nresume m\n:C;\ngoto m").is_ok());
    }

    #[test]
    fn names_plantuml_does_not_find_crash_it() {
        assert_eq!(export_debug(":A;\nresume nowhere").err(), Some(grid::CRASH));
        assert_eq!(export_debug(":A;\ngoto nowhere").err(), Some(grid::CRASH));
    }

    #[test]
    fn a_goto_back_along_its_line_crashes_plantuml() {
        assert_eq!(
            export_debug("m<+>\n:A;\n:B;\ngoto m").err(),
            Some(grid::CRASH)
        );
    }

    #[test]
    fn a_goto_to_itself_crashes_plantuml() {
        assert_eq!(export_debug("m<+>\ngoto m").err(), Some(grid::CRASH));
    }
}
