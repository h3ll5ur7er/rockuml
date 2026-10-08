//! PlantUML's text encoding for diagram sources in URLs: deflate, then a URL-safe base 64 alphabet.

use std::io::Read;
use std::sync::LazyLock;

use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use regex::Regex;

use crate::deflate::deflate;
use crate::java;
use crate::pattern::plantuml_regex;
use crate::preproc::read_uncommented_lines;

#[derive(Debug, PartialEq, Eq)]
pub struct NotPlantUmlCode;

/// Encodes a diagram source. For `@startuml` diagrams only the lines between start and end are kept.
pub fn encode(source: &str) -> String {
    let text = compress_source(source);
    if text.is_empty() {
        return String::new();
    }
    encode_6bit(&deflate(text.as_bytes()))
}

/// Decodes an encoded source; `~0` (deflate), `~1` (old Huffman-only), `~h` (hex) and `~g` (gzip) prefixes
/// select a variant, and an unprefixed code is tried as deflate, then as the old format.
pub fn decode(code: &str) -> Result<String, NotPlantUmlCode> {
    let bytes = if let Some(rest) = code.strip_prefix("~0") {
        inflate(&decode_6bit(rest)?)?
    } else if let Some(rest) = code.strip_prefix("~1") {
        inflate_zlib(&decode_6bit(rest)?)?
    } else if let Some(rest) = code.strip_prefix("~h") {
        decode_hex(rest)?
    } else if let Some(rest) = code.strip_prefix("~g") {
        gunzip(&decode_6bit(rest)?)?
    } else {
        decode_6bit(code)
            .and_then(|bytes| inflate(&bytes))
            .or_else(|_| inflate_zlib(&decode_6bit(code)?))?
    };
    Ok(decompress_source(&String::from_utf8_lossy(&bytes)))
}

fn compress_source(source: &str) -> String {
    let lines = read_uncommented_lines(source, "COMPRESS");
    let mut inside = String::new();
    let mut all = String::new();
    let mut started = false;
    for line in &lines {
        append_line(&mut all, line);
        if line.starts_with("@startuml") {
            started = true;
        } else if line.starts_with("@enduml") {
            return inside;
        } else if started {
            append_line(&mut inside, line);
        }
    }
    if started {
        inside
    } else {
        source_body(&all)
            .map(|body| clean(&body))
            .unwrap_or_default()
    }
}

/// Separates lines with `\n` but only once the text is non-empty, so leading blank lines disappear.
fn append_line(text: &mut String, line: &str) {
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(line);
}

fn decompress_source(text: &str) -> String {
    let cleaned = clean(text);
    if cleaned.starts_with("@start") {
        return cleaned;
    }
    let mut result = format!("@startuml\n{cleaned}");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str("@enduml");
    result
}

static SOURCE_BODY: LazyLock<Regex> = LazyLock::new(|| {
    plantuml_regex("(?s)^[%s]*(@startuml[^\\n\\r]*)?[%s]*(.*?)[%s]*(@enduml)?[%s]*$")
});

fn source_body(text: &str) -> Option<String> {
    SOURCE_BODY
        .captures(text)
        .map(|captures| captures[2].to_owned())
}

fn clean(text: &str) -> String {
    static START_OR_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("@enduml[^\n\r]*|@startuml[^\n\r]*").unwrap());
    let text = java::trim(text);
    let text = source_body(text).unwrap_or_else(|| text.to_owned());
    java::trim(&START_OR_END.replace_all(&text, "")).to_owned()
}

/// Raw deflate, without zlib header.
pub(crate) fn inflate(data: &[u8]) -> Result<Vec<u8>, NotPlantUmlCode> {
    read_all(DeflateDecoder::new(data))
}

/// At most the first `limit` bytes `data` inflates to, so that a small input cannot expand without bound.
pub(crate) fn inflate_prefix(data: &[u8], limit: usize) -> Result<Vec<u8>, NotPlantUmlCode> {
    read_all(DeflateDecoder::new(data).take(limit as u64))
}

fn inflate_zlib(data: &[u8]) -> Result<Vec<u8>, NotPlantUmlCode> {
    read_all(ZlibDecoder::new(data))
}

fn gunzip(data: &[u8]) -> Result<Vec<u8>, NotPlantUmlCode> {
    read_all(GzDecoder::new(data))
}

fn read_all(mut reader: impl Read) -> Result<Vec<u8>, NotPlantUmlCode> {
    let mut result = Vec::new();
    reader
        .read_to_end(&mut result)
        .map_err(|_| NotPlantUmlCode)?;
    Ok(result)
}

