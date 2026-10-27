//! File trees, `@startfiles`: one `/path/to/file` per line, folders made from the paths, with notes between
//! `<note>` and `</note>` (PlantUML's `filesdiagram` package and `FilesDiagramFactory`).

use std::rc::Rc;

use super::json::StyleExtractor;
use super::titled::Titled;
use super::{Diagram, ExportSettings, NotYetPorted, UmlSource};
use crate::creole::{CreoleMode, CreoleParser, Display, SheetBlock1};
use crate::klimt::font::{FontConfiguration, StringBounder};
use crate::klimt::geom::{ClockwiseTopRightBottomLeft, XDimension2D};
use crate::klimt::ugraphic::UGraphic;
use crate::klimt::{HorizontalAlignment, TextBlock};
use crate::skin::SkinParam;
use crate::style::{PName, SName, StyleSignature, ValueReading};
use crate::svek::image::Opale;
use crate::text::StringLocated;

/// How far each level of the tree is indented.
const INDENT: f64 = 21.0;
/// The room below each entry.
const SPACING: f64 = 2.0;

/// A `@startfiles` diagram (PlantUML's `FilesDiagramFactory` and `FilesDiagram`).
pub(super) fn create(source: UmlSource) -> Box<dyn Diagram> {
    let source = Rc::new(source);
    let mut titled = Titled::new(SName::FilesDiagram, "FILES", &source);
    let extractor = StyleExtractor::new(source.lines());
    if extractor.new_skin.is_some() {
        titled.not_ported(NotYetPorted("the skin directive"));
    }
    if let Some(rules) = extractor.style_rules() {
        // PlantUML reports a style it cannot parse and draws without it.
        let _ = titled.skin.apply_style_sheet(&rules);
    }
    let mut listing = FilesListing::default();
    let mut lines = source.lines().iter().skip(1).map(StringLocated::text);
    while let Some(line) = lines.next() {
        if let Some(raw) = line.strip_prefix('/') {
            listing.add_raw_entry(raw);
        } else if line.starts_with("<note>") {
            listing.add_note(get_note(&mut lines));
        }
    }
    Box::new(FilesDiagram {
        source,
        titled,
        listing,
    })
}

/// The lines up to `</note>`, or to the end.
fn get_note<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Vec<String> {
    lines
        .take_while(|line| !line.starts_with("</note>"))
        .map(str::to_owned)
        .collect()
}

pub(super) struct FilesDiagram {
    source: Rc<UmlSource>,
    titled: Titled,
    listing: FilesListing,
}

/// What an entry of the tree is (`FilesType`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FilesType {
    Folder,
    Data,
    Note,
}

/// A folder, file or note of the tree (`FEntry`), its children by index in [`FilesListing::entries`].
struct FEntry {
    parent: Option<usize>,
    kind: FilesType,
    name: String,
    note: Vec<String>,
    children: Vec<usize>,
}

/// The tree, drawn as an indented list (`FilesListing`). Entry 0 is the root folder, which is not drawn.
struct FilesListing {
    entries: Vec<FEntry>,
    last_created: Option<usize>,
}

const ROOT: usize = 0;

impl Default for FilesListing {
    fn default() -> Self {
        Self {
            entries: vec![FEntry {
                parent: None,
                kind: FilesType::Folder,
                name: String::new(),
                note: Vec::new(),
                children: Vec::new(),
            }],
            last_created: None,
        }
    }
}

impl FilesListing {
    fn add_raw_entry(&mut self, raw: &str) {
        self.last_created = self.add_raw_entry_in(ROOT, raw);
    }

    /// Adds the file `raw` names below `folder`, making the folders on its way; a path ending in `/` adds
    /// only folders, and gives `None`.
    fn add_raw_entry_in(&mut self, folder: usize, raw: &str) -> Option<usize> {
        let Some((folder_name, remain)) = raw.split_once('/') else {
            return Some(self.add_child(folder, FilesType::Data, raw.to_owned(), Vec::new()));
        };
        let folder = self.get_or_create_folder(folder, folder_name);
        if remain.is_empty() {
            None
        } else {
            self.add_raw_entry_in(folder, remain)
        }
    }

    /// The note goes beside the last file added, or to the root after a folder.
    fn add_note(&mut self, note: Vec<String>) {
        let folder = self
            .last_created
            .and_then(|entry| self.entries[entry].parent)
            .unwrap_or(ROOT);
        self.add_child(folder, FilesType::Note, "NONE".to_owned(), note);
    }

    fn get_or_create_folder(&mut self, folder: usize, name: &str) -> usize {
        let existing = self.entries[folder]
            .children
            .iter()
            .copied()
            .find(|&child| {
                self.entries[child].kind == FilesType::Folder && self.entries[child].name == name
            });
        existing.unwrap_or_else(|| {
            self.add_child(folder, FilesType::Folder, name.to_owned(), Vec::new())
        })
    }

    fn add_child(
        &mut self,
        parent: usize,
        kind: FilesType,
        name: String,
        note: Vec<String>,
    ) -> usize {
        let index = self.entries.len();
        self.entries.push(FEntry {
            parent: Some(parent),
            kind,
            name,
            note,
            children: Vec::new(),
        });
        self.entries[parent].children.push(index);
        index
    }
}

/// The listing drawn in the diagram's font with its skin (`FilesListing`'s and `FEntry`'s drawing).
struct Drawing<'a> {
    listing: &'a FilesListing,
    skin: &'a SkinParam,
    font: FontConfiguration,
}

