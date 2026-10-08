use crate::color::HColor;
use crate::klimt::HorizontalAlignment;
use crate::klimt::font::UFontFace;

/// A property's value as written, for the light scheme and the `@media dark` one. Later declarations get
/// higher priorities and win merges.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DarkString {
    light: Option<String>,
    dark: Option<String>,
    priority: i32,
}

impl DarkString {
    fn merge_with(&self, other: &DarkString) -> DarkString {
        let higher = || {
            if self.priority > other.priority {
                self.clone()
            } else {
                other.clone()
            }
        };
        if (self.dark.is_none() && other.dark.is_none())
            || (self.light.is_none() && other.light.is_none())
        {
            return higher();
        }
        if self.dark.is_none() && other.light.is_none() {
            return DarkString {
                light: self.light.clone(),
                dark: other.dark.clone(),
                priority: self.priority,
            };
        }
        if other.dark.is_none() && self.light.is_none() {
            return DarkString {
                light: other.light.clone(),
                dark: self.dark.clone(),
                priority: other.priority,
            };
        }
        higher()
    }
}

/// A property's value: as written in a style sheet (PlantUML's `ValueImpl`), or a colour a diagram element
/// overrides it with (`ValueColor`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Written(DarkString),
    Color { color: HColor, priority: i32 },
}

impl Value {
    pub(crate) fn regular(text: &str, priority: i32) -> Self {
        Self::Written(DarkString {
            light: Some(text.to_owned()),
            dark: None,
            priority,
        })
    }

    pub(crate) fn dark(text: &str, priority: i32) -> Self {
        Self::Written(DarkString {
            light: None,
            dark: Some(text.to_owned()),
            priority,
        })
    }

    pub(crate) fn priority(&self) -> i32 {
        match self {
            Self::Written(written) => written.priority,
            Self::Color { priority, .. } => *priority,
        }
    }

    #[must_use]
    pub(crate) fn with_added_priority(&self, delta: i32) -> Self {
        match self {
            Self::Written(written) => Self::Written(DarkString {
                priority: written.priority.wrapping_add(delta),
                ..written.clone()
            }),
            Self::Color { color, priority } => Self::Color {
                color: color.clone(),
                priority: priority.wrapping_add(delta),
            },
        }
    }

    /// This value declared over `previous`. A colour override only gives way to a higher priority.
    #[must_use]
    pub(crate) fn merge_with(&self, previous: Option<&Value>) -> Value {
        match (self, previous) {
            (Self::Written(written), Some(Self::Written(previous))) => {
                Self::Written(written.merge_with(previous))
            }
            (_, Some(previous)) if previous.priority() > self.priority() => previous.clone(),
            _ => self.clone(),
        }
    }

    fn light(&self) -> Option<String> {
        match self {
            Self::Written(written) => written.light.clone(),
            Self::Color { color, .. } => Some(color.as_string()),
        }
    }
}

/// Reading a property, present or not. A missing property reads as a neutral default.
pub(crate) trait ValueReading {
    fn as_string(&self) -> String;
    fn as_int(&self) -> i32;
    fn as_int_or_minus_one(&self) -> i32;
    fn as_double(&self) -> f64;
    fn as_font_face(&self) -> UFontFace;
    fn as_horizontal_alignment(&self) -> Option<HorizontalAlignment>;
    fn as_color(&self) -> HColor;
}

impl ValueReading for Option<&Value> {
    fn as_string(&self) -> String {
        self.and_then(Value::light).unwrap_or_default()
    }

    fn as_int(&self) -> i32 {
        self.map_or(0, |_| digits(&self.as_string()).parse().unwrap_or(0))
    }

    fn as_int_or_minus_one(&self) -> i32 {
        self.map_or(0, |_| digits(&self.as_string()).parse().unwrap_or(-1))
    }

    /// Only digits and dots count, so `1.5px` reads as 1.5; nothing readable gives NaN.
    fn as_double(&self) -> f64 {
        self.map_or(0.0, |_| {
            let number: String = self
                .as_string()
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            number.parse().unwrap_or(f64::NAN)
        })
    }

    fn as_font_face(&self) -> UFontFace {
        let text = self.as_string();
        match text.trim().to_lowercase().as_str() {
            "" | "plain" | "normal" => UFontFace::NORMAL,
            "bold" => UFontFace::BOLD,
            "italic" => UFontFace::ITALIC,
            "lighter" => UFontFace::with_weight(300),
            "bolder" => UFontFace::with_weight(800),
            weight => weight
                .parse()
                .map_or(UFontFace::NORMAL, UFontFace::with_weight),
        }
    }

    /// A missing property reads as left; an unknown name as nothing.
    fn as_horizontal_alignment(&self) -> Option<HorizontalAlignment> {
        match self {
            None => Some(HorizontalAlignment::Left),
            Some(_) => HorizontalAlignment::from_name(&self.as_string()),
        }
    }

    /// Unknown colour names read as white. PlantUML fails on a colour declared only for dark mode; here
    /// it reads as missing.
    fn as_color(&self) -> HColor {
        if let Some(Value::Color { color, .. }) = self {
            return color.clone();
        }
        let Some(text) = self.and_then(Value::light) else {
            return HColor::BLACK;
        };
        if text.eq_ignore_ascii_case("none") || text.eq_ignore_ascii_case("transparent") {
            return HColor::NONE;
        }
        HColor::parse(&text).ok().flatten().unwrap_or(HColor::WHITE)
    }
}

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_later_declaration_wins_a_merge() {
        let early = Value::regular("red", 1);
        let late = Value::regular("blue", 2);
        assert_eq!(late.merge_with(Some(&early)), late);
        assert_eq!(early.merge_with(Some(&late)), late);
    }

    #[test]
    fn light_and_dark_declarations_combine() {
        let light = Value::regular("white", 1);
        let dark = Value::dark("black", 2);
        let combined = dark.merge_with(Some(&light));
        assert_eq!(Some(&combined).as_string(), "white");
        assert_eq!(combined.priority(), 1);
    }

    #[test]
    fn a_colour_declared_only_for_dark_mode_reads_as_missing() {
        let dark = Value::dark("red", 1);
        assert_eq!(Some(&dark).as_color(), HColor::BLACK);
        assert_eq!(
            Some(&Value::regular("red", 1)).as_color(),
            HColor::parse("red").unwrap().unwrap()
        );
    }

    #[test]
    fn numbers_are_read_from_digits_only() {
        let value = Value::regular("1.5px", 1);
        assert_eq!(Some(&value).as_double(), 1.5);
        assert_eq!(Some(&value).as_int(), 15);
        assert!(Some(&Value::regular("x", 1)).as_double().is_nan());
        assert_eq!(None.as_double(), 0.0);
        assert_eq!(Some(&Value::regular("x", 1)).as_int_or_minus_one(), -1);
    }
}
