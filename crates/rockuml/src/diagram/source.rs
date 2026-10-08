use std::collections::HashMap;
use std::path::Path;

use crate::host::Host;
use crate::java;
use crate::preproc::start_utils;
use crate::text::{StringLocated, ends_with_backslash};

/// Where a PNG's base64 data starts in the source.
pub(crate) const BASE64_TAG_START: &str = "data:image/png;base64,";
/// What [`UmlSource::patch_base64`] puts in its place, followed by the data's MD5.
pub(crate) const BASE64_TAG_REPLACEMENT: &str = "data:image/png;md5,";

/// A diagram's preprocessed lines, from its `@start` line to its `@end` line (PlantUML's `UmlSource`).
#[derive(Clone)]
pub struct UmlSource {
    lines: Vec<StringLocated>,
    /// The diagram as written, before preprocessing.
    raw_lines: Vec<String>,
    /// The base64 data of the PNGs `patch_base64` took out, by their MD5.
    md5_map: HashMap<String, String>,
    /// What `read_image_files` read, by the name the source gives it; `None` for what it could not read.
    image_files: HashMap<String, Option<Vec<u8>>>,
    /// When the diagram is drawn, in milliseconds since 1970: what `today` means.
    current_time_millis: i64,
}

impl UmlSource {
    pub fn new(lines: Vec<StringLocated>, raw_lines: Vec<String>) -> Self {
        Self {
            lines,
            raw_lines,
            md5_map: HashMap::new(),
            image_files: HashMap::new(),
            current_time_millis: 0,
        }
    }

    /// For `@startuml` diagrams, a line ending with a single backslash continues on the next line.
    pub fn with_continuations_joined(lines: &[StringLocated], raw_lines: Vec<String>) -> Self {
        let mut joined = Vec::with_capacity(lines.len());
        let mut pending = String::new();
        for line in lines {
            let text = line.text();
            if ends_with_backslash(text) {
                pending.push_str(&text[..text.len() - 1]);
            } else {
                pending.push_str(text);
                joined.push(line.with_text(std::mem::take(&mut pending)));
            }
        }
        Self {
            lines: joined,
            raw_lines,
            md5_map: HashMap::new(),
            image_files: HashMap::new(),
            current_time_millis: 0,
        }
    }

    /// The lines as PlantUML encodes them into a URL: each followed by `\n`.
    pub fn plain_string(&self) -> String {
        self.lines
            .iter()
            .flat_map(|line| [line.text(), "\n"])
            .collect()
    }

    /// What PlantUML embeds into images to recover the source: the raw source, the preprocessed one when it
    /// differs, and the version.
    pub fn metadata(&self) -> String {
        let raw: String = self
            .raw_lines
            .iter()
            .flat_map(|line| [line.as_str(), "\n"])
            .collect();
        let plain = self.plain_string();
        let version = crate::PLANTUML_VERSION;
        if raw == plain {
            format!("{raw}\n{version}")
        } else {
            format!("{raw}\n{plain}\n{version}")
        }
    }

    /// Whether the source holds nothing but its start and end lines, comments and blank lines.
    pub fn is_empty(&self) -> bool {
        self.lines.iter().map(StringLocated::text).all(|line| {
            start_utils::is_start_directive(line)
                || start_utils::is_end_directive(line)
                || line
                    .trim_start_matches(java::is_regex_whitespace)
                    .starts_with('\'')
                || java::trim(line).is_empty()
        })
    }

    pub fn lines(&self) -> &[StringLocated] {
        &self.lines
    }

    /// Seeds everything random in the drawing, so that the same source always gives the same image.
    pub fn seed(&self) -> i64 {
        self.lines
            .iter()
            .fold(1_125_899_906_842_597_i64, |hash, line| {
                let hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(i64::from(java::string_hash_code(line.text())));
                hash.wrapping_mul(31).wrapping_add(i64::from(b'\n'))
            })
    }

    /// Drops empty, `skinparam` and `!pragma` lines right after the start line, which diagrams without
    /// commands have no use for.
    #[must_use]
    pub fn without_initial_noise(self) -> Self {
        let noise = self.initial_noise();
        let mut lines = self.lines;
        lines.drain(1..=noise);
        Self { lines, ..self }
    }

    /// How many lines [`Self::without_initial_noise`] drops.
    pub(crate) fn initial_noise(&self) -> usize {
        self.lines
            .iter()
            .skip(1)
            .take_while(|line| is_noise(line.text()))
            .count()
    }

    /// Replaces each `data:image/png;base64,` payload by `data:image/png;md5,` and the MD5 of the data, which
    /// [`Self::md5_map`] then gives back. PlantUML does so to keep lines short for its regular expressions;
    /// the seed is computed on the shortened lines.
    pub(crate) fn patch_base64(&mut self) {
        for line in &mut self.lines {
            if let Some(patched) = patch_base64_line(line.text(), &mut self.md5_map) {
                *line = line.with_text(patched);
            }
        }
    }

    pub(crate) fn md5_map(&self) -> &HashMap<String, String> {
        &self.md5_map
    }

    /// Reads the files and URLs `<img>`s name, which creole cannot read itself as it draws. Relative paths
    /// resolve against `directory`, the diagram file's, as PlantUML's `FileSystem` does. What cannot be read
    /// is left out, and its `<img>` draws as undecodable.
    /// Remembers what time it is, which the source itself cannot know.
    pub(crate) fn note_current_time(&mut self, host: &dyn Host) {
        self.current_time_millis = host.current_time_millis();
    }

