use semver::Version;

use crate::vendor::{InstallStrategy, VendorEntry, github_latest_tag, parse_semver};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "terraform",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["terraform"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("hashicorp/terraform")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix('v').unwrap_or(version), "terraform")
}

pub fn download_url(version: &Version) -> String {
    format!(
        "https://releases.hashicorp.com/terraform/{version}/terraform_{version}_darwin_arm64.zip"
    )
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: "terraform".to_string(),
        destination_dir: "bin".to_string(),
        destination_name: None,
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