fn decode_hex(text: &str) -> Result<Vec<u8>, NotPlantUmlCode> {
    (0..text.len() / 2)
        .map(|index| {
            let pair = text.get(index * 2..index * 2 + 2).ok_or(NotPlantUmlCode)?;
            u8::from_str_radix(pair, 16).map_err(|_| NotPlantUmlCode)
        })
        .collect()
}

pub(crate) const ALPHABET: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";

pub(crate) fn encode_6bit(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let [b1, b2, b3] = [0, 1, 2].map(|index| chunk.get(index).copied().unwrap_or(0));
        let sextets = [
            b1 >> 2,
            (b1 & 0x3) << 4 | b2 >> 4,
            (b2 & 0xF) << 2 | b3 >> 6,
            b3 & 0x3F,
        ];
        result.extend(sextets.map(|sextet| char::from(ALPHABET[usize::from(sextet)])));
    }
    result
}

/// A character's 6-bit value; characters outside the alphabet count as zero, as in PlantUML.
pub(crate) fn sextet(c: char) -> u8 {
    ALPHABET
        .iter()
        .position(|&known| char::from(known) == c)
        .map_or(0, |index| index as u8)
}

/// Every four characters give three bytes, the last ones padded with `0`; non-ASCII characters are an error.
pub(crate) fn decode_6bit(text: &str) -> Result<Vec<u8>, NotPlantUmlCode> {
    let value = |c: Option<char>| -> Result<u8, NotPlantUmlCode> {
        let c = c.unwrap_or('0');
        if !c.is_ascii() {
            return Err(NotPlantUmlCode);
        }
        Ok(sextet(c))
    };
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::with_capacity(chars.len().div_ceil(4) * 3);
    for chunk in chars.chunks(4) {
        let [c1, c2, c3, c4] = [0, 1, 2, 3].map(|index| chunk.get(index).copied());
        let (c1, c2, c3, c4) = (value(c1)?, value(c2)?, value(c3)?, value(c4)?);
        result.extend([
            c1 << 2 | c2 >> 4,
            (c2 & 0x0F) << 4 | c3 >> 2,
            (c3 & 0x3) << 6 | c4,
        ]);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_inflates_no_further_than_its_limit() {
        let compressed = deflate(&vec![7; 100_000]);
        assert_eq!(inflate_prefix(&compressed, 10), Ok(vec![7; 10]));
    }

    #[test]
    fn encodes_like_plantuml() {
        let cases = [
            (
                "@startuml\nBob -> Alice : hello\n@enduml\n",
                "SyfFKj2rKt3CoKnELR1Io4ZDoSa70000",
            ),
            (
                "@startuml\nA -> B : \u{fc}n\u{ef}code\n@enduml",
                "SrJGjLDmKh1IEBmdx_3wvFoKL000",
            ),
            (
                "@startmindmap\n* root\n@endmindmap",
                "SoWkIImgoStCIybDBE3IKYZApo_XSaW5SY520000",
            ),
            ("Alice -> Bob", "Syp9J4vLqBLJSCfF0W00"),
            ("", ""),
            ("' @startuml\n' A -> B\n' @enduml", "SrJGjLDm0W00"),
            ("@startuml\n\n\nA -> B\n@enduml", "SrJGjLDm0W00"),
        ];
        for (source, expected) in cases {
            assert_eq!(encode(source), expected, "{source:?}");
        }
    }

    #[test]
    fn decodes_like_plantuml() {
        assert_eq!(
            decode("SyfFKj2rKt3CoKnELR1Io4ZDoSa70000"),
            Ok("@startuml\nBob -> Alice : hello\n@enduml".into())
        );
    }

    #[test]
    fn round_trips_through_decode() {
        let source = "@startuml\nA -> B : ünïcode\n@enduml";
        assert_eq!(decode(&encode(source)), Ok(source.to_owned()));
    }

    #[test]
    fn decodes_hex_variant() {
        assert_eq!(
            decode("~h407374617274756d6c0a41202d3e20420a40656e64756d6c"),
            Ok("@startuml\nA -> B\n@enduml".into())
        );
    }

    #[test]
    fn non_uml_diagrams_keep_their_start_and_end_lines() {
        let source = "@startmindmap\n* root\n@endmindmap";
        assert_eq!(decode(&encode(source)), Ok(source.to_owned()));
    }
}
