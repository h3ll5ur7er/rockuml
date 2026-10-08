//! Reading back the diagram source PlantUML embeds in its images, as `-metadata` does.

use std::io::Read;

use flate2::read::ZlibDecoder;

/// Whether an image carries its diagram's source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metadata {
    Embedded,
    Omitted,
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const KEYWORD: &str = "plantuml";

/// The source in an SVG's `<?plantuml-src ...?>` instruction, or in the `<!--SRC=[...]-->` comment older
/// releases wrote.
pub fn from_svg(svg: &str) -> Option<String> {
    const HEADER: &str = "<?plantuml-src ";
    const OLD_HEADER: &str = "<!--SRC=[";
    if let Some(start) = svg.rfind(HEADER).filter(|&start| start > 0) {
        let part = &svg[start + HEADER.len()..];
        if let Some(end) = part.find("?>").filter(|&end| end > 0) {
            return crate::url_code::decode(&part[..end]).ok();
        }
    }
    let start = svg.rfind(OLD_HEADER).filter(|&start| start > 0)?;
    let part = &svg[start + OLD_HEADER.len()..];
    let end = part.find(']').filter(|&end| end > 0)?;
    crate::url_code::decode(&part[..end].replace("- -", "--")).ok()
}

/// The source in a PNG's `plantuml` text chunk, compressed or not, like Java's image metadata reader finds
/// it.
pub fn from_png(png: &[u8]) -> Option<String> {
    let mut rest = png.strip_prefix(PNG_SIGNATURE)?;
    while rest.len() >= 12 {
        let length = usize::try_from(u32::from_be_bytes(rest[..4].try_into().ok()?)).ok()?;
        let chunk_type = &rest[4..8];
        let data = rest.get(8..8 + length)?;
        if let Some(text) = text_of_chunk(chunk_type, data) {
            return Some(text);
        }
        rest = rest.get(12 + length..)?;
    }
    None
}

/// The text of a `tEXt`, `zTXt` or `iTXt` chunk whose keyword is PlantUML's.
fn text_of_chunk(chunk_type: &[u8], data: &[u8]) -> Option<String> {
    let separator = data.iter().position(|&byte| byte == 0)?;
    if &data[..separator] != KEYWORD.as_bytes() {
        return None;
    }
    let after_keyword = &data[separator + 1..];
    match chunk_type {
        b"tEXt" => Some(latin1(after_keyword)),
        b"zTXt" => Some(latin1(&inflate(after_keyword.get(1..)?)?)),
        b"iTXt" => {
            let [compressed, _method, rest @ ..] = after_keyword else {
                return None;
            };
            let language_end = rest.iter().position(|&byte| byte == 0)?;
            let rest = &rest[language_end + 1..];
            let translated_end = rest.iter().position(|&byte| byte == 0)?;
            let text = &rest[translated_end + 1..];
            let text = if *compressed == 0 {
                text.to_vec()
            } else {
                inflate(text)?
            };
            Some(String::from_utf8_lossy(&text).into_owned())
        }
        _ => None,
    }
}

fn inflate(zlib: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    ZlibDecoder::new(zlib).read_to_end(&mut result).ok()?;
    Some(result)
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&byte| char::from(byte)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(chunk_type: &[u8], data: &[u8]) -> Vec<u8> {
        let mut result = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        result.extend(chunk_type);
        result.extend(data);
        result.extend([0; 4]);
        result
    }

    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut result = PNG_SIGNATURE.to_vec();
        result.extend(chunk(b"IHDR", &[0; 13]));
        for chunk in chunks {
            result.extend(chunk);
        }
        result.extend(chunk(b"IEND", &[]));
        result
    }

    fn zlib(text: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, text).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn pngs_keep_the_source_in_any_text_chunk() {
        let text = |chunk_type: &[u8], data: &[u8]| from_png(&png(&[chunk(chunk_type, data)]));
        assert_eq!(
            text(b"tEXt", b"plantuml\0A\xe9").as_deref(),
            Some("A\u{e9}")
        );
        let compressed = [b"plantuml\0\0".as_slice(), &zlib(b"B")].concat();
        assert_eq!(text(b"zTXt", &compressed).as_deref(), Some("B"));
        let international = [
            b"plantuml\0\x01\0\0\0".as_slice(),
            &zlib("C\u{e9}".as_bytes()),
        ]
        .concat();
        assert_eq!(text(b"iTXt", &international).as_deref(), Some("C\u{e9}"));
        assert_eq!(text(b"iTXt", b"plantuml\0\0\0\0\0D").as_deref(), Some("D"));
    }

    #[test]
    fn other_keywords_and_images_have_no_source() {
        assert_eq!(from_png(&png(&[chunk(b"tEXt", b"Software\0x")])), None);
        assert_eq!(from_png(b"GIF89a"), None);
    }

    fn exported(format: crate::diagram::ImageFormat, metadata: Metadata) -> Vec<u8> {
        use crate::host::IsolatedHost;
        let source = crate::preproc::Source {
            text: "@startuml\nA -> B\n@enduml\n",
            description: "t",
            directory: std::path::PathBuf::new(),
            environment: crate::preproc::PreprocessorEnvironment::default(),
        };
        let block = crate::preproc::preprocess(&source, &IsolatedHost).remove(0);
        let diagram = crate::diagram::create(&block, &IsolatedHost).unwrap();
        let fonts = std::sync::Arc::new(crate::fonts::FontRegistry::default());
        crate::diagram::export_with(diagram.as_ref(), 0, format, metadata, &fonts, &IsolatedHost)
            .unwrap()
    }

    #[test]
    fn exported_images_carry_their_source_unless_it_is_omitted() {
        use crate::diagram::ImageFormat::{Png, Svg};
        let svg = |metadata| String::from_utf8(exported(Svg, metadata)).unwrap();
        assert_eq!(
            from_svg(&svg(Metadata::Embedded)).as_deref(),
            Some("@startuml\nA -> B\n@enduml")
        );
        assert!(!svg(Metadata::Omitted).contains("plantuml-src"));
        assert_eq!(
            from_png(&exported(Png, Metadata::Embedded)).as_deref(),
            Some("@startuml\nA -> B\n@enduml\n\n1.2026.8")
        );
        assert_eq!(from_png(&exported(Png, Metadata::Omitted)), None);
    }

    #[test]
    fn svgs_keep_the_source_in_a_processing_instruction() {
        let code = crate::url_code::encode("@startuml\nA -> B\n@enduml\n");
        assert_eq!(
            from_svg(&format!("<svg><g/><?plantuml-src {code}?></svg>")).as_deref(),
            Some("@startuml\nA -> B\n@enduml")
        );
        assert_eq!(
            from_svg(&format!("<svg><!--SRC=[{code}]--></svg>")).as_deref(),
            Some("@startuml\nA -> B\n@enduml")
        );
        assert_eq!(from_svg("<svg/>"), None);
    }
}
