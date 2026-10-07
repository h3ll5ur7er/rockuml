//! PlantUML's standard library (`!include <c4/C4_Container>` and friends).
//!
//! Each library folder holds Brotli-compressed "channels": `info.spm` is `key=value` text, the others are
//! Java `DataOutputStream` records (`int count`, then per entry a modified-UTF-8 name and the data).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;

use crate::assets;
use crate::java::RuntimeException;
use crate::klimt::sprite::{Sprite, SpriteMonochrome};
use crate::svg_parser::SvgNanoParser;

pub(crate) struct Stdlib {
    name: String,
    info: HashMap<String, String>,
    puml: RefCell<Option<HashMap<String, Vec<u8>>>>,
    json: RefCell<Option<HashMap<String, Vec<u8>>>>,
    sprites: RefCell<Option<HashMap<String, StdlibSprite>>>,
    svgs: RefCell<Option<HashMap<String, Rc<SvgNanoParser>>>>,
}

thread_local! {
    static LIBRARIES: RefCell<HashMap<String, Option<Rc<Stdlib>>>> = RefCell::default();
}

impl Stdlib {
    /// The library with this folder name, following `link=` redirections (e.g. versioned names).
    pub(crate) fn retrieve(name: &str) -> Option<Rc<Stdlib>> {
        if let Some(cached) = LIBRARIES.with_borrow(|libraries| libraries.get(name).cloned()) {
            return cached;
        }
        let library = Self::load(name);
        LIBRARIES.with_borrow_mut(|libraries| libraries.insert(name.to_owned(), library.clone()));
        library
    }

