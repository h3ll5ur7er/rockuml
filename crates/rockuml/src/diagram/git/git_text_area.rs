//! Reading a `git log --graph` drawing: the commits on its lines, and the parents its `|`, `/`, `\` and `_`
//! lead to (PlantUML's `GitTextArea`, `CursorPosition` and `Commit`).

use crate::java;

/// The drawing could not be followed; PlantUML throws and draws a crash report.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unreadable;

/// The lines of the drawing, in UTF-16 because the cursor counts Java characters (`GitTextArea`).
pub(super) struct GitTextArea {
    lines: Vec<Vec<u16>>,
}

impl GitTextArea {
    pub(super) fn new<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            lines: lines
                .into_iter()
                .map(|line| line.encode_utf16().collect())
                .collect(),
        }
    }

    /// A commit for each line with a `*` (`getAllCommits`).
    pub(super) fn get_all_commits(&self) -> Result<Vec<Commit<'_>>, Unreadable> {
        let mut commits = Vec::new();
        for (y, line) in self.lines.iter().enumerate() {
            let Some(x) = line.iter().position(|&c| c == u16::from(b'*')) else {
                continue;
            };
            let name = get_commit_name_in_line(&String::from_utf16_lossy(line))
                .expect("a line with a star names a commit");
            commits.push(Commit::new(name, self.at(x as i32, y as i32))?);
        }
        Ok(commits)
    }

    fn at(&self, x: i32, y: i32) -> CursorPosition<'_> {
        CursorPosition { source: self, x, y }
    }

    fn get_line(&self, y: i32) -> &[u16] {
        usize::try_from(y)
            .ok()
            .and_then(|y| self.lines.get(y))
            .map_or(&[], Vec::as_slice)
    }
}

/// What follows the first `*`, without the drawing's characters, up to the first space
/// (`getCommitNameInLine`). `None` without a `*`.
fn get_commit_name_in_line(s: &str) -> Option<String> {
    s.find('*')?;
    let stripped: String = s
        .chars()
        .filter(|c| !matches!(c, '-' | '.' | '*' | '|' | '/' | '\\'))
        .collect();
    let trimmed = java::trim(&stripped);
    Some(match trimmed.find(' ') {
        Some(space) => trimmed[..space].to_owned(),
        None => trimmed.to_owned(),
    })
}

/// A place in the drawing (`CursorPosition`).
#[derive(Clone, Copy)]
pub(super) struct CursorPosition<'a> {
    source: &'a GitTextArea,
    x: i32,
    y: i32,
}

impl<'a> CursorPosition<'a> {
    fn current_line(&self) -> &'a [u16] {
        self.source.get_line(self.y)
    }

    fn moved(&self, dx: i32, dy: i32) -> Self {
        self.source.at(self.x + dx, self.y + dy)
    }

    fn matches(&self, prefix: &str) -> bool {
        let line = self.current_line();
        let Ok(x) = usize::try_from(self.x) else {
            return false;
        };
        x <= line.len() && line[x..].starts_with(&prefix.encode_utf16().collect::<Vec<_>>())
    }

    /// The text between the first `(` after the line's `*` and the next `)` (`getCommentInLine`).
    fn get_comment_in_line(&self) -> Option<String> {
        let line = String::from_utf16_lossy(self.current_line());
        let star = line.find('*')?;
        let open = star + line[star..].find('(')?;
        let close = open + line[open..].find(')')?;
        Some(line[open + 1..close].to_owned())
    }

    fn get_commit_definition(&self) -> Option<String> {
        let line = self.current_line();
        get_commit_name_in_line(&String::from_utf16_lossy(&line[self.x as usize..]))
    }

    fn get_commit(self) -> Result<Commit<'a>, Unreadable> {
        let name = self.get_commit_definition().ok_or(Unreadable)?;
        Commit::new(name, self)
    }

    /// Follows the drawing down to the commit it leads to (`getDownFromHere`).
    fn get_down_from_here(self) -> Result<Self, Unreadable> {
        let mut current = self;
        while !current.matches("* ") {
            let (dx, dy) = DOWN_STEPS
                .iter()
                .find(|step| {
                    step.patterns
                        .iter()
                        .all(|&(dx, dy, prefix)| current.moved(dx, dy).matches(prefix))
                })
                .ok_or(Unreadable)?
                .moves;
            current = current.moved(dx, dy);
        }
        Ok(current)
    }
}

/// A move down the drawing, taken where every pattern matches at its offset from the cursor.
struct DownStep {
    patterns: &'static [(i32, i32, &'static str)],
    moves: (i32, i32),
}

