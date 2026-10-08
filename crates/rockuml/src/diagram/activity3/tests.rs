//! The activity corpus read by the commands, against the instruction trees PlantUML's commands build. The
//! fixture comes from `tools/oracle/activity-unit/ModelDump.java`.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::rc::Rc;

use super::branch::{Branch, InstructionIf, InstructionSwitch};
use super::group::InstructionGroup;
use super::instruction::{
    Instruction, InstructionId, InstructionList, Instructions, NoteType, PositionedNote,
};
use super::link_rendering::LinkRendering;
use super::loops::{InstructionRepeat, InstructionWhile};
use super::parallel::{InstructionFork, InstructionSplit};
use super::swimlanes::{SwimlaneId, Swimlanes};
use super::{ActivityDiagram3, ActivityDiagramFactory3};
use crate::color::{ColorType, Colors, HColor};
use crate::command::factory::{Created, create_system};
use crate::creole::Display;
use crate::decoration::Rainbow;
use crate::decoration::symbol::{USymbol, USymbols};
use crate::diagram::builder::CommandFactory;
use crate::ftile::BoxStyle;
use crate::host::IsolatedHost;
use crate::klimt::url::Url;
use crate::preproc::{PreprocessorEnvironment, Source, preprocess};
use crate::stereo::{Stereogroup, Stereotype};

mod notes_groups;

const FIXTURE: &str = include_str!("../../../tests/data/activity-model.txt");

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus");

/// What the activity commands make of the lines between `@startuml` and `@enduml`.
fn parse(text: &str, case: &str, directory: PathBuf) -> Created<ActivityDiagram3> {
    let source = Source {
        text,
        description: case,
        directory,
        environment: PreprocessorEnvironment::default(),
    };
    let block = preprocess(&source, &IsolatedHost).remove(0);
    let source = Rc::new(crate::diagram::prepare(&block).1);
    let commands = ActivityDiagramFactory3::init_commands_list();
    create_system(
        &source,
        || ActivityDiagramFactory3::create_empty_diagram(&source),
        &commands,
    )
}

fn read(case: &str) -> ActivityDiagram3 {
    let path = PathBuf::from(CORPUS).join(case);
    let text = std::fs::read_to_string(&path).unwrap();
    match parse(&text, case, path.parent().unwrap().to_owned()) {
        Created::Diagram(diagram) => diagram,
        Created::Failure(failure) => panic!("{case}: {failure:?}"),
        Created::Nothing => panic!("{case}: no diagram"),
    }
}

/// The error message the lines fail with, and the line it names.
fn failure(body: &[&str]) -> (String, String) {
    let text = ["@startuml", &body.join("\n"), "@enduml"].join("\n");
    match parse(&text, "test", PathBuf::new()) {
        Created::Failure(failure) => (
            failure.error.message,
            failure.trace.last().unwrap().text().to_owned(),
        ),
        Created::Diagram(_) => panic!("{body:?} made a diagram"),
        Created::Nothing => panic!("{body:?} made nothing"),
    }
}

fn quoted(text: Option<&str>) -> String {
    text.map_or_else(
        || "null".to_owned(),
        |text| {
            format!(
                "\"{}\"",
                text.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            )
        },
    )
}

fn display(display: Option<&Display>) -> String {
    quoted(display.map(|display| display.lines().join("\n")).as_deref())
}

fn as_string(color: &HColor) -> String {
    color.as_string()
}

fn color(name: &str, color: Option<&HColor>) -> String {
    color.map_or_else(String::new, |color| format!(" {name}={}", as_string(color)))
}

fn colors(colors: Option<&Colors>) -> String {
    let Some(colors) = colors else {
        return String::new();
    };
    let mut result = String::new();
    for (kind, name) in [
        (ColorType::Text, "TEXT"),
        (ColorType::Line, "LINE"),
        (ColorType::Back, "BACK"),
        (ColorType::Header, "HEADER"),
        (ColorType::Arrow, "ARROW"),
    ] {
        if let Some(color) = colors.get(kind) {
            write!(result, " {name}={}", as_string(color)).unwrap();
        }
    }
    if let Some(stroke) = colors.get_specific_line_stroke() {
        write!(result, " stroke={stroke}").unwrap();
    }
    result
}

