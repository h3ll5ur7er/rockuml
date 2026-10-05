//! Creole escapes a markup character with `~`. While a line is parsed, escaped characters are swapped for
//! private-use stand-ins so that no markup rule can match them, and swapped back when the text is drawn.

const HIDDEN_BASE: u32 = 0xE000;

fn is_to_be_hidden(c: char) -> bool {
    matches!(c, '_' | '-' | '"' | '#' | ']' | '[' | '*' | '.' | '/' | '<')
}

fn hidden(c: char) -> char {
    char::from_u32(HIDDEN_BASE + u32::from(c)).expect("ASCII stand-ins are private-use characters")
}

pub(super) fn hide(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek().copied()) {
            ('\\', Some('~')) => {
                chars.next();
                result.push(hidden('~'));
            }
            ('~', Some(next)) => {
                chars.next();
                if is_to_be_hidden(next) {
                    result.push(hidden(next));
                } else {
                    result.push(c);
                    result.push(next);
                }
            }
            _ => result.push(c),
        }
    }
    result
}

pub(super) fn unhide(s: &str) -> String {
    s.chars()
        .map(|c| match u32::from(c) {
            code @ 0xE000..=0xE0FF => char::from_u32(code - HIDDEN_BASE).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_markup_survives_parsing_and_comes_back() {
        let hidden_text = hide(r"~**not bold~** ~x \~");
        assert!(!hidden_text.contains("**"));
        assert!(hidden_text.contains("~x"));
        assert_eq!(unhide(&hidden_text), r"**not bold** ~x ~");
    }
}
