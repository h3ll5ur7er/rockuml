//! The engine's view of a web page: no files, URLs or environment, and the clock of the JavaScript host.

use std::path::{Path, PathBuf};

use rockuml::host::Host;

/// IANA time zone names are short; longer ones are cut off and so unknown, which means UTC.
const TIME_ZONE_CAPACITY: usize = 64;

#[allow(unsafe_code)]
mod imports {
    #[link(wasm_import_module = "rockuml")]
    unsafe extern "C" {
        /// Milliseconds since the epoch, like `Date.now()`.
        pub(super) safe fn current_time_millis() -> f64;
        /// Writes the UTF-8 IANA name of the local time zone to `buffer` and returns its length, or 0 when
        /// it is unknown or longer than `capacity`.
        pub(super) unsafe fn local_time_zone(buffer: *mut u8, capacity: usize) -> usize;
    }
}

pub(crate) struct WasmHost;

impl Host for WasmHost {
    fn read_file(&self, _path: &Path) -> Option<Vec<u8>> {
        None
    }

    fn read_url(&self, _url: &str) -> Option<Vec<u8>> {
        None
    }

    fn file_exists(&self, _path: &Path) -> bool {
        false
    }

    fn current_directory(&self) -> PathBuf {
        PathBuf::new()
    }

    fn home_directory(&self) -> Option<PathBuf> {
        None
    }

    fn getenv(&self, _name: &str) -> Option<String> {
        None
    }

    fn current_time_millis(&self) -> i64 {
        imports::current_time_millis() as i64
    }

    fn local_time_zone(&self) -> Option<String> {
        let mut buffer = [0; TIME_ZONE_CAPACITY];
        #[allow(unsafe_code)]
        // SAFETY: the import writes at most `capacity` bytes to the buffer.
        let length = unsafe { imports::local_time_zone(buffer.as_mut_ptr(), buffer.len()) };
        let name = std::str::from_utf8(buffer.get(..length)?).ok()?;
        (!name.is_empty()).then(|| name.to_owned())
    }
}
