//! Fonts and the text styling that travels with them (`klimt.font`).

use std::rc::Rc;

use super::geom::XDimension2D;
use super::ugraphic::UStroke;
use crate::color::HColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UFontFace {
    pub italic: bool,
    /// CSS weight: 400 is normal, 700 bold.
    pub weight: u16,
}

impl UFontFace {
    pub const NORMAL: Self = Self {
        italic: false,
        weight: 400,
    };
    pub const BOLD: Self = Self {
        italic: false,
        weight: 700,
    };
    pub const ITALIC: Self = Self {
        italic: true,
        weight: 400,
    };

    /// An upright face of a CSS weight, rounded to the nearest hundred between 100 and 900.
    pub fn with_weight(css_weight: i32) -> Self {
        let rounded = ((css_weight.clamp(100, 900) + 50) / 100) * 100;
        Self {
            italic: false,
            weight: rounded as u16,
        }
    }

    pub fn is_bold(self) -> bool {
        self.weight >= 700
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UFont {
    /// As written by the user: a family name or a comma-separated fallback list.
    family: Rc<str>,
    face: UFontFace,
    size: i32,
}

impl UFont {
    pub const SERIF: &str = "Serif";

    pub fn new(family: &str, face: UFontFace, size: i32) -> Self {
        Self {
            family: family.into(),
            face,
            size,
        }
    }

    pub fn serif(size: i32) -> Self {
        Self::new(Self::SERIF, UFontFace::NORMAL, size)
    }

    /// As written: a family name or a comma-separated fallback list.
    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn size_2d(&self) -> f64 {
        f64::from(self.size)
    }

    pub fn face(&self) -> UFontFace {
        self.face
    }

    #[must_use]
    pub fn with_size(&self, size: f32) -> Self {
        Self {
            size: size as i32,
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_face(&self, face: UFontFace) -> Self {
        Self {
            face,
            ..self.clone()
        }
    }

    /// The family as SVG names it: generic names for Java's logical fonts, double quotes made single.
    pub fn svg_family(&self) -> String {
        match &*self.family {
            "Serif" => "serif".to_owned(),
            "SansSerif" => "sans-serif".to_owned(),
            "Monospaced" => "monospace".to_owned(),
            family => family.replace('"', "'"),
        }
    }

    /// The name Java's AWT gives the font, like `Serif.bold`. Only Java's logical fonts are named the same on
    /// every machine; any other family is named as on a machine without it, which AWT replaces by `Dialog`.
    fn portable_name(&self) -> String {
        let first_family = self.family.split(',').next().unwrap_or_default();
        let first_family =
            first_family.trim_matches(|c: char| crate::java::is_whitespace(c) || c == '"');
        let logical = ["Serif", "SansSerif", "Monospaced", "Dialog", "DialogInput"]
            .into_iter()
            .find(|logical| logical.eq_ignore_ascii_case(first_family))
            .unwrap_or("Dialog");
        let style = match (self.face.is_bold(), self.face.italic) {
            (true, true) => "bolditalic",
            (true, false) => "bold",
            (false, true) => "italic",
            (false, false) => "plain",
        };
        format!("{logical}.{style}")
    }
}

impl std::fmt::Display for UFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.portable_name(), self.size)
    }
}

/// Text decorations, in the declaration order Java's `EnumSet` prints them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontStyle {
    Plain,
    Italic,
    Bold,
    Underline,
    Strike,
    Wave,
    Backcolor,
}

impl FontStyle {
    const ALL: [FontStyle; 7] = [
        Self::Plain,
        Self::Italic,
        Self::Bold,
        Self::Underline,
        Self::Strike,
        Self::Wave,
        Self::Backcolor,
    ];

    fn mutate_font(self, font: &UFont) -> UFont {
        match self {
            Self::Plain => font.with_face(UFontFace::NORMAL),
            Self::Italic => font.with_face(UFontFace {
                italic: true,
                ..font.face
            }),
            Self::Bold => font.with_face(UFontFace {
                weight: 700,
                ..font.face
            }),
            _ => font.clone(),
        }
    }

    fn java_name(self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
            Self::Italic => "ITALIC",
            Self::Bold => "BOLD",
            Self::Underline => "UNDERLINE",
            Self::Strike => "STRIKE",
            Self::Wave => "WAVE",
            Self::Backcolor => "BACKCOLOR",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontStyles(u8);

impl FontStyles {
    fn of(font: &UFont) -> Self {
        let mut styles = Self::default();
        if font.face.italic {
            styles = styles.with(FontStyle::Italic);
        }
        if font.face.is_bold() {
            styles = styles.with(FontStyle::Bold);
        }
        styles
    }

    #[must_use]
    pub fn with(self, style: FontStyle) -> Self {
        Self(self.0 | 1 << style as u8)
    }

    pub fn contains(self, style: FontStyle) -> bool {
        self.0 & 1 << style as u8 != 0
    }

    fn iter(self) -> impl Iterator<Item = FontStyle> {
        FontStyle::ALL
            .into_iter()
            .filter(move |style| self.contains(*style))
    }
}

impl std::fmt::Display for FontStyles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.iter().map(FontStyle::java_name).collect();
        write!(f, "[{}]", names.join(", "))
    }
}

