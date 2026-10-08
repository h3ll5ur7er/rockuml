//! Fonts set by legacy skin parameters like `classStereotypeFontSize`, for the texts that styles do not
//! cover (PlantUML's `FontParam` and `SkinParam.getFont`).

use super::{SkinParam, is_digits};
use crate::color::HColor;
use crate::klimt::font::{FontConfiguration, UFont, UFontFace};
use crate::stereo::Stereotype;
use crate::text::unquoted;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FontParam {
    CircledCharacter,
    ClassStereotype,
    Note,
    ObjectStereotype,
    PackageStereotype,
}

impl FontParam {
    /// The Java constant's name, which skin parameters are named after.
    fn name(self) -> &'static str {
        match self {
            Self::CircledCharacter => "CIRCLED_CHARACTER",
            Self::ClassStereotype => "CLASS_STEREOTYPE",
            Self::Note => "NOTE",
            Self::ObjectStereotype => "OBJECT_STEREOTYPE",
            Self::PackageStereotype => "PACKAGE_STEREOTYPE",
        }
    }

    fn default_size(self) -> i32 {
        match self {
            Self::CircledCharacter => 17,
            Self::ClassStereotype | Self::ObjectStereotype => 12,
            Self::Note => 13,
            Self::PackageStereotype => 14,
        }
    }

    fn default_face(self) -> UFontFace {
        match self {
            Self::CircledCharacter => UFontFace::BOLD,
            Self::Note => UFontFace::NORMAL,
            Self::ClassStereotype | Self::ObjectStereotype | Self::PackageStereotype => {
                UFontFace::ITALIC
            }
        }
    }

    fn default_family(self) -> &'static str {
        match self {
            Self::CircledCharacter => "Monospaced",
            Self::ClassStereotype
            | Self::Note
            | Self::ObjectStereotype
            | Self::PackageStereotype => "SansSerif",
        }
    }
}

impl SkinParam {
    /// `<param><suffix><<stereotype>>`, then `<param><suffix>`.
    fn font_value(
        &self,
        param: FontParam,
        suffix: &str,
        stereotype: Option<&Stereotype>,
    ) -> Option<String> {
        stereotype
            .and_then(|stereotype| {
                self.value(&format!(
                    "{}{suffix}{}",
                    param.name(),
                    stereotype.label_double_comparator()
                ))
            })
            .or_else(|| self.value(&format!("{}{suffix}", param.name())))
    }

    /// `getFont`: the skin parameters of the font, then the default font's, then the parameter's own.
    pub(crate) fn get_font(&self, param: FontParam, stereotype: Option<&Stereotype>) -> UFont {
        let family = self
            .font_value(param, "fontname", stereotype)
            .or_else(|| {
                (param != FontParam::CircledCharacter)
                    .then(|| self.value("defaultfontname"))
                    .flatten()
            })
            .map_or_else(
                || param.default_family().to_owned(),
                |name| unquoted(&name).to_owned(),
            );
        let face = self
            .font_value(param, "fontstyle", stereotype)
            .or_else(|| self.value("defaultfontstyle"))
            .map_or_else(
                || param.default_face(),
                |value| {
                    let lower = value.to_lowercase();
                    UFontFace {
                        italic: lower.contains("italic"),
                        ..if lower.contains("bold") {
                            UFontFace::BOLD
                        } else {
                            UFontFace::NORMAL
                        }
                    }
                },
            );
        let size = self
            .font_value(param, "fontsize", stereotype)
            .filter(|value| is_digits(value))
            .or_else(|| {
                self.value("defaultfontsize")
                    .filter(|value| is_digits(value))
            })
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| param.default_size());
        UFont::new(&family, face, size)
    }

    /// `FontConfiguration.create(skinParam, fontParam, stereotype)`.
    pub(crate) fn get_font_configuration(
        &self,
        param: FontParam,
        stereotype: Option<&Stereotype>,
    ) -> FontConfiguration {
        let color = self
            .font_value(param, "fontcolor", stereotype)
            .or_else(|| self.value("defaultfontcolor"))
            .map_or(HColor::BLACK, |value| HColor::parse_or_white(&value));
        FontConfiguration::new(self.get_font(param, stereotype), color, 8)
    }
}
