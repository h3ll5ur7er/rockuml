//! The visibility of a class member or of an element, written `-`, `#`, `~`, `+` or `*` before its name,
//! and the small icon that shows it (PlantUML's `VisibilityModifier`).

use super::SkinParam;
use crate::color::{HColor, XColor};
use crate::klimt::TextBlock;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::XDimension2D;
use crate::klimt::group::{UGroup, UGroupType};
use crate::klimt::shape::{UEllipse, URectangle, UShape};
use crate::klimt::ugraphic::UGraphic;
use crate::style::{SName, StyleSignature};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum VisibilityModifier {
    PrivateField,
    ProtectedField,
    PackagePrivateField,
    PublicField,
    PrivateMethod,
    ProtectedMethod,
    PackagePrivateMethod,
    PublicMethod,
    IeMandatory,
}

/// A colour a skin parameter may set, and its default (PlantUML's `ColorParam`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IconColor {
    name: &'static str,
    default: u32,
}

impl IconColor {
    /// `skinparam <name>Color`, or the default.
    pub(crate) fn get(self, skin: &SkinParam) -> HColor {
        skin.value(&format!("{}color", self.name)).map_or_else(
            || HColor::Simple(XColor::from_rgb(self.default)),
            |value| HColor::parse_or_white(&value),
        )
    }
}

const ICON_PRIVATE: IconColor = IconColor {
    name: "iconPrivate",
    default: 0xC8_2930,
};
const ICON_PRIVATE_BACKGROUND: IconColor = IconColor {
    name: "iconPrivateBackground",
    default: 0xF2_4D5C,
};
const ICON_PACKAGE: IconColor = IconColor {
    name: "iconPackage",
    default: 0x19_63A0,
};
const ICON_PACKAGE_BACKGROUND: IconColor = IconColor {
    name: "iconPackageBackground",
    default: 0x41_77AF,
};
const ICON_PROTECTED: IconColor = IconColor {
    name: "iconProtected",
    default: 0xB3_8D22,
};
const ICON_PROTECTED_BACKGROUND: IconColor = IconColor {
    name: "iconProtectedBackground",
    default: 0xFF_FF44,
};
const ICON_PUBLIC: IconColor = IconColor {
    name: "iconPublic",
    default: 0x03_8048,
};
const ICON_PUBLIC_BACKGROUND: IconColor = IconColor {
    name: "iconPublicBackground",
    default: 0x84_BE84,
};
const ICON_IE_MANDATORY: IconColor = IconColor {
    name: "iconIEMandatory",
    default: 0x00_0000,
};

impl VisibilityModifier {
    /// The characters that start a member with a visibility.
    pub(crate) const REGEX_FOR_VISIBILITY_CHARACTER: &str = "[-#+~]";

