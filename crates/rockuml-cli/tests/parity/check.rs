use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::LazyLock;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use regex::Regex;

use crate::corpus::{Case, GoldenKind, file_name};

pub(crate) enum Outcome {
    Pass,
    Fail(String),
}

/// Returns `None` when the golden model produced nothing of this kind for the case.
pub(crate) fn check(rockuml: &Path, case: &Case, kind: GoldenKind) -> Option<Outcome> {
    let goldens = case.goldens(kind);
    if goldens.is_empty() {
        return None;
    }

    let (produced, run) = if kind.writes_to_stdout() {
        run_to_stdout(rockuml, case, kind)
    } else {
        run_to_files(rockuml, case, kind)
    };
    if produced.is_empty() {
        return Some(Outcome::Fail(format!(
            "produced no output ({}): {}",
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        )));
    }

    Some(compare(
        &goldens
            .into_iter()
            .map(|(name, path)| (name, read_normalised(&path, kind)))
            .collect(),
        &produced,
    ))
}

type Produced = (BTreeMap<String, String>, Output);

fn run_to_files(rockuml: &Path, case: &Case, kind: GoldenKind) -> Produced {
    let output_directory = tempfile::tempdir().unwrap();
    let run = Command::new(rockuml)
        .args(kind.cli_arguments())
        .arg("-o")
        .arg(output_directory.path())
        .arg(&case.source)
        .output()
        .unwrap();
    let produced = fs::read_dir(output_directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| {
            // Deterministic SVGs are written as `.svg`, but kept beside the font-measured ones as `.dsvg`.
            let golden_name = path.with_extension(kind.extension());
            (file_name(&golden_name), read_normalised(&path, kind))
        })
        .collect();
    (produced, run)
}

/// The golden model's stdout was saved as `<stem>.<extension>`.
fn run_to_stdout(rockuml: &Path, case: &Case, kind: GoldenKind) -> Produced {
    let run = Command::new(rockuml)
        .args(kind.cli_arguments())
        .arg(&case.source)
        .output()
        .unwrap();
    let stdout = normalise(&String::from_utf8_lossy(&run.stdout));
    let mut produced = BTreeMap::new();
    if !stdout.is_empty() {
        let stem = case.source.file_stem().unwrap().to_string_lossy();
        produced.insert(format!("{stem}.{}", kind.extension()), stdout);
    }
    (produced, run)
}

fn compare(expected: &BTreeMap<String, String>, produced: &BTreeMap<String, String>) -> Outcome {
    let expected_names: Vec<_> = expected.keys().collect();
    let produced_names: Vec<_> = produced.keys().collect();
    if expected_names != produced_names {
        return Outcome::Fail(format!(
            "expected files {expected_names:?}, produced {produced_names:?}"
        ));
    }

    for (name, expected_content) in expected {
        if let Some(difference) = first_difference(expected_content, &produced[name]) {
            return Outcome::Fail(format!("{name}: {difference}"));
        }
    }
    Outcome::Pass
}

fn first_difference(expected: &str, produced: &str) -> Option<String> {
    let mut expected_lines = expected.lines();
    let mut produced_lines = produced.lines();
    for line_number in 1.. {
        match (expected_lines.next(), produced_lines.next()) {
            (None, None) => return None,
            (expected_line, produced_line) if expected_line != produced_line => {
                return Some(format!(
                    "line {line_number}: expected {:?}, produced {:?}",
                    expected_line.unwrap_or("<end of file>"),
                    produced_line.unwrap_or("<end of file>")
                ));
            }
            _ => {}
        }
    }
    unreachable!()
}

/// PNGs are compared by size only: Java and resvg antialias differently. Images embedded in SVG compare by
/// their pixels, as Java's PNG encoder is not worth reproducing.
fn read_normalised(path: &Path, kind: GoldenKind) -> String {
    let bytes = fs::read(path).unwrap();
    match kind {
        GoldenKind::Png => png_size(&bytes),
        GoldenKind::Svg | GoldenKind::DeterministicSvg => {
            embedded_pngs_as_pixels(&normalise(&String::from_utf8_lossy(&bytes)))
        }
        _ => normalise(&String::from_utf8_lossy(&bytes)),
    }
}

/// Width and height from the PNG header.
fn png_size(png: &[u8]) -> String {
    let dimension = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    format!("{} x {}", dimension(16), dimension(20))
}

