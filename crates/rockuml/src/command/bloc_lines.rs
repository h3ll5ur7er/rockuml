//! The lines one command consumes: a single line, or a whole block for multi-line commands.

use crate::text::StringLocated;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BlocLines {
    lines: Vec<StringLocated>,
}

impl BlocLines {
    pub(crate) fn single(line: StringLocated) -> Self {
        Self { lines: vec![line] }
    }

    #[cfg(test)]
    pub(crate) fn from_texts(texts: &[&str]) -> Self {
        let location = crate::text::LineLocation::new("test", None);
        let lines = texts
            .iter()
            .map(|text| StringLocated::new(*text, location.clone()));
        Self {
            lines: lines.collect(),
        }
    }

    #[must_use]
    pub(crate) fn add(mut self, line: StringLocated) -> Self {
        self.lines.push(line);
        self
    }

    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn first(&self) -> Option<&StringLocated> {
        self.lines.first()
    }

    pub(crate) fn last(&self) -> Option<&StringLocated> {
        self.lines.last()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &StringLocated> {
        self.lines.iter()
    }

    /// Without the first `start` and last `end` lines, like the opening and closing lines of a block.
    #[must_use]
    pub(crate) fn sub_extract(&self, start: usize, end: usize) -> Self {
        let last = self.lines.len().saturating_sub(end).max(start);
        Self {
            lines: self.lines[start.min(last)..last].to_vec(),
        }
    }

    /// Every line trimmed.
    #[must_use]
    pub(crate) fn trimmed(&self) -> Self {
        Self {
            lines: self.lines.iter().map(StringLocated::trimmed).collect(),
        }
    }

    /// Without the lines that start with a quote (comments), once trimmed.
    #[must_use]
    pub(crate) fn without_quote_lines(&self) -> Self {
        Self {
            lines: self
                .lines
                .iter()
                .filter(|line| !line.trimmed().text().starts_with('\''))
                .cloned()
                .collect(),
        }
    }

    /// Removes the indentation all non-empty lines share, one column at a time.
    #[must_use]
    pub(crate) fn without_empty_columns(&self) -> Self {
        let removable = |lines: &[StringLocated]| {
            let filled: Vec<&StringLocated> = lines
                .iter()
                .filter(|line| !line.text().is_empty())
                .collect();
            !filled.is_empty()
                && filled
                    .iter()
                    .all(|line| line.text().starts_with([' ', '\t']))
        };
        let mut lines = self.lines.clone();
        while removable(&lines) {
            for line in &mut lines {
                if !line.text().is_empty() {
                    *line = line.with_text(&line.text()[1..]);
                }
            }
        }
        Self { lines }
    }

    /// From line `reference` on, removes up to as much leading space as that line has.
    #[must_use]
    pub(crate) fn trim_smart(&self, reference: usize) -> Self {
        let Some(reference_line) = self.lines.get(reference) else {
            return self.clone();
        };
        let indentation = leading_spaces(reference_line.text());
        let lines = self
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if index < reference {
                    return line.clone();
                }
                let removable = leading_spaces(line.text()).min(indentation);
                line.with_text(&line.text()[removable..])
            })
            .collect();
        Self { lines }
    }

    /// The lines as a label, without the empty line PlantUML leaves after a closing `}}`.
    pub(crate) fn to_display(&self) -> crate::creole::Display {
        let mut texts: Vec<&str> = self.lines.iter().map(StringLocated::text).collect();
        if texts.len() > 2 && texts[texts.len() - 1].is_empty() && texts[texts.len() - 2] == "}}" {
            texts.pop();
        }
        crate::creole::Display::create(texts)
    }
}

/// Spaces and tabs at the start, which are one byte each.
fn leading_spaces(text: &str) -> usize {
    text.len() - text.trim_start_matches([' ', '\t']).len()
}
