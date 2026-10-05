//! The font files text is measured with. rockuml embeds the Liberation fonts, which have the metrics of the
//! Windows fonts PlantUML measures with (Arial, Times New Roman, Courier New), so that its images match PlantUML
//! run on Windows on every machine. Any other font can be registered, and diagrams name it as usual.

mod bounder;

use std::fmt;
use std::sync::Arc;

pub use bounder::StringBounderFonts;
use ttf_parser::{Face, name_id};

use super::font::UFontFace;

/// The fonts available to diagrams, by family name.
#[derive(Clone)]
pub struct FontRegistry {
    faces: Vec<RegisteredFace>,
}

#[derive(Clone)]
struct RegisteredFace {
    /// Lowercase, as Java looks fonts up regardless of case.
    families: Vec<String>,
    bold: bool,
    italic: bool,
    data: FontData,
    index_in_collection: u32,
}

/// Font files are shared by the faces of a collection.
#[derive(Clone)]
enum FontData {
    Embedded(&'static [u8]),
    Registered(Arc<[u8]>),
}

impl FontData {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Embedded(bytes) => bytes,
            Self::Registered(bytes) => bytes,
        }
    }
}

/// Font data that is no font rockuml can read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAFont;

impl fmt::Display for NotAFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a TrueType or OpenType font")
    }
}

impl std::error::Error for NotAFont {}

/// The embedded fonts, and the Windows family each stands in for.
const EMBEDDED: [(&str, &str); 12] = [
    ("fonts/LiberationSans-Regular.ttf", "Arial"),
    ("fonts/LiberationSans-Bold.ttf", "Arial"),
    ("fonts/LiberationSans-Italic.ttf", "Arial"),
    ("fonts/LiberationSans-BoldItalic.ttf", "Arial"),
    ("fonts/LiberationSerif-Regular.ttf", "Times New Roman"),
    ("fonts/LiberationSerif-Bold.ttf", "Times New Roman"),
    ("fonts/LiberationSerif-Italic.ttf", "Times New Roman"),
    ("fonts/LiberationSerif-BoldItalic.ttf", "Times New Roman"),
    ("fonts/LiberationMono-Regular.ttf", "Courier New"),
    ("fonts/LiberationMono-Bold.ttf", "Courier New"),
    ("fonts/LiberationMono-Italic.ttf", "Courier New"),
    ("fonts/LiberationMono-BoldItalic.ttf", "Courier New"),
];

impl Default for FontRegistry {
    /// The embedded fonts.
    fn default() -> Self {
        let mut registry = Self { faces: Vec::new() };
        for (path, stands_in_for) in EMBEDDED {
            let data = crate::assets::get(path).expect("the Liberation fonts are embedded");
            registry
                .add(&FontData::Embedded(data), &[stands_in_for])
                .expect("the embedded fonts are valid");
        }
        registry
    }
}

impl FontRegistry {
    /// Makes the fonts in a TrueType or OpenType file (or collection) available under their family names.
    /// Returns those names. Fonts registered later take precedence over earlier ones of the same family.
    pub fn register(&mut self, data: Vec<u8>) -> Result<Vec<String>, NotAFont> {
        self.add(&FontData::Registered(data.into()), &[])
    }

    /// Registers every face of the file, or none if any of them cannot be read.
    fn add(&mut self, data: &FontData, aliases: &[&str]) -> Result<Vec<String>, NotAFont> {
        let bytes = data.bytes();
        let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
        let faces: Vec<(u32, Face)> = (0..count)
            .map(|index| Face::parse(bytes, index).map(|face| (index, face)))
            .collect::<Result<_, _>>()
            .map_err(|_| NotAFont)?;
        if faces.is_empty() {
            return Err(NotAFont);
        }
        let mut added = Vec::new();
        for (index_in_collection, face) in faces {
            let mut families = family_names(&face);
            added.extend(families.iter().cloned());
            families.extend(aliases.iter().map(|alias| (*alias).to_owned()));
            let registered = RegisteredFace {
                families: families
                    .iter()
                    .map(|family| family.to_lowercase())
                    .collect(),
                bold: face.is_bold(),
                italic: face.is_italic(),
                data: data.clone(),
                index_in_collection,
            };
            self.faces.insert(0, registered);
        }
        added.sort();
        added.dedup();
        Ok(added)
    }

