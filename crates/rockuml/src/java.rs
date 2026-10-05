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
    bucket_order(hashes, capacity)
}

/// Visits entries bucket by bucket; within a bucket in insertion order, which resizing preserves.
fn bucket_order(hashes: &[i32], capacity: usize) -> Vec<usize> {
    let bucket = |hash: i32| {
        let spread = (hash ^ (hash >> 16 & 0xFFFF)) as u32;
        spread as usize & (capacity - 1)
    };
    let mut order: Vec<usize> = (0..hashes.len()).collect();
    order.sort_by_key(|&index| bucket(hashes[index]));
    order
}

/// A `java.util.HashMap<String, V>` reduced to what decides its iteration order: the table capacity, the
/// key hashes and the insertion order within a bucket. Removals are not supported.
#[derive(Clone, Debug)]
pub struct JavaHashMap<V> {
    entries: Vec<(String, V)>,
    capacity: usize,
    threshold: usize,
}

impl<V> Default for JavaHashMap<V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            capacity: 0,
            threshold: 0,
        }
    }
}

impl<V> JavaHashMap<V> {
    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries.iter().find(|(existing, _)| existing == key).map(|(_, value)| value)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn put(&mut self, key: String, value: V) {
        if let Some(existing) = self.entries.iter_mut().find(|(existing, _)| *existing == key) {
            existing.1 = value;
            return;
        }
        if self.capacity == 0 {
            self.resize();
        }
        self.entries.push((key, value));
        if self.entries.len() > self.threshold {
            self.resize();
        }
    }

    /// `putAll` sizes an empty table for the incoming map up front, unlike a series of `put`s.
    pub fn put_all(&mut self, other: Self) {
        let incoming = other.len();
        if incoming == 0 {
            return;
        }
        if self.capacity == 0 {
            let wanted = (incoming * 4).div_ceil(3);
            if wanted > self.threshold {
                self.threshold = wanted.next_power_of_two();
            }
        } else {
            while incoming > self.threshold {
                self.resize();
            }
        }
        for (key, value) in other.into_iter() {
            self.put(key, value);
        }
    }

    /// When the table does not exist yet, `threshold` holds the requested initial capacity.
    fn resize(&mut self) {
        if self.capacity == 0 {
            self.capacity = if self.threshold > 0 { self.threshold } else { 16 };
        } else {
            self.capacity *= 2;
        }
        self.threshold = self.capacity * 3 / 4;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.iteration_order().into_iter().map(|index| (self.entries[index].0.as_str(), &self.entries[index].1))
    }

    pub fn into_iter(mut self) -> impl Iterator<Item = (String, V)> {
        let order = self.iteration_order();
        let mut slots: Vec<Option<(String, V)>> = self.entries.drain(..).map(Some).collect();
        order.into_iter().map(move |index| slots[index].take().expect("each entry is visited once"))
    }

    fn iteration_order(&self) -> Vec<usize> {
        let hashes: Vec<i32> = self.entries.iter().map(|(key, _)| string_hash_code(key)).collect();
        bucket_order(&hashes, self.capacity.max(1))
    }
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
        if bound.cast_unsigned().is_power_of_two() {
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
        assert_eq!(Random::new(7).next_int(16), Some(11));
        assert_eq!(Random::new(-3).next_int(1000), Some(164));
        assert_eq!(Random::new(1).next_int(0), None);
    }

    /// Orders observed on the JDK for maps built like PlantUML's regex results: putAll of child maps.
    #[test]
    fn java_hash_map_iterates_like_the_jdk() {
        let cases: [(&[usize], &str); 7] = [
            (&[3, 4], "2 1 0 6 5 4 3"),
            (&[12], "2 1 0 6 5 4 3 9 8 7 11 10"),
            (&[1, 11], "2 1 0 6 5 4 3 9 8 7 11 10"),
            (&[6, 6, 1], "12 2 1 0 6 5 4 3 9 8 7 11 10"),
            (&[13], "12 2 1 0 6 5 4 3 9 8 7 11 10"),
            (&[2, 2, 2, 2, 2, 2, 2], "13 12 2 1 0 6 5 4 3 9 8 7 11 10"),
            (&[25], "24 23 11 10 13 12 15 14 17 16 19 18 2 1 0 6 5 4 3 9 8 7 20 22 21"),
        ];
        for (child_sizes, expected) in cases {
            let mut map = JavaHashMap::default();
            let mut next = 0;
            for &size in child_sizes {
                let mut child = JavaHashMap::default();
                for _ in 0..size {
                    child.put(format!("KEY{next}"), ());
                    next += 1;
                }
                map.put_all(child);
            }
            let order: Vec<String> = map.iter().map(|(key, ())| key.trim_start_matches("KEY").to_owned()).collect();
            assert_eq!(order.join(" "), expected, "{child_sizes:?}");
        }
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
