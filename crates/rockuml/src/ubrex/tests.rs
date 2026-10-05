//! Every expectation here was observed on PlantUML 1.2026.8's `com.plantuml.ubrex`, running on JDK 21.

use super::{UMatcher, UnicodeBracketedExpression};

const NONE: [&str; 0] = [];

/// The accepted match, or `None` where Java's `startMatch()` is false.
fn accepted<'a>(pattern: &str, text: &'a str, position: usize) -> Option<&'a str> {
    UnicodeBracketedExpression::build(pattern)
        .match_at(text, position)
        .as_ref()
        .map(UMatcher::accepted_match)
}

/// The values captured under `key`, none where nothing matched.
fn found<'a>(matcher: Option<&UMatcher<'a>>, key: &str) -> Vec<&'a str> {
    matcher.map_or_else(Vec::new, |matcher| matcher.find_values_by_key(key))
}

fn values<'a>(pattern: &str, text: &'a str, key: &str) -> Vec<&'a str> {
    found(
        UnicodeBracketedExpression::build(pattern)
            .match_at(text, 0)
            .as_ref(),
        key,
    )
}

/// The match at `position`, which must exist.
fn matched<'a>(pattern: &str, text: &'a str, position: usize) -> UMatcher<'a> {
    UnicodeBracketedExpression::build(pattern)
        .match_at(text, position)
        .expect("the pattern matches")
}

fn assert_cases(pattern: &str, cases: &[(&str, Option<&str>)]) {
    for &(text, expected) in cases {
        assert_eq!(
            accepted(pattern, text, 0),
            expected,
            "{pattern} on {text:?}"
        );
    }
}

mod creole {
    //! The patterns `CommandCreoleStyle` and `CommandCreoleExposantChange` build from `FontStyle`.

    use super::*;

    const UNDERLINE_ACTIVATION: &str = "<「uU」〇?〘:〶$XC=【#〇{6}「0〜9a〜fA〜F」┇〇+〴w】〙>";
    const BACKCOLOR_ACTIVATION: &str = "<「bB」「aA」「cC」「kK」〇?〘:〶$XC=〘\
        【#〇{6}「0〜9a〜fA〜F」┇〇+〴w 】 \
        〇?〘「-\\|/」【〇{6}「0〜9a〜fA〜F」┇〇+〴w】 〙\
        〙 〙>";
    const STRIKE_ACTIVATION: &str =
        "<【strike┇STRIKE┇s┇S┇del┇DEL】〇?〘:〶$XC=【#〇{6}「0〜9a〜fA〜F」┇〇+〴w】〙>";
    const STRIKE_DEACTIVATION: &str = "</【strike┇STRIKE┇s┇S┇del┇DEL】>";

    fn legacy(activation: &str, deactivation: &str) -> String {
        format!("{activation}〶$V=〄>〘{deactivation}〙")
    }

    fn creole(syntax: &str) -> String {
        format!("{syntax}〶$V=〄+〴.->〘{syntax}〙")
    }

