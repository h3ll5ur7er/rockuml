/// The kinds of diagram a start line can announce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiagramType {
    /// `@startuml`: sequence, class, activity and the other UML diagrams, told apart by their content.
    Uml,
    /// The `@startuml` diagrams, told apart by the lines they accept.
    Sequence,
    State,
    Class,
    Activity,
    /// Usecase, component and deployment diagrams.
    Description,
    Timing,
    Help,
    Bpm,
    Board,
    Chart,
    Creole,
    Chronology,
    ChenEer,
    Crash,
    Dot,
    Ditaa,
    Definition,
    Ebnf,
    Flow,
    Files,
    Gantt,
    Git,
    Hcl,
    Json,
    Latex,
    Math,
    MindMap,
    NwDiag,
    Packet,
    Regex,
    Salt,
    Sprites,
    Wire,
    Wbs,
    Yaml,
    Unknown,
}

/// Prefixes of the word after `@start`, checked in PlantUML's order: `@startcreolefoo` is still creole.
const KEYWORDS: [(&str, DiagramType); 30] = [
    ("bpm", DiagramType::Bpm),
    ("board", DiagramType::Board),
    ("chart", DiagramType::Chart),
    ("creole", DiagramType::Creole),
    ("chronology", DiagramType::Chronology),
    ("chen", DiagramType::ChenEer),
    ("crash", DiagramType::Crash),
    ("dot", DiagramType::Dot),
    ("ditaa", DiagramType::Ditaa),
    ("def", DiagramType::Definition),
    ("ebnf", DiagramType::Ebnf),
    ("flow", DiagramType::Flow),
    ("files", DiagramType::Files),
    ("gantt", DiagramType::Gantt),
    ("git", DiagramType::Git),
    ("hcl", DiagramType::Hcl),
    ("json", DiagramType::Json),
    ("latex", DiagramType::Latex),
    ("math", DiagramType::Math),
    ("mindmap", DiagramType::MindMap),
    ("nwdiag", DiagramType::NwDiag),
    ("project", DiagramType::Gantt),
    ("packetdiag", DiagramType::Packet),
    ("regex", DiagramType::Regex),
    ("salt", DiagramType::Salt),
    ("sprites", DiagramType::Sprites),
    ("uml", DiagramType::Uml),
    ("wire", DiagramType::Wire),
    ("wbs", DiagramType::Wbs),
    ("yaml", DiagramType::Yaml),
];

impl DiagramType {
    /// How error messages name the type, like PlantUML's `humanReadableName`.
    pub(super) fn human_readable_name(self) -> &'static str {
        match self {
            Self::Uml => {
                unreachable!("`@startuml` diagrams are named by the type their content reveals")
            }
            Self::Sequence => "sequence",
            Self::State => "state",
            Self::Class => "class",
            Self::Activity => "activity",
            Self::Description => "component",
            Self::Timing => "timing",
            Self::Help => "help",
            Self::Bpm => "bpm",
            Self::Board => "board",
            Self::Chart => "chart",
            Self::Creole => "creole",
            Self::Chronology => "chronology",
            Self::ChenEer => "chen_eer",
            Self::Crash => "crash",
            Self::Dot => "dot",
            Self::Ditaa => "ditaa",
            Self::Definition => "definition",
            Self::Ebnf => "ebnf",
            Self::Flow => "flow",
            Self::Files => "files",
            Self::Gantt => "gantt",
            Self::Git => "git",
            Self::Hcl => "hcl",
            Self::Json => "json",
            Self::Latex => "latex",
            Self::Math => "math",
            Self::MindMap => "mindmap",
            Self::NwDiag => "nwdiag",
            Self::Packet => "packet",
            Self::Regex => "regex",
            Self::Salt => "salt",
            Self::Sprites => "sprites",
            Self::Wire => "wire",
            Self::Wbs => "wbs",
            Self::Yaml => "yaml",
            Self::Unknown => "unknown",
        }
    }

    /// The type a start line like `@startmindmap` announces; `None` if it is no start line.
    pub(super) fn of_start_line(line: &str) -> Option<Self> {
        let line = line.trim_start_matches(crate::java::is_whitespace);
        let rest = line.strip_prefix(['@', '\\'])?;
        let word = rest
            .get(..5)
            .filter(|start| start.eq_ignore_ascii_case("start"))
            .map(|_| &rest[5..])?;
        if word.is_empty() {
            return None;
        }
        let word = word.to_ascii_lowercase();
        let found = KEYWORDS
            .iter()
            .find(|(keyword, _)| word.starts_with(keyword));
        Some(found.map_or(DiagramType::Unknown, |&(_, diagram_type)| diagram_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_lines_announce_their_type_by_prefix() {
        assert_eq!(
            DiagramType::of_start_line("@startcreole"),
            Some(DiagramType::Creole)
        );
        assert_eq!(
            DiagramType::of_start_line("  \\StartUML(id=x)"),
            Some(DiagramType::Uml)
        );
        assert_eq!(
            DiagramType::of_start_line("@startprojectfoo"),
            Some(DiagramType::Gantt)
        );
        assert_eq!(
            DiagramType::of_start_line("@startnothing"),
            Some(DiagramType::Unknown)
        );
        assert_eq!(DiagramType::of_start_line("@start"), None);
        assert_eq!(DiagramType::of_start_line("x @startuml"), None);
    }
}
