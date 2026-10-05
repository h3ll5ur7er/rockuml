//! The engine's view of the real machine: files, environment variables and the clock.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rockuml::host::Host;

pub struct SystemHost;

impl Host for SystemHost {
    fn read_file(&self, path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn current_directory(&self) -> PathBuf {
        std::env::current_dir().unwrap_or_default()
    }

    fn home_directory(&self) -> Option<PathBuf> {
        std::env::home_dir()
    }

    fn getenv(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn current_time_millis(&self) -> i64 {
        millis_since_epoch(SystemTime::now())
    }

    fn local_time_zone(&self) -> Option<String> {
        jiff::tz::TimeZone::system().iana_name().map(str::to_owned)
    }
}

pub fn millis_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
}
