//! Java library semantics that leak into PlantUML's output and therefore have to be reproduced exactly.

use unicode_general_category::{GeneralCategory, get_general_category};

/// Marks the places where PlantUML throws an unchecked Java exception (index out of bounds, null pointer,
/// number format...). Callers turn it into whatever PlantUML does with such exceptions at that level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeException;

/// `Character.isWhitespace`: Unicode separators except the non-breaking ones, plus ASCII control spaces.
pub(crate) fn is_whitespace(c: char) -> bool {
    match c {
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{1C}'..='\u{1F}' => true,
        '\u{A0}' | '\u{2007}' | '\u{202F}' => false,
        _ => is_space_char(c),
    }
}

/// `Character.isSpaceChar`: any Unicode space, line or paragraph separator.
pub(crate) fn is_space_char(c: char) -> bool {
    matches!(
        get_general_category(c),
        GeneralCategory::SpaceSeparator
            | GeneralCategory::LineSeparator
            | GeneralCategory::ParagraphSeparator
    )
}

/// `Character.isLetter`.
pub(crate) fn is_letter(c: char) -> bool {
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
pub(crate) fn is_letter_or_digit(c: char) -> bool {
    is_letter(c) || get_general_category(c) == GeneralCategory::DecimalNumber
}

/// Whether `c` fits in one Java `char`. Java holds a supplementary character as two surrogate halves, and
/// `Character`'s tests on a `char` take neither half for a letter, digit or space.
pub(crate) fn is_one_char(c: char) -> bool {
    u32::from(c) <= 0xFFFF
}

/// A Java string from its UTF-16 units, as PlantUML writes it out: an unpaired surrogate half becomes `?`.
/// Java measures the half itself, which is wider.
pub(crate) fn string_from_utf16(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .map(|unit| unit.unwrap_or('?'))
        .collect()
}

/// The regex class `\s`, which in Java is ASCII-only.
pub(crate) fn is_regex_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r')
}

