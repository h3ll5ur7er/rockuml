//! What a JSON or YAML diagram says besides its data, before the data starts: `<style>` blocks, `title`, `scale`
//! and `skin`; other directives there are skipped (PlantUML's `StyleExtractor`).

use crate::text::StringLocated;

#[derive(Default)]
pub(super) struct StyleExtractor {
    /// The start line, then the data and the end line.
    pub(super) list: Vec<String>,
    /// The `<style>` blocks, their tags included.
    pub(super) style: Vec<String>,
    pub(super) title: Option<String>,
    pub(super) handwritten: bool,
    /// The `scale` line, trimmed.
    pub(super) scale: Option<StringLocated>,
    pub(super) new_skin: Option<String>,
}

impl StyleExtractor {
    pub(super) fn new(data: &[StringLocated]) -> Self {
        let mut result = Self::default();
        let mut lines = data.iter();
        while let Some(mut line) = lines.next() {
            let s = crate::java::trim(line.text());
            if s.is_empty() {
                continue;
            }
            let before_data = result.list.len() <= 1;
            if s == "<style>" {
                while lines.len() > 0 {
                    result.style.push(line.text().to_owned());
                    if crate::java::trim(line.text()) == "</style>" {
                        break;
                    }
                    line = lines.next().expect("a line is left");
                }
            } else if before_data
                && (s.starts_with("!assume ")
                    || s.starts_with("!pragma ")
                    || s.starts_with("hide "))
            {
            } else if before_data && s.starts_with("scale ") {
                result.scale = Some(StringLocated::new(s, line.location().clone()));
            } else if before_data && s.starts_with("title ") {
                result.title = Some(crate::java::trim(&s["title ".len()..]).to_owned());
            } else if before_data && s.starts_with("skin ") {
                result.new_skin = Some(crate::java::trim(&s["skin ".len()..]).to_owned());
            } else if before_data && s.starts_with("skinparam ") {
                if s.contains("handwritten") && s.contains("true") {
                    result.handwritten = true;
                }
                if s.contains('{') {
                    while crate::java::trim(line.text()) != "}" {
                        match lines.next() {
                            Some(next) => line = next,
                            None => break,
                        }
                    }
                }
            } else {
                result.list.push(line.text().to_owned());
            }
        }
        result
    }

    /// The data lines: between the start line and the last line, which ends the diagram.
    pub(super) fn data(&self) -> &[String] {
        match self.list.as_slice() {
            [_start, data @ .., _end] => data,
            _ => &[],
        }
    }
}
