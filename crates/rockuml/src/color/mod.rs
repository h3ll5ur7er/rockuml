//! Colours as PlantUML parses and transforms them (`klimt.color`).

mod hsl;
mod hsluv;
mod named;

pub use hsl::to_rgb as hsl_to_rgb;
use named::NAMED_COLORS;

use crate::java::{self, RuntimeException};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl XColor {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// An opaque colour from `0xRRGGBB`; higher bits are ignored.
    pub const fn from_rgb(rgb: u32) -> Self {
        Self::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Perceived brightness, 0 to 255.
    pub fn gray_scale(self) -> u32 {
        (u32::from(self.red) * 299 + u32::from(self.green) * 587 + u32::from(self.blue) * 114)
            / 1000
    }
}

/// A parsed colour. Only plain colours take part in arithmetic; the others pass through unchanged.
#[derive(Clone, Debug, PartialEq)]
pub enum HColor {
    Simple(XColor),
    Automagic,
    Scheme,
    Gradient,
}

impl HColor {
    /// Parses a colour name, `#rgb`, `#rrggbb`, `#rrggbbaa`, gradient (`red-blue`) or scheme (`?a:b`).
    pub fn parse(text: &str) -> Result<Option<HColor>, RuntimeException> {
        let text = text.strip_prefix('#').unwrap_or(text);
        if text.eq_ignore_ascii_case("transparent") || text.eq_ignore_ascii_case("background") {
            return Ok(Some(HColor::Simple(XColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            })));
        }
        if text.eq_ignore_ascii_case("automatic") {
            return Ok(Some(HColor::Automagic));
        }
        if let Some(color) = parse_simple_color(text) {
            return Ok(Some(HColor::Simple(color)));
        }
        if let Some(scheme) = text.strip_prefix('?') {
            let parts = java::split(scheme, ":");
            if parts.len() == 2 || parts.len() == 3 {
                let known = |part: &String| parse_simple_color(part).is_some();
                if parts.len() == 2 && known(&parts[0]) && known(&parts[1]) {
                    return Ok(Some(HColor::Scheme));
                }
                // PlantUML reads a third colour here even when only two were given.
                if known(parts.get(2).ok_or(RuntimeException)?) {
                    return Ok(Some(HColor::Scheme));
                }
            }
        }
        for (index, c) in text.char_indices() {
            if matches!(c, '-' | '\\' | '|' | '/')
                && parse_simple_color(&text[..index]).is_some()
                && parse_simple_color(&text[index + 1..]).is_some()
            {
                return Ok(Some(HColor::Gradient));
            }
        }
        Ok(None)
    }

    /// Java computes `l += l * (ratio / 100.0)` in `double`, then narrows to `float`.
    pub fn lighten(&self, ratio: i32) -> HColor {
        self.with_luminance(|luminance| {
            let luminance = f64::from(luminance);
            (luminance + luminance * (f64::from(ratio) / 100.0)) as f32
        })
    }

    pub fn darken(&self, ratio: i32) -> HColor {
        self.with_luminance(|luminance| {
            let luminance = f64::from(luminance);
            (luminance - luminance * (f64::from(ratio) / 100.0)) as f32
        })
    }

    fn with_luminance(&self, change: impl Fn(f32) -> f32) -> HColor {
        match self {
            HColor::Simple(color) => {
                let [hue, saturation, luminance] = hsl::from_rgb(*color);
                HColor::Simple(hsl::to_rgb(hue, saturation, change(luminance), 1.0))
            }
            other => other.clone(),
        }
    }

    /// The RGB complement.
    pub fn reverse(&self) -> HColor {
        match self {
            HColor::Simple(color) => HColor::Simple(XColor::rgb(
                255 - color.red,
                255 - color.green,
                255 - color.blue,
            )),
            other => other.clone(),
        }
    }

    /// Shifts lightness by half the range in the perceptual `HSLuv` space, keeping hue and saturation.
    pub fn reverse_hsluv(&self) -> HColor {
        match self {
            HColor::Simple(color) => HColor::Simple(hsluv::reverse(*color)),
            other => other.clone(),
        }
    }

    pub fn is_dark(&self) -> bool {
        match self {
            HColor::Simple(color) => color.gray_scale() < 128,
            _ => true,
        }
    }

    /// `#RRGGBB`, `#aarrggbb` when translucent, or `transparent`.
    pub fn as_string(&self) -> String {
        match self {
            HColor::Simple(color) if color.alpha == 0 => "transparent".to_owned(),
            HColor::Simple(color) if color.alpha == 255 => {
                format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue)
            }
            HColor::Simple(color) => {
                format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    color.alpha, color.red, color.green, color.blue
                )
            }
            HColor::Automagic => "?HColorAutomagic".to_owned(),
            HColor::Scheme => "?HColorScheme".to_owned(),
            HColor::Gradient => "?HColorGradient".to_owned(),
        }
    }
}