/// `String.trim`: strips every character up to and including the space character.
pub(crate) fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// `String.split(regex)` drops trailing empty strings; this is the literal-separator version.
pub(crate) fn split(s: &str, separator: &str) -> Vec<String> {
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
pub(crate) fn regex_split(separator: &regex::Regex, s: &str) -> Vec<String> {
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
pub(crate) fn string_hash_code(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

/// The order in which `java.util.HashMap` iterates `hashes`, given in insertion order with no removals.
/// Entries go by bucket, then by insertion within a bucket; the table starts at 16 buckets and doubles
/// whenever it is more than three-quarters full.
pub(crate) fn hash_map_iteration_order(hashes: &[i32]) -> Vec<usize> {
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
pub(crate) struct JavaHashMap<V> {
    entries: Vec<(String, V)>,
    table: TableSize,
}

impl<V> Default for JavaHashMap<V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            table: TableSize::default(),
        }
    }
}

/// How `java.util.HashMap` sizes its table: created at 16 buckets (or as requested) on the first insertion,
/// doubled whenever it gets more than three-quarters full.
#[derive(Clone, Copy, Debug, Default)]
struct TableSize {
    capacity: usize,
    /// Before the table exists, the requested initial capacity.
    threshold: usize,
}

impl TableSize {
    fn before_insert(&mut self) {
        if self.capacity == 0 {
            self.resize();
        }
    }

    fn after_insert(&mut self, len: usize) {
        if len > self.threshold {
            self.resize();
        }
    }

    /// `putAll` sizes an empty table for the incoming entries up front, unlike a series of `put`s.
    fn prepare_for(&mut self, incoming: usize) {
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
    }

    fn resize(&mut self) {
        self.capacity = match (self.capacity, self.threshold) {
            (0, 0) => 16,
            (0, requested) => requested,
            (capacity, _) => capacity * 2,
        };
        self.threshold = self.capacity * 3 / 4;
    }

    fn iteration_order(self, hashes: &[i32]) -> Vec<usize> {
        bucket_order(hashes, self.capacity.max(1))
    }
}

/// A `java.util.HashSet` reduced to what decides its iteration order, for values with a Java hash code.
#[derive(Clone, Debug, Default)]
pub(crate) struct JavaHashSet<T> {
    entries: Vec<(T, i32)>,
    table: TableSize,
}

impl<T: PartialEq> JavaHashSet<T> {
    pub(crate) fn insert(&mut self, value: T, hash: i32) {
        if self.entries.iter().any(|(existing, _)| *existing == value) {
            return;
        }
        self.table.before_insert();
        self.entries.push((value, hash));
        self.table.after_insert(self.entries.len());
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        let hashes: Vec<i32> = self.entries.iter().map(|&(_, hash)| hash).collect();
        self.table
            .iteration_order(&hashes)
            .into_iter()
            .map(|index| &self.entries[index].0)
    }
}

impl<V> JavaHashMap<V> {
    pub(crate) fn get(&self, key: &str) -> Option<&V> {
        self.entries
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn put(&mut self, key: String, value: V) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|(existing, _)| *existing == key)
        {
            existing.1 = value;
            return;
        }
        self.table.before_insert();
        self.entries.push((key, value));
        self.table.after_insert(self.entries.len());
    }

    pub(crate) fn put_all(&mut self, other: Self) {
        let incoming = other.len();
        if incoming == 0 {
            return;
        }
        self.table.prepare_for(incoming);
        for (key, value) in other.into_iter() {
            self.put(key, value);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.iteration_order()
            .into_iter()
            .map(|index| (self.entries[index].0.as_str(), &self.entries[index].1))
    }

    pub(crate) fn into_iter(mut self) -> impl Iterator<Item = (String, V)> {
        let order = self.iteration_order();
        let mut slots: Vec<Option<(String, V)>> = self.entries.drain(..).map(Some).collect();
        order
            .into_iter()
            .map(move |index| slots[index].take().expect("each entry is visited once"))
    }

    fn iteration_order(&self) -> Vec<usize> {
        let hashes: Vec<i32> = self
            .entries
            .iter()
            .map(|(key, _)| string_hash_code(key))
            .collect();
        self.table.iteration_order(&hashes)
    }
}

/// `Double.toString`: the shortest digits that read back the same, in plain notation from 10^-3 up to 10^7
/// and in Java's own scientific notation (`1.0E-5`) outside it.
pub(crate) fn double_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let magnitude = value.abs();
    if magnitude == 0.0 || (1e-3..1e7).contains(&magnitude) {
        return format!("{value:?}");
    }
    let (significant, exponent) = java_decimal(magnitude);
    let digits: String = significant
        .iter()
        .map(|digit| char::from(b'0' + digit))
        .collect();
    let (integer, fraction) = digits.split_at(1);
    let fraction = if fraction.is_empty() { "0" } else { fraction };
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{sign}{integer}.{fraction}E{exponent}")
}

/// The significant digits and decimal exponent of a finite `magnitude` that `Double.toString` writes, and
/// `String.format` rounds: the shortest that read back the same, except that a single digit gives way to the
/// closest pair of digits (`4.9E-324`, where the shortest is `5E-324`), which only subnormals tell apart.
fn java_decimal(magnitude: f64) -> (Vec<u8>, i32) {
    let shortest = format!("{magnitude:e}");
    let scientific = if shortest.find('e') == Some(1) {
        format!("{magnitude:.1e}")
    } else {
        shortest
    };
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("{:e} always has an exponent");
    let significant = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|digit| digit - b'0')
        .collect();
    (
        significant,
        exponent.parse().expect("{:e} exponents are integers"),
    )
}