    pub(crate) fn current_time_millis(&self) -> i64 {
        self.current_time_millis
    }

    pub(crate) fn read_image_files(&mut self, directory: &Path, host: &dyn Host) {
        for line in &self.lines {
            for src in crate::creole::image_sources(line.text()) {
                if self.image_files.contains_key(src) || src.starts_with("data:") {
                    continue;
                }
                let content = if src.starts_with("http:") || src.starts_with("https:") {
                    if crate::url_policy::is_forbidden(src) {
                        None
                    } else {
                        host.read_url(src)
                    }
                } else {
                    let path = directory.join(src);
                    if crate::file_policy::is_forbidden(&path, host) {
                        None
                    } else {
                        host.read_file(&path)
                    }
                };
                self.image_files.insert(src.to_owned(), content);
            }
        }
    }

    pub(crate) fn image_files(&self) -> &HashMap<String, Option<Vec<u8>>> {
        &self.image_files
    }
}

fn patch_base64_line(line: &str, md5_map: &mut HashMap<String, String>) -> Option<String> {
    let is_base64_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=');
    if !line.contains(BASE64_TAG_START) {
        return None;
    }
    let mut patched = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(BASE64_TAG_START) {
        let data_start = &rest[start + BASE64_TAG_START.len()..];
        let data_length = data_start
            .find(|c: char| !is_base64_char(c))
            .unwrap_or(data_start.len());
        let data = &data_start[..data_length];
        let md5 = format!("{:x}", md5::compute(data));
        patched.push_str(&rest[..start]);
        patched.push_str(BASE64_TAG_REPLACEMENT);
        patched.push_str(&md5);
        md5_map.insert(md5, data.to_owned());
        rest = &data_start[data_length..];
    }
    patched.push_str(rest);
    Some(patched)
}

fn is_noise(line: &str) -> bool {
    line.is_empty()
        || line.starts_with("skinparam ")
        || line.starts_with("skinparamlocked ")
        || line.starts_with("!pragma ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::FakeHost;
    use crate::text::LineLocation;

    fn source(lines: &[&str]) -> UmlSource {
        let location = LineLocation::new("test", None);
        UmlSource::new(
            lines
                .iter()
                .map(|line| StringLocated::new(*line, location.clone()))
                .collect(),
            lines.iter().map(|line| (*line).to_owned()).collect(),
        )
    }

    fn texts(source: &UmlSource) -> Vec<&str> {
        source.lines().iter().map(StringLocated::text).collect()
    }

    /// The seed PlantUML printed for `tests/corpus/creole/plain.puml`.
    #[test]
    fn the_seed_hashes_every_line() {
        let plain = source(&[
            "@startcreole",
            "Hello world",
            "This is a second line",
            "@endcreole",
        ]);
        assert_eq!(plain.seed(), 2_574_378_694_402_089_019);
    }

    #[test]
    fn base64_pngs_are_replaced_by_their_md5() {
        let mut patched = source(&["a data:image/png;base64,QUJD+/= b data:image/png;base64,"]);
        patched.patch_base64();
        let abc = "72b852f66ac03ece4c8c26b9f893ecce";
        assert_eq!(
            texts(&patched),
            [format!(
                "a data:image/png;md5,{abc} b data:image/png;md5,d41d8cd98f00b204e9800998ecf8427e"
            )]
        );
        assert_eq!(patched.md5_map()[abc], "QUJD+/=");
    }

    #[test]
    fn each_image_file_is_read_once_even_when_it_cannot_be() {
        let mut images = source(&["<img:missing.png> <img:missing.png>", "<img:missing.png>"]);
        let host = FakeHost::default();
        images.read_image_files(Path::new("diagrams"), &host);
        assert_eq!(
            *host.reads.borrow(),
            [Path::new("diagrams").join("missing.png")]
        );
        assert_eq!(images.image_files()["missing.png"], None);
    }

    #[test]
    fn image_files_the_security_profile_refuses_are_not_read() {
        let mut images = source(&["<img:/etc/logo.png>"]);
        let mut host = FakeHost::default();
        host.files.insert("/etc/logo.png".into(), vec![1]);
        images.read_image_files(Path::new(""), &host);
        assert!(host.reads.borrow().is_empty());
        assert_eq!(images.image_files()["/etc/logo.png"], None);
    }

    #[test]
    fn continuations_are_joined_onto_the_last_line() {
        let lines = source(&["@startuml", "a \\", "b \\", "c", "d \\\\", "@enduml"]);
        let joined = UmlSource::with_continuations_joined(lines.lines(), Vec::new());
        assert_eq!(texts(&joined), ["@startuml", "a b c", "d \\\\", "@enduml"]);
    }

    #[test]
    fn initial_noise_is_removed_up_to_the_first_real_line() {
        let noisy = source(&[
            "@startcreole",
            "",
            "skinparam x y",
            "!pragma a b",
            "text",
            "",
            "@endcreole",
        ]);
        assert_eq!(
            texts(&noisy.without_initial_noise()),
            ["@startcreole", "text", "", "@endcreole"]
        );
    }
}
