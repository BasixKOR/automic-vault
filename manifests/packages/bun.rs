use semver::Version;

use crate::vendor::{
    InstallStrategy, VendorEntry, github_latest_tag, github_release_url, parse_semver,
};

pub static ENTRY: VendorEntry = VendorEntry {
    name: "bun",
    dependencies: None,
    executables,
    version,
    download_url: Some(download_url),
    install,
};

pub fn executables() -> &'static [&'static str] {
    &["bun"]
}

pub fn version() -> Result<Version, String> {
    let tag = github_latest_tag("oven-sh/bun")?;
    parse_version(&tag)
}

pub fn parse_version(version: &str) -> Result<Version, String> {
    parse_semver(version.strip_prefix("bun-v").unwrap_or(version), "bun")
}

pub fn download_url(version: &Version) -> String {
    github_release_url(
        "oven-sh/bun",
        &format!("bun-v{version}"),
        "bun-darwin-aarch64.zip",
    )
}

pub fn install(_version: &Version) -> InstallStrategy {
    InstallStrategy::CopyFile {
        source: "bun-darwin-aarch64/bun".to_string(),
        destination_dir: "bin".to_string(),
        destination_name: None,
        mode: 0o755,
        create_dirs: vec!["bin".to_string()],
    }
}
