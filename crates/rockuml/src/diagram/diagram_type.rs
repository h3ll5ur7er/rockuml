/// The kinds of diagram a start line can announce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagramType {
    /// `@startuml`: sequence, class, activity and the other UML diagrams, told apart by their content.
    Uml,
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
    /// The type a start line like `@startmindmap` announces; `None` if it is no start line.
    pub fn of_start_line(line: &str) -> Option<Self> {
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