/// Superscript and subscript shift text vertically and shrink it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontPosition {
    #[default]
    Normal,
    /// Superscript.
    Exposant,
    /// Subscript.
    Indice,
}

impl FontPosition {
    fn space(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Exposant => -6,
            Self::Indice => 3,
        }
    }

    /// Raised and lowered text is 3 points smaller, but at least 2.
    fn mute(self, font: UFont) -> UFont {
        match self {
            Self::Normal => font,
            Self::Exposant | Self::Indice => {
                let size = (font.size - 3).max(2);
                font.with_size(size as f32)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontConfiguration {
    styles: FontStyles,
    font: UFont,
    color: HColor,
    extended_color: Option<HColor>,
    position: FontPosition,
    tab_size: i32,
    hyperlink_color: HColor,
    /// How links are underlined; PlantUML draws no underlines at all when it has no thickness.
    underline_stroke: UStroke,
}

impl FontConfiguration {
    /// With blue links underlined by a simple line.
    pub fn new(font: UFont, color: HColor, tab_size: i32) -> Self {
        Self {
            styles: FontStyles::of(&font),
            font,
            color,
            extended_color: None,
            position: FontPosition::Normal,
            tab_size,
            hyperlink_color: HColor::BLUE,
            underline_stroke: UStroke::SIMPLE,
        }
    }

    #[must_use]
    pub fn with_hyperlink_style(&self, color: HColor, underline_stroke: UStroke) -> Self {
        Self {
            hyperlink_color: color,
            underline_stroke,
            ..self.clone()
        }
    }

    /// The configuration links are drawn in: underlined, in the link colour.
    #[must_use]
    pub fn hyperlink(&self) -> Self {
        Self {
            color: self.hyperlink_color.clone(),
            ..self.with_style(FontStyle::Underline)
        }
    }

    pub fn underline_stroke(&self) -> UStroke {
        self.underline_stroke
    }

    /// Black text with blue links, as for diagrams that have no skin.
    pub fn black_blue_true(font: UFont) -> Self {
        Self::new(font, HColor::BLACK, 8)
    }

    /// The font with styles and position applied.
    pub fn font(&self) -> UFont {
        let styled = self
            .styles
            .iter()
            .fold(self.font.clone(), |font, style| style.mutate_font(&font));
        self.position.mute(styled)
    }

    pub fn color(&self) -> &HColor {
        &self.color
    }

    /// The face of the font before styles apply.
    pub fn base_face(&self) -> UFontFace {
        self.font.face
    }

    pub fn contains_style(&self, style: FontStyle) -> bool {
        self.styles.contains(style)
    }

    pub fn extended_color(&self) -> Option<&HColor> {
        self.extended_color.as_ref()
    }

    pub fn space(&self) -> i32 {
        self.position.space()
    }

    pub fn tab_size(&self) -> i32 {
        self.tab_size
    }

    #[must_use]
    pub fn with_style(&self, style: FontStyle) -> Self {
        let styles = if style == FontStyle::Plain {
            FontStyles::default()
        } else {
            self.styles
        };
        Self {
            styles: styles.with(style),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn bigger(&self, delta: f64) -> Self {
        self.with_size((f64::from(self.font.size) + delta) as f32)
    }

    #[must_use]
    pub fn with_size(&self, size: f32) -> Self {
        Self {
            font: self.font.with_size(size),
            ..self.clone()
        }
    }

    /// The font family changes; the face and size stay.
    #[must_use]
    pub fn with_family(&self, family: &str) -> Self {
        Self {
            font: UFont::new(family, self.font.face, self.font.size),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_color(&self, color: HColor) -> Self {
        Self {
            color,
            ..self.clone()
        }
    }

    /// The colour of an underline, strike-through, wave or text background.
    #[must_use]
    pub fn with_extended_color(&self, color: HColor) -> Self {
        Self {
            extended_color: Some(color),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn with_position(&self, position: FontPosition) -> Self {
        Self {
            position,
            ..self.clone()
        }
    }

    /// How PlantUML's debug output describes the font, like `SansSerif.bold/14 [BOLD]`.
    pub fn to_string_debug(&self) -> String {
        format!("{} {}", self.font(), self.styles)
    }
}

/// Measures text. Each output format measures differently, which is why layout depends on the format.
pub trait StringBounder {
    fn calculate_dimension(&self, font: &UFont, text: &str) -> XDimension2D;

    fn descent(&self, font: &UFont, _text: &str) -> f64 {
        font.size_2d() / 4.5
    }
}
