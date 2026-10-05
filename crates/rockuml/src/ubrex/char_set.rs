//! Character classes (`〴s`, `〴W`...) and character sets (`「a〜z〴d」`), tested one UTF-16 unit at a time.

use crate::java;

/// `CaseMode.ensureLowercase`: ASCII letters only.
pub fn ensure_lowercase(unit: u16) -> u16 {
    if (u16::from(b'A')..=u16::from(b'Z')).contains(&unit) {
        unit + u16::from(b'a' - b'A')
    } else {
        unit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharClassRaw {
    Any,
    Space,
    Guillemet,
    Word,
    Digit,
    AlphaNumeric,
    Letter,
}

impl CharClassRaw {
    /// Two-letter classes are recognised by their first letter; the second is skipped unread.
    pub fn from_definition(first: char) -> Self {
        match first.to_ascii_lowercase() {
            's' => Self::Space,
            'g' => Self::Guillemet,
            'd' => Self::Digit,
            'w' => Self::Word,
            'a' => Self::AlphaNumeric,
            'l' => Self::Letter,
            '.' => Self::Any,
            _ => panic!("unknown char class 〴{first}"),
        }
    }

    pub fn definition_length(self) -> usize {
        match self {
            Self::AlphaNumeric | Self::Letter => 2,
            _ => 1,
        }
    }

    fn internal_matches(self, unit: u16) -> bool {
        // A surrogate half is no character at all to Java's `char` predicates.
        let Some(ch) = char::from_u32(u32::from(unit)) else {
            return self == Self::Any;
        };
        match self {
            Self::Any => true,
            Self::Space => ch == ' ',
            Self::Guillemet => matches!(ch, '"' | '“' | '”'),
            Self::Word => ch.is_ascii_alphanumeric() || ch == '_',
            Self::Digit => ch.is_ascii_digit() || ch == '_',
            Self::AlphaNumeric => java::is_letter_or_digit(ch),
            Self::Letter => java::is_letter(ch),
        }
    }
}

/// `〴` outside a set: an uppercase letter negates the class.
#[derive(Clone, Debug)]
pub struct CharClass {
    raw: CharClassRaw,
    negative: bool,
}

impl CharClass {
    pub fn from_definition(first: char) -> Self {
        Self {
            raw: CharClassRaw::from_definition(first),
            negative: first.is_ascii_uppercase(),
        }
    }

    pub fn definition_length(&self) -> usize {
        self.raw.definition_length()
    }

    pub fn matches(&self, unit: u16) -> bool {
        self.raw.internal_matches(unit) != self.negative
    }
}

#[derive(Clone, Debug, Default)]
pub struct ChallengeCharSet {
    char_classes: Vec<CharClassRaw>,
    char_set: CharSet,
    reversed: bool,
}

impl ChallengeCharSet {
    pub fn reverse(&mut self) {
        self.reversed = true;
    }

    pub fn add_class(&mut self, class: CharClassRaw) {
        self.char_classes.push(class);
    }

    pub fn add_char(&mut self, unit: u16) {
        self.char_set.add_char(unit);
    }

    pub fn add_range(&mut self, start: u16, end: u16) {
        self.char_set.add_range(start, end);
    }

    pub fn matches(&self, unit: u16) -> bool {
        let found = self.char_set.contains(unit)
            || self
                .char_classes
                .iter()
                .any(|class| class.internal_matches(unit));
        found != self.reversed
    }
}

/// Case-insensitive bitmask of the characters 32 to 160. Java's long shifts take their distance modulo 64,
/// so characters outside that span alias characters inside it; the arithmetic is kept to reproduce that.
#[derive(Clone, Debug, Default)]
struct CharSet {
    mask1: u64,
    mask2: u64,
}

impl CharSet {
    fn add_char(&mut self, unit: u16) {
        let unit = ensure_lowercase(unit);
        assert!(
            (32..=128).contains(&unit),
            "bad char {unit:#x} in a char set"
        );
        let offset = offset(unit);
        if offset < 64 {
            self.mask1 |= bit(offset);
        } else {
            self.mask2 |= bit(offset - 64);
        }
    }

    fn add_range(&mut self, start: u16, end: u16) {
        assert!(start <= end, "invalid range {start:#x}〜{end:#x}");
        assert!(
            start >= 32 && end <= 128,
            "range {start:#x}〜{end:#x} leaves ASCII"
        );
        let start_offset = offset(ensure_lowercase(start));
        let end_offset = offset(ensure_lowercase(end));
        if end_offset < 64 {
            self.mask1 |= low_bits(end_offset - start_offset + 1).wrapping_shl(start_offset as u32);
        } else if start_offset >= 64 {
            self.mask2 |=
                low_bits(end_offset - start_offset + 1).wrapping_shl((start_offset - 64) as u32);
        } else {
            self.mask1 |= low_bits(64 - start_offset).wrapping_shl(start_offset as u32);
            self.mask2 |= low_bits(end_offset - 64 + 1);
        }
    }

    fn contains(&self, unit: u16) -> bool {
        let offset = offset(ensure_lowercase(unit));
        if offset > 128 {
            false
        } else if offset < 64 {
            self.mask1 & bit(offset) != 0
        } else {
            self.mask2 & bit(offset - 64) != 0
        }
    }
}

fn offset(unit: u16) -> i32 {
    i32::from(unit) - 32
}

/// Java's `1L << distance`.
fn bit(distance: i32) -> u64 {
    1u64.wrapping_shl(distance as u32)
}

/// Java's `(1L << length) - 1`, with PlantUML's guard for a full 64-bit mask.
fn low_bits(length: i32) -> u64 {
    if length == 64 {
        u64::MAX
    } else {
        bit(length).wrapping_sub(1)
    }
}