/// `String.format(Locale.US, "%.Nf", value)`. Java rounds the shortest decimal representation half-up, so
/// 0.15 becomes "0.2" where rounding the exact binary value would give "0.1".
pub(crate) fn format_fixed(value: f64, decimals: usize) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let (significant, exponent) = java_decimal(value.abs());

    // Digits of the integer part and the first `decimals` fraction digits, as one number scaled by 10^decimals.
    let kept_len = exponent + 1 + decimals as i32;
    let digit_at = |index: i32| -> u8 {
        usize::try_from(index)
            .ok()
            .and_then(|index| significant.get(index).copied())
            .unwrap_or(0)
    };
    let mut kept: Vec<u8> = (0..kept_len.max(0)).map(digit_at).collect();
    if digit_at(kept_len) >= 5 && kept_len >= 0 {
        round_up(&mut kept);
    }

    let integer_len = kept.len().saturating_sub(decimals);
    let (integer, fraction) = kept.split_at(integer_len);
    let mut text = String::new();
    if value.is_sign_negative() {
        text.push('-');
    }
    if integer.is_empty() {
        text.push('0');
    }
    text.extend(integer.iter().map(|digit| char::from(b'0' + digit)));
    if decimals > 0 {
        text.push('.');
        text.extend(std::iter::repeat_n('0', decimals - fraction.len()));
        text.extend(fraction.iter().map(|digit| char::from(b'0' + digit)));
    }
    text
}

/// `String.format(Locale.US, "%.Ne", value)` for a finite value: like [`format_fixed`], the shortest decimal
/// representation rounded half-up, then an exponent of at least two digits (`1.25e-03`).
pub(crate) fn format_scientific(value: f64, decimals: usize) -> String {
    let (significant, mut exponent) = java_decimal(value.abs());
    let mut kept: Vec<u8> = (0..=decimals)
        .map(|index| significant.get(index).copied().unwrap_or(0))
        .collect();
    if significant
        .get(decimals + 1)
        .is_some_and(|&digit| digit >= 5)
    {
        round_up(&mut kept);
        if kept.len() > decimals + 1 {
            kept.pop();
            exponent += 1;
        }
    }
    let digits: String = kept.iter().map(|digit| char::from(b'0' + digit)).collect();
    let (integer, fraction) = digits.split_at(1);
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let point = if decimals > 0 { "." } else { "" };
    let exponent_sign = if exponent < 0 { '-' } else { '+' };
    format!(
        "{sign}{integer}{point}{fraction}e{exponent_sign}{:02}",
        exponent.abs()
    )
}

/// `Double.parseDouble`: surrounding control characters and spaces are ignored, a trailing `f` or `d` is
/// allowed, and so is hexadecimal notation (`0x1.8p1`).
pub(crate) fn parse_double(text: &str) -> Option<f64> {
    let text = trim(text);
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if unsigned == "NaN" || unsigned == "Infinity" {
        return text.replace("Infinity", "inf").parse().ok();
    }
    let number = unsigned
        .strip_suffix(['f', 'F', 'd', 'D'])
        .unwrap_or(unsigned);
    if let Some(hexadecimal) = number
        .strip_prefix("0x")
        .or_else(|| number.strip_prefix("0X"))
    {
        let magnitude = parse_hexadecimal(hexadecimal)?;
        return Some(if text.starts_with('-') {
            -magnitude
        } else {
            magnitude
        });
    }
    let (mantissa, exponent) = match number.find(['e', 'E']) {
        Some(at) => (&number[..at], Some(&number[at + 1..])),
        None => (number, None),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    let valid_mantissa =
        all_digits(integer) && all_digits(fraction) && !(integer.is_empty() && fraction.is_empty());
    let valid_exponent = exponent.is_none_or(|exponent| {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        !digits.is_empty() && all_digits(digits)
    });
    if !(valid_mantissa && valid_exponent) {
        return None;
    }
    let sign = if text.starts_with('-') { "-" } else { "" };
    let exponent = exponent.map(|exponent| format!("e{exponent}"));
    format!(
        "{sign}{integer}.{fraction}0{}",
        exponent.unwrap_or_default()
    )
    .parse()
    .ok()
}

/// `Integer.parseInt`: an optional sign, then decimal digits of any script; `None` where it throws, past
/// `int` too.
pub(crate) fn parse_int(text: &str) -> Option<i32> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() {
        return None;
    }
    let mut magnitude = 0_i64;
    for c in digits.chars() {
        magnitude = magnitude * 10 + i64::from(decimal_digit(c)?);
        if magnitude > -i64::from(i32::MIN) {
            return None;
        }
    }
    i32::try_from(if negative { -magnitude } else { magnitude }).ok()
}

