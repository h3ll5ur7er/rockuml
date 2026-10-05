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

pub struct Stdlib {
    name: String,
    info: HashMap<String, String>,
    puml: RefCell<Option<HashMap<String, Vec<u8>>>>,
    json: RefCell<Option<HashMap<String, Vec<u8>>>>,
}

thread_local! {
    static LIBRARIES: RefCell<HashMap<String, Option<Rc<Stdlib>>>> = RefCell::default();
}

impl Stdlib {
    /// The library with this folder name, following `link=` redirections (e.g. versioned names).
    pub fn retrieve(name: &str) -> Option<Rc<Stdlib>> {
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
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn has_info(&self) -> bool {
        !self.info.is_empty()
    }

    pub fn metadata(&self) -> impl Iterator<Item = (&String, &String)> {
        self.info.iter()
    }

    pub fn metadata_value(&self, key: &str) -> Option<String> {
        self.info.get(key).cloned()
    }

    pub fn version(&self) -> Option<String> {
        self.metadata_value("VERSION").or_else(|| self.metadata_value("version"))
    }

    pub fn source(&self) -> Option<String> {
        self.metadata_value("SOURCE").or_else(|| self.metadata_value("source"))
    }

    /// `file` is the lowercase path inside the library without the `.puml` extension.
    pub fn puml_resource(&self, file: &str) -> Option<Vec<u8>> {
        let mut puml = self.puml.borrow_mut();
        let entries = puml.get_or_insert_with(|| self.read_channel("puml").unwrap_or_default());
        entries.get(file).cloned()
    }

    fn json_resource(&self, file: &str) -> Option<Vec<u8>> {
        let mut json = self.json.borrow_mut();
        let entries = json.get_or_insert_with(|| self.read_channel("json").unwrap_or_default());
        entries.get(file).cloned()
    }

    fn read_channel(&self, channel: &str) -> Option<HashMap<String, Vec<u8>>> {
        let data = decompress(assets::get(&format!("stdlib/{}/{channel}.spm", self.name))?)?;
        let mut input = DataInput { data: &data, position: 0 };
        let count = input.read_int()?;
        let mut entries = HashMap::new();
        for _ in 0..count {
            let name = input.read_utf()?.to_lowercase();
            let length = usize::try_from(input.read_int()?).ok()?;
            entries.insert(name, input.read_bytes(length)?.to_vec());
        }
        Some(entries)
    }
}

/// A `.puml` file of the standard library, named like `c4/C4_Container` or `c4/C4_Container.puml`.
pub fn puml_resource(full_name: &str) -> Option<Vec<u8>> {
    let full_name = full_name.to_lowercase().replace(".puml", "");
    let (library, file) = full_name.split_once('/')?;
    let library = Stdlib::retrieve(library)?;
    if !library.has_info() {
        return None;
    }
    library.puml_resource(file)
}

/// The folder names of all libraries, sorted.
pub fn library_names() -> Vec<String> {
    let mut names: Vec<String> = assets::get("stdlib/home.spm")
        .map(|home| String::from_utf8_lossy(home).lines().map(str::to_owned).collect())
        .unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

/// A JSON file of the standard library, for `%load_json(<library/file>)`. PlantUML crashes when the
/// library exists but the file does not.
pub fn json_resource(full_name: &str) -> Result<Option<Vec<u8>>, RuntimeException> {
    let Some((library, file)) = full_name.split_once('/') else {
        return Ok(None);
    };
    let Some(library) = Stdlib::retrieve(library).filter(|library| library.has_info()) else {
        return Ok(None);
    };
    library.json_resource(file).map(Some).ok_or(RuntimeException)
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
                0xC0..=0xDF => ((first & 0x1F) << 6 | u16::from(*bytes.get(index + 1)?) & 0x3F, 2),
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
        let mut input = DataInput { data: &bytes, position: 0 };
        assert_eq!(input.read_utf().as_deref(), Some("\0😀"));
    }
}