/// Replaces each `data:image/png;base64,` payload by the image's size and a hash of its pixels. Fully
/// transparent pixels count as equal whatever colour they carry.
fn embedded_pngs_as_pixels(text: &str) -> String {
    static PNG_DATA: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"data:image/png;base64,([A-Za-z0-9+/=]+)").unwrap());
    PNG_DATA
        .replace_all(text, |captures: &regex::Captures| {
            BASE64_STANDARD
                .decode(&captures[1])
                .ok()
                .and_then(|png| pixels_description(&png))
                .unwrap_or_else(|| captures[0].to_owned())
        })
        .into_owned()
}

fn pixels_description(png: &[u8]) -> Option<String> {
    let mut decoder = png::Decoder::new(Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let mut hash = Fnv1a::default();
    for sample in buffer[..frame.buffer_size()].chunks_exact(frame.color_type.samples()) {
        let rgba = match *sample {
            [gray] => [gray, gray, gray, 255],
            [gray, alpha] => [gray, gray, gray, alpha],
            [red, green, blue] => [red, green, blue, 255],
            [red, green, blue, alpha] => [red, green, blue, alpha],
            _ => return None,
        };
        hash.write(if rgba[3] == 0 { [0; 4] } else { rgba });
    }
    Some(format!(
        "data:image/png;pixels={}x{}:{:016x}",
        frame.width, frame.height, hash.0
    ))
}

/// A hash that stays the same across runs and platforms.
struct Fnv1a(u64);

impl Default for Fnv1a {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv1a {
    fn write(&mut self, bytes: [u8; 4]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// Line endings depend on how git checked out the goldens, and PlantUML's debug output stamps the
/// current time next to shapes it cannot describe; neither says anything about rendering.
fn normalise(text: &str) -> String {
    static RENDER_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(Mon|Tue|Wed|Thu|Fri|Sat|Sun) [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \S+ \d{4}")
            .unwrap()
    });
    RENDER_TIMESTAMP
        .replace_all(&text.replace("\r\n", "\n"), "<timestamp>")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_first_differing_line() {
        assert_eq!(
            first_difference("a\nb\nc", "a\nx\nc").as_deref(),
            Some(r#"line 2: expected "b", produced "x""#)
        );
    }

    #[test]
    fn reports_missing_trailing_lines() {
        assert_eq!(
            first_difference("a\nb", "a").as_deref(),
            Some(r#"line 2: expected "b", produced "<end of file>""#)
        );
    }

    #[test]
    fn pngs_compare_by_size() {
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        header.extend(300_u32.to_be_bytes());
        header.extend(20_u32.to_be_bytes());
        assert_eq!(png_size(&header), "300 x 20");
    }

    fn png_data_uri(rgba: &[u8], compression: png::Compression) -> String {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_compression(compression);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(rgba).unwrap();
        writer.finish().unwrap();
        format!(
            r#"<image xlink:href="data:image/png;base64,{}"/>"#,
            BASE64_STANDARD.encode(png)
        )
    }

    #[test]
    fn embedded_pngs_compare_by_pixels() {
        let red_then_clear = [255, 0, 0, 255, 0, 0, 0, 0];
        let red_then_clear_blue = [255, 0, 0, 255, 0, 0, 255, 0];
        let red_then_blue = [255, 0, 0, 255, 0, 0, 255, 255];
        let fast =
            embedded_pngs_as_pixels(&png_data_uri(&red_then_clear, png::Compression::Fastest));
        let best =
            embedded_pngs_as_pixels(&png_data_uri(&red_then_clear_blue, png::Compression::High));
        assert_eq!(fast, best);
        assert!(fast.starts_with(r#"<image xlink:href="data:image/png;pixels=2x1:"#));
        assert_ne!(
            fast,
            embedded_pngs_as_pixels(&png_data_uri(&red_then_blue, png::Compression::Fastest))
        );
    }

    #[test]
    fn identical_text_has_no_difference() {
        assert_eq!(first_difference("a\nb\n", "a\nb\n"), None);
    }

    #[test]
    fn masks_render_timestamps_and_line_endings() {
        assert_eq!(
            normalise(
                "  backcolor: HColorGradient Mon Oct 05 14:49:18 CEST 2026\r\nUGraphicDebug UImage Sun Jan 01 09:00:00 GMT+02:00 2027\r\n"
            ),
            "  backcolor: HColorGradient <timestamp>\nUGraphicDebug UImage <timestamp>\n"
        );
    }
}
