//! Everything the engine needs from the outside world. The engine performs no I/O of its own so that it
//! also runs in WebAssembly; the command line provides a file-system implementation.

use std::path::{Path, PathBuf};

pub trait Host {
    fn read_file(&self, path: &Path) -> Option<Vec<u8>>;
    /// Called only for URLs `url_policy` allows; `None` when the URL cannot be fetched.
    fn read_url(&self, url: &str) -> Option<Vec<u8>>;
    fn file_exists(&self, path: &Path) -> bool;
    /// What relative paths resolve against outside any diagram file, like Java's `Paths.get("")`.
    fn current_directory(&self) -> PathBuf;
    fn home_directory(&self) -> Option<PathBuf>;
    fn getenv(&self, name: &str) -> Option<String>;
    fn current_time_millis(&self) -> i64;
    /// IANA name such as `Europe/Zurich`; `None` means UTC.
    fn local_time_zone(&self) -> Option<String>;
}

/// A host without files, environment or clock, for embedding and tests.
#[derive(Default)]
pub struct IsolatedHost;

impl Host for IsolatedHost {
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
        0
    }

    fn local_time_zone(&self) -> Option<String> {
        None
    }
}