fn rainbow(rainbow: &Rainbow) -> String {
    let colors: Vec<String> = rainbow
        .get_colors()
        .iter()
        .map(|color| {
            format!(
                "{}/{}/{}",
                as_string(color.get_arrow_color()),
                as_string(color.get_arrow_head_color()),
                color.get_style()
            )
        })
        .collect();
    let mut result = format!("[{}]", colors.join(","));
    if rainbow.get_color_arrow_separation_space() != 0 {
        write!(result, "~{}", rainbow.get_color_arrow_separation_space()).unwrap();
    }
    result
}

fn link(link: Option<&LinkRendering>) -> String {
    link.map_or_else(
        || "null".to_owned(),
        |link| {
            format!(
                "({} {})",
                display(link.display.as_ref()),
                rainbow(&link.rainbow)
            )
        },
    )
}

fn box_style(style: Option<BoxStyle>) -> &'static str {
    style.and_then(BoxStyle::name).unwrap_or("null")
}

fn stereo(stereotype: Option<&Stereotype>) -> String {
    stereotype.map_or_else(String::new, |stereotype| {
        format!(" stereo={}", quoted(Some(&stereotype.to_string())))
    })
}

fn stereotype_text(stereotype: Option<&Stereotype>) -> String {
    quoted(stereotype.map(ToString::to_string).as_deref())
}

fn stereogroup(stereogroup: &Stereogroup) -> String {
    stereotype_text(stereogroup.build_stereotype().as_ref())
}

fn url(url: Option<&Url>) -> String {
    url.map_or_else(String::new, |url| {
        format!(" url={}", quoted(Some(&url.href)))
    })
}

fn flag(set: bool, name: &str) -> String {
    if set {
        format!(" {name}")
    } else {
        String::new()
    }
}

fn symbol(symbol: USymbol) -> &'static str {
    ["PARTITION", "PACKAGE", "RECTANGLE", "CARD", "GROUP"]
        .into_iter()
        .find(|code| USymbols::by_code(code) == Some(symbol))
        .unwrap()
}

/// The tree as `ModelDump` writes it.
struct Dump<'a> {
    swimlanes: &'a Swimlanes,
    out: String,
}