    /// The face of a family closest to the asked one: the exact style, else the same boldness, else any.
    fn physical_face(&self, family: &str, face: UFontFace) -> Option<Face<'_>> {
        let family = family.to_lowercase();
        let candidates: Vec<&RegisteredFace> = self
            .faces
            .iter()
            .filter(|registered| registered.families.contains(&family))
            .collect();
        let bold = face.is_bold();
        let find = |matches: &dyn Fn(&RegisteredFace) -> bool| {
            candidates
                .iter()
                .copied()
                .find(|registered| matches(registered))
        };
        find(&|registered| registered.bold == bold && registered.italic == face.italic)
            .or_else(|| find(&|registered| registered.bold == bold))
            .or_else(|| candidates.first().copied())
            .and_then(|registered| {
                Face::parse(registered.data.bytes(), registered.index_in_collection).ok()
            })
    }

    /// Every font file, once, for drawing text.
    pub(crate) fn files(&self) -> Vec<Arc<dyn AsRef<[u8]> + Send + Sync>> {
        let mut files: Vec<Arc<dyn AsRef<[u8]> + Send + Sync>> = Vec::new();
        let mut seen: Vec<*const u8> = Vec::new();
        for registered in &self.faces {
            let bytes = registered.data.bytes();
            if seen.contains(&bytes.as_ptr()) {
                continue;
            }
            seen.push(bytes.as_ptr());
            files.push(match &registered.data {
                FontData::Embedded(bytes) => Arc::new(*bytes),
                FontData::Registered(bytes) => Arc::new(bytes.clone()),
            });
        }
        files
    }

    /// The family name, as its font file names it, of the font Java draws a family name with.
    pub(crate) fn drawn_family(&self, family: &str) -> String {
        family_names(self.resolve(family, UFontFace::NORMAL).face())
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// The font Java uses for a family name: a logical font, or a font of that name, or else `Dialog`.
    fn resolve(&self, family: &str, face: UFontFace) -> ResolvedFont<'_> {
        if let Some(logical) = LogicalFont::named(family)
            && let Some(primary) = self.physical_face(logical.windows_family(), face)
        {
            return ResolvedFont::Logical(primary);
        }
        match self.physical_face(family, face) {
            Some(physical) => ResolvedFont::Physical(physical),
            None => self.resolve("Dialog", face),
        }
    }
}

fn family_names(face: &Face) -> Vec<String> {
    let mut names: Vec<String> = face
        .names()
        .into_iter()
        .filter(|name| matches!(name.name_id, name_id::FAMILY | name_id::TYPOGRAPHIC_FAMILY))
        .filter_map(|name| name.to_string())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Java's logical fonts, which it maps to installed fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogicalFont {
    SansSerif,
    Serif,
    Monospaced,
}

impl LogicalFont {
    fn named(family: &str) -> Option<Self> {
        match family.to_lowercase().as_str() {
            "sansserif" | "dialog" => Some(Self::SansSerif),
            "serif" => Some(Self::Serif),
            "monospaced" | "dialoginput" => Some(Self::Monospaced),
            _ => None,
        }
    }

    /// The font Java's Windows runtime maps the logical font to.
    fn windows_family(self) -> &'static str {
        match self {
            Self::SansSerif => "Arial",
            Self::Serif => "Times New Roman",
            Self::Monospaced => "Courier New",
        }
    }
}

