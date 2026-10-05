//! Classifies each source line as a preprocessor directive, a comment, or plain diagram text.

use std::sync::LazyLock;

use regex::Regex;

use crate::java;
use crate::pattern::plantuml_regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineType {
    Plain,
    AffectationDefine,
    Affectation,
    Assert,
    If,
    Ifdef,
    Undef,
    Ifndef,
    Else,
    Elseif,
    Endif,
    While,
    Endwhile,
    Foreach,
    Endforeach,
    DeclareReturnFunction,
    DeclareProcedure,
    EndFunction,
    Return,
    LegacyDefine,
    LegacyDefinelong,
    Theme,
    Include,
    IncludeDef,
    Import,
    Startsub,
    Endsub,
    Includesub,
    Log,
    DumpMemory,
    CommentSimple,
    CommentLongStart,
    Option,
}

/// Java matches surrogate halves; here a supplementary character is a single `char`.
const IDENTIFIER: &str = r"[\p{L}\x{10000}-\x{10FFFF}_][\p{L}\x{10000}-\x{10FFFF}_0-9]*";

fn keyword(word: &str) -> String {
    format!(r"^[%s]*{word}\b")
}

static PATTERNS: LazyLock<Vec<(LineType, Regex)>> = LazyLock::new(|| {
    let patterns = [
        (
            LineType::LegacyDefine,
            format!(r"^[%s]*!define[%s]+{IDENTIFIER}\("),
        ),
        (
            LineType::LegacyDefinelong,
            format!(r"^[%s]*!definelong[%s]+{IDENTIFIER}\b"),
        ),
        (
            LineType::AffectationDefine,
            format!(r"^[%s]*!define[%s]+{IDENTIFIER}\b"),
        ),
        (
            LineType::Affectation,
            format!(r"^[%s]*![%s]*(local|global)?[%s]*\$?{IDENTIFIER}[%s]*\??="),
        ),
        (LineType::Ifdef, keyword("!ifdef")),
        (LineType::Undef, keyword("!undef")),
        (LineType::Ifndef, keyword("!ifndef")),
        (LineType::Assert, keyword("!assert")),
        (LineType::If, keyword("!if")),
        (
            LineType::DeclareReturnFunction,
            format!(r"^[%s]*!(unquoted\s|final\s)*function[%s]+\$?{IDENTIFIER}"),
        ),
        (
            LineType::DeclareProcedure,
            format!(r"^[%s]*!(unquoted\s|final\s)*procedure[%s]+\$?{IDENTIFIER}"),
        ),
        (LineType::Else, keyword("!else")),
        (LineType::Elseif, keyword("!elseif")),
        (LineType::Endif, keyword("!endif")),
        (LineType::While, keyword("!while")),
        (LineType::Endwhile, keyword("!endwhile")),
        (LineType::Foreach, keyword("!foreach")),
        (LineType::Endforeach, keyword("!endfor")),
        (
            LineType::EndFunction,
            r"^[%s]*!end[%s]*(?:function|definelong|procedure)\b".to_owned(),
        ),
        (LineType::Return, keyword("!return")),
        (LineType::Theme, keyword("!theme")),
        (
            LineType::Include,
            r"^[%s]*!include[%s]*(?:(?:url|_many|_once))?\b".to_owned(),
        ),
        (LineType::IncludeDef, keyword("!includedef")),
        (LineType::Import, keyword("!import")),
        (LineType::Startsub, keyword("!startsub")),
        (LineType::Endsub, keyword("!endsub")),
        (LineType::Includesub, keyword("!includesub")),
        (LineType::Log, keyword("!log")),
        (LineType::DumpMemory, keyword("!dump_memory")),
        (LineType::Option, keyword("!option")),
    ];
    patterns
        .into_iter()
        .map(|(line_type, pattern)| (line_type, plantuml_regex(&pattern)))
        .collect()
});

static COMMENT_SIMPLE: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^[%s]*'"));
static COMMENT_ONE_LINE_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| plantuml_regex("^[%s]*/'.*'/[%s]*$"));
static COMMENT_LONG_START: LazyLock<Regex> = LazyLock::new(|| plantuml_regex("^[%s]*/'"));

pub fn line_type(line: &str) -> LineType {
    if COMMENT_SIMPLE.is_match(line) || COMMENT_ONE_LINE_BLOCK.is_match(line) {
        return LineType::CommentSimple;
    }
    if COMMENT_LONG_START.is_match(line) && !line.contains("'/") {
        return LineType::CommentLongStart;
    }
    if !line.contains('!') {
        return LineType::Plain;
    }
    PATTERNS
        .iter()
        .find(|(_, pattern)| pattern.is_match(line))
        .map_or(LineType::Plain, |(line_type, _)| *line_type)
}

pub fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}

pub fn is_letter_or_emoji_or_underscore_or_digit(c: char) -> bool {
    is_letter_or_emoji_or_underscore(c) || c.is_ascii_digit()
}

pub fn is_letter_or_emoji_or_underscore_or_dollar(c: char) -> bool {
    is_letter_or_emoji_or_underscore(c) || c == '$'
}

/// Java sees emoji as surrogate halves, i.e. any supplementary character.
fn is_letter_or_emoji_or_underscore(c: char) -> bool {
    java::is_letter(c) || c == '_' || u32::from(c) > 0xFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_recognised_first() {
        assert_eq!(line_type("  ' !if"), LineType::CommentSimple);
        assert_eq!(line_type("/' one line '/"), LineType::CommentSimple);
        assert_eq!(line_type("/' starts a block"), LineType::CommentLongStart);
    }

    #[test]
    fn lines_without_exclamation_mark_are_plain() {
        assert_eq!(line_type("Alice -> Bob"), LineType::Plain);
    }

    #[test]
    fn define_variants_are_told_apart() {
        assert_eq!(line_type("!define F(x) x"), LineType::LegacyDefine);
        assert_eq!(line_type("!define V value"), LineType::AffectationDefine);
        assert_eq!(line_type("!definelong L"), LineType::LegacyDefinelong);
    }

    #[test]
    fn keywords_are_matched_case_insensitively_and_in_order() {
        assert_eq!(line_type("!IF 1"), LineType::If);
        assert_eq!(line_type("!ifdef X"), LineType::Ifdef);
        assert_eq!(line_type("!elseif 1"), LineType::Elseif);
        assert_eq!(line_type("!endfor"), LineType::Endforeach);
        assert_eq!(line_type("!endfunction"), LineType::EndFunction);
        assert_eq!(line_type("!include_once x"), LineType::Include);
        assert_eq!(line_type("!includesub f!X"), LineType::Includesub);
    }

    #[test]
    fn affectations_allow_scope_and_conditional_assignment() {
        assert_eq!(line_type("!$x = 1"), LineType::Affectation);
        assert_eq!(line_type("!local $x ?= 1"), LineType::Affectation);
        assert_eq!(line_type("! global y=1"), LineType::Affectation);
    }

    #[test]
    fn function_declarations_accept_modifiers() {
        assert_eq!(
            line_type("!unquoted function $f()"),
            LineType::DeclareReturnFunction
        );
        assert_eq!(
            line_type("!final procedure P()"),
            LineType::DeclareProcedure
        );
    }
}