impl Dump<'_> {
    fn lane(&self, lane: Option<SwimlaneId>) -> String {
        quoted(lane.map(|lane| self.swimlanes.swimlanes()[lane.0].name.as_str()))
    }

    fn line(&mut self, depth: usize, text: &str) {
        writeln!(self.out, "{}{text}", "  ".repeat(depth)).unwrap();
    }

    fn list(&mut self, name: &str, list: &InstructionList, depth: usize) {
        let text = format!(
            "{name} lane={} out={}",
            self.lane(list.default_swimlane),
            link(list.outlink_rendering.as_ref())
        );
        self.line(depth, &text);
        self.notes(&list.notes.notes, depth + 1);
        for ins in &list.all {
            self.instruction(*ins, depth + 1);
        }
    }

    fn notes(&mut self, notes: &[PositionedNote], depth: usize) {
        for note in notes {
            let text = self.note(note);
            self.line(depth, &text);
        }
    }

    fn note(&self, note: &PositionedNote) -> String {
        format!(
            "note {} {} {} lane={}{}{}",
            format!("{:?}", note.note_position).to_uppercase(),
            match note.type_ {
                NoteType::Note => "NOTE",
                NoteType::FloatingNote => "FLOATING_NOTE",
            },
            display(Some(&note.display)),
            self.lane(note.swimlane_note),
            colors(Some(&note.colors)),
            stereo(note.stereotype.as_ref())
        )
    }

    fn instruction(&mut self, id: InstructionId, depth: usize) {
        let swimlanes = self.swimlanes;
        let instructions = &swimlanes.instructions;
        let in_link = link(Some(instructions.get_in_link_rendering(id)));
        match instructions.get(id) {
            Instruction::Simple(ins) => {
                let text = format!(
                    "simple {} lane={} box={} in={in_link}{}{}{}{}",
                    display(Some(&ins.label)),
                    self.lane(ins.mono.swimlane),
                    box_style(Some(ins.box_style)),
                    colors(Some(&ins.colors)),
                    stereo(ins.stereotype.as_ref()),
                    url(ins.url.as_ref()),
                    flag(ins.killed, "killed")
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Spot(ins) => {
                let text = format!(
                    "spot {} lane={} in={in_link}{}{}",
                    quoted(Some(&ins.spot)),
                    self.lane(ins.mono.swimlane),
                    color("color", ins.color.as_ref()),
                    flag(ins.killed, "killed")
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Start(ins) => {
                let text = format!(
                    "start lane={} in={in_link}{}",
                    self.lane(ins.mono.swimlane),
                    colors(Some(&ins.colors))
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Stop(ins) => {
                let text = format!(
                    "stop lane={} in={in_link}{}",
                    self.lane(ins.mono.swimlane),
                    colors(Some(&ins.colors))
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::End(ins) => {
                let text = format!(
                    "end lane={} in={in_link}{}",
                    self.lane(ins.mono.swimlane),
                    colors(Some(&ins.colors))
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Break(ins) => {
                let text = format!("break lane={} in={in_link}", self.lane(ins.mono.swimlane));
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Goto(ins) => {
                let text = format!(
                    "goto {} lane={}",
                    quoted(Some(&ins.name)),
                    self.lane(ins.mono.swimlane)
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::Label(ins) => {
                let text = format!(
                    "label {} lane={}",
                    quoted(Some(&ins.name)),
                    self.lane(ins.mono.swimlane)
                );
                self.line(depth, &text);
                self.notes(&ins.mono.notes.notes, depth + 1);
            }
            Instruction::If(ins) => self.instruction_if(ins, &in_link, depth),
            Instruction::Switch(ins) => self.instruction_switch(ins, &in_link, depth),
            Instruction::While(ins) => self.instruction_while(ins, &in_link, depth),
            Instruction::Repeat(ins) => self.instruction_repeat(ins, &in_link, depth),
            Instruction::Fork(ins) => self.instruction_fork(ins, &in_link, depth),
            Instruction::Split(ins) => self.instruction_split(ins, &in_link, depth),
            Instruction::Group(ins) => self.instruction_group(ins, &in_link, depth),
            Instruction::List(_) => unreachable!("only the root is a list"),
        }
    }

    fn instruction_if(&mut self, ins: &InstructionIf, in_link: &str, depth: usize) {
        let text = format!(
            "if lane={} in={in_link} out={}{}{}{}",
            self.lane(ins.swimlane),
            link(Some(&ins.out_color)),
            url(ins.url.as_ref()),
            stereo(ins.stereotype.as_ref()),
            flag(ins.endif_called, "endif")
        );
        self.line(depth, &text);
        self.notes(&ins.notes.notes, depth + 1);
        for branch in &ins.thens {
            self.branch("then", branch, depth + 1);
        }
        if let Some(else_branch) = &ins.else_branch {
            self.branch("else", else_branch, depth + 1);
        }
    }

    fn instruction_switch(&mut self, ins: &InstructionSwitch, in_link: &str, depth: usize) {
        let text = format!(
            "switch lane={} test={} in={in_link} colors=[{} ] endColors=[{} ]",
            self.lane(ins.swimlane),
            display(ins.label_test.as_ref()),
            colors(Some(&ins.colors)),
            colors(ins.end_colors.as_ref())
        );
        self.line(depth, &text);
        self.notes(&ins.notes.notes, depth + 1);
        for branch in &ins.switches {
            self.branch("case", branch, depth + 1);
        }
    }

    fn instruction_while(&mut self, ins: &InstructionWhile, in_link: &str, depth: usize) {
        let text = format!(
            "while lane={} test={} yes={} in={in_link} out={}{} backward={} box={}{} incoming1={} \
             incoming2={}{}{}{}",
            self.lane(ins.swimlane),
            display(Some(&ins.test)),
            display(ins.yes.as_ref()),
            link(Some(&ins.out_color)),
            color("color", ins.color.as_ref()),
            display(ins.backward.as_ref()),
            box_style(ins.box_style),
            stereo(ins.stereotype.as_ref()),
            link(Some(&ins.incoming1)),
            link(Some(&ins.incoming2)),
            flag(ins.test_called, "testCalled"),
            flag(ins.backward_called, "backwardCalled"),
            flag(ins.killed, "killed")
        );
        self.line(depth, &text);
        self.notes(&ins.notes.notes, depth + 1);
        if let Some(special) = ins.special_out {
            self.line(depth + 1, "special");
            self.instruction(special, depth + 2);
        }
        self.list("list", &ins.repeat_list, depth + 1);
    }

    fn instruction_repeat(&mut self, ins: &InstructionRepeat, in_link: &str, depth: usize) {
        let text = format!(
            "repeat lane={} laneOut={} laneBackward={} in={in_link} start={} boxIn={} stereoLoop={} \
             backward={} box={} stereoBack={} test={} yes={} out={} incoming1={} incoming2={} end={} \
             stereo2={}{}{}",
            self.lane(ins.swimlane),
            self.lane(ins.swimlane_out),
            self.lane(ins.swimlane_backward),
            display(ins.start_label.as_ref()),
            box_style(Some(ins.box_style_in)),
            stereogroup(&ins.stereogroup_loop),
            display(ins.backward.as_ref()),
            box_style(ins.box_style),
            stereotype_text(ins.stereotype_back.as_ref()),
            display(ins.test.as_ref()),
            display(ins.yes.as_ref()),
            display(ins.out.as_ref()),
            link(Some(&ins.incoming1)),
            link(Some(&ins.incoming2)),
            link(Some(&ins.end_repeat_link_rendering)),
            stereogroup(&ins.stereotype2),
            flag(ins.test_called, "testCalled"),
            flag(ins.killed, "killed")
        );
        self.line(depth, &text);
        for note in &ins.backward_notes {
            let text = format!("backward {}", self.note(note));
            self.line(depth + 1, &text);
        }
        self.list("list", &ins.repeat_list, depth + 1);
    }

    fn instruction_fork(&mut self, ins: &InstructionFork, in_link: &str, depth: usize) {
        let text = format!(
            "fork laneIn={} laneOut={} in={in_link} style={} label={}{}{}",
            self.lane(ins.swimlane_in),
            self.lane(ins.swimlane_out),
            format!("{:?}", ins.style).to_uppercase(),
            quoted(ins.label.as_deref()),
            flag(ins.finished, "finished"),
            colors(Some(&ins.colors))
        );
        self.line(depth, &text);
        self.notes(&ins.notes.notes, depth + 1);
        for list in &ins.forks {
            self.list("list", list, depth + 1);
        }
    }

    fn instruction_split(&mut self, ins: &InstructionSplit, in_link: &str, depth: usize) {
        let text = format!(
            "split laneIn={} laneOut={} in={in_link}",
            self.lane(ins.swimlane_in),
            self.lane(ins.swimlane_out)
        );
        self.line(depth, &text);
        for list in &ins.splits {
            self.list("list", list, depth + 1);
        }
    }

    fn instruction_group(&mut self, ins: &InstructionGroup, in_link: &str, depth: usize) {
        let text = format!(
            "group {} title={} in={in_link}{}",
            symbol(ins.type_),
            display(Some(&ins.title)),
            color("back", ins.back_color.as_ref())
        );
        self.line(depth, &text);
        if let Some(note) = &ins.note {
            let text = self.note(note);
            self.line(depth + 1, &text);
        }
        self.list("list", &ins.list, depth + 1);
    }

    fn branch(&mut self, name: &str, branch: &Branch, depth: usize) {
        let text = format!(
            "{name} test={} positive={} inlabel={} inlink={} special={}{} specialColors=[{} ]",
            display(branch.label_test.as_ref()),
            link(Some(&branch.label_positive)),
            link(Some(&branch.inlabel)),
            link(Some(&branch.inlink_rendering)),
            link(branch.special.as_ref()),
            color("color", branch.color.as_ref()),
            colors(branch.special_colors.as_ref())
        );
        self.line(depth, &text);
        self.list("list", &branch.list, depth + 1);
    }
}

fn current_name(instruction: &Instruction) -> &'static str {
    match instruction {
        Instruction::List(_) => "InstructionList",
        Instruction::If(_) => "InstructionIf",
        Instruction::Switch(_) => "InstructionSwitch",
        Instruction::While(_) => "InstructionWhile",
        Instruction::Repeat(_) => "InstructionRepeat",
        Instruction::Fork(_) => "InstructionFork",
        Instruction::Split(_) => "InstructionSplit",
        Instruction::Group(_) => "InstructionGroup",
        _ => "a single instruction",
    }
}

fn dump(diagram: &ActivityDiagram3) -> String {
    let swimlanes = &diagram.swimlanes;
    let mut dump = Dump {
        swimlanes,
        out: String::new(),
    };
    for lane in swimlanes.swimlanes() {
        let text = format!(
            "lane {} display={}{}",
            quoted(Some(&lane.name)),
            display(Some(&lane.display)),
            colors(Some(&lane.colors))
        );
        dump.line(0, &text);
    }
    let text = format!(
        "next={} current={}",
        link(Some(swimlanes.next_link_renderer())),
        current_name(swimlanes.instructions.get(swimlanes.get_current()))
    );
    dump.line(0, &text);
    let Instruction::List(root) = swimlanes.instructions.get(Instructions::ROOT) else {
        unreachable!("the root is a list");
    };
    dump.list("root", root, 0);
    dump.out
}

#[test]
fn the_commands_build_the_instruction_trees_plantuml_builds() {
    let cases: BTreeMap<&str, String> = FIXTURE
        .split("\n=== ")
        .skip(1)
        .map(|case| {
            let (header, body) = case.split_once('\n').unwrap();
            (header, format!("{}\n", body.trim_end_matches('\n')))
        })
        .collect();
    assert_eq!(cases.len(), 96);
    let mismatches: Vec<&str> = cases
        .iter()
        .filter(|(case, expected)| {
            let actual = dump(&read(case));
            if actual != **expected {
                eprintln!("=== {case}\n--- expected\n{expected}--- actual\n{actual}");
            }
            actual != **expected
        })
        .map(|(case, _)| *case)
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn swimlanes_come_before_the_first_activity() {
    assert_eq!(
        failure(&["start", "|Lane|", ":a;"]),
        (
            "This swimlane must be defined at the start of the diagram.".to_owned(),
            "|Lane|".to_owned()
        )
    );
}

#[test]
fn closing_what_is_not_open_fails() {
    for (line, message) in [
        ("else", "Cannot find if"),
        ("endif", "Cannot find if"),
        ("fork again", "Cannot find fork"),
        ("end fork", "Cannot find fork"),
        ("split again", "Cannot find split"),
        ("end split", "Cannot find split"),
        ("endswitch", "Cannot find switch"),
        ("endwhile", "Cannot find while"),
        ("repeat while (more?)", "Cannot find repeat"),
        ("backward :back;", "Cannot find repeat"),
        ("}", "Cannot find group"),
    ] {
        assert_eq!(
            failure(&["start", line]),
            (message.to_owned(), line.to_owned()),
            "{line}"
        );
    }
}

#[test]
fn branches_follow_their_conditional() {
    assert_eq!(
        failure(&["if (a?) then", "else", "elseif (b?) then"]).0,
        "You cannot put an elseIf here"
    );
    assert_eq!(
        failure(&["if (a?) then", "else", "else"]).0,
        "Cannot find if"
    );
    assert_eq!(
        failure(&["switch (x?)", ":a;"]).0,
        "No 'case' in this switch"
    );
    assert_eq!(failure(&["start", "kill"]).0, "kill cannot be used here");
}
