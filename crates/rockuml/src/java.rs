//! Java library semantics that leak into PlantUML's output and therefore have to be reproduced exactly.

use unicode_general_category::{GeneralCategory, get_general_category};

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

#[cfg(test)]
mod tests {
    use super::*;

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