fn parse_simple_color(text: &str) -> Option<XColor> {
    let text = text.strip_prefix('#').unwrap_or(text);
    let nibbles: Option<Vec<u8>> = text
        .chars()
        .map(|c| c.to_digit(16).map(|digit| digit as u8))
        .collect();
    let from_hex = match nibbles.as_deref() {
        Some(&[gray]) => Some(XColor::rgb(gray * 17, gray * 17, gray * 17)),
        Some(&[red, green, blue]) => Some(XColor::rgb(red * 17, green * 17, blue * 17)),
        Some(&[r1, r2, g1, g2, b1, b2]) => {
            Some(XColor::rgb(r1 << 4 | r2, g1 << 4 | g2, b1 << 4 | b2))
        }
        Some(&[r1, r2, g1, g2, b1, b2, a1, a2]) => Some(XColor {
            red: r1 << 4 | r2,
            green: g1 << 4 | g2,
            blue: b1 << 4 | b2,
            alpha: a1 << 4 | a2,
        }),
        _ => None,
    };
    from_hex.or_else(|| named_color(text))
}

fn named_color(name: &str) -> Option<XColor> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    NAMED_COLORS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, rgb)| XColor::from_rgb(rgb))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> String {
        HColor::parse(text)
            .unwrap()
            .map_or("none".to_owned(), |color| color.as_string())
    }

    #[test]
    fn parses_names_and_hex_forms() {
        assert_eq!(parsed("Red"), "#FF0000");
        assert_eq!(parsed("#aliceblue"), "#F0F8FF");
        assert_eq!(parsed("#3"), "#333333");
        assert_eq!(parsed("#abc"), "#AABBCC");
        assert_eq!(parsed("#11223344"), "#44112233");
        assert_eq!(parsed("transparent"), "transparent");
        assert_eq!(parsed("red-blue"), "?HColorGradient");
        assert_eq!(parsed("?red:blue"), "?HColorScheme");
        assert_eq!(parsed("nosuchcolor"), "none");
    }

    #[test]
    fn a_two_part_scheme_with_an_unknown_colour_crashes_like_plantuml() {
        assert_eq!(HColor::parse("?red:nosuch"), Err(RuntimeException));
    }

    #[test]
    fn lighten_and_darken_match_plantuml() {
        let cases = [
            ("#FFFFFF", "#CCCCCC", "#FFFFFF", "#FFFFFF", "#1A1A1A"),
            ("#1E90FF", "#0074E4", "#57ACFF", "#F4F9FF", "#000E1C"),
            ("#FF0000", "#CC0000", "#FF3333", "#FFBFBF", "#1A0000"),
            ("#808080", "#666666", "#9A9A9A", "#E0E0E0", "#0D0D0D"),
            ("#abcdef", "#66A4E2", "#F0F6FC", "#FFFFFF", "#071522"),
        ];
        for (input, darken_20, lighten_20, lighten_75, darken_90) in cases {
            let color = HColor::parse(input).unwrap().unwrap();
            assert_eq!(color.darken(20).as_string(), darken_20, "{input}");
            assert_eq!(color.lighten(20).as_string(), lighten_20, "{input}");
            assert_eq!(color.lighten(75).as_string(), lighten_75, "{input}");
            assert_eq!(color.darken(90).as_string(), darken_90, "{input}");
        }
    }

    #[test]
    fn zero_ratio_keeps_the_colour() {
        let color = HColor::parse("#1E90FF").unwrap().unwrap();
        assert_eq!(color.darken(0), color);
        assert_eq!(color.lighten(0), color);
    }

    #[test]
    fn darkness_uses_weighted_gray_scale() {
        assert!(HColor::parse("navy").unwrap().unwrap().is_dark());
        assert!(!HColor::parse("yellow").unwrap().unwrap().is_dark());
    }
}