    /// The Java constant's name, which SVG writes on the icon's group.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::PrivateField => "PRIVATE_FIELD",
            Self::ProtectedField => "PROTECTED_FIELD",
            Self::PackagePrivateField => "PACKAGE_PRIVATE_FIELD",
            Self::PublicField => "PUBLIC_FIELD",
            Self::PrivateMethod => "PRIVATE_METHOD",
            Self::ProtectedMethod => "PROTECTED_METHOD",
            Self::PackagePrivateMethod => "PACKAGE_PRIVATE_METHOD",
            Self::PublicMethod => "PUBLIC_METHOD",
            Self::IeMandatory => "IE_MANDATORY",
        }
    }

    /// Whether `s` starts with a visibility character: longer than two characters, the second not the
    /// same as the first, so that `--` or `##` start no member.
    pub(crate) fn is_visibility_character(s: &str) -> bool {
        Self::first_two(s).is_some_and(|c| matches!(c, '-' | '#' | '+' | '~' | '*'))
    }

    pub(crate) fn get_visibility_modifier(s: &str, is_field: bool) -> Option<Self> {
        let c = Self::first_two(s)?;
        Some(match (c, is_field) {
            ('-', true) => Self::PrivateField,
            ('#', true) => Self::ProtectedField,
            ('+', true) => Self::PublicField,
            ('~', true) => Self::PackagePrivateField,
            ('-', false) => Self::PrivateMethod,
            ('#', false) => Self::ProtectedMethod,
            ('+', false) => Self::PublicMethod,
            ('~', false) => Self::PackagePrivateMethod,
            ('*', _) => Self::IeMandatory,
            _ => return None,
        })
    }

    /// The first character, if the text is longer than two characters (in UTF-16 units) and does not start
    /// with it twice.
    fn first_two(s: &str) -> Option<char> {
        if s.encode_utf16().count() <= 2 {
            return None;
        }
        let mut chars = s.chars();
        let first = chars.next()?;
        (chars.next() != Some(first)).then_some(first)
    }

    pub(crate) fn is_field(self) -> bool {
        matches!(
            self,
            Self::PublicField
                | Self::PrivateField
                | Self::ProtectedField
                | Self::PackagePrivateField
        )
    }

    pub(crate) fn get_foreground(self) -> IconColor {
        match self {
            Self::PrivateField | Self::PrivateMethod => ICON_PRIVATE,
            Self::ProtectedField | Self::ProtectedMethod => ICON_PROTECTED,
            Self::PackagePrivateField | Self::PackagePrivateMethod => ICON_PACKAGE,
            Self::PublicField | Self::PublicMethod => ICON_PUBLIC,
            Self::IeMandatory => ICON_IE_MANDATORY,
        }
    }

    /// Fields draw hollow icons.
    pub(crate) fn get_background(self) -> Option<IconColor> {
        match self {
            Self::PrivateMethod => Some(ICON_PRIVATE_BACKGROUND),
            Self::ProtectedMethod => Some(ICON_PROTECTED_BACKGROUND),
            Self::PackagePrivateMethod => Some(ICON_PACKAGE_BACKGROUND),
            Self::PublicMethod => Some(ICON_PUBLIC_BACKGROUND),
            Self::IeMandatory => Some(ICON_IE_MANDATORY),
            Self::PrivateField
            | Self::ProtectedField
            | Self::PackagePrivateField
            | Self::PublicField => None,
        }
    }

    pub(crate) fn get_style_signature(self) -> StyleSignature {
        let kind = match self {
            Self::IeMandatory => SName::IeMandatory,
            Self::PublicField | Self::PublicMethod => SName::Public,
            Self::PrivateField | Self::PrivateMethod => SName::Private,
            Self::ProtectedField | Self::ProtectedMethod => SName::Protected,
            Self::PackagePrivateField | Self::PackagePrivateMethod => SName::Package,
        };
        StyleSignature::of(&[SName::Root, SName::Element, SName::VisibilityIcon, kind])
    }

    /// The icon in a square of `size` + 1, drawn over an invisible rectangle twice as wide when it is a
    /// link's.
    pub(crate) fn get_u_block(
        self,
        size: i32,
        foreground: HColor,
        background: Option<HColor>,
        with_invisible_rectangle: bool,
    ) -> VisibilityIcon {
        VisibilityIcon {
            modifier: self,
            size,
            foreground,
            background,
            with_invisible_rectangle,
        }
    }

    fn draw_internal(
        self,
        ug: &UGraphic,
        size: i32,
        foreground: &HColor,
        background: Option<&HColor>,
    ) {
        let ug = ug
            .with_backcolor(background.cloned().unwrap_or(HColor::NONE))
            .with_color(foreground.clone());
        let size = f64::from(size - size % 2);
        match self {
            Self::PackagePrivateField | Self::PackagePrivateMethod => draw_triangle(&ug, size),
            Self::PrivateField | Self::PrivateMethod => ug
                .translated(2.0, 2.0)
                .draw(&UShape::Rectangle(URectangle::new(size - 4.0, size - 4.0))),
            Self::ProtectedField | Self::ProtectedMethod => draw_diamond(&ug, size),
            Self::PublicField | Self::PublicMethod | Self::IeMandatory => ug
                .translated(2.0, 2.0)
                .draw(&UShape::Ellipse(UEllipse::new(size - 4.0, size - 4.0))),
        }
    }
}

fn draw_diamond(ug: &UGraphic, size: f64) {
    let size = size - 2.0;
    let points = vec![
        (size / 2.0, 0.0),
        (size, size / 2.0),
        (size / 2.0, size),
        (0.0, size / 2.0),
    ];
    ug.translated(1.0, 0.0).draw(&UShape::polygon(points));
}

fn draw_triangle(ug: &UGraphic, size: f64) {
    let size = size - 2.0;
    let points = vec![(size / 2.0, 1.0), (0.0, size - 1.0), (size, size - 1.0)];
    ug.translated(1.0, 0.0).draw(&UShape::polygon(points));
}

/// The icon of a visibility (`VisibilityModifier.getUBlock`).
pub(crate) struct VisibilityIcon {
    modifier: VisibilityModifier,
    size: i32,
    foreground: HColor,
    background: Option<HColor>,
    with_invisible_rectangle: bool,
}

impl TextBlock for VisibilityIcon {
    fn calculate_dimension(&self, _string_bounder: &dyn StringBounder) -> XDimension2D {
        let side = f64::from(self.size + 1);
        XDimension2D::new(side, side)
    }

    fn draw_u(&self, ug: &UGraphic) {
        if self.with_invisible_rectangle {
            let size = f64::from(self.size);
            ug.with_color(HColor::NONE)
                .draw(&UShape::Rectangle(URectangle::new(size * 2.0, size)));
        }
        ug.start_group(&UGroup::singleton(
            UGroupType::DataVisibilityModifier,
            self.modifier.name(),
        ));
        self.modifier
            .draw_internal(ug, self.size, &self.foreground, self.background.as_ref());
        ug.close_group();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_visibility_needs_a_name_after_its_character() {
        assert!(VisibilityModifier::is_visibility_character("-ab"));
        assert!(VisibilityModifier::is_visibility_character("*ab"));
        assert!(!VisibilityModifier::is_visibility_character("-a"));
        assert!(!VisibilityModifier::is_visibility_character("--ab"));
        assert!(!VisibilityModifier::is_visibility_character("ab"));
        assert_eq!(
            VisibilityModifier::get_visibility_modifier("#foo", true),
            Some(VisibilityModifier::ProtectedField)
        );
        assert_eq!(
            VisibilityModifier::get_visibility_modifier("~foo()", false),
            Some(VisibilityModifier::PackagePrivateMethod)
        );
        assert_eq!(
            VisibilityModifier::get_visibility_modifier("*id", false),
            Some(VisibilityModifier::IeMandatory)
        );
        assert_eq!(VisibilityModifier::get_visibility_modifier("x", true), None);
    }
}