impl Drawing<'_> {
    fn draw_and_move(&self, entry: usize, ug: &UGraphic, deltax: f64) -> UGraphic {
        let text = self.get_text_block(entry);
        text.draw_u(&ug.translated(deltax, 0.0));
        let height = text.calculate_dimension(ug.string_bounder()).height;
        let mut ug = ug.translated(0.0, height + SPACING);
        for &child in &self.listing.entries[entry].children {
            ug = self.draw_and_move(child, &ug, deltax + INDENT);
        }
        ug
    }

    fn calculate_dimension_of(
        &self,
        entry: usize,
        string_bounder: &dyn StringBounder,
        deltax: f64,
    ) -> XDimension2D {
        let dim = self
            .get_text_block(entry)
            .calculate_dimension(string_bounder);
        let mut width = deltax + dim.width;
        let mut height = dim.height + SPACING;
        for &child in &self.listing.entries[entry].children {
            let child_dim = self.calculate_dimension_of(child, string_bounder, deltax + INDENT);
            width = width.max(child_dim.width);
            height += child_dim.height;
        }
        XDimension2D::new(width, height)
    }

    fn get_text_block(&self, entry: usize) -> Box<dyn TextBlock + '_> {
        let entry = &self.listing.entries[entry];
        if entry.kind == FilesType::Note {
            return Box::new(self.create_opale(&entry.note));
        }
        let emoticon = if entry.kind == FilesType::Folder {
            "<:1f4c2:>"
        } else {
            "<:1f4c4:>"
        };
        Box::new(
            Display::with_newlines(&format!("{emoticon}{}", entry.name)).create0(
                &self.font,
                HorizontalAlignment::Left,
                self.skin,
                0.0,
                CreoleMode::NoCreole,
            ),
        )
    }

    fn create_opale(&self, note: &[String]) -> Opale<'static> {
        let style = StyleSignature::of(&[
            SName::Root,
            SName::Element,
            SName::FilesDiagram,
            SName::Note,
        ])
        .get_merged_style(&self.skin.current_style_builder());
        let font = style.font_configuration();
        let sheet = CreoleParser::with_mode(
            font.clone(),
            self.skin
                .get_default_text_alignment(HorizontalAlignment::Left),
            CreoleMode::Full,
            self.skin,
        )
        .create_display_sheet(&Display::create(note.iter().cloned()), &font);
        Opale::new(
            style.shadowing(),
            style.value(PName::LineColor).as_color(),
            style.value(PName::BackGroundColor).as_color(),
            Box::new(SheetBlock1::new(sheet, self.skin.get_padding())),
            style.stroke(),
            0.0,
        )
    }
}

impl TextBlock for Drawing<'_> {
    fn calculate_dimension(&self, string_bounder: &dyn StringBounder) -> XDimension2D {
        let mut width: f64 = 0.0;
        let mut height = 0.0;
        for &entry in &self.listing.entries[ROOT].children {
            let dim = self.calculate_dimension_of(entry, string_bounder, 0.0);
            width = width.max(dim.width);
            height += dim.height;
        }
        XDimension2D::new(width, height)
    }

    fn draw_u(&self, ug: &UGraphic) {
        let mut ug = ug.clone();
        for &entry in &self.listing.entries[ROOT].children {
            ug = self.draw_and_move(entry, &ug, 0.0);
        }
    }
}

impl Diagram for FilesDiagram {
    fn source(&self) -> &UmlSource {
        &self.source
    }

    fn text_block(
        &self,
        _page: usize,
        string_bounder: &Rc<dyn StringBounder>,
    ) -> Result<Box<dyn TextBlock + '_>, NotYetPorted> {
        if let Some(not_ported) = self.titled.not_ported_part() {
            return Err(not_ported);
        }
        let style = StyleSignature::of(&[SName::Root, SName::Element, SName::FilesDiagram])
            .get_merged_style(&self.titled.skin.current_style_builder());
        let drawing = Drawing {
            listing: &self.listing,
            skin: &self.titled.skin,
            font: style.font_configuration(),
        };
        Ok(self.titled.add_chrome(Box::new(drawing), string_bounder))
    }

    fn export_settings(&self, _page: usize) -> ExportSettings {
        self.titled
            .export_settings(self.source.seed(), ClockwiseTopRightBottomLeft::same(10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(lines: &[&str]) -> FilesListing {
        let mut listing = FilesListing::default();
        for line in lines {
            match line.strip_prefix('/') {
                Some(raw) => listing.add_raw_entry(raw),
                None => listing.add_note(vec![(*line).to_owned()]),
            }
        }
        listing
    }

    /// Each entry as `name` indented by its depth, in drawing order.
    fn outline(listing: &FilesListing) -> Vec<String> {
        fn walk(listing: &FilesListing, entry: usize, depth: usize, out: &mut Vec<String>) {
            for &child in &listing.entries[entry].children {
                let child_entry = &listing.entries[child];
                let name = match child_entry.kind {
                    FilesType::Note => format!("note {}", child_entry.note.join(" ")),
                    FilesType::Folder => format!("{}/", child_entry.name),
                    FilesType::Data => child_entry.name.clone(),
                };
                out.push(format!("{}{name}", "  ".repeat(depth)));
                walk(listing, child, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        walk(listing, ROOT, 0, &mut out);
        out
    }

    #[test]
    fn paths_share_their_folders_and_files_repeat() {
        let listing = listing(&["/a/b/c", "/a/d", "/a/d", "/a/b/", "/a"]);
        assert_eq!(
            outline(&listing),
            ["a/", "  b/", "    c", "  d", "  d", "a"]
        );
    }

    #[test]
    fn a_note_follows_the_last_file_or_goes_to_the_root() {
        let listing = listing(&["first", "/x/y", "beside y", "/x/z/", "at the root"]);
        assert_eq!(
            outline(&listing),
            [
                "note first",
                "x/",
                "  y",
                "  note beside y",
                "  z/",
                "note at the root"
            ]
        );
    }
}
