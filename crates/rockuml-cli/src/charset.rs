//! The text encodings `--charset` names, for reading diagram sources and writing preprocessed ones.

/// The encodings rockuml knows, by the names and aliases Java's `Charset.forName` accepts for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Charset {
    Utf8,
    /// UTF-16 led by a byte order mark, big-endian without one; written big-endian with the mark.
    Utf16,
    Utf16BigEndian,
    Utf16LittleEndian,
    Latin1,
    Windows1252,
    Ascii,
}

/// What `windows-1252` puts at 0x80 to 0x9F, where Latin-1 has control characters; `None` where it has
/// nothing.
const WINDOWS_1252_HIGH: [Option<char>; 32] = [
    Some('\u{20AC}'),
    None,
    Some('\u{201A}'),
    Some('\u{0192}'),
    Some('\u{201E}'),
    Some('\u{2026}'),
    Some('\u{2020}'),
    Some('\u{2021}'),
    Some('\u{02C6}'),
    Some('\u{2030}'),
    Some('\u{0160}'),
    Some('\u{2039}'),
    Some('\u{0152}'),
    None,
    Some('\u{017D}'),
    None,
    None,
    Some('\u{2018}'),
    Some('\u{2019}'),
    Some('\u{201C}'),
    Some('\u{201D}'),
    Some('\u{2022}'),
    Some('\u{2013}'),
    Some('\u{2014}'),
    Some('\u{02DC}'),
    Some('\u{2122}'),
    Some('\u{0161}'),
    Some('\u{203A}'),
    Some('\u{0153}'),
    None,
    Some('\u{017E}'),
    Some('\u{0178}'),
];

const BYTE_ORDER_MARK: char = '\u{FEFF}';

impl Charset {
    pub(crate) fn for_name(name: &str) -> Result<Self, String> {
        let normalized = name.to_ascii_lowercase().replace('_', "-");
        Ok(match normalized.as_str() {
            "utf-8" | "utf8" => Self::Utf8,
            "utf-16" | "utf16" => Self::Utf16,
            "utf-16be" | "unicodebigunmarked" => Self::Utf16BigEndian,
            "utf-16le" | "unicodelittleunmarked" => Self::Utf16LittleEndian,
            "iso-8859-1" | "iso8859-1" | "8859-1" | "latin1" | "l1" | "iso-latin-1" | "cp819" => {
                Self::Latin1
            }
            "windows-1252" | "cp1252" => Self::Windows1252,
            "us-ascii" | "ascii" | "us" | "iso646-us" => Self::Ascii,
            _ => return Err(format!("unsupported charset: {name}")),
        })
    }

    /// Bytes that cannot be decoded become U+FFFD, as with Java's decoders.
    pub(crate) fn decode(self, bytes: &[u8]) -> String {
        match self {
            Self::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Self::Utf16 => match bytes {
                [0xFF, 0xFE, rest @ ..] => decode_utf16(rest, u16::from_le_bytes),
                [0xFE, 0xFF, rest @ ..] => decode_utf16(rest, u16::from_be_bytes),
                _ => decode_utf16(bytes, u16::from_be_bytes),
            },
            Self::Utf16BigEndian => decode_utf16(bytes, u16::from_be_bytes),
            Self::Utf16LittleEndian => decode_utf16(bytes, u16::from_le_bytes),
            Self::Latin1 => bytes.iter().map(|&byte| char::from(byte)).collect(),
            Self::Windows1252 => bytes
                .iter()
                .map(|&byte| match byte {
                    0x80..=0x9F => {
                        WINDOWS_1252_HIGH[usize::from(byte - 0x80)].unwrap_or('\u{FFFD}')
                    }
                    _ => char::from(byte),
                })
                .collect(),
            Self::Ascii => bytes
                .iter()
                .map(|&byte| {
                    if byte.is_ascii() {
                        char::from(byte)
                    } else {
                        '\u{FFFD}'
                    }
                })
                .collect(),
        }
    }

    /// Characters the encoding lacks become `?`, as with Java's encoders.
    pub(crate) fn encode(self, text: &str) -> Vec<u8> {
        let single_bytes = |byte_of: &dyn Fn(char) -> Option<u8>| -> Vec<u8> {
            text.chars().map(|c| byte_of(c).unwrap_or(b'?')).collect()
        };
        match self {
            Self::Utf8 => text.as_bytes().to_vec(),
            Self::Utf16 => std::iter::once(BYTE_ORDER_MARK)
                .chain(text.chars())
                .collect::<String>()
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect(),
            Self::Utf16BigEndian => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            Self::Utf16LittleEndian => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            Self::Latin1 => single_bytes(&|c| u8::try_from(u32::from(c)).ok()),
            Self::Windows1252 => single_bytes(&|c| {
                WINDOWS_1252_HIGH
                    .iter()
                    .position(|&high| high == Some(c))
                    .map(|index| 0x80 + index as u8)
                    .or_else(|| {
                        u8::try_from(u32::from(c))
                            .ok()
                            .filter(|byte| !(0x80..=0x9F).contains(byte))
                    })
            }),
            Self::Ascii => single_bytes(&|c| u8::try_from(u32::from(c)).ok().filter(u8::is_ascii)),
        }
    }
}

fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|pair| match pair {
            [high, low] => unit([*high, *low]),
            _ => 0xFFFD,
        })
        .collect();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_aliases_ignore_case() {
        assert_eq!(Charset::for_name("UTF-8"), Ok(Charset::Utf8));
        assert_eq!(Charset::for_name("latin1"), Ok(Charset::Latin1));
        assert_eq!(Charset::for_name("ISO8859_1"), Ok(Charset::Latin1));
        assert_eq!(Charset::for_name("Cp1252"), Ok(Charset::Windows1252));
        assert!(Charset::for_name("EBCDIC").is_err());
    }

    #[test]
    fn single_byte_encodings_round_trip() {
        let text = "caf\u{e9} \u{20AC}";
        assert_eq!(Charset::Windows1252.encode(text), b"caf\xe9 \x80");
        assert_eq!(Charset::Windows1252.decode(b"caf\xe9 \x80"), text);
        assert_eq!(Charset::Latin1.encode(text), b"caf\xe9 ?");
        assert_eq!(Charset::Latin1.decode(b"caf\xe9"), "caf\u{e9}");
    }

    #[test]
    fn utf16_reads_the_byte_order_mark_and_writes_big_endian() {
        assert_eq!(Charset::Utf16.decode(b"\xff\xfeA\0"), "A");
        assert_eq!(Charset::Utf16.decode(b"\xfe\xff\0A"), "A");
        assert_eq!(Charset::Utf16.decode(b"\0A"), "A");
        assert_eq!(Charset::Utf16.encode("A"), b"\xfe\xff\0A");
    }
}