/// A font as Java measures it.
enum ResolvedFont<'a> {
    /// A logical font: the glyphs of its primary font, the line height of all the fonts it falls back on.
    Logical(Face<'a>),
    Physical(Face<'a>),
}

/// Java's logical fonts on Windows fall back on fonts for other scripts, whose extents raise the composite
/// font's ascent and descent to at least these, in 2048ths of the size.
const LOGICAL_MIN_ASCENT: f64 = 2059.0 / 2048.0;
const LOGICAL_MIN_DESCENT: f64 = 450.0 / 2048.0;

impl ResolvedFont<'_> {
    fn face(&self) -> &Face<'_> {
        match self {
            Self::Logical(face) | Self::Physical(face) => face,
        }
    }

    fn em_fraction(&self, units: i16) -> f64 {
        f64::from(units) / f64::from(self.face().units_per_em())
    }

    /// Ascent, descent and leading, as fractions of the size.
    fn vertical_metrics(&self) -> (f64, f64, f64) {
        let face = self.face();
        let ascent = self.em_fraction(face.ascender());
        let descent = -self.em_fraction(face.descender());
        let leading = self.em_fraction(face.line_gap());
        match self {
            Self::Logical(_) => (
                ascent.max(LOGICAL_MIN_ASCENT),
                descent.max(LOGICAL_MIN_DESCENT),
                leading,
            ),
            Self::Physical(_) => (ascent, descent, leading),
        }
    }

    /// The advance of a character as a fraction of the size; `None` when the font has no glyph for it.
    fn advance(&self, c: char) -> Option<f64> {
        let face = self.face();
        let glyph = face.glyph_index(c)?;
        let advance = face.glyph_hor_advance(glyph)?;
        Some(f64::from(advance) / f64::from(face.units_per_em()))
    }

    /// Whether the font draws every character itself, which Java asks to pick a font from a fallback list.
    /// Logical fonts fall back on fonts for other scripts.
    fn can_display(&self, text: &str) -> bool {
        match self {
            Self::Logical(_) => true,
            Self::Physical(face) => text.chars().all(|c| face.glyph_index(c).is_some()),
        }
    }

    /// The advance of a character the font has no glyph for: for logical fonts, as wide as in the fonts they
    /// fall back on (which PlantUML's width table approximates); physical fonts draw their missing glyph.
    fn missing_glyph_advance(&self, c: char) -> f64 {
        match self {
            Self::Logical(_) => super::width_table::relative_width(c),
            Self::Physical(face) => face
                .glyph_hor_advance(ttf_parser::GlyphId(0))
                .map_or(0.0, |advance| {
                    f64::from(advance) / f64::from(face.units_per_em())
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved_name(registry: &FontRegistry, family: &str) -> Vec<String> {
        family_names(registry.resolve(family, UFontFace::NORMAL).face())
    }

    #[test]
    fn logical_and_windows_names_resolve_to_the_embedded_fonts() {
        let registry = FontRegistry::default();
        assert_eq!(resolved_name(&registry, "SansSerif"), ["Liberation Sans"]);
        assert_eq!(
            resolved_name(&registry, "times new roman"),
            ["Liberation Serif"]
        );
        assert_eq!(resolved_name(&registry, "Monospaced"), ["Liberation Mono"]);
        assert_eq!(
            resolved_name(&registry, "Liberation Serif"),
            ["Liberation Serif"]
        );
    }

    #[test]
    fn unknown_families_fall_back_to_dialog() {
        let registry = FontRegistry::default();
        let resolved = registry.resolve("Helvetica", UFontFace::NORMAL);
        assert!(matches!(resolved, ResolvedFont::Logical(_)));
        assert_eq!(family_names(resolved.face()), ["Liberation Sans"]);
    }

    #[test]
    fn styles_pick_the_matching_face() {
        let registry = FontRegistry::default();
        let bold = registry.resolve("Arial", UFontFace::BOLD);
        assert!(bold.face().is_bold());
        let italic = registry.resolve("Arial", UFontFace::ITALIC);
        assert!(italic.face().is_italic() && !italic.face().is_bold());
    }

    #[test]
    fn registered_fonts_are_found_by_family_and_take_precedence() {
        let mut registry = FontRegistry::default();
        let sans = crate::assets::get("fonts/LiberationSans-Regular.ttf").unwrap();
        assert_eq!(
            registry.register(sans.to_vec()),
            Ok(vec!["Liberation Sans".to_owned()])
        );
        assert_eq!(registry.faces.len(), EMBEDDED.len() + 1);
        assert!(registry.register(b"not a font".to_vec()).is_err());
    }
}
