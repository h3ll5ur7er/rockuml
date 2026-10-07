//! Links: `[[url]]`, `[[url label]]`, `[[url{tooltip} label]]`, `[["quoted url"]]` and `[[{tooltip}]]`
//! (PlantUML's `Url` and `UrlBuilder`).

use std::sync::LazyLock;

use regex::Regex;

use crate::pattern::plantuml_regex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    pub href: String,
    /// The URL itself unless one is given.
    pub tooltip: String,
    /// The URL itself unless one is given.
    pub label: String,
}

impl Url {
    fn new(url: &str, tooltip: Option<&str>, label: Option<&str>) -> Self {
        let url = without_surrounding_quotes(url);
        Self {
            href: url.to_owned(),
            tooltip: tooltip.unwrap_or(url).to_owned(),
            label: label
                .filter(|label| !label.is_empty())
                .unwrap_or(url)
                .to_owned(),
        }
    }

    /// The link a whole `[[...]]` markup describes.
    pub fn parse(markup: &str) -> Option<Self> {
        let captures = |pattern: &Regex| pattern.captures(markup);
        let group = |captures: &regex::Captures<'_>, index| {
            captures.get(index).map(|group| group.as_str().to_owned())
        };
        if let Some(c) = captures(&forms().quoted) {
            return Some(Self::new(
                &c[1],
                group(&c, 2).as_deref(),
                group(&c, 3).as_deref(),
            ));
        }
        if let Some(c) = captures(&forms().only_tooltip) {
            return Some(Self::new("", Some(&c[1]), None));
        }
        if let Some(c) = captures(&forms().tooltip_and_label) {
            return Some(Self::new("", Some(&c[1]), Some(&c[2])));
        }
        if let Some(c) = captures(&forms().link_and_tooltip) {
            return Some(Self::new(&c[1], Some(&c[2]), None));
        }
        let c = captures(&forms().link)?;
        Some(Self::new(
            &c[1],
            group(&c, 2).as_deref(),
            group(&c, 3).as_deref(),
        ))
    }

    /// The pattern of a `[[...]]` markup in a command, with its 12 groups (`UrlBuilder.MANDATORY`).
    pub fn command_pattern() -> String {
        format!("({})", Forms::alternatives())
    }

    /// The length of the `[[...]]` markup at the start of `text`, if there is one.
    pub fn markup_length(text: &str) -> Option<usize> {
        static AT_START: LazyLock<Regex> =
            LazyLock::new(|| plantuml_regex(&format!("^(?:{})", Forms::alternatives())));
        AT_START.find(text).map(|found| found.end())
    }
}

/// The markup forms, each matched against the whole markup in turn.
struct Forms {
    quoted: Regex,
    only_tooltip: Regex,
    tooltip_and_label: Regex,
    link_and_tooltip: Regex,
    link: Regex,
}

const START: &str = r"\[\[[%s]*";
const END: &str = r"[%s]*\]\]";
const QUOTED: &str = r"[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?";
const ONLY_TOOLTIP: &str = r"\{(.*)\}";
const TOOLTIP_AND_LABEL: &str = r"\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)";
const LINK_AND_TOOLTIP: &str = r"([^\s%g{}\[\]]+?)[%s]*\{(.+)\}";
const LINK: &str = r"([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?";

impl Forms {
    fn alternatives() -> String {
        [
            QUOTED,
            ONLY_TOOLTIP,
            TOOLTIP_AND_LABEL,
            LINK_AND_TOOLTIP,
            LINK,
        ]
        .map(|form| format!("{START}{form}{END}"))
        .join("|")
    }
}

fn forms() -> &'static Forms {
    static FORMS: LazyLock<Forms> = LazyLock::new(|| {
        let whole = |form: &str| plantuml_regex(&format!("^{START}{form}{END}$"));
        Forms {
            quoted: whole(QUOTED),
            only_tooltip: whole(ONLY_TOOLTIP),
            tooltip_and_label: whole(TOOLTIP_AND_LABEL),
            link_and_tooltip: whole(LINK_AND_TOOLTIP),
            link: whole(LINK),
        }
    });
    &FORMS
}

fn without_surrounding_quotes(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(href: &str, tooltip: &str, label: &str) -> Url {
        Url {
            href: href.to_owned(),
            tooltip: tooltip.to_owned(),
            label: label.to_owned(),
        }
    }

    #[test]
    fn markup_forms_give_url_tooltip_and_label() {
        let site = "https://plantuml.com";
        assert_eq!(
            Url::parse("[[https://plantuml.com]]"),
            Some(expected(site, site, site))
        );
        assert_eq!(
            Url::parse("[[https://plantuml.com the site]]"),
            Some(expected(site, site, "the site"))
        );
        assert_eq!(
            Url::parse("[[https://plantuml.com{Tip} the site]]"),
            Some(expected(site, "Tip", "the site"))
        );
        assert_eq!(
            Url::parse("[[\"a b\" label]]"),
            Some(expected("a b", "a b", "label"))
        );
        assert_eq!(
            Url::parse("[[{Just a tip}]]"),
            Some(expected("", "Just a tip", ""))
        );
    }

    #[test]
    fn markup_length_covers_the_brackets() {
        assert_eq!(Url::markup_length("[[a b]] rest"), Some(7));
        assert_eq!(Url::markup_length("no link"), None);
    }
}
