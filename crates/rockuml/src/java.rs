//! Java library semantics that leak into PlantUML's output and therefore have to be reproduced exactly.

use unicode_general_category::{GeneralCategory, get_general_category};

/// Marks the places where PlantUML throws an unchecked Java exception (index out of bounds, null pointer,
/// number format...). Callers turn it into whatever PlantUML does with such exceptions at that level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeException;

/// `Character.isWhitespace`: Unicode separators except the non-breaking ones, plus ASCII control spaces.
pub fn is_whitespace(c: char) -> bool {
    match c {
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{1C}'..='\u{1F}' => true,
        '\u{A0}' | '\u{2007}' | '\u{202F}' => false,
        _ => is_space_char(c),
    }
}

/// `Character.isSpaceChar`: any Unicode space, line or paragraph separator.
pub fn is_space_char(c: char) -> bool {
    matches!(
        get_general_category(c),
        GeneralCategory::SpaceSeparator
            | GeneralCategory::LineSeparator
            | GeneralCategory::ParagraphSeparator
    )
}

/// `Character.isLetter`.
pub fn is_letter(c: char) -> bool {
    matches!(
        get_general_category(c),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
    )
}

/// `Character.isLetterOrDigit`.
pub fn is_letter_or_digit(c: char) -> bool {
    is_letter(c) || get_general_category(c) == GeneralCategory::DecimalNumber
}

/// `String.trim`: strips every character up to and including the space character.
pub fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// `String.split(regex)` drops trailing empty strings; this is the literal-separator version.
pub fn split(s: &str, separator: &str) -> Vec<String> {
    if !s.contains(separator) {
        return vec![s.to_owned()];
    }
    let mut parts: Vec<String> = s.split(separator).map(str::to_owned).collect();
    while parts.last().is_some_and(String::is_empty) {
        parts.pop();
    }
    parts
}

/// `String.split(regex)`: an empty match at the very start yields no leading piece, and trailing empty
/// pieces are dropped.
pub fn regex_split(separator: &regex::Regex, s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut index = 0;
    let mut found = false;
    for matched in separator.find_iter(s) {
        if matched.end() == 0 {
            continue;
        }
        found = true;
        parts.push(s[index..matched.start()].to_owned());
        index = matched.end();
    }
    if !found {
        return vec![s.to_owned()];
    }
    parts.push(s[index..].to_owned());
    while parts.last().is_some_and(String::is_empty) {
        parts.pop();
    }
    parts
}

/// `String.hashCode`, computed over UTF-16 code units.
pub fn string_hash_code(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

/// The order in which `java.util.HashMap` iterates `hashes`, given in insertion order with no removals.
/// Entries go by bucket, then by insertion within a bucket; the table starts at 16 buckets and doubles
/// whenever it is more than three-quarters full.
pub fn hash_map_iteration_order(hashes: &[i32]) -> Vec<usize> {
    let mut capacity = 16usize;
    while hashes.len() * 4 > capacity * 3 {
        capacity *= 2;
    }
    let bucket = |hash: i32| {
        let spread = (hash ^ (hash >> 16 & 0xFFFF)) as u32;
        spread as usize & (capacity - 1)
    };
    let mut order: Vec<usize> = (0..hashes.len()).collect();
    order.sort_by_key(|&index| bucket(hashes[index]));
    order
}

/// `java.util.Random`'s 48-bit linear congruential generator, for output that PlantUML seeds.
pub struct Random {
    seed: u64,
}

impl Random {
    const MULTIPLIER: u64 = 0x5_DEEC_E66D;
    const MASK: u64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(0xB)) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// `None` where Java throws because `bound` is not positive.
    pub fn next_int(&mut self, bound: i32) -> Option<i32> {
        if bound <= 0 {
            return None;
        }
        if bound & -bound == bound {
            return Some(((i64::from(bound) * i64::from(self.next(31))) >> 31) as i32);
        }
        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
                return Some(value);
            }
        }
    }

    pub fn next_double(&mut self) -> f64 {
        let high = i64::from(self.next(26)) << 27;
        let low = i64::from(self.next(27));
        (high + low) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_matches_java_sequences() {
        let mut random = Random::new(42);
        assert_eq!(random.next_int(100), Some(30));
        assert_eq!(random.next_int(100), Some(63));
        assert_eq!(random.next_int(100), Some(48));
        assert_eq!(Random::new(42).next_double(), 0.727_563_680_032_868_1);
        assert_eq!(Random::new(7).next_int(16), Some(11));
        assert_eq!(Random::new(-3).next_int(1000), Some(164));
        assert_eq!(Random::new(1).next_int(0), None);
    }

    #[test]
    fn regex_split_follows_java_edge_cases() {
        let comma = regex::Regex::new(",").unwrap();
        assert_eq!(regex_split(&comma, "a,,b,,"), ["a", "", "b"]);
        assert_eq!(regex_split(&comma, ",a"), ["", "a"]);
        assert_eq!(regex_split(&comma, "abc"), ["abc"]);
        let empty = regex::Regex::new("").unwrap();
        assert_eq!(regex_split(&empty, "ab"), ["a", "b"]);
    }

    #[test]
    fn string_hash_code_matches_java() {
        assert_eq!(string_hash_code(""), 0);
        assert_eq!(string_hash_code("hello"), 99_162_322);
        assert_eq!(string_hash_code("%strlen"), 1_880_881_833);
        assert_eq!(string_hash_code("\u{1F600}"), 1_772_899);
    }

    #[test]
    fn hash_map_iterates_by_bucket_then_insertion() {
        assert_eq!(hash_map_iteration_order(&[17, 1, 33, 2]), [0, 1, 2, 3]);
        assert_eq!(hash_map_iteration_order(&[5, 3, 21]), [1, 0, 2]);
    }

    #[test]
    fn hash_map_spreads_high_bits_into_the_bucket_index() {
        assert_eq!(hash_map_iteration_order(&[0x0001_0000, 0]), [1, 0]);
    }

    #[test]
    fn non_breaking_spaces_are_space_chars_but_not_whitespace() {
        assert!(is_space_char('\u{A0}'));
        assert!(!is_whitespace('\u{A0}'));
        assert!(is_whitespace('\u{1F}'));
        assert!(!is_space_char('\t'));
    }

    #[test]
    fn letters_follow_the_general_category_not_the_alphabetic_property() {
        assert!(is_letter('é'));
        assert!(!is_letter('Ⅻ'));
        assert!(is_letter_or_digit('٣'));
    }

    #[test]
    fn trim_removes_control_characters_but_not_unicode_spaces() {
        assert_eq!(trim("\u{1}\t x \n"), "x");
        assert_eq!(trim("\u{A0}x"), "\u{A0}x");
    }

    #[test]
    fn split_drops_trailing_empty_parts_like_java() {
        assert_eq!(split("a\n\nb\n\n", "\n"), ["a", "", "b"]);
        assert_eq!(split("", "\n"), [""]);
        assert_eq!(split("\n\n", "\n"), Vec::<String>::new());
    }
}