/// `Character.digit(c, 10)` of a UTF-16 unit: the value of a decimal digit of the Basic Multilingual Plane.
fn decimal_digit(c: char) -> Option<u8> {
    let is_digit = |c: char| {
        u32::from(c) <= 0xFFFF && get_general_category(c) == GeneralCategory::DecimalNumber
    };
    if !is_digit(c) {
        return None;
    }
    // Unicode assigns decimal digits in runs from 0 to 9, some runs right after others.
    let digits_up_to_c = (0..=u32::from(c))
        .rev()
        .map_while(|code| char::from_u32(code).filter(|&digit| is_digit(digit)))
        .count();
    Some(u8::try_from((digits_up_to_c - 1) % 10).expect("a digit is below ten"))
}

/// `1.8p1`: hexadecimal digits around an optional point, then a binary exponent, rounded to the nearest
/// double, ties to even.
fn parse_hexadecimal(text: &str) -> Option<f64> {
    // Far beyond the double range already, so that the arithmetic below cannot overflow.
    const EXPONENT_LIMIT: i64 = 1 << 20;
    let (significand, exponent) = text.split_once(['p', 'P'])?;
    let exponent_digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
    if exponent_digits.is_empty() || !exponent_digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let magnitude = exponent_digits.bytes().fold(0_i64, |magnitude, digit| {
        (magnitude * 10 + i64::from(digit - b'0')).min(EXPONENT_LIMIT)
    });
    let exponent = if exponent.starts_with('-') {
        -magnitude
    } else {
        magnitude
    };

    let (integer, fraction) = significand.split_once('.').unwrap_or((significand, ""));
    if integer.is_empty() && fraction.is_empty() {
        return None;
    }
    // A double has 53 significant bits; 15 hexadecimal digits keep 57 or more, and a sticky bit the rest.
    let mut mantissa = 0_u64;
    let mut kept_digits = 0;
    let mut dropped_digits = 0_i64;
    let mut sticky = false;
    for ch in integer.chars().chain(fraction.chars()) {
        let digit = u64::from(ch.to_digit(16)?);
        if kept_digits < 15 {
            mantissa = mantissa * 16 + digit;
            if mantissa > 0 {
                kept_digits += 1;
            }
        } else {
            dropped_digits += 1;
            sticky |= digit != 0;
        }
    }
    let fraction_len = i64::try_from(fraction.len()).ok()?;
    Some(round_to_double(
        mantissa,
        exponent + 4 * (dropped_digits - fraction_len),
        sticky,
    ))
}

/// `mantissa × 2^exponent`, plus a little more if `sticky`, rounded to the nearest double, ties to even.
fn round_to_double(mantissa: u64, exponent: i64, sticky: bool) -> f64 {
    if mantissa == 0 {
        return 0.0;
    }
    let leading_bit = exponent + i64::from(mantissa.ilog2());
    if leading_bit > 1023 {
        return f64::INFINITY;
    }
    // The weight of the last bit the double keeps, never below the smallest subnormal.
    let last_bit = (leading_bit - 52).max(-1074);
    let shift = last_bit - exponent;
    let kept = if shift <= 0 {
        mantissa << -shift
    } else {
        let mantissa = u128::from(mantissa);
        let shift = shift.min(127) as u32;
        let kept = mantissa >> shift;
        let rest = mantissa & ((1 << shift) - 1);
        let half = 1 << (shift - 1);
        let rounds_up = rest > half || rest == half && (sticky || kept & 1 == 1);
        u64::try_from(kept).expect("a double's bits fit") + u64::from(rounds_up)
    };
    // `kept` has at most 54 bits and is exact; the product only overflows into infinity.
    kept as f64 * power_of_two(last_bit)
}

/// `2^exponent` for an exponent a double reaches exactly, subnormals included.
fn power_of_two(exponent: i64) -> f64 {
    if exponent >= -1022 {
        f64::from_bits(((exponent + 1023) as u64) << 52)
    } else {
        f64::from_bits(1 << (exponent + 1074))
    }
}

