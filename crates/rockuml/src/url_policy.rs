//! Which URLs a diagram may load (`!include`, `%load_json`, `!theme ... from`), as PlantUML's default
//! "legacy" security profile decides. Hosts additionally refuse addresses that resolve to private networks.

use std::sync::LazyLock;

use regex::Regex;

pub fn is_forbidden(url: &str) -> bool {
    static NUMERIC_HOST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^https?://[-#.0-9:\[\]+]+/.*").unwrap());
    static SINGLE_LABEL_HOST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^https?://[^.]+(/.*)?$").unwrap());

    if url.contains('@') || !(url.starts_with("http://") || url.starts_with("https://")) {
        return true;
    }
    if NUMERIC_HOST.is_match(url) || SINGLE_LABEL_HOST.is_match(url) {
        return true;
    }
    let Some(host) = host_of(url) else {
        return true;
    };
    host.is_empty()
        || !host.contains('.')
        || host
            .chars()
            .all(|c| matches!(c, '-' | '#' | '.' | '0'..='9' | ':' | '[' | ']' | '+'))
        || host.contains('%')
}

/// The host part of an `http(s)://` URL, without port.
pub fn host_of(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    Some(
        authority
            .rsplit_once(':')
            .map_or(authority, |(host, _port)| host),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_web_urls_are_allowed() {
        assert!(!is_forbidden(
            "https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4.puml"
        ));
        assert!(!is_forbidden("http://example.com:8080/x.puml"));
    }

    #[test]
    fn credentials_numeric_and_local_hosts_are_refused() {
        assert!(is_forbidden("https://user:secret@example.com/x"));
        assert!(is_forbidden("http://127.0.0.1/x"));
        assert!(is_forbidden("http://localhost/x"));
        assert!(is_forbidden("http://intranet"));
        assert!(is_forbidden("file:///etc/passwd"));
        assert!(is_forbidden("http://exa%6dple.com/x"));
    }
}