    fn load(name: &str) -> Option<Rc<Stdlib>> {
        let info_text = decompress(assets::get(&format!("stdlib/{name}/info.spm"))?)?;
        let info: HashMap<String, String> = String::from_utf8_lossy(&info_text)
            .lines()
            .filter_map(|line| match line.split('=').collect::<Vec<_>>()[..] {
                [key, value] => Some((key.to_owned(), value.to_owned())),
                _ => None,
            })
            .collect();
        if let Some(link) = info.get("link") {
            return Self::retrieve(link);
        }
        Some(Rc::new(Stdlib {
            name: name.to_owned(),
            info,
            puml: RefCell::default(),
            json: RefCell::default(),
            sprites: RefCell::default(),
            svgs: RefCell::default(),
        }))
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn has_info(&self) -> bool {
        !self.info.is_empty()
    }

    pub(crate) fn metadata(&self) -> impl Iterator<Item = (&String, &String)> {
        self.info.iter()
    }

    pub(crate) fn metadata_value(&self, key: &str) -> Option<String> {
        self.info.get(key).cloned()
    }

    pub(crate) fn version(&self) -> Option<String> {
        self.metadata_value("VERSION")
            .or_else(|| self.metadata_value("version"))
    }

    pub(crate) fn source(&self) -> Option<String> {
        self.metadata_value("SOURCE")
            .or_else(|| self.metadata_value("source"))
    }

    /// `file` is the lowercase path inside the library without the `.puml` extension.
    pub(crate) fn puml_resource(&self, file: &str) -> Option<Vec<u8>> {
        let mut puml = self.puml.borrow_mut();
        let entries = puml.get_or_insert_with(|| self.read_channel("puml").unwrap_or_default());
        entries.get(file).cloned()
    }

    fn json_resource(&self, file: &str) -> Option<Vec<u8>> {
        let mut json = self.json.borrow_mut();
        let entries = json.get_or_insert_with(|| self.read_channel("json").unwrap_or_default());
        entries.get(file).cloned()
    }

    /// A gray-level sprite of the `sprite` channel, by its exported name.
    pub(crate) fn read_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>> {
        let mut sprites = self.sprites.borrow_mut();
        let entries = sprites.get_or_insert_with(|| {
            self.read_records("sprite", |input| {
                let name = input.read_utf()?;
                let width = usize::try_from(input.read_int()?).ok()?;
                let height = usize::try_from(input.read_int()?).ok()?;
                let data = input.read_bytes(width * height.div_ceil(2))?.to_vec();
                Some((
                    name,
                    StdlibSprite {
                        width,
                        height,
                        data,
                    },
                ))
            })
            .unwrap_or_default()
        });
        entries
            .get(name)
            .map(|sprite| Rc::new(sprite.decode()) as Rc<dyn Sprite>)
    }

    /// An SVG sprite of the `svg` channel, by its exported name.
    pub(crate) fn read_svg_sprite(&self, name: &str) -> Option<Rc<dyn Sprite>> {
        let mut svgs = self.svgs.borrow_mut();
        let entries = svgs.get_or_insert_with(|| {
            self.read_records("svg", |input| {
                let name = input.read_utf()?;
                Some((name, Rc::new(SvgNanoParser::new(input.read_utf()?))))
            })
            .unwrap_or_default()
        });
        entries
            .get(name)
            .map(|sprite| sprite.clone() as Rc<dyn Sprite>)
    }

    /// Files by lowercase name.
    fn read_channel(&self, channel: &str) -> Option<HashMap<String, Vec<u8>>> {
        self.read_records(channel, |input| {
            let name = input.read_utf()?.to_lowercase();
            let length = usize::try_from(input.read_int()?).ok()?;
            Some((name, input.read_bytes(length)?.to_vec()))
        })
    }

    /// A channel: a count, then that many records.
    fn read_records<T>(
        &self,
        channel: &str,
        mut read_record: impl FnMut(&mut DataInput) -> Option<(String, T)>,
    ) -> Option<HashMap<String, T>> {
        let data = decompress(assets::get(&format!("stdlib/{}/{channel}.spm", self.name))?)?;
        let mut input = DataInput {
            data: &data,
            position: 0,
        };
        let count = input.read_int()?;
        (0..count).map(|_| read_record(&mut input)).collect()
    }
}

/// A `.puml` file of the standard library, named like `c4/C4_Container` or `c4/C4_Container.puml`.
pub(crate) fn puml_resource(full_name: &str) -> Option<Vec<u8>> {
    let full_name = full_name.to_lowercase().replace(".puml", "");
    let (library, file) = full_name.split_once('/')?;
    let library = Stdlib::retrieve(library)?;
    if !library.has_info() {
        return None;
    }
    library.puml_resource(file)
}

/// The folder names of all libraries, sorted.
pub(crate) fn library_names() -> Vec<String> {
    let mut names: Vec<String> = assets::get("stdlib/home.spm")
        .map(|home| {
            String::from_utf8_lossy(home)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

/// A JSON file of the standard library, for `%load_json(<library/file>)`. PlantUML crashes when the
/// library exists but the file does not.
pub(crate) fn json_resource(full_name: &str) -> Result<Option<Vec<u8>>, RuntimeException> {
    let Some((library, file)) = full_name.split_once('/') else {
        return Ok(None);
    };
    let Some(library) = Stdlib::retrieve(library).filter(|library| library.has_info()) else {
        return Ok(None);
    };
    library
        .json_resource(file)
        .map(Some)
        .ok_or(RuntimeException)
}

/// A sprite of 16 gray levels as the `sprite` channel stores it: two pixels a byte, the upper one in the high
/// nibble.
struct StdlibSprite {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl StdlibSprite {
    fn decode(&self) -> SpriteMonochrome {
        let mut sprite = SpriteMonochrome::new(self.width, self.height, 16);
        for (pair, row) in self.data.chunks(self.width.max(1)).enumerate() {
            for (x, &levels) in row.iter().enumerate() {
                sprite.set_gray(x, pair * 2, usize::from(levels >> 4));
                sprite.set_gray(x, pair * 2 + 1, usize::from(levels & 0x0F));
            }
        }
        sprite
    }
}

fn decompress(compressed: &[u8]) -> Option<Vec<u8>> {
    let mut decompressed = Vec::new();
    brotli_decompressor::Decompressor::new(compressed, 4096)
        .read_to_end(&mut decompressed)
        .ok()?;
    Some(decompressed)
}

struct DataInput<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> DataInput<'a> {
    fn read_bytes(&mut self, length: usize) -> Option<&'a [u8]> {
        let bytes = self.data.get(self.position..self.position + length)?;
        self.position += length;
        Some(bytes)
    }

    fn read_int(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.read_bytes(4)?.try_into().ok()?))
    }

    /// Java's modified UTF-8: a 16-bit length, `\0` as two bytes, supplementary characters as surrogate pairs.
    fn read_utf(&mut self) -> Option<String> {
        let length = usize::from(u16::from_be_bytes(self.read_bytes(2)?.try_into().ok()?));
        let bytes = self.read_bytes(length)?;
        let mut units = Vec::with_capacity(length);
        let mut index = 0;
        while index < bytes.len() {
            let first = u16::from(bytes[index]);
            let (unit, width) = match first {
                0x00..=0x7F => (first, 1),
                0xC0..=0xDF => (
                    (first & 0x1F) << 6 | u16::from(*bytes.get(index + 1)?) & 0x3F,
                    2,
                ),
                _ => (
                    (first & 0x0F) << 12
                        | (u16::from(*bytes.get(index + 1)?) & 0x3F) << 6
                        | u16::from(*bytes.get(index + 2)?) & 0x3F,
                    3,
                ),
            };
            units.push(unit);
            index += width;
        }
        Some(String::from_utf16_lossy(&units))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_library_files_case_insensitively() {
        let content = puml_resource("C4/C4_Container").expect("C4_Container is bundled");
        assert!(String::from_utf8_lossy(&content).contains("Container"));
        assert_eq!(puml_resource("c4/c4_container.puml"), Some(content));
    }

    #[test]
    fn unknown_libraries_and_files_are_absent() {
        assert!(puml_resource("nosuchlib/file").is_none());
        assert!(puml_resource("c4/nosuchfile").is_none());
        assert!(puml_resource("nofolder").is_none());
    }

    #[test]
    fn modified_utf8_decodes_nul_and_surrogate_pairs() {
        let bytes = [0, 8, 0xC0, 0x80, 0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80];
        let mut input = DataInput {
            data: &bytes,
            position: 0,
        };
        assert_eq!(input.read_utf().as_deref(), Some("\0😀"));
    }

    #[test]
    fn stdlib_sprites_hold_two_rows_a_byte() {
        let sprite = StdlibSprite {
            width: 2,
            height: 3,
            data: vec![0x12, 0x34, 0x56, 0x78],
        }
        .decode();
        let rows: Vec<Vec<usize>> = (0..3)
            .map(|y| (0..2).map(|x| sprite.get_gray(x, y)).collect())
            .collect();
        assert_eq!(rows, [[1, 3], [2, 4], [5, 7]]);
    }

    #[test]
    fn sprites_are_found_by_their_exported_name() {
        let office = Stdlib::retrieve("office").expect("the office library is bundled");
        assert!(
            office
                .read_sprite("servers/database_server:database_server")
                .is_some()
        );
        assert!(office.read_sprite("servers/no_such_server").is_none());
        let archimate = Stdlib::retrieve("archimate").expect("the archimate library is bundled");
        assert!(
            archimate
                .read_svg_sprite("archimatesprites:application-component-svg")
                .is_some()
        );
    }
}
