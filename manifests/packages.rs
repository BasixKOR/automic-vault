use std::collections::HashMap;
use std::io::Read;

use semver::Version;
use serde::Deserialize;
use ureq::Error as UreqError;

pub struct VendorPackage {
    pub name: &'static str,
    pub dependencies: &'static [&'static str],
    pub executables: &'static [&'static str],
    pub version: fn() -> Result<Version, String>,
    pub download_url: Option<fn(&Version) -> String>,
    pub install: fn(&Version) -> InstallStrategy,
}

#[derive(Clone)]
pub enum InstallStrategy {
    NpmGlobal {
        package: String,
    },
    CopyFile {
        source: String,
        destination_dir: String,
        destination_name: Option<String>,
        mode: u32,
        create_dirs: Vec<String>,
    },
    CopyTree {
        source: String,
    },
}

pub struct VendorEntry {
    pub name: &'static str,
    pub dependencies: Option<fn() -> &'static [&'static str]>,
    pub executables: fn() -> &'static [&'static str],
    pub version: fn() -> Result<Version, String>,
    pub download_url: Option<fn(&Version) -> String>,
    pub install: fn(&Version) -> InstallStrategy,
}

impl VendorEntry {
    pub fn package(&self) -> VendorPackage {
        VendorPackage {
            name: self.name,
            dependencies: self.dependencies.map(|f| f()).unwrap_or(&[]),
            executables: (self.executables)(),
            version: self.version,
            download_url: self.download_url,
            install: self.install,
        }
    }
}

pub fn github_release_url(repo: &str, tag: &str, asset: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
}

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn github_api_root() -> String {
    crate::config::github_api_root()
}

fn npm_registry_root() -> String {
    crate::config::npm_registry_root()
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

#[derive(Deserialize)]
struct NpmPackageVersion {
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
    versions: HashMap<String, NpmPublishedVersion>,
}

#[derive(Deserialize)]
struct NpmDistTags {
    latest: String,
}

#[derive(Deserialize)]
struct NpmPublishedVersion {
    dist: NpmDist,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct NpmDist {
    tarball: String,
}

pub fn github_latest_tag(repo: &str) -> Result<String, String> {
    let url = format!("{}/repos/{repo}/releases/latest", github_api_root());
    let release: GithubRelease =
        fetch_json(&url, &format!("failed to fetch latest release for {repo}"))?;
    Ok(release.tag_name)
}

pub fn npm_latest_tag(package: &str) -> Result<String, String> {
    let url = format!("{}/{}", npm_registry_root(), urlencoding::encode(package));
    let package: NpmPackageVersion =
        fetch_json(&url, &format!("failed to fetch npm metadata for {package}"))?;
    Ok(package.dist_tags.latest)
}

pub fn npm_tarball_url(package: &str, version: &Version) -> Result<String, String> {
    let url = format!("{}/{}", npm_registry_root(), urlencoding::encode(package));
    let metadata: NpmPackageVersion = fetch_json(
        &url,
        &format!("failed to fetch npm metadata for {package}@{version}"),
    )?;
    metadata
        .versions
        .get(&version.to_string())
        .map(|published| published.dist.tarball.clone())
        .ok_or_else(|| format!("npm metadata for {package} is missing version {version}"))
}

pub fn npm_versions_desc(package: &str) -> Result<Vec<Version>, String> {
    let url = format!("{}/{}", npm_registry_root(), urlencoding::encode(package));
    let metadata: NpmPackageVersion =
        fetch_json(&url, &format!("failed to fetch npm metadata for {package}"))?;
    let mut versions = metadata
        .versions
        .keys()
        .filter_map(|version| Version::parse(version).ok())
        .filter(|version| version.pre.is_empty())
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| right.cmp(left));
    versions.dedup();
    Ok(versions)
}

pub fn npm_dependency_constraint(
    package: &str,
    version: &Version,
    dependency: &str,
) -> Result<Option<String>, String> {
    let url = format!("{}/{}", npm_registry_root(), urlencoding::encode(package));
    let metadata: NpmPackageVersion = fetch_json(
        &url,
        &format!("failed to fetch npm metadata for {package}@{version}"),
    )?;
    Ok(metadata
        .versions
        .get(&version.to_string())
        .and_then(|published| published.dependencies.as_ref())
        .and_then(|dependencies| dependencies.get(dependency).cloned()))
}

pub fn parse_semver(version: &str, context: &str) -> Result<Version, String> {
    Version::parse(version)
        .map_err(|err| format!("failed to parse semver {version} for {context}: {err}"))
}

fn fetch_json<T>(url: &str, context: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| match err {
            UreqError::Status(code, _) => format!("{context}: http {code}"),
            UreqError::Transport(err) => format!("{context}: {err}"),
        })?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("{context}: {err}"))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("{context}: {err}"))
}

#[path = "packages/bun.rs"]
pub mod bun;

pub static PACKAGES: &[&VendorEntry] = &[&bun::ENTRY];

pub fn get(name: &str) -> Option<VendorPackage> {
    PACKAGES
        .iter()
        .copied()
        .find(|entry| entry.name == name)
        .map(VendorEntry::package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_registry_contains_all_packages() {
        let mut names = PACKAGES.iter().map(|entry| entry.name).collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, vec!["bun"]);
    }

    #[test]
    fn vendor_packages_expose_executables() {
        assert_eq!(get("bun").unwrap().executables, ["bun"]);
    }

    #[test]
    fn vendor_packages_return_semver() {
        assert_eq!(
            bun::parse_version("bun-v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
    }

    #[test]
    fn vendor_packages_compute_download_urls_in_code() {
        let bun = get("bun").unwrap();
        let bun_version = Version::parse("1.2.3").unwrap();

        assert_eq!(
            bun.download_url.unwrap()(&bun_version),
            "https://github.com/oven-sh/bun/releases/download/bun-v1.2.3/bun-darwin-aarch64.zip"
        );
    }

    #[test]
    fn bun_installs_platform_binary_from_archive_subdirectory() {
        let version = Version::parse("1.2.3").unwrap();
        let strategy = bun::install(&version);
        match strategy {
            InstallStrategy::CopyFile {
                source,
                destination_dir,
                destination_name,
                mode,
                create_dirs,
            } => {
                assert_eq!(source, "bun-darwin-aarch64/bun");
                assert_eq!(destination_dir, "bin");
                assert_eq!(destination_name, None);
                assert_eq!(mode, 0o755);
                assert_eq!(create_dirs, vec!["bin".to_string()]);
            }
            _ => panic!("bun should install from the extracted archive directory"),
        }
    }
}
