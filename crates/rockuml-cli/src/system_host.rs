//! The engine's view of the real machine: files, environment variables and the clock.

use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rockuml::host::{Graphviz, Host};

use crate::graphviz::{AbstractGraphviz, TIMEOUT_MS, getenv_graphviz_dot};

#[derive(Default)]
pub(crate) struct SystemHost {
    /// `-DPLANTUML_LIMIT_SIZE=...`, which PlantUML sets as a system property, ahead of the environment.
    limit_size: Option<String>,
    /// The `dot` the command line names, ahead of the environment's `GRAPHVIZ_DOT`.
    graphviz_dot: Option<String>,
    /// `--graphviz-timeout`.
    graphviz_timeout: Option<Duration>,
}

impl SystemHost {
    pub(crate) fn new(
        limit_size: Option<String>,
        graphviz_dot: Option<String>,
        graphviz_timeout: Option<Duration>,
    ) -> Self {
        Self {
            limit_size,
            graphviz_dot,
            graphviz_timeout,
        }
    }
}

impl Host for SystemHost {
    fn read_file(&self, path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn read_url(&self, url: &str) -> Option<Vec<u8>> {
        if resolves_to_private_network(url) {
            return None;
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into();
        agent.get(url).call().ok()?.body_mut().read_to_vec().ok()
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
        match &self.limit_size {
            Some(limit_size) if name == crate::cli_options::LIMIT_SIZE => Some(limit_size.clone()),
            _ => std::env::var(name).ok(),
        }
    }

    fn current_time_millis(&self) -> i64 {
        millis_since_epoch(SystemTime::now())
    }

    fn local_time_zone(&self) -> Option<String> {
        jiff::tz::TimeZone::system().iana_name().map(str::to_owned)
    }

    fn create_for_system_dot(&self, dot_string: &str, format: &str) -> Option<Box<dyn Graphviz>> {
        let getenv_graphviz_dot = getenv_graphviz_dot(self.graphviz_dot.as_deref());
        Some(Box::new(AbstractGraphviz::new(
            getenv_graphviz_dot,
            dot_string,
            format,
            self.graphviz_timeout.unwrap_or(TIMEOUT_MS),
        )))
    }
}

/// Like PlantUML, diagrams may not reach into loopback, link-local or private networks.
fn resolves_to_private_network(url: &str) -> bool {
    let Some(host) = rockuml::url_policy::host_of(url) else {
        return true;
    };
    let Ok(addresses) = (host, 0).to_socket_addrs() else {
        return true;
    };
    addresses.into_iter().any(|address| match address.ip() {
        IpAddr::V4(ip) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            let site_local = ip.segments()[0] & 0xffc0 == 0xfec0;
            ip.is_loopback() || ip.is_unspecified() || ip.is_unicast_link_local() || site_local
        }
    })
}

pub(crate) fn millis_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
    })
}