fn round_up(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == 9 {
            *digit = 0;
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, 1);
}

/// `StringUtils.seed`: a 64-bit hash of the UTF-16 units, used to seed per-text randomness.
pub(crate) fn string_seed(text: &str) -> i64 {
    text.encode_utf16()
        .fold(1_125_899_906_842_597_i64, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i64::from(unit))
        })
}

/// `java.util.Random`'s 48-bit linear congruential generator, for output that PlantUML seeds.
pub(crate) struct Random {
    seed: u64,
}

impl Random {
    const MULTIPLIER: u64 = 0x5_DEEC_E66D;
    const MASK: u64 = (1 << 48) - 1;

    pub(crate) fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(0xB)) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    pub(crate) fn next_double(&mut self) -> f64 {
        let high = i64::from(self.next(26)) << 27;
        let low = i64::from(self.next(27));
        (high + low) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// `None` where Java throws because `bound` is not positive.
    pub(crate) fn next_int(&mut self, bound: i32) -> Option<i32> {
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
        assert_eq!(
            Random::new(string_seed("Hello world")).next_double(),
            0.523_651_822_298_342_1
        );
        assert_eq!(Random::new(-5).next_double(), 0.269_300_957_969_324_85);
        assert_eq!(Random::new(0).next_double(), 0.730_967_787_376_657);
    }

    #[test]
    fn doubles_print_like_java() {
        let cases = [
            (1.0, "1.0"),
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.5, "1.5"),
            (0.001, "0.001"),
            (0.000_1, "1.0E-4"),
            (1.0e7, "1.0E7"),
            (-1.25e-5, "-1.25E-5"),
            (9_999_999.0, "9999999.0"),
            (108.388_888_888_888_89, "108.38888888888889"),
            (1.0e23, "1.0E23"),
            (f64::from_bits(1), "4.9E-324"),
            (f64::from_bits(2), "9.9E-324"),
            (f64::from_bits(100), "4.94E-322"),
        ];
        for (value, expected) in cases {
            assert_eq!(double_to_string(value), expected);
        }
    }

    /// Expectations printed by the JDK's `String.format(Locale.US, ...)`.
    #[test]
    fn fixed_point_formatting_rounds_like_java() {
        let cases = [
            (0.15, 4, "0.1500"),
            (0.15, 1, "0.2"),
            (1.000_05, 4, "1.0001"),
            (2.5, 0, "3"),
            (-0.000_01, 4, "-0.0000"),
            (-0.0, 0, "-0"),
            (0.031_25, 4, "0.0313"),
            (0.031_25, 1, "0.0"),
            (108.388_888_888_888_89, 4, "108.3889"),
            (1e-7, 4, "0.0000"),
            (123_456_789.123_456_79, 4, "123456789.1235"),
            (0.000_05, 4, "0.0001"),
            (0.999_95, 4, "1.0000"),
            (2.675, 1, "2.7"),
        ];
        for (value, decimals, expected) in cases {
            assert_eq!(
                format_fixed(value, decimals),
                expected,
                "{value} with {decimals} decimals"
            );
        }
    }

    /// Expectations printed by the JDK's `String.format(Locale.US, "%.2e", ...)`.
    #[test]
    fn scientific_formatting_rounds_like_java() {
        let cases = [
            (0.005, "5.00e-03"),
            (-0.001_234_5, "-1.23e-03"),
            (0.009_995, "1.00e-02"),
            (0.001_235, "1.24e-03"),
            (1.5e-12, "1.50e-12"),
            (123_456.0, "1.23e+05"),
            (0.0, "0.00e+00"),
            (f64::from_bits(1), "4.90e-324"),
            (f64::from_bits(2), "9.90e-324"),
            (f64::from_bits(3), "1.50e-323"),
            (f64::from_bits(100), "4.94e-322"),
        ];
        for (value, expected) in cases {
            assert_eq!(format_scientific(value, 2), expected, "{value}");
        }
    }

    #[test]
    fn doubles_parse_like_java() {
        let cases = [
            ("1", Some(1.0)),
            (" -2.5 ", Some(-2.5)),
            ("1.", Some(1.0)),
            (".5", Some(0.5)),
            ("+3e2", Some(300.0)),
            ("4d", Some(4.0)),
            ("1E-2f", Some(0.01)),
            ("-Infinity", Some(f64::NEG_INFINITY)),
            (".", None),
            ("", None),
            ("1.2.3", None),
            ("inf", None),
            ("1e", None),
            ("(1", None),
            ("Mar", None),
        ];
        for (text, expected) in cases {
            assert_eq!(parse_double(text), expected, "{text:?}");
        }
        assert!(parse_double("NaN").is_some_and(f64::is_nan));
    }

    /// Expectations printed by the JDK's `Double.parseDouble`.
    #[test]
    fn hexadecimal_doubles_parse_like_java() {
        let cases = [
            ("0x1p4", Some(16.0)),
            ("0X1.8P1", Some(3.0)),
            ("0x.8p1", Some(1.0)),
            ("-0x1p-2d", Some(-0.25)),
            ("0x1P+3f", Some(8.0)),
            (" 0x1p4 ", Some(16.0)),
            ("0x1.p0", Some(1.0)),
            ("+0x10p0", Some(16.0)),
            ("0x1.fffffffffffff8p0", Some(2.0)),
            ("0x1.0000000000000801p0", Some(1.000_000_000_000_000_2)),
            ("0x1.00000000000008p0", Some(1.0)),
            ("0x1.00000000000018p0", Some(1.000_000_000_000_000_4)),
            ("0x123456789abcdef0p0", Some(1.311_768_467_463_790_3e18)),
            ("0x00000000000000000000001p0", Some(1.0)),
            ("0x1.00000000000000000000001p0", Some(1.0)),
            ("0x1p-1074", Some(f64::from_bits(1))),
            ("0x1.8p-1075", Some(f64::from_bits(1))),
            ("0x0.0000000000001p-1022", Some(f64::from_bits(1))),
            ("0x1p-1075", Some(0.0)),
            ("0x1p-99999999999", Some(0.0)),
            ("0x1.fffffffffffff7ffp1023", Some(f64::MAX)),
            ("0x1.fffffffffffff8p1023", Some(f64::INFINITY)),
            ("0x1p99999999999", Some(f64::INFINITY)),
            ("0x1", None),
            ("0xp0", None),
            ("0x.p0", None),
            ("0x1.0p", None),
            ("0x1p+", None),
            ("0x1_0p0", None),
            ("0xGp0", None),
            ("0x1p4x", None),
        ];
        for (text, expected) in cases {
            assert_eq!(parse_double(text), expected, "{text:?}");
        }
        let negative_zero = parse_double("-0x0p0").expect("a number");
        assert!(negative_zero == 0.0 && negative_zero.is_sign_negative());
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
            (
                &[25],
                "24 23 11 10 13 12 15 14 17 16 19 18 2 1 0 6 5 4 3 9 8 7 20 22 21",
            ),
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
            let order: Vec<String> = map
                .iter()
                .map(|(key, ())| key.trim_start_matches("KEY").to_owned())
                .collect();
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

    #[test]
    fn parse_int_reads_decimal_digits_of_any_script_like_java() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("+7"), Some(7));
        assert_eq!(parse_int("-0012"), Some(-12));
        assert_eq!(parse_int("\u{663}"), Some(3));
        assert_eq!(parse_int("\u{FF11}\u{FF10}"), Some(10));
        assert_eq!(parse_int("\u{966}\u{96F}"), Some(9));
        assert_eq!(parse_int("2147483647"), Some(i32::MAX));
        assert_eq!(parse_int("-2147483648"), Some(i32::MIN));
    }

    #[test]
    fn parse_int_rejects_what_java_throws_on() {
        for text in [
            "",
            "+",
            "-",
            " 1",
            "1 ",
            "1.0",
            "abc",
            "\u{BD}",
            "\u{2163}",
            "\u{1D7CE}",
            "2147483648",
            "-2147483649",
            "99999999999999999999",
        ] {
            assert_eq!(parse_int(text), None, "{text:?}");
        }
    }
}