/// The moves `getDownFromHere` tries, in its order.
const DOWN_STEPS: &[DownStep] = &[
    step(&[(0, 0, "/"), (-2, 0, "_|/")], (-2, 0)),
    step(&[(0, 0, "_"), (-2, 0, "_|_")], (-2, 0)),
    step(&[(0, 0, "_"), (-2, 1, "/")], (-2, 1)),
    step(&[(0, 0, "|"), (0, 1, "* ")], (0, 1)),
    step(&[(0, 0, "|"), (0, 1, "|")], (0, 1)),
    step(&[(0, 0, "| "), (0, 1, " \\"), (2, 2, "|")], (2, 2)),
    step(&[(0, 0, "| "), (0, 1, " \\"), (2, 2, "\\")], (2, 2)),
    step(&[(0, 0, "| "), (-1, 1, "/ "), (-2, 2, "* ")], (-2, 2)),
    step(&[(0, 0, "| "), (-3, 1, "_|/ ")], (-3, 1)),
    step(&[(0, 0, "| "), (-1, 1, "/ "), (-2, 2, "| ")], (-2, 2)),
    step(&[(0, 0, "| "), (-3, 1, "_|/")], (-3, 1)),
    step(&[(0, 0, "| "), (0, 1, " \\"), (1, 2, "/ ")], (1, 2)),
    step(&[(0, 0, "| "), (0, 1, " \\"), (2, 2, "* ")], (2, 2)),
    step(&[(0, 0, "\\ "), (1, 1, "* ")], (1, 1)),
    step(&[(0, 0, " \\")], (2, 1)),
    step(&[(0, 0, "\\ "), (0, 1, "/ ")], (0, 1)),
    step(&[(0, 0, "\\ "), (1, 1, "\\ ")], (1, 1)),
    step(&[(0, 0, "\\ "), (1, 1, "|")], (1, 1)),
    step(&[(0, 0, "/ "), (-1, 1, "/ ")], (-1, 1)),
    step(&[(0, 0, "/ "), (-2, 1, "/| ")], (-2, 1)),
    step(&[(0, 0, "/"), (-1, 1, "* ")], (-1, 1)),
    step(&[(0, 0, "/"), (-1, 1, "|")], (-1, 1)),
];

const fn step(patterns: &'static [(i32, i32, &'static str)], moves: (i32, i32)) -> DownStep {
    DownStep { patterns, moves }
}

/// A line's commit: its name, the text in its first parentheses and its `*` (`Commit`).
pub(super) struct Commit<'a> {
    pub name: String,
    pub comment: Option<String>,
    position: CursorPosition<'a>,
}

impl<'a> Commit<'a> {
    fn new(name: String, position: CursorPosition<'a>) -> Result<Self, Unreadable> {
        if !position.matches("* ") && !position.matches("*-") {
            return Err(Unreadable);
        }
        Ok(Self {
            name,
            comment: position.get_comment_in_line(),
            position,
        })
    }

    /// Where the lines to the parents start: below the `*`, and below each `.` of an octopus merge's `*-.-.`
    /// (`getCandidatesForDown`).
    fn get_candidates_for_down(&self) -> Vec<CursorPosition<'a>> {
        let mut result = Vec::new();
        if self.position.moved(0, 1).matches("*") {
            result.push(self.position.moved(0, 1));
        }
        let mut current = self.position;
        add_above(&mut result, current);
        loop {
            current = current.moved(1, 0);
            if current.matches(".") {
                add_above(&mut result, current);
            } else if !current.matches("-") {
                return result;
            }
        }
    }

    /// The parents, in the order the drawing leads to them (`getAncestors`).
    pub(super) fn get_ancestors(&self) -> Result<Vec<Commit<'a>>, Unreadable> {
        self.get_candidates_for_down()
            .into_iter()
            .map(|position| position.get_down_from_here()?.get_commit())
            .collect()
    }
}

fn add_above<'a>(result: &mut Vec<CursorPosition<'a>>, here: CursorPosition<'a>) {
    for (dx, prefix) in [(0, "|"), (1, "\\"), (-1, "/")] {
        if here.moved(dx, 1).matches(prefix) {
            result.push(here.moved(dx, 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each commit with its parents, as `name: parent parent`.
    fn ancestry(lines: &[&str]) -> Result<Vec<String>, Unreadable> {
        let area = GitTextArea::new(lines.iter().copied());
        area.get_all_commits()?
            .iter()
            .map(|commit| {
                let parents = commit.get_ancestors()?;
                let names: Vec<_> = parents.iter().map(|parent| parent.name.as_str()).collect();
                Ok(format!("{}: {}", commit.name, names.join(" ")))
            })
            .collect()
    }

    #[test]
    fn names_drop_the_drawing_and_comments_are_the_first_parentheses() {
        let area = GitTextArea::new(["| * 4c5d6e7 (HEAD -> main) Fix (urgent) typo"]);
        let commits = area.get_all_commits().unwrap();
        assert_eq!(commits[0].name, "4c5d6e7");
        assert_eq!(commits[0].comment.as_deref(), Some("HEAD -> main"));
    }

    #[test]
    fn lines_lead_down_through_branches_and_merges() {
        let lines = [
            "* d (HEAD -> main) Work on main",
            "| * c (feature) Merge main into feature",
            "|/|",
            "* | b Fix main",
            "| * a Start the feature",
            "|/",
            "* r Root",
        ];
        assert_eq!(
            ancestry(&lines).unwrap(),
            ["d: b", "c: a b", "b: r", "a: r", "r: "]
        );
    }

    #[test]
    fn backslashes_and_octopus_merges_lead_right() {
        let lines = [
            "*-.   m Octopus",
            "|\\ \\ x",
            "| | * c Third",
            "| * | b Second",
            "| |/ x",
            "* | a First",
            "| | x",
            "|/ x",
            "* r Root",
        ];
        assert_eq!(
            ancestry(&lines).unwrap(),
            ["m: a b c", "c: r", "b: r", "a: r", "r: "]
        );
    }

    #[test]
    fn a_drawing_that_leads_nowhere_cannot_be_read() {
        assert!(ancestry(&["* a Top", "|", "x"]).is_err());
        assert!(ancestry(&["*a Top"]).is_err());
        assert!(ancestry(&["*"]).is_err());
    }
}
