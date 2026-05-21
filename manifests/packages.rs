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
#[path = "packages/terraform.rs"]
pub mod terraform;

pub static PACKAGES: &[&VendorEntry] = &[&bun::ENTRY, &terraform::ENTRY];

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
    use std::io::{Read, Write};
    use std::sync::Arc;
    use std::thread;

    struct TestEndpointGuard;

    impl TestEndpointGuard {
        fn set(overrides: crate::config::TestEndpointOverrides) -> Self {
            crate::config::set_test_endpoint_overrides(overrides);
            Self
        }
    }

    impl Drop for TestEndpointGuard {
        fn drop(&mut self) {
            crate::config::clear_test_endpoint_overrides();
        }
    }

    fn start_test_http_server(
        routes: Vec<(String, u16, Vec<u8>)>,
        request_count: usize,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let routes = Arc::new(routes.into_iter().collect::<Vec<_>>());
        let handle = thread::spawn(move || {
            for _ in 0..request_count {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_string();
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap();
                let (status_code, body) = routes
                    .iter()
                    .find(|(route, _, _)| route == path)
                    .map(|(_, status, body)| (*status, body.clone()))
                    .unwrap_or((404, Vec::new()));
                let status_text = match status_code {
                    200 => "200 OK",
                    404 => "404 Not Found",
                    500 => "500 Internal Server Error",
                    _ => "400 Bad Request",
                };
                let response = format!(
                    "HTTP/1.1 {status_text}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
                stream.flush().unwrap();
            }
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn vendor_registry_contains_all_packages() {
        let mut names = PACKAGES.iter().map(|entry| entry.name).collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, vec!["bun", "terraform"]);
    }

    #[test]
    fn vendor_packages_expose_executables() {
        assert_eq!(get("bun").unwrap().executables, ["bun"]);
        assert_eq!(get("terraform").unwrap().executables, ["terraform"]);
    }

    #[test]
    fn vendor_packages_return_semver() {
        assert_eq!(
            bun::parse_version("bun-v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
        assert_eq!(
            terraform::parse_version("v1.2.3").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
    }

    #[test]
    fn vendor_packages_compute_download_urls_in_code() {
        let bun = get("bun").unwrap();
        let bun_version = Version::parse("1.2.3").unwrap();
        let terraform = get("terraform").unwrap();
        let terraform_version = Version::parse("1.2.3").unwrap();

        assert_eq!(
            bun.download_url.unwrap()(&bun_version),
            "https://github.com/oven-sh/bun/releases/download/bun-v1.2.3/bun-darwin-aarch64.zip"
        );
        assert_eq!(
            terraform.download_url.unwrap()(&terraform_version),
            "https://releases.hashicorp.com/terraform/1.2.3/terraform_1.2.3_darwin_arm64.zip"
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

    #[test]
    fn terraform_installs_platform_binary_from_archive_root() {
        let version = Version::parse("1.2.3").unwrap();
        let strategy = terraform::install(&version);
        match strategy {
            InstallStrategy::CopyFile {
                source,
                destination_dir,
                destination_name,
                mode,
                create_dirs,
            } => {
                assert_eq!(source, "terraform");
                assert_eq!(destination_dir, "bin");
                assert_eq!(destination_name, None);
                assert_eq!(mode, 0o755);
                assert_eq!(create_dirs, vec!["bin".to_string()]);
            }
            _ => panic!("terraform should install from the extracted archive root"),
        }
    }

    #[test]
    fn vendor_registry_http_helpers_cover_success_paths() {
        let _env_lock = crate::global_test_env_lock().lock().unwrap();
        let package_metadata = br#"{
            "dist-tags": {"latest": "1.2.3"},
            "versions": {
                "1.2.3": {
                    "dist": {"tarball": "https://example.test/archive.tgz"},
                    "dependencies": {"node": "^22.0.0"}
                },
                "1.2.4-beta.1": {
                    "dist": {"tarball": "https://example.test/beta.tgz"}
                },
                "1.2.2": {
                    "dist": {"tarball": "https://example.test/older.tgz"}
                }
            }
        }"#
        .to_vec();
        let release = br#"{"tag_name":"bun-v1.2.3"}"#.to_vec();
        let (server_root, handle) = start_test_http_server(
            vec![
                (
                    "/repos/oven-sh/bun/releases/latest".to_string(),
                    200,
                    release,
                ),
                ("/bun".to_string(), 200, package_metadata),
            ],
            6,
        );

        let _endpoints = TestEndpointGuard::set(crate::config::TestEndpointOverrides {
            github_api_root: Some(server_root.clone()),
            npm_registry_root: Some(server_root.clone()),
            ..Default::default()
        });
        assert_eq!(
            github_latest_tag("oven-sh/bun").unwrap(),
            "bun-v1.2.3".to_string()
        );
        assert_eq!(npm_latest_tag("bun").unwrap(), "1.2.3");
        assert_eq!(
            npm_tarball_url("bun", &Version::parse("1.2.3").unwrap()).unwrap(),
            "https://example.test/archive.tgz"
        );
        assert_eq!(
            npm_versions_desc("bun").unwrap(),
            vec![
                Version::parse("1.2.3").unwrap(),
                Version::parse("1.2.2").unwrap(),
            ]
        );
        assert_eq!(
            npm_dependency_constraint("bun", &Version::parse("1.2.3").unwrap(), "node").unwrap(),
            Some("^22.0.0".to_string())
        );
        assert_eq!(
            npm_dependency_constraint("bun", &Version::parse("1.2.2").unwrap(), "node").unwrap(),
            None
        );

        handle.join().unwrap();
    }

    #[test]
    fn vendor_registry_http_helpers_cover_error_paths() {
        let _env_lock = crate::global_test_env_lock().lock().unwrap();
        let invalid_json = b"{".to_vec();
        let package_metadata = br#"{
            "dist-tags": {"latest": "1.2.3"},
            "versions": {
                "1.2.3": {"dist": {"tarball": "https://example.test/archive.tgz"}}
            }
        }"#
        .to_vec();
        let (server_root, handle) = start_test_http_server(
            vec![
                (
                    "/repos/oven-sh/bun/releases/latest".to_string(),
                    500,
                    Vec::new(),
                ),
                ("/broken".to_string(), 200, invalid_json),
                ("/bun".to_string(), 200, package_metadata),
            ],
            3,
        );

        let _endpoints = TestEndpointGuard::set(crate::config::TestEndpointOverrides {
            github_api_root: Some(server_root.clone()),
            npm_registry_root: Some(server_root.clone()),
            ..Default::default()
        });
        let err = github_latest_tag("oven-sh/bun").unwrap_err();
        assert!(err.contains("failed to fetch latest release for oven-sh/bun: http 500"));

        let err = npm_latest_tag("broken").unwrap_err();
        assert!(err.contains("failed to fetch npm metadata for broken"));

        let err = npm_tarball_url("bun", &Version::parse("9.9.9").unwrap()).unwrap_err();
        assert!(err.contains("missing version 9.9.9"));

        assert_eq!(
            parse_semver("1.2.3", "bun").unwrap(),
            Version::parse("1.2.3").unwrap()
        );
        assert!(parse_semver("not-semver", "bun").is_err());

        handle.join().unwrap();
    }
}