    /// (text, accepted match, `V`, `XC`) at position 0.
    type StyleCase<'a> = (&'a str, Option<&'a str>, &'a [&'a str], &'a [&'a str]);

    fn assert_style(pattern: &str, cases: &[StyleCase]) {
        let ubrex = UnicodeBracketedExpression::build(pattern);
        for &(text, accepted, value, extended_color) in cases {
            let matcher = ubrex.match_at(text, 0);
            let matcher = matcher.as_ref();
            assert_eq!(matcher.map(UMatcher::accepted_match), accepted, "{text:?}");
            assert_eq!(found(matcher, "V"), value, "{text:?}");
            assert_eq!(found(matcher, "XC"), extended_color, "{text:?}");
        }
    }

    #[test]
    fn legacy_bold_captures_up_to_the_closing_tag() {
        assert_style(
            &legacy("<「bB」>", "</「bB」>"),
            &[
                ("<b>bold</b> rest", Some("<b>bold</b>"), &["bold"], &[]),
                ("<b>bold", None, &[], &[]),
                ("<b></b>", Some("<b></b>"), &[""], &[]),
            ],
        );
    }

    #[test]
    fn legacy_bold_matches_from_a_position() {
        let matcher = matched(&legacy("<「bB」>", "</「bB」>"), "xx<B>y</b>", 2);
        assert_eq!(matcher.accepted_match(), "<B>y</b>");
        assert_eq!(matcher.find_values_by_key("V"), ["y"]);
    }

    #[test]
    fn creole_bold_needs_one_character_before_the_closing_stars() {
        assert_style(
            &creole("**"),
            &[
                ("**bold** x", Some("**bold**"), &["bold"], &[]),
                ("****", None, &[], &[]),
                ("*****", Some("*****"), &["*"], &[]),
                ("**a**b**", Some("**a**"), &["a"], &[]),
            ],
        );
    }

    #[test]
    fn underline_takes_an_optional_extended_color() {
        assert_style(
            &legacy(UNDERLINE_ACTIVATION, "</「uU」>"),
            &[
                (
                    "<u:#FF00aa>text</u>",
                    Some("<u:#FF00aa>text</u>"),
                    &["text"],
                    &["#FF00aa"],
                ),
                ("<u:red>t</U>", Some("<u:red>t</U>"), &["t"], &["red"]),
                ("<u:#FF00aa1>t</u>", None, &[], &[]),
                ("<u>t</u>", Some("<u>t</u>"), &["t"], &[]),
            ],
        );
    }

    #[test]
    fn backcolor_takes_an_optional_gradient() {
        assert_style(
            &legacy(BACKCOLOR_ACTIVATION, "</「bB」「aA」「cC」「kK」>"),
            &[
                (
                    "<back:red/blue>x</back>",
                    Some("<back:red/blue>x</back>"),
                    &["x"],
                    &["red/blue"],
                ),
                (
                    "<BACK:#ffffff|000000>hi</back>",
                    Some("<BACK:#ffffff|000000>hi</back>"),
                    &["hi"],
                    &["#ffffff|000000"],
                ),
                (
                    "<back:#ffffff\\green>hi</back>",
                    Some("<back:#ffffff\\green>hi</back>"),
                    &["hi"],
                    &["#ffffff\\green"],
                ),
            ],
        );
    }

    #[test]
    fn strike_tags_are_tried_in_order() {
        assert_style(
            &legacy(STRIKE_ACTIVATION, STRIKE_DEACTIVATION),
            &[
                ("<s>x</s>", Some("<s>x</s>"), &["x"], &[]),
                (
                    "<del:blue>x</del>",
                    Some("<del:blue>x</del>"),
                    &["x"],
                    &["blue"],
                ),
                (
                    "<sTrike>x</strike>",
                    Some("<sTrike>x</strike>"),
                    &["x"],
                    &[],
                ),
                ("<strike>x</s>", Some("<strike>x</s>"), &["x"], &[]),
            ],
        );
    }

    #[test]
    fn legacy_end_of_line_style_takes_the_rest_of_the_line() {
        assert_style(
            "<「bB」>〶$V=〇+〴.",
            &[
                (
                    "<b>rest of line",
                    Some("<b>rest of line"),
                    &["rest of line"],
                    &[],
                ),
                ("<b>", None, &[], &[]),
            ],
        );
    }

    #[test]
    fn exposant_change() {
        let matcher = matched("<sub>〶$V=〄>〘</sub>〙", "H<sub>2</sub>O", 1);
        assert_eq!(matcher.accepted_match(), "<sub>2</sub>");
        assert_eq!(matcher.find_values_by_key("V"), ["2"]);
    }
}

mod matcher {
    use super::*;

    #[test]
    fn a_failed_match_gives_no_matcher() {
        let ubrex = UnicodeBracketedExpression::build("〶$V=〄>z");
        assert!(ubrex.match_at("abc", 0).is_none());
    }

    #[test]
    fn a_match_may_leave_text_after_it() {
        assert_eq!(accepted("a", "ab", 0), Some("a"));
        assert_eq!(accepted("b", "ab", 1), Some("b"));
    }

    #[test]
    fn empty_matches_at_the_end_of_the_text() {
        for pattern in ["〒$", "〇*〴s"] {
            assert_eq!(accepted(pattern, "ab", 2), Some(""), "{pattern}");
        }
    }
}

mod repetition {
    use super::*;

    #[test]
    fn repetitions_are_greedy_and_never_backtrack() {
        assert_eq!(accepted("〇*〴.x", "abx", 0), None);
        assert_cases(
            "〇{2}a",
            &[("aaa", None), ("aa", Some("aa")), ("aab", Some("aa"))],
        );
    }

    #[test]
    fn repetition_counts_combine_ranges_and_minimums() {
        assert_cases(
            "〇{2-3;5+}a",
            &[
                ("a", None),
                ("aa", Some("aa")),
                ("aaa", Some("aaa")),
                ("aaaa", None),
                ("aaaaa", Some("aaaaa")),
                ("aaaaaaa", Some("aaaaaaa")),
            ],
        );
        assert_cases("〇{3+}a", &[("aaa", Some("aaa")), ("aa", None)]);
        assert_cases("〇{1;3}a", &[("aaa", Some("aaa"))]);
        assert_cases("〇{+2}a", &[("aa", Some("aa"))]);
    }

    #[test]
    fn a_repetition_never_accepts_zero_occurrences() {
        assert_eq!(accepted("〇{0}a", "b", 0), None);
        assert_eq!(accepted("〇{-2+}a", "b", 0), None);
        assert_eq!(accepted("〇{5--3}a", "a", 0), None);
    }

    #[test]
    fn quantifiers_on_empty_text() {
        assert_eq!(accepted("〇*a", "", 0), Some(""));
        assert_eq!(accepted("〇+a", "", 0), None);
        assert_eq!(accepted("〇?a", "", 0), Some(""));
    }

    #[test]
    fn repeated_named_captures_accumulate() {
        assert_eq!(values("〇*〘〶$D=〴d,〙", "1,2,3,", "D"), ["1", "2", "3"]);
    }

    #[test]
    #[should_panic(expected = "infinite loop")]
    fn zero_or_more_of_an_empty_match_is_an_infinite_loop() {
        accepted("〇*〘〇?a〙", "aab", 0);
    }

    #[test]
    #[should_panic(expected = "infinite loop")]
    fn one_or_more_of_an_empty_match_is_an_infinite_loop() {
        accepted("〇+〘〇?a〙", "b", 0);
    }

    #[test]
    #[should_panic(expected = "infinite loop")]
    fn counted_repetition_of_an_empty_match_is_an_infinite_loop() {
        accepted("〇{2}〘〇?a〙", "b", 0);
    }

    #[test]
    #[should_panic(expected = "infinite loop")]
    fn up_to_of_an_empty_match_is_an_infinite_loop() {
        accepted("〄+〘〇?a〙->x", "b", 0);
    }
}

mod lazy {
    use super::*;

    const IF: &str = "if 〇+〴s 〶$IF2=〇l+〴. 〘then〙 ";

    #[test]
    fn lazy_repetition_stops_before_the_rest_of_the_pattern() {
        let ubrex = UnicodeBracketedExpression::build(IF);
        for (text, accepted, if2) in [
            ("if a then b then", "if a then", "a "),
            ("if a then", "if a then", "a "),
            ("if  then then", "if  then then", "then "),
        ] {
            let matcher = ubrex.match_at(text, 0).unwrap();
            assert_eq!(matcher.accepted_match(), accepted);
            assert_eq!(matcher.find_values_by_key("IF2"), [if2]);
        }
        assert!(ubrex.match_at("if then", 0).is_none());
    }

    #[test]
    fn the_rest_of_the_pattern_ends_at_the_enclosing_group() {
        assert_eq!(accepted("〇l+〴.", "abc", 0), Some("a"));
        assert_eq!(accepted("〘〇l+〴.x〙y", "aaxy", 0), Some("aaxy"));
        assert_eq!(accepted("〇l+〴.x", "aaa", 0), None);
        assert_eq!(values("〶$V=〇l+〴.", "abc", "V"), ["a"]);
        assert_eq!(values("〶$V=〇l+〴.c", "abc", "V"), ["ab"]);
        assert_eq!(accepted("〘〶$V=〇l+〴.〙c", "abc", 0), None);
    }

    #[test]
    #[should_panic(expected = "infinite loop")]
    fn lazy_repetition_of_an_empty_match_is_an_infinite_loop() {
        accepted("〇l+〘〇?a〙x", "b", 0);
    }
}

mod named {
    use super::*;

    #[test]
    fn nested_names_are_prefixed_and_listed_first() {
        let matcher = matched("〶$A=〘x〶$B=〇+〴d〙", "x12", 0);
        assert_eq!(matcher.find_values_by_key("A"), ["x12"]);
        assert_eq!(matcher.find_values_by_key("B"), NONE);
        assert_eq!(matcher.find_values_by_key("A/B"), ["12"]);
    }

    #[test]
    fn values_are_found_by_their_whole_key() {
        assert_eq!(values("〶$A1=a〶$A2=b", "ab", "A"), NONE);
    }

    #[test]
    fn a_name_directly_inside_a_name() {
        let matcher = matched("〶$X_y=〶$Z=a", "a", 0);
        assert_eq!(matcher.find_values_by_key("X_y"), ["a"]);
        assert_eq!(matcher.find_values_by_key("X_y/Z"), ["a"]);
        assert_eq!(matcher.find_values_by_key("Z"), NONE);
    }

    #[test]
    fn a_skipped_optional_captures_nothing() {
        assert_eq!(values("〇?〘〶$A=a〙b", "b", "A"), NONE);
    }

    #[test]
    fn up_to_leaves_its_stop_pattern_out_of_the_name() {
        for pattern in ["〶$V=〄+〴.->〘;〙", "〶$V=〄+ 〴. -> ;"] {
            let matcher = matched(pattern, "abc;d", 0);
            assert_eq!(matcher.accepted_match(), "abc;");
            assert_eq!(matcher.find_values_by_key("V"), ["abc"]);
        }
        let matcher = matched("〶$V=〄>〒$", "abc", 0);
        assert_eq!(matcher.accepted_match(), "abc");
        assert_eq!(matcher.find_values_by_key("V"), ["abc"]);
        let matcher = matched("〶$V=〄>a", "abc", 0);
        assert_eq!(matcher.accepted_match(), "a");
        assert_eq!(matcher.find_values_by_key("V"), [""]);
    }
}

mod alternative {
    use super::*;

    #[test]
    fn the_first_matching_alternative_wins() {
        assert_eq!(accepted("【a┇ab】", "ab", 0), Some("a"));
        assert_eq!(
            accepted("【hide-class┇hide┇show-class┇show】", "hide-class", 0),
            Some("hide-class")
        );
    }

    #[test]
    fn a_blank_alternative_matches_empty() {
        assert_eq!(accepted("【 ┇a】b", "b", 0), Some("b"));
    }

    #[test]
    fn alternatives_nest() {
        assert_eq!(accepted("【a【b┇c】┇d】", "ac", 0), Some("ac"));
    }

    #[test]
    #[should_panic(expected = "〘")]
    fn a_separator_inside_a_group_still_splits_the_alternative() {
        UnicodeBracketedExpression::build("【〘a┇b〙┇c】");
    }

    #[test]
    #[should_panic(expected = "「")]
    fn a_separator_inside_a_char_set_still_splits_the_alternative() {
        UnicodeBracketedExpression::build("【「a┇」┇b】");
    }
}

mod look_around {
    use super::*;

    #[test]
    fn look_ahead() {
        assert_cases("a〒=b", &[("ab", Some("a")), ("ac", None)]);
        assert_cases("a〒!b", &[("ac", Some("a")), ("ab", None)]);
    }

    #[test]
    fn look_behind_reads_its_pattern_backwards() {
        assert_cases(
            "〴.〴.〒<=〘ab〙〴.",
            &[("abc", None), ("bac", Some("bac"))],
        );
        assert_eq!(accepted("〒<=a", "ba", 1), None);
        assert_cases("〴.〒<!x〴.", &[("xy", None), ("zy", Some("zy"))]);
        assert_eq!(accepted("〒<!a", "a", 0), Some(""));
    }

    #[test]
    fn look_around_quantifies_a_single_atom() {
        assert_eq!(accepted("〴.〴.〒<=ab〴.", "bac", 0), None);
    }

    #[test]
    fn end_of_text_looked_behind_is_the_start_of_the_text() {
        assert_eq!(accepted("〒<=〒$a", "a", 0), Some("a"));
        assert_eq!(accepted("〒<=〒$a", "ba", 1), None);
    }

    #[test]
    fn end_of_text() {
        assert_cases("a〒$", &[("a", Some("a")), ("ab", None)]);
    }

    #[test]
    fn look_arounds_drop_their_captures() {
        assert_eq!(values("〴.〒<=〘〶$X=〴.〙", "ab", "X"), NONE);
        assert_eq!(values("a〒=〘〶$X=b〙", "ab", "X"), NONE);
    }

    #[test]
    #[should_panic(expected = "look-behind")]
    fn look_behind_cannot_nest() {
        accepted("〒<=〘〒<=a〙", "ab", 1);
    }
}

mod char_class {
    use super::*;

    #[test]
    fn spaces() {
        assert_eq!(accepted("〴s", " ", 0), Some(" "));
        assert_eq!(accepted("〴s", "\t", 0), None);
        assert_eq!(accepted("〴S", "\t", 0), Some("\t"));
        assert_eq!(accepted("〴S", " ", 0), None);
    }

    #[test]
    fn digits_and_words_are_ascii_and_include_the_underscore() {
        assert_eq!(accepted("〴d", "_", 0), Some("_"));
        assert_eq!(accepted("〴w", "_", 0), Some("_"));
        assert_eq!(accepted("〴w", "é", 0), None);
        assert_eq!(accepted("〴W", "é", 0), Some("é"));
        assert_eq!(accepted("〴Dx", "ax", 0), Some("ax"));
    }

    #[test]
    fn guillemets() {
        assert_eq!(accepted("〴g〴g〴g", "\"“”", 0), Some("\"“”"));
        assert_eq!(accepted("〴G", "“", 0), None);
    }

    #[test]
    fn letters_and_alphanumerics_are_unicode() {
        assert_eq!(accepted("〴an", "é", 0), Some("é"));
        assert_eq!(accepted("〴an", "\u{663}", 0), Some("\u{663}"));
        assert_eq!(accepted("〴le", "é", 0), Some("é"));
        assert_eq!(accepted("〴le", "1", 0), None);
        assert_eq!(accepted("〴L", "1", 0), Some("1"));
    }

    #[test]
    fn two_letter_classes_skip_their_second_letter_unread() {
        assert_eq!(accepted("〴ax!", "1!", 0), Some("1!"));
        assert_eq!(accepted("〇+〴a", "ab1-", 0), Some("ab1"));
        assert_eq!(accepted("〇+〴l", "ab1-", 0), Some("ab"));
    }

    #[test]
    fn any_includes_line_breaks() {
        assert_eq!(accepted("〴.", "\n", 0), Some("\n"));
    }
}

mod char_set {
    use super::*;

    const HEX: &str = "「0〜9a〜fA〜F」";

    #[test]
    fn ranges_ignore_ascii_case() {
        assert_cases(HEX, &[("F", Some("F")), ("g", None), ("G", None)]);
        assert_eq!(accepted("「a」", "A", 0), Some("A"));
        assert_eq!(accepted("「A」", "a", 0), Some("a"));
    }

    #[test]
    fn negation_applies_to_the_whole_set_wherever_it_is_written() {
        assert_cases("「〤abc」", &[("d", Some("d")), ("A", None)]);
        assert_eq!(accepted("「ab〤c」", "c", 0), None);
    }

    #[test]
    fn escapes_and_spaces() {
        assert_eq!(accepted("「〃」", "\"", 0), Some("\""));
        assert_eq!(accepted("「∙」", " ", 0), Some(" "));
        assert_eq!(accepted("「 a」", " ", 0), None);
        let set = "「-\\|/」";
        assert_eq!(accepted(&set.repeat(4), "-\\|/", 0), Some("-\\|/"));
    }

    #[test]
    fn classes_inside_a_set_are_never_negated() {
        assert_cases("「〴S」", &[(" ", Some(" ")), ("\t", None)]);
        assert_cases("「〴an_.」", &[("é", Some("é")), ("-", None)]);
        assert_eq!(accepted("「〴a」", "1", 0), Some("1"));
        assert_eq!(accepted("「〴le」", "é", 0), Some("é"));
        assert_eq!(accepted("「〴anx」", "x", 0), Some("x"));
        assert_eq!(accepted("〇+「〴a」", "ab1-", 0), Some("ab1"));
    }

    #[test]
    fn characters_outside_the_bitmask_alias_characters_inside() {
        // Java masks the shift distance, so control characters and U+00A0 land on printable bits.
        assert_cases(
            "「!〜~」",
            &[
                ("\t", Some("\t")),
                ("\u{A0}", Some("\u{A0}")),
                ("\u{A1}", None),
            ],
        );
        assert_eq!(accepted("「`」", "\u{A0}", 0), Some("\u{A0}"));
        assert_cases(
            "「\u{80}」",
            &[("\u{80}", Some("\u{80}")), ("\u{C0}", None)],
        );
        assert_cases("「`〜\u{80}」", &[("\u{A0}", Some("\u{A0}")), ("@", None)]);
    }

    #[test]
    fn a_range_reversed_by_lowercasing_sets_unrelated_bits() {
        assert_cases("「Z〜a」", &[("a", None), ("{", Some("{")), ("\"", None)]);
    }

    #[test]
    #[should_panic(expected = "〜")]
    fn a_range_needs_an_end() {
        UnicodeBracketedExpression::build("「a〜」");
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn a_set_cannot_be_empty() {
        UnicodeBracketedExpression::build("「」");
    }

    #[test]
    #[should_panic(expected = "bad char")]
    fn a_set_only_holds_ascii() {
        UnicodeBracketedExpression::build("「é」");
    }

    #[test]
    #[should_panic(expected = "invalid range")]
    fn a_range_must_be_ordered() {
        UnicodeBracketedExpression::build("「z〜a」");
    }

    #[test]
    #[should_panic(expected = "range")]
    fn a_range_must_stay_in_ascii() {
        UnicodeBracketedExpression::build("「a〜é」");
    }
}

mod single_char {
    use super::*;

    #[test]
    fn letters_ignore_ascii_case_only() {
        assert_eq!(accepted("ABC", "abc", 0), Some("abc"));
        assert_eq!(accepted("abc", "ABC", 0), Some("ABC"));
        assert_eq!(accepted("É", "é", 0), None);
    }

    #[test]
    fn spaces_are_layout_and_escapes_are_literal() {
        assert_eq!(accepted("a b", "ab", 0), Some("ab"));
        assert_eq!(
            accepted("left∙to∙right", "left to right", 0),
            Some("left to right")
        );
        assert_eq!(accepted("〃x〃", "\"x\"", 0), Some("\"x\""));
    }

    #[test]
    #[should_panic(expected = "reserved")]
    fn reserved_characters_cannot_be_literals() {
        UnicodeBracketedExpression::build("」");
    }
}

mod syntax_errors {
    use super::*;

    #[test]
    #[should_panic(expected = "no space allowed")]
    fn a_quantified_atom_cannot_be_a_space() {
        UnicodeBracketedExpression::build("〇+ a");
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn empty_pattern() {
        UnicodeBracketedExpression::build("");
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn empty_group() {
        UnicodeBracketedExpression::build("〘〙");
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn empty_alternative() {
        UnicodeBracketedExpression::build("【a┇】");
    }

    #[test]
    #[should_panic(expected = "no name")]
    fn missing_name() {
        UnicodeBracketedExpression::build("〶$=a");
    }

    #[test]
    #[should_panic(expected = "$")]
    fn name_without_dollar() {
        UnicodeBracketedExpression::build("〶X=a");
    }

    #[test]
    #[should_panic(expected = "unsupported name")]
    fn name_with_a_dash() {
        UnicodeBracketedExpression::build("〶$A-B=a");
    }

    #[test]
    #[should_panic(expected = "quantifier")]
    fn unknown_quantifier() {
        UnicodeBracketedExpression::build("〇!a");
    }

    #[test]
    #[should_panic(expected = "〄")]
    fn unknown_up_to() {
        UnicodeBracketedExpression::build("〄a");
    }

    #[test]
    #[should_panic(expected = "->")]
    fn up_to_without_arrow() {
        UnicodeBracketedExpression::build("〄+a b");
    }

    #[test]
    #[should_panic(expected = "〒")]
    fn unknown_look_around() {
        UnicodeBracketedExpression::build("〒x");
    }

    #[test]
    #[should_panic(expected = "〘")]
    fn unclosed_group() {
        UnicodeBracketedExpression::build("〘a");
    }

    #[test]
    #[should_panic(expected = "【")]
    fn unclosed_alternative() {
        UnicodeBracketedExpression::build("【a");
    }

    #[test]
    #[should_panic(expected = "「")]
    fn unclosed_set() {
        UnicodeBracketedExpression::build("「a");
    }

    #[test]
    #[should_panic(expected = "┇")]
    fn separator_outside_an_alternative() {
        UnicodeBracketedExpression::build("┇");
    }

    #[test]
    #[should_panic(expected = "single atom")]
    fn quantified_lazy_repetition() {
        UnicodeBracketedExpression::build("〇+〇l+ab");
    }

    #[test]
    #[should_panic(expected = "class")]
    fn unknown_class() {
        UnicodeBracketedExpression::build("〴q");
    }

    #[test]
    #[should_panic(expected = "empty repetition token")]
    fn empty_repetition() {
        UnicodeBracketedExpression::build("〇{}a");
    }

    #[test]
    #[should_panic(expected = "repetition")]
    fn open_ended_range() {
        UnicodeBracketedExpression::build("〇{2-}a");
    }
}

mod non_bmp {
    //! Java steps through UTF-16 units, so a surrogate pair is two characters to every pattern.

    use super::*;

    const GRIN: &str = "😀";
    /// U+1D400 MATHEMATICAL BOLD CAPITAL A, a letter outside the BMP.
    const BOLD_A: &str = "\u{1D400}";

    #[test]
    fn whole_characters_pass_through_runs() {
        assert_eq!(accepted("〇+〴.", "a😀b", 0), Some("a😀b"));
        assert_eq!(accepted("〇+〴.", BOLD_A, 0), Some(BOLD_A));
        assert_eq!(accepted("〇+〴W", GRIN, 0), Some(GRIN));
        assert_eq!(accepted("〇+〴S", GRIN, 0), Some(GRIN));
        assert_eq!(accepted("〇+〴G", "😀\"", 0), Some(GRIN));
        assert_eq!(accepted("〇+〴D", "😀\"", 0), Some("😀\""));
        assert_eq!(accepted("〇+「〤a」", "😀a", 0), Some(GRIN));
        assert_eq!(values("〶$V=〄>a", "😀a", "V"), [GRIN]);
        assert_eq!(values("<b>〶$V=〄>〘</b>〙", "<b>😀</b>", "V"), [GRIN]);
        assert_eq!(values("〶$V=〄+〴.->〘**〙", "x😀**", "V"), ["x😀"]);
    }

    #[test]
    fn a_surrogate_is_not_a_letter() {
        assert_eq!(accepted("〇+〴le", BOLD_A, 0), None);
        assert_eq!(accepted("〇+〴an", BOLD_A, 0), None);
    }

    #[test]
    fn counted_repetitions_count_utf16_units() {
        assert_eq!(accepted("〇{2}〴.", GRIN, 0), Some(GRIN));
        assert_eq!(accepted("〴.〴.〒$", GRIN, 0), Some(GRIN));
    }

    #[test]
    fn literal_pairs_match_as_two_units() {
        assert_eq!(accepted(GRIN, GRIN, 0), Some(GRIN));
        assert_eq!(accepted("😀a", "😀A", 0), Some("😀A"));
    }

    #[test]
    fn look_behind_sees_the_pair_reversed() {
        assert_eq!(accepted("〴.〴.〒<=〘😀〙", GRIN, 0), None);
    }

    /// Java accepts the high surrogate alone; a `&str` cannot be cut there, so the accepted text widens
    /// to the whole character.
    #[test]
    fn a_match_ending_inside_a_pair_widens_to_the_whole_character() {
        let matcher = matched("〶$V=〴.", GRIN, 0);
        assert_eq!(matcher.accepted_match(), GRIN);
        assert_eq!(matcher.find_values_by_key("V"), [GRIN]);
        assert_eq!(accepted("「〤a」", GRIN, 0), Some(GRIN));
    }

    #[test]
    fn positions_are_byte_offsets() {
        assert_eq!(accepted("〶$V=〴.", "😀x", 4), Some("x"));
    }
}
