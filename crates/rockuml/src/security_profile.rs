//! How much a diagram may reach outside itself, as `PLANTUML_SECURITY_PROFILE` chooses (PlantUML's
//! `SecurityProfile`).

use crate::host::Host;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SecurityProfile {
    /// No files and no URLs.
    Sandbox,
    Allowlist,
    Internet,
    InternetWithDotSvg,
    /// The default: no system folders.
    Legacy,
    /// Everything.
    Insecure,
}

impl SecurityProfile {
    pub(crate) fn init(host: &dyn Host) -> Self {
        let profile = host.getenv("PLANTUML_SECURITY_PROFILE");
        match profile.map(|profile| profile.to_uppercase()).as_deref() {
            Some("SANDBOX") => Self::Sandbox,
            Some("ALLOWLIST") => Self::Allowlist,
            Some("INTERNET") => Self::Internet,
            Some("INTERNET_WITH_DOTSVG") => Self::InternetWithDotSvg,
            // PlantUML accepts its old misspelling too.
            Some("INSECURE" | "UNSECURE") => Self::Insecure,
            _ => Self::Legacy,
        }
    }
}
