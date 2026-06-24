use std::collections::{HashMap, HashSet};
use std::env;
#[cfg(target_os = "macos")]
use std::ffi::CString;
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, IsTerminal, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{self, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::TempDir;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ureq::Error as UreqError;
use walkdir::WalkDir;

mod brew;
mod cask;
mod catalog;
mod cli_help;
mod config;
mod core;
mod dotenv;
mod gate;
mod npm;
mod ops;
mod package;
mod pip;
mod protocol;
mod scanner;
mod script_resolution;
mod state;
mod transfer;
#[path = "../../../manifests/packages.rs"]
pub mod vendor;

mod cli;
mod info;
mod install;
mod isotope;
mod trace;
#[allow(clippy::all, dead_code, unused_parens, unused_variables)]
mod isotope_integrations {
    include!(concat!(env!("OUT_DIR"), "/isotope_integrations.rs"));
}
mod stubs;
mod vault;

pub use catalog::refresh_remote_combined_data;
pub(crate) use catalog::*;
pub(crate) use cli::*;
pub use cli::{main_entry, scanner_main_entry};
pub(crate) use cli_help::*;
pub(crate) use config::{
    formula_api_root, homebrew_debug_allowance_enabled, install_requires_root, managed_bin_root,
    opt_npm_root, opt_pip_root, opt_pkg_root, pypi_root,
};
pub(crate) use core::*;
pub use dotenv::{DotenvApprovalMode, DotenvApprovalPolicy, DotenvRunProvenance};
pub(crate) use info::*;
pub(crate) use install::*;
pub use isotope::isotope_main_entry;
pub(crate) use isotope::*;
pub use ops::{
    HelperCommand, HelperCommandResult, HelperCommandSuccess, PackageSpec, ProgressEvent,
    check_for_updates, execute_helper_command, verify_helper_codesign_identity,
};
pub(crate) use package::*;
pub(crate) use scanner::*;
pub(crate) use stubs::*;
pub(crate) use trace::*;
pub use vault::vault_main_entry;
pub use vault::{
    DotenvKeychainDeleteRequest, DotenvKeychainDeleteResponse, DotenvKeychainLoadRequest,
    DotenvKeychainLoadResponse, DotenvKeychainStoreRequest, DotenvKeychainStoreResponse,
    ExecutionIntent, KeyTransferApprovalItem, KeyTransferApprovalRequest,
    KeyTransferApprovalSource, KeyTransferImportItem, KeyTransferImportRequest,
    KeyTransferImportResponse, VaultApprovalRequest, VaultApprovalResponse, VaultClientRequest,
    VaultContainmentSession, VaultDaemonEvent, VaultExecChunk, VaultExecCompletion,
    VaultExecutionEnvironment, VaultProcessSnapshot, VaultToolAlias, VaultToolchainManifest,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vendor::{bun, get, github_release_url, parse_semver};
    use semver::Version;

    fn test_db(entries: &[(&str, &str)]) -> Db {
        Db {
            schema: DB_SCHEMA_VERSION,
            generated_at: String::new(),
            entries: entries
                .iter()
                .map(|(tool, formula)| (tool.to_string(), formula.to_string()))
                .collect(),
            formulas: HashMap::new(),
            casks: HashMap::new(),
            npms: HashMap::new(),
        }
    }

    fn write_executable(path: &Path) {
        write_executable_with_body(path, "")
    }

    fn write_executable_with_body(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    fn fixed_i_plan(package_name: &str, root_formula: &str) -> InstallPlan {
        InstallPlan {
            mode: Mode::I,
            package_name: package_name.to_string(),
            root_formula: root_formula.to_string(),
            stable_root: PathBuf::from("/opt").join(package_name),
            install_root: PathBuf::from("/opt").join(package_name),
            tmp_root: PathBuf::from("/opt/.tmp"),
        }
    }

    fn formula_info(post_install_defined: bool) -> FormulaInfo {
        FormulaInfo {
            desc: String::new(),
            homepage: String::new(),
            license: None,
            versions: FormulaVersions::default(),
            revision: 0,
            dependencies: Vec::new(),
            bottle: Bottle {
                stable: Some(BottleStable {
                    files: HashMap::new(),
                }),
            },
            disabled: false,
            post_install_defined,
        }
    }

    fn formula_index_entry(name: &str, aliases: &[&str], oldnames: &[&str]) -> FormulaIndexEntry {
        FormulaIndexEntry {
            name: name.to_string(),
            summary: String::new(),
            aliases: aliases.iter().map(|value| value.to_string()).collect(),
            oldnames: oldnames.iter().map(|value| value.to_string()).collect(),
            category: String::new(),
            homepage: String::new(),
            repository: String::new(),
            upstream_docs: String::new(),
            docs: Vec::new(),
            popularity: None,
            last_updated_at: None,
            pulse_kind: None,
        }
    }

    fn package_search_result(
        package_name: &str,
        source: PackageReceiptSource,
        summary: Option<&str>,
        rank: Option<u32>,
    ) -> PackageSearchResult {
        PackageSearchResult {
            package_name: package_name.to_string(),
            source,
            summary: summary.map(str::to_string),
            latest_version: None,
            homepage: None,
            repository: None,
            upstream_docs: None,
            docs: Vec::new(),
            category: None,
            dependencies: Vec::new(),
            install_package_names: Vec::new(),
            security_state: None,
            rank,
            last_updated_at: None,
            pulse_kind: None,
        }
    }

    #[test]
    fn formula_metadata_decodes_repo_alias_as_repository() {
        let metadata: EmbeddedFormulaMetadata =
            serde_json::from_str(r#"{"repo":"https://github.com/astral-sh/uv"}"#).unwrap();
        assert_eq!(metadata.repository, "https://github.com/astral-sh/uv");

        let entry: FormulaIndexEntry =
            serde_json::from_str(r#"{"name":"uv","repo":"https://github.com/astral-sh/uv"}"#)
                .unwrap();
        assert_eq!(entry.repository, "https://github.com/astral-sh/uv");
    }

    #[test]
    fn cask_metadata_tolerates_listing_only_rows() {
        let metadata: EmbeddedCaskMetadata = serde_json::from_str(
            r#"{
              "aliases": ["op"],
              "binaries": [{"source": "op", "target": "op"}],
              "homepage": "https://developer.1password.com/docs/cli",
              "summary": "Command-line interface for 1Password"
            }"#,
        )
        .unwrap();

        assert_eq!(metadata.summary, "Command-line interface for 1Password");
        assert!(metadata.url.is_empty());
        assert!(metadata.sha256.is_empty());
        assert!(metadata.version.is_empty());
        assert!(
            ensure_cask_install_metadata("1password-cli", &metadata)
                .unwrap_err()
                .contains("missing version metadata")
        );
    }

    #[test]
    fn resolve_i_root_formula_keeps_ffmpeg() {
        let db = test_db(&[]);
        assert_eq!(
            resolve_i_root_package_with_db("ffmpeg", &db, |_| Ok(true)).unwrap(),
            EmbeddedPackage::Formula("ffmpeg".to_string())
        );
    }

    #[test]
    fn resolve_i_root_formula_keeps_imagemagick() {
        let db = test_db(&[]);
        assert_eq!(
            resolve_i_root_package_with_db("imagemagick", &db, |_| Ok(true)).unwrap(),
            EmbeddedPackage::Formula("imagemagick".to_string())
        );
    }

    #[test]
    fn resolve_i_root_formula_uses_executable_mapping_when_no_formula_exists() {
        let db = test_db(&[("zopflipng", "zopfli")]);
        assert_eq!(
            resolve_i_root_package_with_db("zopflipng", &db, |_| Ok(false)).unwrap(),
            EmbeddedPackage::Formula("zopfli".to_string())
        );
    }

    #[test]
    fn formula_install_package_name_uses_canonical_provider_name() {
        let aliases = HashMap::from([("protoc".to_string(), "protobuf".to_string())]);

        assert_eq!(
            formula_install_package_name_with_aliases("protoc", &aliases),
            "protobuf"
        );
        assert_eq!(
            formula_install_package_name_with_aliases("protobuf", &aliases),
            "protobuf"
        );
    }

    #[test]
    fn resolve_i_root_formula_rejects_ambiguous_package_and_executable_names() {
        let db = test_db(&[("foo", "bar")]);
        assert_eq!(
            resolve_i_root_package_with_db("foo", &db, |_| Ok(true)),
            Err(
                "ambiguous install target 'foo': use `brew:foo` for the Homebrew \
package or `bar` for the package that provides the `foo` executable"
                    .to_string()
            )
        );
    }

    #[test]
    fn resolve_i_root_formula_keeps_exact_formula_name_when_executable_matches() {
        let db = test_db(&[("ripgrep", "ripgrep")]);
        assert_eq!(
            resolve_i_root_package_with_db("ripgrep", &db, |_| Ok(true)).unwrap(),
            EmbeddedPackage::Formula("ripgrep".to_string())
        );
    }

    #[test]
    fn resolve_i_root_package_supports_cask_providers() {
        let db = test_db(&[("codex", "cask:codex")]);
        assert_eq!(
            resolve_i_root_package_with_db("codex", &db, |_| Ok(false)).unwrap(),
            EmbeddedPackage::Cask("codex".to_string())
        );
    }

    #[test]
    fn resolve_i_root_package_supports_npm_providers() {
        let db = test_db(&[("tsx", "npm:tsx")]);
        assert_eq!(
            resolve_i_root_package_with_db("tsx", &db, |_| Ok(false)).unwrap(),
            EmbeddedPackage::NpmPackage("tsx".to_string())
        );
    }

    #[test]
    fn resolve_i_root_package_supports_scoped_npm_providers() {
        let db = test_db(&[("scoped-tool", "npm:@scope/scoped-tool")]);
        assert_eq!(
            resolve_i_root_package_with_db("scoped-tool", &db, |_| Ok(false)).unwrap(),
            EmbeddedPackage::NpmPackage("@scope/scoped-tool".to_string())
        );
    }

    #[test]
    fn resolve_i_root_package_rejects_formula_and_npm_executable_ambiguity() {
        let db = test_db(&[("tsx", "npm:tsx")]);
        assert_eq!(
            resolve_i_root_package_with_db("tsx", &db, |_| Ok(true)),
            Err(
                "ambiguous install target 'tsx': use `brew:tsx` for the Homebrew \
package or `npm:tsx` for the package that provides the `tsx` executable"
                    .to_string()
            )
        );
    }

    #[test]
    fn homebrew_executables_from_db_lists_formula_tools_without_prefix() {
        let db = test_db(&[
            ("ffmpeg", "ffmpeg"),
            ("ffplay", "ffmpeg"),
            ("ffprobe", "ffmpeg"),
            ("rg", "ripgrep"),
        ]);
        assert_eq!(
            homebrew_executables_from_db("ffmpeg", &db),
            vec![
                "ffmpeg".to_string(),
                "ffplay".to_string(),
                "ffprobe".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_i_root_package_ignores_unknown_qualified_entry_providers() {
        let db = test_db(&[("future-tool", "future:provider")]);
        assert_eq!(
            resolve_i_root_package_with_db("future-tool", &db, |_| Ok(false)).unwrap(),
            EmbeddedPackage::Formula("future-tool".to_string())
        );
    }

    #[test]
    fn npm_package_executable_name_falls_back_to_install_leaf_name() {
        assert_eq!(
            npm_package_executable_name("unindexed-tool"),
            "unindexed-tool"
        );
        assert_eq!(
            npm_package_executable_name("@scope/unindexed-tool"),
            "unindexed-tool"
        );
    }

    #[test]
    fn collect_formula_aliases_maps_aliases_to_canonical_formula_names() {
        let aliases = collect_formula_aliases(vec![formula_index_entry(
            "python@3.14",
            &["python", "python3"],
            &[],
        )]);

        assert_eq!(
            aliases.get("python").map(String::as_str),
            Some("python@3.14")
        );
        assert_eq!(
            aliases.get("python3").map(String::as_str),
            Some("python@3.14")
        );
    }

    #[test]
    fn collect_formula_aliases_maps_old_names_to_canonical_formula_names() {
        let aliases = collect_formula_aliases(vec![formula_index_entry("foo", &[], &["foo-old"])]);

        assert_eq!(aliases.get("foo-old").map(String::as_str), Some("foo"));
    }

    #[test]
    fn canonical_formula_name_with_aliases_prefers_canonical_formula_name() {
        let aliases = collect_formula_aliases(vec![formula_index_entry(
            "python@3.14",
            &["python", "python3"],
            &[],
        )]);

        assert_eq!(
            canonical_formula_name_with_aliases("python", &aliases),
            "python@3.14"
        );
        assert_eq!(
            canonical_formula_name_with_aliases("python@3.14", &aliases),
            "python@3.14"
        );
    }

    #[test]
    fn mode_from_name_accepts_subcommands_and_aliases() {
        assert_eq!(Mode::from_name("run"), None);
        assert_eq!(Mode::from_name("use"), None);
        assert_eq!(Mode::from_name("x"), None);
        assert_eq!(Mode::from_name("install"), Some(Mode::I));
        assert_eq!(Mode::from_name("i"), Some(Mode::I));
        assert_eq!(Mode::from_name("av"), None);
    }

    #[test]
    fn invocation_from_program_uses_direct_mode_for_renamed_entrypoints() {
        let av = Invocation::from_program(&OsString::from("av"));
        assert_eq!(av.binary_name, "av");
        assert_eq!(av.name, "av");
        assert_eq!(av.mode, None);

        let install_invocation = Invocation::from_program(&OsString::from("install"));
        assert_eq!(install_invocation.mode, Some(Mode::I));

        let i_invocation = Invocation::from_program(&OsString::from("i"));
        assert_eq!(i_invocation.mode, Some(Mode::I));
    }

    #[test]
    fn invocation_for_subcommand_uses_requested_alias_in_display_name() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        assert_eq!(invocation.binary_name, "av");
        assert_eq!(invocation.name, "av i");
        assert_eq!(invocation.mode, Some(Mode::I));
    }

    #[test]
    fn parse_i_request_collects_multiple_packages() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![
                OsString::from("cargo-binstall"),
                OsString::from("cargo-zigbuild"),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![
                    RequestedPackage::Auto("cargo-binstall".to_string()),
                    RequestedPackage::Auto("cargo-zigbuild".to_string()),
                ],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_force_flag() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![
                OsString::from("--force"),
                OsString::from("cargo-binstall"),
                OsString::from("-f"),
                OsString::from("cargo-zigbuild"),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![
                    RequestedPackage::Auto("cargo-binstall".to_string()),
                    RequestedPackage::Auto("cargo-zigbuild".to_string()),
                ],
                force: true,
            })
        );
    }

    #[test]
    fn parse_i_request_rejects_path_separator_in_any_package() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("cargo-binstall"), OsString::from("foo/bar")].into_iter(),
        );

        assert_eq!(
            request,
            Err("package name must not contain path separators".to_string())
        );
    }

    #[test]
    fn parse_i_request_accepts_qualified_homebrew_formula_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("brew:zopflipng")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::HomebrewFormula("zopflipng".to_string(),)],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_qualified_cask_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("cask:codex")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::HomebrewCask("codex".to_string())],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_qualified_isotope_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("isotope:gh")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::Isotope("gh".to_string())],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_unqualified_package_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("clawhub")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::Auto("clawhub".to_string())],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_keeps_unknown_unqualified_package_names_auto() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("qmd")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::Auto("qmd".to_string())],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_qualified_npm_package_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![
                OsString::from("npm:openclaw"),
                OsString::from("npm:@tobilu/qmd"),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![
                    RequestedPackage::NpmPackage {
                        package: "openclaw".to_string(),
                        version: None,
                    },
                    RequestedPackage::NpmPackage {
                        package: "@tobilu/qmd".to_string(),
                        version: None,
                    },
                ],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_versioned_qualified_npm_package_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("npm:openclaw@2026.4.5")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::NpmPackage {
                    package: "openclaw".to_string(),
                    version: Some("2026.4.5".to_string()),
                }],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_accepts_qualified_pip_package_names() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("pip:Psycopg2")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(IRequest {
                packages: vec![RequestedPackage::PipPackage("psycopg2".to_string())],
                force: false,
            })
        );
    }

    #[test]
    fn parse_i_request_rejects_invalid_npm_package_name() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("npm:foo/bar")].into_iter());

        assert_eq!(
            request,
            Err("npm package names must not contain path separators".to_string())
        );
    }

    #[test]
    fn parse_i_request_rejects_invalid_pip_package_name() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("pip:foo/bar")].into_iter());

        assert_eq!(
            request,
            Err("pip package names must not contain path separators".to_string())
        );
    }

    #[test]
    fn parse_i_request_rejects_unsupported_pip_package_characters() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("pip:foo[bar]")].into_iter(),
        );

        assert_eq!(
            request,
            Err(
                "pip package names may only contain ASCII letters, numbers, '.', '-' and '_'"
                    .to_string()
            )
        );
    }

    #[test]
    fn parse_i_request_rejects_empty_qualified_homebrew_formula_name() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request =
            parse_i_request_from_iter(&invocation, vec![OsString::from("brew:")].into_iter());

        assert_eq!(
            request,
            Err("package qualifier 'brew:' is missing a formula name".to_string())
        );
    }

    #[test]
    fn parse_i_request_rejects_additional_slashes_in_qualified_formula_name() {
        let invocation = Invocation::for_subcommand("av", "i", Mode::I);
        let request = parse_i_request_from_iter(
            &invocation,
            vec![OsString::from("brew:foo/bar")].into_iter(),
        );

        assert_eq!(
            request,
            Err("qualified package name must not contain additional path separators".to_string())
        );
    }

    #[test]
    fn parse_uninstall_request_accepts_alias_and_qualified_formula_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("brew:python@3.12")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["python@3.12".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_accepts_qualified_cask_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("cask:codex")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["codex".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_accepts_qualified_npm_package_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("npm:@tobilu/qmd")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["npm:@tobilu/qmd".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_preserves_qualified_isotope_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("isotope:gh")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["isotope:gh".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_accepts_unqualified_package_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("clawhub")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["clawhub".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_uses_homebrew_provider_names_for_executables() {
        let _env_lock = test_env_lock().lock().unwrap();
        let legacy_root = opt_pkg_root().join("rg");
        if fs::symlink_metadata(&legacy_root).is_ok() {
            remove_path(&legacy_root).unwrap();
        }
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request =
            parse_uninstall_request_from_iter(&invocation, vec![OsString::from("rg")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["ripgrep".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_preserves_existing_legacy_executable_root() {
        let _env_lock = test_env_lock().lock().unwrap();
        let opt_root = opt_pkg_root();
        let install_root = opt_root.join("rg");
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_path(&install_root).unwrap();
        }
        fs::create_dir_all(&install_root).unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "rg".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "ripgrep".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request =
            parse_uninstall_request_from_iter(&invocation, vec![OsString::from("rg")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["rg".to_string()],
            })
        );

        remove_path(&install_root).unwrap();
    }

    #[test]
    fn parse_uninstall_request_resolves_unique_installed_isotope_from_stub_name() {
        let _env_lock = test_env_lock().lock().unwrap();
        let opt_root = opt_pkg_root();
        let install_root = opt_root.join("awscli");
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_path(&install_root).unwrap();
        }
        fs::create_dir_all(&install_root).unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "isotope:aws-cli".to_string(),
                version: "2.0.0".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "aws-cli".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_stub_manifest(
            &install_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["aws".to_string()],
            },
        )
        .unwrap();

        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request =
            parse_uninstall_request_from_iter(&invocation, vec![OsString::from("aws")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["isotope:aws-cli".to_string()],
            })
        );

        remove_path(&install_root).unwrap();
    }

    #[test]
    fn parse_uninstall_request_ignores_unknown_installed_isotopes_for_other_names() {
        let _env_lock = test_env_lock().lock().unwrap();
        let opt_root = opt_pkg_root();
        let install_root = opt_root.join(ISOTOPE_INSTALL_ROOT_DIR).join("flyctl");
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_path(&install_root).unwrap();
        }
        fs::create_dir_all(&install_root).unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "isotope:flyctl".to_string(),
                version: "0.3.0".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "flyctl".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_stub_manifest(
            &install_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["flyctl".to_string()],
            },
        )
        .unwrap();

        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request =
            parse_uninstall_request_from_iter(&invocation, vec![OsString::from("uv")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["uv".to_string()],
            })
        );

        remove_path(&install_root).unwrap();
    }

    #[test]
    fn parse_uninstall_request_rejects_ambiguous_installed_names() {
        let _env_lock = test_env_lock().lock().unwrap();
        let opt_root = opt_pkg_root();
        let aws_root = opt_root.join("aws");
        let awscli_root = opt_root.join("awscli");
        for install_root in [&aws_root, &awscli_root] {
            if fs::symlink_metadata(install_root).is_ok() {
                remove_path(install_root).unwrap();
            }
        }
        fs::create_dir_all(&aws_root).unwrap();
        fs::create_dir_all(&awscli_root).unwrap();
        write_package_receipt(
            &aws_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "aws".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "aws".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &awscli_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "isotope:aws-cli".to_string(),
                version: "2.0.0".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "aws-cli".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_stub_manifest(
            &awscli_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["aws".to_string()],
            },
        )
        .unwrap();

        let err = parse_uninstall_package_name(&OsString::from("aws")).unwrap_err();

        assert!(err.contains("package name aws is ambiguous"));
        assert!(err.contains("aws"));
        assert!(err.contains("isotope:aws-cli"));

        remove_path(&aws_root).unwrap();
        remove_path(&awscli_root).unwrap();
    }

    #[test]
    fn parse_uninstall_request_keeps_unknown_unqualified_package_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request =
            parse_uninstall_request_from_iter(&invocation, vec![OsString::from("qmd")].into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["qmd".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_accepts_qualified_pip_package_names() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av rm".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("pip:Psycopg2")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UninstallRequest {
                packages: vec!["pip:psycopg2".to_string()],
            })
        );
    }

    #[test]
    fn parse_uninstall_request_rejects_paths() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av uninstall".to_string(),
            mode: None,
        };
        let request = parse_uninstall_request_from_iter(
            &invocation,
            vec![OsString::from("foo/bar")].into_iter(),
        );

        assert_eq!(
            request,
            Err("package name must not contain path separators".to_string())
        );
    }

    #[test]
    fn parse_update_request_without_args_selects_all_installed() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av update".to_string(),
            mode: None,
        };
        let request =
            parse_update_request_from_iter(&invocation, Vec::<OsString>::new().into_iter())
                .unwrap();

        assert_eq!(
            request,
            Some(UpdateRequest {
                selection: PackageSelection::AllInstalled,
                no_self_update: false,
            })
        );
    }

    #[test]
    fn parse_update_request_accepts_packages_and_hidden_self_update_flag() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av update".to_string(),
            mode: None,
        };
        let request = parse_update_request_from_iter(
            &invocation,
            vec![
                OsString::from("ffmpeg"),
                OsString::from(SELF_UPDATE_DISABLE_FLAG),
                OsString::from("brew:python@3.12"),
                OsString::from("npm:openclaw"),
                OsString::from("pip:psycopg2"),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(UpdateRequest {
                selection: PackageSelection::Requested(vec![
                    RequestedPackage::Auto("ffmpeg".to_string()),
                    RequestedPackage::HomebrewFormula("python@3.12".to_string()),
                    RequestedPackage::NpmPackage {
                        package: "openclaw".to_string(),
                        version: None,
                    },
                    RequestedPackage::PipPackage("psycopg2".to_string()),
                ]),
                no_self_update: true,
            })
        );
    }

    #[test]
    fn parse_info_request_accepts_single_package() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av info".to_string(),
            mode: None,
        };
        let request = parse_info_request_from_iter(
            &invocation,
            vec![OsString::from("npm:openclaw")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(InfoRequest {
                package: RequestedPackage::NpmPackage {
                    package: "openclaw".to_string(),
                    version: None,
                },
                output: OutputMode::Human,
            })
        );
    }

    #[test]
    fn parse_info_request_rejects_multiple_packages() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av info".to_string(),
            mode: None,
        };
        let request = parse_info_request_from_iter(
            &invocation,
            vec![OsString::from("ffmpeg"), OsString::from("deno")].into_iter(),
        );

        assert_eq!(request, Err("supports a single package".to_string()));
    }

    #[test]
    fn parse_info_request_accepts_json_output_flags() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av info".to_string(),
            mode: None,
        };
        let request = parse_info_request_from_iter(
            &invocation,
            vec![OsString::from("--json"), OsString::from("ffmpeg")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(InfoRequest {
                package: RequestedPackage::Auto("ffmpeg".to_string()),
                output: OutputMode::Json,
            })
        );
    }

    #[test]
    fn parse_search_request_accepts_single_query() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av search".to_string(),
            mode: None,
        };
        let request = parse_search_request_from_iter(
            &invocation,
            vec![OsString::from("--json"), OsString::from("rip")].into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(SearchRequest {
                query: "rip".to_string(),
                output: OutputMode::Json,
            })
        );
    }

    #[test]
    fn parse_search_request_rejects_multiple_query_tokens() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av search".to_string(),
            mode: None,
        };
        let request = parse_search_request_from_iter(
            &invocation,
            vec![OsString::from("rip"), OsString::from("grep")].into_iter(),
        );

        assert_eq!(request, Err("supports a single query string".to_string()));
    }

    #[test]
    fn ensure_package_installed_reports_missing_package() {
        let temp = TempDir::new().unwrap();

        assert_eq!(
            ensure_package_installed(temp.path(), "python"),
            Err("package python is not installed".to_string())
        );
    }

    #[test]
    fn format_installed_paths_returns_installed_for_empty_list() {
        assert_eq!(format_installed_paths(&[]), "installed");
    }

    #[test]
    fn format_installed_paths_separates_paths_with_newlines() {
        assert_eq!(
            format_installed_paths(&[
                "/usr/local/bin/node".to_string(),
                "/usr/local/bin/npm".to_string(),
            ]),
            "/usr/local/bin/node\n/usr/local/bin/npm"
        );
    }

    #[test]
    fn format_package_info_reports_homebrew_metadata() {
        let info = PackageInfo {
            package_name: "ffmpeg".to_string(),
            qualified_name: "brew:ffmpeg".to_string(),
            install_root: PathBuf::from("/opt/ffmpeg"),
            installed: true,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "ffmpeg".to_string(),
            }),
            source_error: None,
            aliases: vec!["ffmpeg4".to_string()],
            aliases_error: None,
            installed_version: Some("7.1".to_string()),
            latest_version: Some("7.2".to_string()),
            latest_version_error: None,
            executable_paths: vec![
                "/usr/local/bin/ffmpeg".to_string(),
                "/usr/local/bin/ffprobe".to_string(),
            ],
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: Some(HomebrewPackageInfo {
                formula: "ffmpeg".to_string(),
                description: Some("Play, record, convert, and stream audio and video".to_string()),
                homepage: Some("https://ffmpeg.org/".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: Some("GPL-2.0-or-later".to_string()),
                dependencies: vec!["aom".to_string(), "x264".to_string()],
            }),
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("📦 brew:ffmpeg"));
        assert!(rendered.contains("Aliases       ffmpeg4"));
        assert!(rendered.contains("Source        Homebrew"));
        assert!(rendered.contains("Formula Page  https://formulae.brew.sh/formula/ffmpeg"));
        assert!(rendered.contains("╭─ Dependencies "));
        assert!(rendered.contains("aom   x264"));
        assert!(rendered.contains("╭─ Executables "));
        assert!(rendered.contains("/usr/local/bin/ffmpeg"));
        assert!(rendered.contains("/usr/local/bin/ffprobe"));
        assert!(
            rendered
                .lines()
                .all(|line| line.chars().count() <= INFO_WIDTH)
        );
    }

    #[test]
    fn format_package_info_reports_unavailable_homebrew_metadata() {
        let info = PackageInfo {
            package_name: "foo".to_string(),
            qualified_name: "brew:foo".to_string(),
            install_root: PathBuf::from("/opt/foo"),
            installed: false,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "foo".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: Some("failed to fetch Homebrew formula index".to_string()),
            installed_version: None,
            latest_version: None,
            latest_version_error: Some("failed to fetch formula metadata".to_string()),
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: Some("failed to fetch formula metadata".to_string()),
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("📦 brew:foo"));
        assert!(rendered.contains("Installed     no"));
        assert!(rendered.contains("Source        Homebrew"));
        assert!(rendered.contains("Formula Page  https://formulae.brew.sh/formula/foo"));
        assert!(rendered.contains("Homebrew Info unavailable (failed to fetch formula metadata)"));
        assert!(
            rendered
                .lines()
                .all(|line| line.chars().count() <= INFO_WIDTH)
        );
    }

    #[test]
    fn format_package_info_reports_uninstalled_homebrew_executables_without_prefix() {
        let info = PackageInfo {
            package_name: "ffmpeg".to_string(),
            qualified_name: "brew:ffmpeg".to_string(),
            install_root: PathBuf::from("/opt/ffmpeg"),
            installed: false,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "ffmpeg".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: None,
            latest_version: Some("7.2".to_string()),
            latest_version_error: None,
            executable_paths: vec![
                "ffmpeg".to_string(),
                "ffplay".to_string(),
                "ffprobe".to_string(),
            ],
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: Some(HomebrewPackageInfo {
                formula: "ffmpeg".to_string(),
                description: Some("Play, record, convert, and stream audio and video".to_string()),
                homepage: Some("https://ffmpeg.org/".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: Some("GPL-2.0-or-later".to_string()),
                dependencies: vec![],
            }),
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("╭─ Executables "));
        assert!(rendered.contains("ffmpeg"));
        assert!(rendered.contains("ffplay"));
        assert!(rendered.contains("ffprobe"));
        assert!(!rendered.contains("/usr/local/bin/ffmpeg"));
    }

    #[test]
    fn format_package_info_reports_cask_metadata() {
        let info = PackageInfo {
            package_name: "codex".to_string(),
            qualified_name: "cask:codex".to_string(),
            install_root: PathBuf::from("/opt/codex"),
            installed: true,
            source: Some(PackageReceiptSource::Cask {
                cask_name: "codex".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: Some("0.1.2505231602".to_string()),
            latest_version: Some("0.1.2505231602".to_string()),
            latest_version_error: None,
            executable_paths: vec!["/usr/local/bin/codex".to_string()],
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: Some(HomebrewPackageInfo {
                formula: "codex".to_string(),
                description: Some("OpenAI codex CLI".to_string()),
                homepage: Some("https://github.com/openai/codex".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: None,
                dependencies: vec!["ripgrep".to_string()],
            }),
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("📦 cask:codex"));
        assert!(rendered.contains("Source        Homebrew Cask"));
        assert!(rendered.contains("Description   OpenAI codex CLI"));
        assert!(rendered.contains("Homepage      https://github.com/openai/codex"));
        assert!(rendered.contains("ripgrep"));
    }

    #[test]
    fn format_package_info_reports_vendor_package_with_subs_prefix() {
        let info = PackageInfo {
            package_name: "deno".to_string(),
            qualified_name: "av:deno".to_string(),
            install_root: PathBuf::from("/opt/deno"),
            installed: false,
            source: Some(PackageReceiptSource::Vendor {
                vendor_name: "deno".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: None,
            latest_version: Some("2.7.9".to_string()),
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("📦 av:deno"));
        assert!(rendered.contains("Version       2.7.9"));
        assert!(rendered.contains("Installed     no"));
        assert!(rendered.contains("Source        Subs"));
        assert!(!rendered.contains("Aliases"));
        assert!(
            rendered
                .lines()
                .all(|line| line.chars().count() <= INFO_WIDTH)
        );
    }

    #[test]
    fn format_package_info_reports_npm_homepage() {
        let info = PackageInfo {
            package_name: "openclaw".to_string(),
            qualified_name: "npm:openclaw".to_string(),
            install_root: PathBuf::from("/opt/npm/openclaw"),
            installed: false,
            source: Some(PackageReceiptSource::Npm {
                package_name: "openclaw".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: None,
            latest_version: Some("4.5.6".to_string()),
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: Some("https://www.example.com/openclaw".to_string()),
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let rendered = format_package_info(&info);
        assert!(rendered.contains("📦 npm:openclaw"));
        assert!(rendered.contains("https://www.example.com/openclaw"));
        assert!(
            rendered
                .lines()
                .all(|line| line.chars().count() <= INFO_WIDTH)
        );
    }

    #[test]
    fn package_info_helpers_cover_identity_formatting_and_wrapping() {
        assert_eq!(
            requested_package_name(&RequestedPackage::Isotope("gh".to_string())),
            "isotope:gh"
        );
        let status = PackageStatus {
            package_name: "npm:openclaw".to_string(),
            source: PackageReceiptSource::Npm {
                package_name: "openclaw".to_string(),
            },
            installed_version: "1.0.0".to_string(),
            latest_version: "1.0.0".to_string(),
        };
        assert_eq!(
            requested_package_from_status(&status),
            RequestedPackage::NpmPackage {
                package: "openclaw".to_string(),
                version: None,
            }
        );
        assert!(!status.is_outdated());
        assert_eq!(
            compare_package_names_for_search_order("npm:@scope/zeta", "brew:alpha"),
            std::cmp::Ordering::Greater
        );

        for (source, qualified, label) in [
            (
                PackageReceiptSource::Formula {
                    root_formula: "python@3.12".to_string(),
                },
                "brew:python@3.12",
                "Homebrew",
            ),
            (
                PackageReceiptSource::Cask {
                    cask_name: "visual-studio-code".to_string(),
                },
                "cask:visual-studio-code",
                "Homebrew Cask",
            ),
            (
                PackageReceiptSource::Isotope {
                    isotope_name: "gh".to_string(),
                },
                "isotope:gh",
                "Isotope",
            ),
            (
                PackageReceiptSource::Vendor {
                    vendor_name: "deno".to_string(),
                },
                "av:deno",
                "Subs",
            ),
            (
                PackageReceiptSource::Npm {
                    package_name: "openclaw".to_string(),
                },
                "npm:openclaw",
                "npm",
            ),
            (
                PackageReceiptSource::Pip {
                    package_name: "psycopg2".to_string(),
                },
                "pip:psycopg2",
                "PyPI",
            ),
        ] {
            assert_eq!(package_source_qualified_name(&source), qualified);
            assert_eq!(format_source_field(Some(&source)), label);
        }
        assert_eq!(format_source_field(None), "Unknown");

        let mut info = PackageInfo {
            package_name: "python@3.12".to_string(),
            qualified_name: String::new(),
            install_root: PathBuf::from("/opt/python@3.12"),
            installed: true,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "python@3.12".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: Some("3.12.1".to_string()),
            latest_version: Some("3.12.2".to_string()),
            latest_version_error: None,
            executable_paths: vec!["/usr/local/bin/python3.12".to_string()],
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: Some(HomebrewPackageInfo {
                formula: "python@3.12".to_string(),
                description: Some("A language runtime".to_string()),
                homepage: Some("https://www.python.org".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: Some("Python-2.0".to_string()),
                dependencies: vec!["openssl@3".to_string(), "sqlite".to_string()],
            }),
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };
        populate_package_info_identity(&mut info);
        assert_eq!(info.qualified_name, "brew:python@3.12");
        assert_eq!(
            format_version_status(&info),
            Some("update available (3.12.2)".to_string())
        );
        let rendered = format_package_info(&info);
        assert!(rendered.contains("Dependencies"));
        assert!(rendered.contains("Formula Page"));
        assert!(rendered.contains("/usr/local/bin/python3.12"));
        assert!(
            rendered
                .lines()
                .all(|line| line.chars().count() <= INFO_WIDTH)
        );

        assert_eq!(string_or_none("  value  "), Some("value".to_string()));
        assert_eq!(string_or_none(" \n\t "), None);
        assert_eq!(split_text_hard("abcdefgh", 3), vec!["abc", "def", "gh"]);
        assert_eq!(
            wrap_text("alpha beta\n\nsupercalifragilistic", 8),
            vec!["alpha", "beta", "", "supercal", "ifragili", "stic"]
        );
        assert_eq!(
            wrap_tokens(&["alpha".to_string(), "beta".to_string()], 2, 3),
            vec!["  alpha   beta"]
        );
        assert_eq!(
            homebrew_formula_page_url("python@3.12"),
            "https://formulae.brew.sh/formula/python@3.12"
        );
        assert!(plain_box_top().starts_with("╭"));
        assert!(section_top("Executables").contains("Executables"));
        assert!(plain_box_bottom().starts_with("╰"));
        assert!(section_bottom().starts_with("╰"));
    }

    #[test]
    fn package_info_source_resolution_and_scanning_cover_fallbacks() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let formula_root = opt_root.join("python@3.12");
        let npm_root = opt_root.join("npm/openclaw");
        let scoped_npm_root = opt_root.join("npm/@scope/tool");
        let pip_root = opt_root.join("pip/psycopg2");
        let isotope_root = opt_root.join("iso/gh");
        fs::create_dir_all(&formula_root).unwrap();
        fs::create_dir_all(&npm_root).unwrap();
        fs::create_dir_all(&scoped_npm_root).unwrap();
        fs::create_dir_all(&pip_root).unwrap();
        fs::create_dir_all(&isotope_root).unwrap();
        fs::write(opt_root.join("README"), b"skip").unwrap();
        fs::create_dir_all(opt_root.join(".tmp")).unwrap();
        fs::create_dir_all(opt_root.join("homebrew")).unwrap();
        write_package_receipt(
            &formula_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "python@3.12".to_string(),
                version: "3.12.1".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "python@3.12".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &npm_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "npm:openclaw".to_string(),
                version: "4.5.6".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "openclaw".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_stub_manifest(
            &formula_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["python3.12".to_string(), "pip3.12".to_string()],
            },
        )
        .unwrap();

        let refs = installed_package_refs(&opt_root).unwrap();
        assert!(
            refs.iter()
                .any(|package| package.package_name == "python@3.12")
        );
        assert!(
            refs.iter()
                .any(|package| package.package_name == "npm:openclaw")
        );
        assert!(
            refs.iter()
                .any(|package| package.package_name == "npm:@scope/tool")
        );
        assert!(
            refs.iter()
                .any(|package| package.package_name == "pip:psycopg2")
        );
        assert!(
            refs.iter()
                .any(|package| package.package_name == "isotope:gh")
        );
        assert_eq!(
            installed_stub_paths_at(&formula_root).unwrap(),
            vec![
                managed_bin_root().join("pip3.12").display().to_string(),
                managed_bin_root().join("python3.12").display().to_string(),
            ]
        );
        assert!(
            load_or_resolve_package_receipt("missing", temp.path())
                .unwrap_err()
                .contains("missing package metadata")
        );
        assert!(
            resolve_installed_package_record_at("file", &opt_root.join("README"))
                .unwrap_err()
                .contains("not a directory")
        );
        assert!(
            resolve_installed_package_record_at("absent", &opt_root.join("absent"))
                .unwrap_err()
                .contains("is not installed")
        );

        let mut warnings = Vec::new();
        let records = resolve_scanned_package_records(
            refs.clone(),
            |package| {
                if package.package_name == "pip:psycopg2" {
                    Err("bad receipt".to_string())
                } else {
                    Ok(InstalledPackageRecord {
                        package_name: package.package_name.clone(),
                        source: PackageReceiptSource::Formula {
                            root_formula: package.package_name.clone(),
                        },
                        installed_version: "1.0.0".to_string(),
                    })
                }
            },
            |message| warnings.push(message),
        )
        .unwrap();
        assert!(
            records
                .iter()
                .any(|record| record.package_name == "python@3.12")
        );
        assert!(
            warnings
                .iter()
                .any(|message| message.contains("bad receipt"))
        );

        let statuses = resolve_scanned_package_statuses(
            refs,
            |package| {
                Ok(PackageStatus {
                    package_name: package.package_name.clone(),
                    source: PackageReceiptSource::Formula {
                        root_formula: package.package_name.clone(),
                    },
                    installed_version: "1.0.0".to_string(),
                    latest_version: if package.package_name == "npm:openclaw" {
                        "2.0.0".to_string()
                    } else {
                        "1.0.0".to_string()
                    },
                })
            },
            |_message| {},
        )
        .unwrap();
        assert_eq!(
            filter_outdated_package_statuses(statuses)
                .into_iter()
                .map(|status| status.package_name)
                .collect::<Vec<_>>(),
            vec!["npm:openclaw".to_string()]
        );
    }

    #[test]
    fn package_record_and_status_wrappers_use_requested_selection() {
        let _env_lock = test_env_lock().lock().unwrap();
        let opt_root = opt_pkg_root();
        let record_name = "coverage-record";
        let status_name = "coverage-cask-status";
        let record_root = opt_root.join(record_name);
        let status_root = opt_root.join(status_name);
        for root in [&record_root, &status_root] {
            if fs::symlink_metadata(root).is_ok() {
                remove_path(root).unwrap();
            }
            fs::create_dir_all(root).unwrap();
        }
        write_package_receipt(
            &record_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: record_name.to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: record_name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &status_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: status_name.to_string(),
                version: "0.0.1".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "gh".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let records = resolve_installed_package_records(&PackageSelection::Requested(vec![
            RequestedPackage::Auto(record_name.to_string()),
            RequestedPackage::Auto(record_name.to_string()),
        ]))
        .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].package_name, record_name);
        assert_eq!(
            resolve_installed_package_record(record_name)
                .unwrap()
                .installed_version,
            "1.0.0"
        );

        let config = Config {
            bottle_tag: "all".to_string(),
        };
        let statuses = resolve_package_statuses(
            &config,
            &PackageSelection::Requested(vec![RequestedPackage::Auto(status_name.to_string())]),
        )
        .unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].package_name, status_name);
        assert_eq!(statuses[0].installed_version, "0.0.1");
        assert!(statuses[0].is_outdated());
        assert_eq!(
            resolve_outdated_package_statuses(
                &config,
                &PackageSelection::Requested(vec![RequestedPackage::Auto(status_name.to_string())])
            )
            .unwrap()
            .len(),
            1
        );

        remove_path(&record_root).unwrap();
        remove_path(&status_root).unwrap();
    }

    #[test]
    fn secret_scanner_warnings_cover_path_and_source_only_errors() {
        let report = SecretScannerReport {
            scope: SecretScannerScope::Full,
            findings: Vec::new(),
            errors: vec![
                SecretScannerError {
                    source: "filesystem".to_string(),
                    path: Some("/tmp/secret".to_string()),
                    message: "permission denied".to_string(),
                },
                SecretScannerError {
                    source: "detector".to_string(),
                    path: None,
                    message: "unavailable".to_string(),
                },
            ],
            summary: SecretScannerSummary {
                scanned_files: 0,
                findings: 0,
                errors: 2,
                isotope_detectors: 0,
                file_probes: 0,
            },
        };

        for error in &report.errors {
            print_secret_scanner_warning_line(error, false);
            print_wrapped_secret_scanner_warning_line(error, false);
        }
    }

    #[test]
    fn secret_scanner_stream_printer_covers_wrapped_events_and_empty_summary() {
        let finding = SecretScannerFinding {
            source: "dotenv".to_string(),
            kind: "plaintext-secret".to_string(),
            severity: "high".to_string(),
            path: Some("/tmp/project/.env".to_string()),
            line: Some(3),
            message: "API_KEY is plaintext".to_string(),
        };
        let error = SecretScannerError {
            source: "file-probe:zsh".to_string(),
            path: Some("/tmp/.zshrc".to_string()),
            message: "permission denied".to_string(),
        };
        let report = SecretScannerReport {
            scope: SecretScannerScope::Full,
            findings: vec![finding.clone()],
            errors: vec![error.clone()],
            summary: SecretScannerSummary {
                scanned_files: 2,
                findings: 1,
                errors: 1,
                isotope_detectors: 1,
                file_probes: 1,
            },
        };
        let mut printer = SecretScannerStreamPrinter {
            format: SecretScannerStreamFormat::Wrapped,
            color: false,
            scope: SecretScannerScope::Full,
            finding_count: 0,
            printed_findings_header: false,
            printed_warnings_header: false,
        };
        printer.begin().unwrap();
        printer
            .print_event(SecretScannerEvent::Finding(&finding))
            .unwrap();
        printer
            .print_event(SecretScannerEvent::Error(&error))
            .unwrap();
        printer.finish(&report).unwrap();
        assert_eq!(printer.finding_count, 1);
        assert!(printer.printed_findings_header);
        assert!(printer.printed_warnings_header);

        let empty_report = SecretScannerReport {
            scope: SecretScannerScope::IsotopesOnly,
            findings: Vec::new(),
            errors: Vec::new(),
            summary: SecretScannerSummary {
                scanned_files: 0,
                findings: 0,
                errors: 0,
                isotope_detectors: 0,
                file_probes: 0,
            },
        };
        let mut empty_printer = SecretScannerStreamPrinter {
            format: SecretScannerStreamFormat::Wrapped,
            color: false,
            scope: SecretScannerScope::IsotopesOnly,
            finding_count: 0,
            printed_findings_header: false,
            printed_warnings_header: false,
        };
        empty_printer.begin().unwrap();
        empty_printer.finish(&empty_report).unwrap();
        assert_eq!(empty_printer.finding_count, 0);

        for format in [
            SecretScannerStreamFormat::Plain,
            SecretScannerStreamFormat::Rich,
        ] {
            let mut printer = SecretScannerStreamPrinter {
                format,
                color: true,
                scope: SecretScannerScope::Full,
                finding_count: 0,
                printed_findings_header: false,
                printed_warnings_header: false,
            };
            printer.begin().unwrap();
            printer
                .print_event(SecretScannerEvent::Finding(&finding))
                .unwrap();
            printer
                .print_event(SecretScannerEvent::Error(&error))
                .unwrap();
            printer.finish(&report).unwrap();
            assert_eq!(printer.finding_count, 1);
            assert!(printer.printed_findings_header);
            assert!(printer.printed_warnings_header);

            let mut empty_printer = SecretScannerStreamPrinter {
                format,
                color: true,
                scope: SecretScannerScope::IsotopesOnly,
                finding_count: 0,
                printed_findings_header: false,
                printed_warnings_header: false,
            };
            empty_printer.begin().unwrap();
            empty_printer.finish(&empty_report).unwrap();
            assert_eq!(empty_printer.finding_count, 0);
        }

        print_scan_box(
            "Scan",
            &[
                "short".to_string(),
                "a much longer scanner line that exercises clamped box width".to_string(),
            ],
            true,
        );
        assert_eq!(strip_ansi_width("\u{1b}[31mred\u{1b}[0m"), 3);
        assert_eq!(
            secret_scanner_file_probe_summary(&empty_report),
            "file probes skipped"
        );
        assert_eq!(pluralize(1, "finding", "findings"), "1 finding");
        assert!(matches!(
            scan_severity_style(&SecretScannerFinding {
                severity: "low".to_string(),
                ..finding.clone()
            }),
            ScanStyle::Warning
        ));
        assert!(scan_paint("x", ScanStyle::Error, true).contains("\u{1b}[31;1m"));
        assert_eq!(scan_paint("x", ScanStyle::Error, false), "x");

        let _env_lock = test_env_lock().lock().unwrap();
        let _clean_env =
            TestEnvGuard::unset(&["NO_COLOR", "CLICOLOR_FORCE", "TERM", SCANNER_WRAPPER_UI_ENV]);
        assert!(!scanner_wrapper_ui_enabled());
        assert!(!output_supports_ansi(false));
        {
            let _env = TestEnvGuard::set(&[(SCANNER_WRAPPER_UI_ENV, "1")]);
            assert!(scanner_wrapper_ui_enabled());
        }
        {
            let _env = TestEnvGuard::set(&[("CLICOLOR_FORCE", "1")]);
            assert!(scan_stdout_is_rich());
            assert!(output_supports_ansi(false));
        }
        {
            let _env = TestEnvGuard::set(&[("NO_COLOR", "1")]);
            assert!(!output_supports_ansi(true));
        }
        {
            let _env = TestEnvGuard::set(&[("TERM", "dumb")]);
            assert!(!output_supports_ansi(true));
        }
    }

    #[test]
    fn package_info_metadata_helpers_cover_source_variants() {
        assert_eq!(
            explicit_requested_package_source(&RequestedPackage::HomebrewCask(
                "visual-studio-code".to_string()
            )),
            Some(PackageReceiptSource::Cask {
                cask_name: "visual-studio-code".to_string()
            })
        );
        assert_eq!(
            infer_requested_package_source(&RequestedPackage::Auto("bun".to_string())).unwrap(),
            PackageReceiptSource::Vendor {
                vendor_name: "bun".to_string()
            }
        );
        assert_eq!(
            infer_requested_package_source(&RequestedPackage::Auto(
                "definitely-not-a-package".to_string()
            ))
            .unwrap(),
            PackageReceiptSource::Formula {
                root_formula: "definitely-not-a-package".to_string(),
            }
        );
        let (cask_aliases, cask_alias_error) =
            resolve_aliases_for_source(&PackageReceiptSource::Cask {
                cask_name: "visual-studio-code".to_string(),
            });
        assert!(cask_alias_error.is_none());
        assert!(cask_aliases.is_empty());
        assert!(
            homebrew_aliases_for_formula("nonexistent-formula")
                .unwrap()
                .is_empty()
        );
        assert_eq!(formula_versioned_base("openssl@3"), Some("openssl"));
        assert_eq!(formula_versioned_base("@3"), None);
        assert_eq!(formula_versioned_base("openssl@stable"), None);

        let mut formula = formula_info(false);
        formula.desc = " Demo formula ".to_string();
        formula.homepage = "https://example.com".to_string();
        formula.license = Some(" MIT ".to_string());
        formula.dependencies = vec!["openssl@3".to_string()];
        assert_eq!(
            homebrew_package_info_from_formula_info("demo", &formula),
            HomebrewPackageInfo {
                formula: "demo".to_string(),
                description: Some("Demo formula".to_string()),
                homepage: Some("https://example.com".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: Some("MIT".to_string()),
                dependencies: vec!["openssl@3".to_string()],
            }
        );

        let isotope = isotope_package_data("gh").unwrap();
        let info = isotope_homebrew_info("gh", isotope);
        assert_eq!(info.formula, "gh");
        assert!(info.description.unwrap().contains("replacing brew:gh"));

        let mut results = vec![
            PackageSearchResult {
                package_name: "openssl".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "openssl".to_string(),
                },
                summary: None,
                latest_version: None,
                homepage: None,
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                category: None,
                dependencies: Vec::new(),
                install_package_names: Vec::new(),
                security_state: None,
                rank: None,
                last_updated_at: None,
                pulse_kind: None,
            },
            PackageSearchResult {
                package_name: "openssl@3".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "openssl@3".to_string(),
                },
                summary: None,
                latest_version: None,
                homepage: None,
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                category: None,
                dependencies: Vec::new(),
                install_package_names: Vec::new(),
                security_state: None,
                rank: None,
                last_updated_at: None,
                pulse_kind: None,
            },
            PackageSearchResult {
                package_name: "pip:openssl".to_string(),
                source: PackageReceiptSource::Pip {
                    package_name: "openssl".to_string(),
                },
                summary: None,
                latest_version: None,
                homepage: None,
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                category: None,
                dependencies: Vec::new(),
                install_package_names: Vec::new(),
                security_state: None,
                rank: None,
                last_updated_at: None,
                pulse_kind: None,
            },
        ];
        suppress_unversioned_formulae_with_versioned_search_results(&mut results);
        assert_eq!(
            results
                .into_iter()
                .map(|result| result.package_name)
                .collect::<Vec<_>>(),
            vec!["openssl@3".to_string(), "pip:openssl".to_string()]
        );
        assert!(formula_index_entry_matches(
            &formula_index_entry("ripgrep", &["rg"], &["old-rg"]),
            "old-rg"
        ));
    }

    #[test]
    fn resolve_uninstalled_package_info_populates_all_source_metadata() {
        let _env_lock = test_env_lock().lock().unwrap();
        let (base, server) = start_test_http_server(
            vec![
                (
                    "/node.json".to_string(),
                    br#"{
                        "desc":"Node runtime",
                        "homepage":"https://nodejs.org",
                        "license":"MIT",
                        "versions":{"stable":"22.0.0"},
                        "dependencies":["openssl@3"],
                        "bottle":{
                            "stable":{
                                "files":{
                                    "all":{
                                        "sha256":"node-sha",
                                        "url":"https://example.test/node.tar.gz"
                                    }
                                }
                            }
                        },
                        "disabled":false
                    }"#
                    .to_vec(),
                ),
                (
                    "/coverage-npm".to_string(),
                    br#"{
                        "description":"Coverage npm package",
                        "homepage":"https://example.test/coverage-npm",
                        "dist-tags":{"latest":"1.2.3"},
                        "versions":{
                            "1.2.3":{
                                "dist":{"tarball":"https://example.test/coverage-npm.tgz"}
                            }
                        }
                    }"#
                    .to_vec(),
                ),
                (
                    "/coverage-pip/json".to_string(),
                    br#"{
                        "info":{
                            "version":"2.3.4",
                            "summary":"Coverage pip package",
                            "home_page":"https://example.test/coverage-pip"
                        }
                    }"#
                    .to_vec(),
                ),
            ],
            6,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(base.clone()),
            npm_registry_root: Some(base.clone()),
            pypi_root: Some(base.clone()),
            ..Default::default()
        });
        let config = Config {
            bottle_tag: "all".to_string(),
        };

        let formula = resolve_package_info(
            &config,
            &RequestedPackage::HomebrewFormula("node".to_string()),
        )
        .unwrap();
        let expected_node_summary = crate::cli::load_db()
            .expect("embedded DB fixture must be available")
            .formulas
            .get("node")
            .and_then(|metadata| string_or_none(&metadata.summary))
            .expect("expected embedded DB to include a non-empty summary for formula `node`");
        let formula_homebrew_info = formula.homebrew_info.unwrap();
        assert!(!formula.installed);
        assert_eq!(formula.latest_version, Some("22.0.0".to_string()));
        assert_eq!(
            formula_homebrew_info.description,
            Some(expected_node_summary)
        );
        assert_eq!(formula_homebrew_info.license, Some("MIT".to_string()));
        assert_eq!(
            formula_homebrew_info.dependencies,
            vec!["openssl@3".to_string()]
        );

        let cask = resolve_package_info(
            &config,
            &RequestedPackage::HomebrewCask("codex".to_string()),
        )
        .unwrap();
        assert_eq!(
            cask.source,
            Some(PackageReceiptSource::Cask {
                cask_name: "codex".to_string()
            })
        );
        assert_eq!(cask.latest_version, Some("1.0.0".to_string()));

        let isotope =
            resolve_package_info(&config, &RequestedPackage::Isotope("gh".to_string())).unwrap();
        assert!(isotope.latest_version.is_some());
        assert!(
            isotope
                .homebrew_info
                .unwrap()
                .description
                .unwrap()
                .contains("replacing")
        );

        let npm = resolve_package_info(
            &config,
            &RequestedPackage::NpmPackage {
                package: "coverage-npm".to_string(),
                version: None,
            },
        )
        .unwrap();
        assert_eq!(npm.latest_version, Some("1.2.3".to_string()));
        assert_eq!(
            npm.npm_homepage,
            Some("https://example.test/coverage-npm".to_string())
        );

        let pip = resolve_package_info(
            &config,
            &RequestedPackage::PipPackage("coverage-pip".to_string()),
        )
        .unwrap();
        assert_eq!(pip.latest_version, Some("2.3.4".to_string()));
        server.join().unwrap();
    }

    #[test]
    fn parse_package_status_request_without_args_selects_all_installed() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av outdated".to_string(),
            mode: None,
        };
        let request = parse_package_status_request_from_iter(
            &invocation,
            Vec::<OsString>::new().into_iter(),
            print_outdated_usage,
        )
        .unwrap();

        assert_eq!(
            request,
            Some(PackageStatusRequest {
                selection: PackageSelection::AllInstalled,
                output: OutputMode::Human,
            })
        );
    }

    #[test]
    fn parse_package_status_request_accepts_multiple_packages() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av outdated".to_string(),
            mode: None,
        };
        let request = parse_package_status_request_from_iter(
            &invocation,
            vec![
                OsString::from("ffmpeg"),
                OsString::from("brew:python@3.12"),
                OsString::from("npm:openclaw"),
                OsString::from("pip:psycopg2"),
            ]
            .into_iter(),
            print_outdated_usage,
        )
        .unwrap();

        assert_eq!(
            request,
            Some(PackageStatusRequest {
                selection: PackageSelection::Requested(vec![
                    RequestedPackage::Auto("ffmpeg".to_string()),
                    RequestedPackage::HomebrewFormula("python@3.12".to_string()),
                    RequestedPackage::NpmPackage {
                        package: "openclaw".to_string(),
                        version: None,
                    },
                    RequestedPackage::PipPackage("psycopg2".to_string()),
                ]),
                output: OutputMode::Human,
            })
        );
    }

    #[test]
    fn parse_package_status_request_accepts_jsonl_output_flags() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av list".to_string(),
            mode: None,
        };
        let request = parse_package_status_request_from_iter(
            &invocation,
            vec![OsString::from("--jsonl"), OsString::from("ffmpeg")].into_iter(),
            print_list_usage,
        )
        .unwrap();

        assert_eq!(
            request,
            Some(PackageStatusRequest {
                selection: PackageSelection::Requested(vec![RequestedPackage::Auto(
                    "ffmpeg".to_string(),
                )]),
                output: OutputMode::Jsonl,
            })
        );
    }

    #[test]
    fn parse_package_status_request_rejects_conflicting_output_flags() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av list".to_string(),
            mode: None,
        };
        let request = parse_package_status_request_from_iter(
            &invocation,
            vec![OsString::from("--json"), OsString::from("--jsonl")].into_iter(),
            print_list_usage,
        );

        assert_eq!(
            request,
            Err("cannot combine --json and --jsonl".to_string())
        );
    }

    #[test]
    fn parse_secret_scanner_request_accepts_path_and_output_flags() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av scan".to_string(),
            mode: None,
        };
        let request = parse_secret_scanner_request_from_iter(
            &invocation,
            vec![
                OsString::from("--json"),
                OsString::from("--path"),
                OsString::from("/tmp/project"),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            request,
            Some(SecretScannerRequest {
                path: Some(PathBuf::from("/tmp/project")),
                skip_paths: Vec::new(),
                output: OutputMode::Json,
                isotopes_only: false,
            })
        );
    }

    #[test]
    fn parse_secret_scanner_request_rejects_missing_path_value() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av scan".to_string(),
            mode: None,
        };
        let request = parse_secret_scanner_request_from_iter(
            &invocation,
            vec![OsString::from("--path")].into_iter(),
        );

        assert_eq!(request, Err("missing value for --path".to_string()));
    }

    #[test]
    fn secret_file_scanner_detects_env_tokens_without_printing_values() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(
            &env_path,
            "OPENAI_API_KEY=sk-test_1234567890abcdef\nPLACEHOLDER=${TOKEN}\n",
        )
        .unwrap();

        let findings = scan_secret_file(&env_path).unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "secret-assignment");
        assert_eq!(findings[0].line, Some(1));
        assert!(!findings[0].message.contains("sk-test"));
    }

    #[test]
    fn secret_file_scanner_ignores_encrypted_dotenv_values() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(
            &env_path,
            [
                "DOTENV_PUBLIC_KEY=abc",
                "POSTHOG_API_KEY=\"encrypted:BHvhiFrrSNTU2wyZKZZyXTJkeE/viMW2B4L40PlAwhMif8P5BPhG1ew9D7pmU3VFAejrrcQhqjiSog/vM8/wIGBHBYpM+0776ulrLQGbSrLtzjMyh0ig0AimnI9YFrctRb2bWkG7bqASerIwV+xvzQ==\"",
                "OPENAI_API_KEY=sk-test_1234567890abcdef",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&env_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].kind, "secret-assignment");
        assert_eq!(findings[0].line, Some(3));
        assert!(findings[0].message.contains("assigned to OPENAI_API_KEY"));
    }

    #[test]
    fn secret_file_scanner_ignores_source_code_token_references() {
        let temp = TempDir::new().unwrap();
        let swift_path = temp.path().join("SpotifyHelperBridge.swift");
        fs::write(
            &swift_path,
            [
                "private struct HelperEnvelope: Decodable {",
                "    let accessToken: String?",
                "    let refreshToken: String?",
                "}",
                "private enum HelperCommand: String {",
                "    case accessToken = \"access_token\"",
                "}",
                "private func token(from response: HelperEnvelope) throws -> SpotifyToken {",
                "    let accessToken = response.accessToken,",
                "    let refreshToken = response.refreshToken,",
                "    return SpotifyToken(accessToken: accessToken, refreshToken: refreshToken)",
                "}",
                "let apiKey = \"sk-test_1234567890abcdef\"",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&swift_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(13));
        assert_eq!(findings[0].kind, "secret-assignment");
    }

    #[test]
    fn secret_file_scanner_ignores_source_constants_and_parser_tables() {
        let temp = TempDir::new().unwrap();
        let python_path = temp.path().join("tokenize.py");
        fs::write(
            &python_path,
            [
                "TOKEN_ENDS = TSPECIALS | WSP",
                "password = password or \"\"",
                "passwd = passwd or ''",
                "token_range = \"%d,%d-%d,%d:\" % (token.start + token.end)",
                "token = \"'\", token[0][1:-1]",
                "'a4337bc45a8fc544c03f52dc550cd6e1e87021bc896588bd79e901e2'",
                "'1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.8.b.d.0.1.0.0.2.ip6.arpa'",
                "\"application/vnd.pypi.simple.v1+json\"",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&python_path).unwrap();

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn secret_file_scanner_ignores_shell_variable_expansions() {
        let temp = TempDir::new().unwrap();
        let shell_path = temp.path().join("dev.sh");
        fs::write(
            &shell_path,
            [
                "local npm_default_cache=\"$HOME/.npm\"",
                "local -a npm_residual_dirs=(\"_cacache\" \"_npx\" \"_logs\" \"_prebuilds\")",
                "local -a npm_descriptions=(\"npm cache directory\" \"npm npx cache\" \"npm logs\" \"npm prebuilds\")",
                "if [[ \"$npm_cache_path_normalized\" != \"$npm_default_cache_normalized\" ]]; then",
                "    for i in \"${!npm_residual_dirs[@]}\"; do",
                "        safe_clean \"$npm_cache_path/${npm_residual_dirs[$i]}\"/* \"${npm_descriptions[$i]} (custom path)\"",
                "    done",
                "fi",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&shell_path).unwrap();

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn secret_file_scanner_detects_source_secret_literals() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("credentials.ts");
        fs::write(
            &source_path,
            [
                r#"const apiKey = "sk-live_1234567890abcdefghijklmnop";"#,
                r#"export const opaqueToken = "Rdb0XGysWuBnveWaNkyiM8Qz1Lp2";"#,
                r#"return "ghp_1234567890abcdefghijkl";"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&source_path).unwrap();

        assert_eq!(findings.len(), 3, "{findings:?}");
        assert_eq!(findings[0].kind, "secret-assignment");
        assert_eq!(findings[1].kind, "secret-assignment");
        assert_eq!(findings[2].kind, "token-literal");
    }

    #[test]
    fn secret_file_scanner_ignores_json_boolean_and_null_values() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("models.json");
        fs::write(
            &json_path,
            [
                r#"{"requiresAPIKey": false,"#,
                r#""remoteAuthentication": true,"#,
                r#""clientSecret": null,"#,
                r#""apiKey": "sk-test_1234567890abcdef","#,
                r#""token": "secret""#,
                r#"}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&json_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(4));
        assert_eq!(findings[0].kind, "secret-assignment");
    }

    #[test]
    fn secret_file_scanner_requires_stronger_values_outside_credential_files() {
        let temp = TempDir::new().unwrap();
        let notes_path = temp.path().join("notes.txt");
        fs::write(
            &notes_path,
            [
                "TOKEN_ENDS = TSPECIALS | WSP",
                "API_KEY=supervaultcodeqx",
                "API_KEY=Rdb0XGysWuBnveWaNkyiM8Qz1Lp2",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&notes_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(3));
    }

    #[test]
    fn secret_file_scanner_ignores_code_docs_and_fixture_false_positives() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("test");
        fs::create_dir_all(&source_dir).unwrap();
        let source_path = source_dir.join("clawlicious-fixtures.test.ts");
        let lines = [
            "token.fromMs <= absoluteTimeMs && token.toMs > absoluteTimeMs;",
            "The user needs to create an access token by visiting https://console.mapbox.com/account/access-tokens/.",
            "REMOTION_MAPBOX_TOKEN==pk.your-mapbox-access-token",
            "mapboxgl.accessToken = process.env.REMOTION_MAPBOX_TOKEN as string;",
            r#""xi-api-key": process.env.ELEVENLABS_API_KEY!,"#,
            r#"const apiKey = typeof payload.apiKey === "string""#,
            r#"apiKey: typeof stored.apiKey === "string" ? stored.apiKey : "","#,
            r#"if (tokenScope === "full") {"#,
            "type ByoClawPollTokenRecord = NonNullable<ReturnType<typeof getByoClawPollToken>>;",
            "struct SpotifyToken: Codable {",
            "var plainTextSecretAlertSource: PackageSecurityNotice.Source? {",
            "secrets: BTreeMap<String, Result<String, String>>,",
            "if (!forumToken || forumToken.forum_id !== parsedParams.data.forumId) {",
            r#"const CHECKOUT_SUCCESS_TOKEN = "{CHECKOUT_SESSION_ID}";"#,
            "WHERE poll_token_id = ? AND id != ?`,",
            r#"data-api-key="{{ claw.api_key }}""#,
            r#"password: "password123","#,
            "const POLL_TOKEN_PATTERN = /^claw_poll_[a-f0-9]{48}$/;",
            r#"token: "handoff-token","#,
            "const renewedToken = tokenMatch[1];",
            r#"TOKEN_SERVICE = "https://ghcr.io/token""#,
            "def _fetch_json(url, github_token=None):",
            r#""Authorization": f"Bearer {token}","#,
            r#""token": bearer,"#,
            "metadata[token] = supported",
            ".secret-art::before {",
            "export AWS_SECRET_ACCESS_KEY=%awssecret%",
            "password=mb_password",
            r#""challengeToken": "f7D4...base64url...","#,
            r#""tokenType": "integration","#,
            r#""token": "clawlt_7wYx...base64url...","#,
            "export OUTCLAW_SSH_PRIVATE_KEY=~/.ssh/smbh-api-ec2-us-east-2.pem",
            r#"const TOKEN_PREFIX = "outclawclaw_";"#,
            r#"challengeToken: "string","#,
            "tokenHash: hashed,",
            r#""js-tokens": "^4.0.0","#,
            "id-token: write   # to verify the deployment originates from an appropriate source",
            "self.md.toc_tokens = toc_tokens",
            "MaxScanTokenSize = 64 * 1024",
            "token: &'static str,",
            "let executable = npm_package_executable_name(&npm_package);",
            "package_name: npm_package.clone(),",
            r#""[default]\naws_secret_access_key = secret\n","#,
            r#"static const char TestTokenLabel[] = "Test PKCS11 Token Label";"#,
            r#""input_token": "nextToken","#,
            "password: bytes | None,",
            "PASSWORD: optional password used to decrypt the structure",
            r#""detect-secrets": "detect-secrets","#,
            r#"export function randomToken(prefix = "pincerspace_", size = 24) {"#,
            "char const* token_last = nullptr;",
            "sso_token_cache=None):",
            r#"const apiKey = "sk-test_1234567890abcdef""#,
        ];
        fs::write(&source_path, lines.join("\n")).unwrap();

        let findings = scan_secret_file(&source_path).unwrap();

        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn secret_file_scanner_detects_jwt_tokens() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(
            &env_path,
            "MAILERLITE_TOKEN=eyJhbGciOiJSUzI1NiJ9.eyJhdWQiOiIxMjM0NTY3ODkwIn0.signature_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890\n",
        )
        .unwrap();

        let findings = scan_secret_file(&env_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].kind, "secret-assignment");
    }

    #[test]
    fn secret_file_scanner_ignores_private_key_reference_fixtures() {
        let temp = TempDir::new().unwrap();
        let fixture_dir = temp.path().join("testdata");
        fs::create_dir_all(&fixture_dir).unwrap();
        let json_path = fixture_dir.join("wycheproof.json");
        fs::write(
            &json_path,
            r#""privateKeyPem": "-----BEGIN RSA PRIVATE KEY-----\nMIIEfixture\n-----END RSA PRIVATE KEY-----""#,
        )
        .unwrap();
        let source_path = temp.path().join("pubkey_pem.erl");
        fs::write(&source_path, r#"<<\"-----BEGIN RSA PRIVATE KEY-----\">>;"#).unwrap();

        assert!(scan_secret_file(&json_path).unwrap().is_empty());
        assert!(scan_secret_file(&source_path).unwrap().is_empty());
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_parser_edges() {
        let assignment = parse_secret_assignment("- token: value").unwrap();
        assert_eq!(assignment.key, "token");
        assert_eq!(assignment.value, " value");
        assert!(matches!(
            assignment.separator,
            SecretAssignmentSeparator::Colon
        ));

        let assignment = parse_secret_assignment("TOKEN=value").unwrap();
        assert_eq!(assignment.key, "TOKEN");
        assert_eq!(assignment.value, "value");
        assert!(matches!(
            assignment.separator,
            SecretAssignmentSeparator::Equals
        ));

        let assignment = parse_secret_assignment("URL=http://example.test/token").unwrap();
        assert_eq!(assignment.key, "URL");
        assert_eq!(assignment.value, "http://example.test/token");
        assert!(matches!(
            assignment.separator,
            SecretAssignmentSeparator::Equals
        ));

        let assignment = parse_secret_assignment(r#""token": "value""#).unwrap();
        assert_eq!(assignment.key, r#""token""#);
        assert_eq!(assignment.value, r#" "value""#);
        assert!(matches!(
            assignment.separator,
            SecretAssignmentSeparator::Colon
        ));

        for line in [
            "token == value",
            "token != value",
            "token <= value",
            "token >= value",
            "token => value",
            "SecretScannerStreamFormat::Plain => {",
            ".secret-art::before {",
            "https://example.test/token",
            "no assignment here",
        ] {
            assert!(parse_secret_assignment(line).is_none(), "{line}");
        }
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_source_code_rejection() {
        for line in [
            r#"case accessToken = "access_token""#,
            "type PollToken = string",
            "interface TokenRecord = {}",
            "union APIKeyPropertiesResult = APIKeyPropertiesOutput | UserFacingError",
            "def _fetch_json(url, github_token=None):",
            "function randomToken(prefix = \"pkg_\") {",
            "export function randomToken(prefix = \"pkg_\") {",
            "return accessToken = response.accessToken",
            "if token = value",
            "if(token = value)",
            "WHERE poll_token_id = ?",
            "where poll_token_id = ?",
            "fd->secret->state = _PR_FILEDESC_OPEN",
            "left, right = tokens",
            "metadata[token] = supported",
            "self.md.toc_tokens = toc_tokens",
            "This freeform token heading: has explanatory prose",
            "`/secret-scanner-for-ai-agents/`: 332 words",
            "Authorization: optional bearer token used by the request",
            "token: &'static str,",
            "token: bytes | None,",
            "token: ResponseToken,",
            "token = ?",
            "token = // comment",
            "token = {{ template.token }}",
            "token = <% template %>",
            "token = {CHECKOUT_SESSION_ID}",
            "token = /token-.*/",
            "token = f\"Bearer {token}\"",
            "token = f'Bearer {token}'",
            "token = process.env.API_TOKEN",
            "token = &self.external_secret",
            "token = !ready",
            "token = if conv_summary.token_count > 0 {",
            "token = self.apiKey!",
            "token = match launch_mode {",
            "token = typeof payload.token",
            "token = ReturnType<TokenFactory>",
            "token = payload.token as string",
            "token = 64 * 1024",
            "token = closeStart + closeDuration",
            "token = argument_idx - 1",
            r#"token = "\(editableNamePrefix)\(name)""#,
            r#"token = Settings.apiKey ?? """#,
            "token = condition ? a : b",
            "token = a && b",
            "token = a || b",
            "token = a === b",
            "token = a !== b",
            "token = a == b",
            "token = a != b",
            "token = a <= b",
            "token = a >= b",
            "token = tokenMatch[1]",
            "token = .leading.member",
            "token = tokenFactory()",
            "token = fd->secret",
            "token = Namespace::Token",
            "token = response.accessToken",
            "token = RefreshToken",
            "token = query?",
            "token = SecretRange {",
            "token: BTreeMap<String, Result<String, String>>,",
            "let token_syntax_color: AnsiColorIdentifier =",
            "pub parsed_token: &'a ParsedToken,",
            "token: FileIndexScanToken? = nil,",
            r#"secret: "fixture".to_owned(),"#,
        ] {
            let assignment = parse_secret_assignment(line).unwrap();
            assert!(
                secret_assignment_looks_like_source_code(&assignment),
                "{line}"
            );
        }

        for line in [
            "TOKEN=secret_secret",
            "OPENAI_API_KEY=sk-test_1234567890abcdef",
            "Authorization: Bearer realtoken1234567890",
        ] {
            let assignment = parse_secret_assignment(line).unwrap();
            assert!(
                !secret_assignment_looks_like_source_code(&assignment),
                "{line}"
            );
        }
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_sensitive_keys_and_metadata() {
        for key in [
            "token",
            "password",
            "passwd",
            "authorization",
            "API_KEY",
            "api.key",
            "apikey",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "auth_token",
            "private_key",
            "refresh_token",
            "id_token",
            "client_secret",
            "export OPENAI_API_KEY",
        ] {
            assert!(secret_key_name_is_sensitive(key), "{key}");
        }

        for key in [
            "tokenType",
            "token-types",
            "tokenName",
            "token_names",
            "TOKEN_PREFIX",
            "TOKEN_SUFFIX",
            "TOKEN_SERVICE",
            "tokenHash",
            "tokenLabel",
            "tokenLabels",
            "TOKEN_PATTERN",
            "tokenPatterns",
            "tokenClass",
            "MaxScanTokenSize",
            "SOFTOKEN_LIB_DIR",
            "PRIVATE_KEY_PATH",
            "private-key-file",
            "token.url",
            "token_uri",
        ] {
            assert!(secret_key_name_is_noncredential_metadata(key), "{key}");
            assert!(!secret_key_name_is_sensitive(key), "{key}");
        }
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_real_value_classification() {
        for value in [
            "short",
            "${TOKEN}",
            "secret",
            "password",
            "token",
            "example",
            "changeme",
            "change_me",
            "replace_me",
            "redacted",
            "access_token",
            "refresh_token",
            "id_token",
            "client_secret",
            "api_key",
            "none",
            "null",
            "true",
            "false",
            "string",
            "bytes",
            "write",
            "read",
            "hashed",
            "nullptr",
            "nil",
            "example-token",
            "placeholder-token",
            "your_token_here",
            "your-token-here",
            "clawlt_7wYx...base64url...",
            "gho_************************************",
            "gho_******",
            "fake-key",
            "fake-admin-key",
            "fake-token",
            "env(OPENAI_API_KEY)",
            "$tokens",
            "200000",
            "options:name1: blue,red,green",
            r#""[default]\naws_secret_access_key = secret\n""#,
            "Bearer <temporary_token>",
            "Bearer smbhclaw_\u{2026}",
            "xxxxxxxx",
            "********",
            "{TOKEN}",
            "{{ TOKEN }}",
            "<TOKEN>",
            "%awssecret%",
            "~/secret.pem",
            "./secret.key",
            "../secret.key",
            "/Users/me/secret.key",
            "https://ghcr.io/token",
            "^4.0.0",
            "3.0.0 || ^4.0.0",
            "cfengine",
            "detect-secrets",
            "nextToken",
            "NSS Certificate DB",
        ] {
            assert!(!secret_value_is_real(value), "{value}");
        }

        for value in [
            "secret_secret",
            "sk-test_1234567890abcdef",
            "phc_1234567890abcdefghijkl",
            "xai-CaxcatEA921Wrn5N6GyOuEfUrWwK90J1yBvn5Ehou5pUxWzgh0vGFBHrWCXAiBn68Z",
            "Rdb0XGysWuBnveWaNkyi",
            "dY3v9zk5epFZDMgoxUfDNp7fO2bGKQW4tT8wy58gGmHgg5oHPOeT9TdPDnzCINj3",
            "eyJhbGciOiJSUzI1NiJ9.eyJhdWQiOiIxMjM0NTY3ODkwIn0.signature_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890",
        ] {
            assert!(secret_value_is_real(value), "{value}");
        }
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_token_shapes_and_normalization() {
        for value in [
            "ghp_1234567890abcdefghijkl",
            "gho_1234567890abcdefghijkl",
            "ghs_1234567890abcdefghijkl",
            "github_pat_1234567890abcdefghijklmnopqrstuvwxyz",
            "glpat-1234567890abcdefghijkl",
            "xoxb-1234567890abcdefghijkl",
            "xoxp-1234567890abcdefghijkl",
            "sk_live_1234567890abcdefghijkl",
            "npm_1234567890abcdef",
            "sk-proj-1234567890abcdef",
            "xai-1234567890abcdefghi",
            "AKIA1234567890ABCDEF",
        ] {
            assert!(secret_value_has_known_token_shape(value), "{value}");
        }

        assert!(!secret_value_has_known_token_shape("npm_pkg"));
        assert!(!secret_value_has_known_token_shape("gho_abc123"));
        assert!(!secret_value_has_known_token_shape("github_pat_abc123"));
        assert!(secret_value_has_high_entropy_shape(
            "Rdb0XGysWuBnveWaNkyiM8Qz1Lp2"
        ));
        assert!(!secret_value_has_high_entropy_shape("TSPECIALS | WSP"));
        assert!(!secret_value_has_high_entropy_shape("supervaultcodeqx"));
        assert!(secret_value_looks_like_jwt(
            "eyJhbGciOiJSUzI1NiJ9.eyJhdWQiOiIxMjM0NTY3ODkwIn0.signature_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890"
        ));
        assert!(!secret_value_looks_like_jwt("response.accessToken"));
        assert!(!secret_value_looks_like_jwt("one.two.three.four"));

        assert_eq!(normalized_secret_value(r#" "false", "#), "false");
        assert_eq!(normalized_secret_value(" write   # comment"), "write");
        assert_eq!(normalized_secret_value("'secret');"), "secret");
        assert_eq!(
            normalized_secret_key_name("export API-KEY.name"),
            "api_key_name"
        );
        assert!(!secret_value_has_known_token_shape("npm_payloads or {}"));
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_source_string_fixtures() {
        for line in [
            r###""    let accessToken = response.accessToken,","###,
            r###""TOKEN=secret_secret","#"###,
            r###""OPENAI_API_KEY=sk-test_1234567890abcdef","#"###,
            r###"r#""apiKey": "sk-test_1234567890abcdef","#,"###,
            r#####"r###"r#""apiKey": "sk-test_1234567890abcdef","#,"#####,
            r###"br#""token": "fake-token""#,"###,
        ] {
            assert!(
                secret_line_looks_like_source_string_fixture(Path::new("/repo/src/lib.rs"), line),
                "{line}"
            );
            assert!(
                !secret_line_looks_like_source_string_fixture(Path::new("/repo/config.json"), line),
                "{line}"
            );
        }
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_fixture_paths() {
        for path in [
            "/repo/test/file.ts",
            "/repo/tests/file.ts",
            r"C:\repo\test\file.ts",
            r"C:\repo\tests\file.ts",
            "/repo/token.test.ts",
            "/repo/token.spec.ts",
            "/repo/token_tests.rs",
        ] {
            assert!(
                secret_path_looks_like_test_fixture(Path::new(path)),
                "{path}"
            );
        }

        for path in [
            "/repo/testdata/vector.json",
            "/repo/fixtures/vector.json",
            "/repo/fixture/vector.json",
            "/repo/examples/key.rst",
            "/repo/example/key.rst",
            "/repo/samples/key.req",
            "/repo/sample/key.req",
            "/repo/cavs_samples/key.req",
            "/repo/wycheproof/key.json",
            "/repo/doc/key.rst",
            "/repo/docs/key.rst",
            "/repo/share/man/man5/key.5",
            "/repo/share/info/key.info",
            "/repo/man/man3/key.3",
            "/repo/resources/bundled/skills/README.md",
            "/repo/hooks/fsmonitor-watchman.sample",
            "/repo/en.lproj/Localizable.strings",
        ] {
            assert!(
                secret_path_looks_like_reference_fixture(Path::new(path)),
                "{path}"
            );
            assert!(secret_value_is_test_fixture(
                Path::new(path),
                "sk-real_1234567890abcdef"
            ));
        }

        for value in [
            "password123",
            "handoff-token",
            "test-token",
            "test-password",
            "polar_test_token",
            "polar_webhook_secret",
        ] {
            assert!(secret_value_is_test_fixture(
                Path::new("/repo/test/auth.ts"),
                value
            ));
        }

        assert!(!secret_value_is_test_fixture(
            Path::new("/repo/src/auth.ts"),
            "sk-test_1234567890abcdef"
        ));
    }

    #[test]
    fn secret_file_scanner_ignores_sample_hook_source() {
        let temp = TempDir::new().unwrap();
        let sample_path = temp.path().join("hooks/fsmonitor-watchman.sample");
        fs::create_dir_all(sample_path.parent().unwrap()).unwrap();
        fs::write(
            &sample_path,
            [
                "\t# further constrain the results.",
                "\tmy $last_update_line = \"\";",
                "\tif (substr($last_update_token, 0, 1) eq \"c\") {",
                "\t\t$last_update_token = \"\\\"$last_update_token\\\"\";",
                "\t\t$last_update_line = qq[\\n\"since\": $last_update_token,];",
                "\t}",
            ]
            .join("\n"),
        )
        .unwrap();

        assert!(scan_secret_file(&sample_path).unwrap().is_empty());
    }

    #[test]
    fn secret_file_scanner_ignores_sensitive_keys_in_test_fixtures() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        fs::create_dir_all(&root).unwrap();

        let test_path = root.join("secrets_test.rs");
        fs::write(
            &test_path,
            [
                r#"api_key = "sk-live_1234567890abcdefghijklmnop""#,
                r#"secret: "码1234".to_owned(),"#,
                r#"stripe_restricted_api_key = "rk_live_1234567890abcdefghijklmnop""#,
            ]
            .join("\n"),
        )
        .unwrap();

        assert!(scan_secret_file(&test_path).unwrap().is_empty());
    }

    #[test]
    fn secret_file_scanner_detects_posthog_keys_in_env_files() {
        let temp = TempDir::new().unwrap();
        let envrc_path = temp.path().join(".envrc");
        fs::write(
            &envrc_path,
            "export POSTHOG_API_KEY=phc_1234567890abcdefghijklmnop\n",
        )
        .unwrap();

        let findings = scan_secret_file(&envrc_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(1));
        assert_eq!(findings[0].kind, "secret-assignment");
    }

    #[test]
    fn secret_file_scanner_detects_lowercase_env_secret_values() {
        let temp = TempDir::new().unwrap();
        let envrc_path = temp.path().join(".envrc");
        fs::write(
            &envrc_path,
            [
                "export TMDB_API_KEY=5368abcd9012efab3456abcd9012efab",
                "export TWITCH_CLIENT_SECRET=mbji9xv2qlemn8n2sk4pxh71r03j2x",
                "export JEWELFORM_ADMIN_TOKEN=supervaultcodeqx",
                "export AWS_REGION=us-east-1",
                "export API_KEY=example",
                "export API_KEY=nextToken",
            ]
            .join("\n"),
        )
        .unwrap();

        let findings = scan_secret_file(&envrc_path).unwrap();

        assert_eq!(findings.len(), 3, "{findings:?}");
        assert_eq!(findings[0].line, Some(1));
        assert_eq!(findings[1].line, Some(2));
        assert_eq!(findings[2].line, Some(3));
    }

    #[test]
    fn secret_file_scanner_detects_shell_startup_secret_assignments() {
        let temp = TempDir::new().unwrap();
        let bash_profile = temp.path().join(".bash_profile");
        let zshenv = temp.path().join(".zshenv");
        fs::write(
            &bash_profile,
            [
                "declare -x OPENAI_API_KEY=sk-test_1234567890abcdef",
                "export AWS_REGION=us-east-1",
                "export GITHUB_TOKEN=$(gh auth token)",
            ]
            .join("\n"),
        )
        .unwrap();
        fs::write(
            &zshenv,
            [
                "typeset -gx TWITCH_CLIENT_SECRET=mbji9xv2qlemn8n2sk4pxh71r03j2x",
                "typeset -gx API_KEY=example",
            ]
            .join("\n"),
        )
        .unwrap();

        let bash_findings = scan_secret_file(&bash_profile).unwrap();
        let zsh_findings = scan_secret_file(&zshenv).unwrap();

        assert_eq!(bash_findings.len(), 1, "{bash_findings:?}");
        assert_eq!(bash_findings[0].source, "file-probe:bash");
        assert_eq!(bash_findings[0].line, Some(1));
        assert!(
            bash_findings[0]
                .message
                .contains("assigned to OPENAI_API_KEY")
        );
        assert_eq!(zsh_findings.len(), 1, "{zsh_findings:?}");
        assert_eq!(zsh_findings[0].source, "file-probe:zsh");
        assert_eq!(zsh_findings[0].line, Some(1));
        assert!(
            zsh_findings[0]
                .message
                .contains("assigned to TWITCH_CLIENT_SECRET")
        );
    }

    #[test]
    fn shell_secret_detectors_scan_bash_and_zsh_startup_files() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let zdotdir = temp.path().join("zdotdir");
        let bash_env = temp.path().join("bash-env");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&zdotdir).unwrap();
        fs::write(
            home.join(".profile"),
            "export SERVICE_TOKEN=secret_secret\n",
        )
        .unwrap();
        fs::write(&bash_env, "readonly BASH_ENV_TOKEN=secret_secret\n").unwrap();
        fs::write(
            zdotdir.join(".zprofile"),
            "typeset -gx ZED_CLIENT_SECRET=zedsecret1234\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            ("BASH_ENV", bash_env.to_str().unwrap()),
            ("ZDOTDIR", zdotdir.to_str().unwrap()),
        ]);

        let paths = default_secret_scan_paths();
        assert!(paths.iter().any(|path| path == &home.join(".bash_profile")));
        assert!(paths.iter().any(|path| path == &bash_env));
        assert!(paths.iter().any(|path| path == &zdotdir.join(".zprofile")));

        let profile_findings = scan_secret_file(&home.join(".profile")).unwrap();
        let bash_env_findings = scan_secret_file(&bash_env).unwrap();
        let zsh_findings = scan_secret_file(&zdotdir.join(".zprofile")).unwrap();

        assert_eq!(profile_findings.len(), 1, "{profile_findings:?}");
        assert_eq!(profile_findings[0].source, "file-probe:bash");
        assert_eq!(bash_env_findings.len(), 1, "{bash_env_findings:?}");
        assert_eq!(bash_env_findings[0].source, "file-probe:bash");
        assert_eq!(zsh_findings.len(), 1, "{zsh_findings:?}");
        assert_eq!(zsh_findings[0].source, "file-probe:zsh");
    }

    #[test]
    fn secret_file_scanner_detects_standalone_posthog_key_literals_in_config() {
        let temp = TempDir::new().unwrap();
        let gradle_path = temp.path().join("build.gradle.kts");
        fs::write(
            &gradle_path,
            r#"val posthogKey = providers.environmentVariable("POSTHOG_PROJECT_TOKEN").orNull
        ?: "phc_1234567890abcdefghijklmnop""#,
        )
        .unwrap();

        let findings = scan_secret_file(&gradle_path).unwrap();

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(2));
        assert_eq!(findings[0].kind, "token-literal");
    }

    #[test]
    fn secret_file_scanner_ignores_secret_named_cargo_dependencies() {
        let temp = TempDir::new().unwrap();
        let cargo_path = temp.path().join("Cargo.toml");
        fs::write(
            &cargo_path,
            r#"warp_managed_secrets = { package = "warp-managed-secrets", workspace = true }
managed_secrets = ["dep:managed-secrets"]"#,
        )
        .unwrap();

        assert!(scan_secret_file(&cargo_path).unwrap().is_empty());
    }

    #[test]
    fn secret_file_scanner_helper_assumptions_cover_private_key_handling() {
        for path in [
            "/repo/pubkey_pem.c",
            "/repo/pubkey_pem.cc",
            "/repo/pubkey_pem.cpp",
            "/repo/pubkey_pem.cxx",
            "/repo/pubkey_pem.h",
            "/repo/pubkey_pem.hh",
            "/repo/pubkey_pem.hpp",
            "/repo/pubkey_pem.hxx",
            "/repo/pubkey_pem.go",
            "/repo/pubkey_pem.rs",
            "/repo/pubkey_pem.swift",
            "/repo/pubkey_pem.js",
            "/repo/pubkey_pem.jsx",
            "/repo/pubkey_pem.ts",
            "/repo/pubkey_pem.tsx",
            "/repo/pubkey_pem.py",
            "/repo/pubkey_pem.rb",
            "/repo/pubkey_pem.pm",
            "/repo/pubkey_pem.erl",
            "/repo/pubkey_pem.hrl",
        ] {
            assert!(
                secret_path_looks_like_source_file(Path::new(path)),
                "{path}"
            );
            assert!(secret_private_key_line_is_fixture(
                Path::new(path),
                r#"<<\"-----BEGIN RSA PRIVATE KEY-----\">>;"#
            ));
        }

        assert!(secret_private_key_line_is_fixture(
            Path::new("/repo/testdata/key.pem"),
            "-----BEGIN RSA PRIVATE KEY-----"
        ));
        assert!(!secret_private_key_line_is_fixture(
            Path::new("/repo/.env"),
            "-----BEGIN RSA PRIVATE KEY-----"
        ));
    }

    #[test]
    fn secret_file_probes_skip_generated_dependency_directories() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::create_dir_all(root.join("DerivedData")).unwrap();
        fs::create_dir_all(root.join(".codex-worktrees")).unwrap();
        fs::create_dir_all(root.join(".build")).unwrap();
        fs::create_dir_all(root.join(".next")).unwrap();
        fs::create_dir_all(root.join("cache")).unwrap();
        fs::create_dir_all(root.join("Vendor")).unwrap();
        fs::create_dir_all(root.join("isotopes/example")).unwrap();
        fs::create_dir_all(root.join("radioisotopes/example")).unwrap();
        fs::write(root.join("artifacts/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join("DerivedData/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join(".codex-worktrees/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join(".build/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join(".next/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join("cache/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join("Vendor/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(root.join("isotopes/example/.env"), "TOKEN=secret_secret\n").unwrap();
        fs::write(
            root.join("radioisotopes/example/.env"),
            "TOKEN=secret_secret\n",
        )
        .unwrap();
        fs::write(root.join(".env"), "TOKEN=secret_secret\n").unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        scan_secret_file_probes(
            Some(&root),
            &[],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        )
        .unwrap();

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0]
                .path
                .as_deref()
                .is_some_and(|path| path.ends_with("/.env"))
        );
    }

    #[test]
    fn secret_file_scanner_ignores_missing_default_candidates() {
        let temp = TempDir::new().unwrap();
        let findings = scan_secret_file(&temp.path().join(".env")).unwrap();

        assert!(findings.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn secret_file_probes_warn_for_unreadable_subdirectories() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let restricted = root.join("restricted");
        let env_path = root.join(".env");
        fs::create_dir_all(&restricted).unwrap();
        fs::write(&env_path, "TOKEN=secret_secret\n").unwrap();
        let mut permissions = fs::metadata(&restricted).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&restricted, permissions).unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let result = scan_secret_file_probes(
            Some(&root),
            &[],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        );

        let mut permissions = fs::metadata(&restricted).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&restricted, permissions).unwrap();
        result.unwrap();
        let env_path_display = env_path.display().to_string();
        assert!(findings.iter().any(|finding| {
            finding
                .path
                .as_deref()
                .is_some_and(|path| path == env_path_display)
        }));
        if unsafe { libc::geteuid() } != 0 {
            assert!(
                errors.iter().any(|error| error
                    .path
                    .as_deref()
                    .is_some_and(|path| path.contains("restricted"))),
                "{errors:?}"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn secret_file_probes_error_when_requested_root_is_unreadable() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        fs::create_dir_all(&root).unwrap();
        let mut permissions = fs::metadata(&root).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&root, permissions).unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let result = scan_secret_file_probes(
            Some(&root),
            &[],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        );

        let mut permissions = fs::metadata(&root).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&root, permissions).unwrap();
        let err = result.unwrap_err();
        assert!(err.contains("failed to read scan path"));
    }

    #[test]
    fn secret_file_probes_emit_events_while_building_report_parts() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let env_path = root.join(".env");
        fs::create_dir_all(&root).unwrap();
        fs::write(&env_path, "SERVICE_TOKEN=secret_secret\n").unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let mut events = Vec::new();

        let (scanned_files, file_probes) = scan_secret_file_probes(
            Some(&root),
            &[],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |event| {
                match event {
                    SecretScannerEvent::Finding(finding) => {
                        events.push(format!("finding:{}", finding.source));
                    }
                    SecretScannerEvent::Error(error) => {
                        events.push(format!("error:{}", error.source));
                    }
                }
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(scanned_files, 1);
        assert_eq!(file_probes, 1);
        assert_eq!(findings.len(), 1);
        assert!(errors.is_empty());
        assert_eq!(events, vec!["finding:file-probe"]);
    }

    #[test]
    fn secret_file_probes_skip_files_and_prune_directories() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let ignored_dir = root.join("ignored");
        let skipped_file = root.join("skip.env");
        let kept_file = root.join("keep.env");
        fs::create_dir_all(&ignored_dir).unwrap();
        fs::write(ignored_dir.join(".env"), "IGNORED_TOKEN=secret_secret\n").unwrap();
        fs::write(&skipped_file, "SKIPPED_TOKEN=secret_secret\n").unwrap();
        fs::write(&kept_file, "KEPT_TOKEN=secret_secret\n").unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let (scanned_files, file_probes) = scan_secret_file_probes(
            Some(&root),
            &[PathBuf::from("ignored"), skipped_file.clone()],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        )
        .unwrap();

        assert_eq!(scanned_files, 1);
        assert_eq!(file_probes, 1);
        assert!(errors.is_empty());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].path, Some(kept_file.display().to_string()));
    }

    #[test]
    fn secret_file_probes_skip_direct_file_scan_targets() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(&env_path, "SERVICE_TOKEN=secret_secret\n").unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let (scanned_files, file_probes) = scan_secret_file_probes(
            Some(&env_path),
            std::slice::from_ref(&env_path),
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        )
        .unwrap();

        assert_eq!(scanned_files, 0);
        assert_eq!(file_probes, 0);
        assert!(findings.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn secret_file_probe_paths_cover_direct_files_defaults_and_skip_resolution() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(&env_path, "SERVICE_TOKEN=secret_secret\n").unwrap();

        let mut findings = Vec::new();
        let mut errors = Vec::new();
        let mut seen_findings = HashSet::new();
        let mut seen_errors = HashSet::new();
        let (scanned_files, file_probes) = scan_secret_file_probes(
            Some(&env_path),
            &[],
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!((scanned_files, file_probes), (1, 1));
        assert_eq!(findings.len(), 1);
        assert!(errors.is_empty());

        let root_file_skips = SecretScanSkips::new(Some(&env_path), &[PathBuf::from(".env")]);
        assert!(root_file_skips.should_skip(&env_path));
        let relative_skip = PathBuf::from("relative-secret.env");
        let relative_skips = SecretScanSkips::new(None, std::slice::from_ref(&relative_skip));
        assert!(relative_skips.should_skip(&relative_skip));
        assert!(!relative_skips.should_skip(Path::new("other-secret.env")));

        let mut none_findings = Vec::new();
        let mut none_errors = Vec::new();
        let mut none_seen_findings = HashSet::new();
        let mut none_seen_errors = HashSet::new();
        let skipped_defaults = default_secret_scan_paths();
        assert_eq!(
            scan_secret_file_probes(
                None,
                &skipped_defaults,
                &mut none_findings,
                &mut none_errors,
                &mut none_seen_findings,
                &mut none_seen_errors,
                &mut |_| Ok(())
            )
            .unwrap(),
            (0, 0)
        );

        assert!(
            scan_secret_file_probes(
                Some(&temp.path().join("missing")),
                &[],
                &mut Vec::new(),
                &mut Vec::new(),
                &mut HashSet::new(),
                &mut HashSet::new(),
                &mut |_| Ok(())
            )
            .unwrap_err()
            .contains("scan path does not exist")
        );

        let fifo_path = temp.path().join("secret.pipe");
        let fifo_c_path = CString::new(fifo_path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_c_path.as_ptr(), 0o600) }, 0);
        assert!(
            scan_secret_file_probes(
                Some(&fifo_path),
                &[],
                &mut Vec::new(),
                &mut Vec::new(),
                &mut HashSet::new(),
                &mut HashSet::new(),
                &mut |_| Ok(())
            )
            .unwrap_err()
            .contains("not a file or directory")
        );

        assert_eq!(
            scan_secret_file_probes(
                Some(temp.path()),
                &[temp.path().to_path_buf()],
                &mut Vec::new(),
                &mut Vec::new(),
                &mut HashSet::new(),
                &mut HashSet::new(),
                &mut |_| Ok(())
            )
            .unwrap(),
            (0, 0)
        );
    }

    #[test]
    fn secret_scanner_runs_isotope_detectors_and_file_probes() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let scan_root = temp.path().join("project");
        let aws_credentials = home.join(".aws/credentials");
        fs::create_dir_all(aws_credentials.parent().unwrap()).unwrap();
        fs::create_dir_all(&scan_root).unwrap();
        fs::write(
            &aws_credentials,
            "[default]\naws_secret_access_key = secretsecret1234\n",
        )
        .unwrap();
        fs::write(scan_root.join(".npmrc"), "_authToken=npm_secret_token\n").unwrap();

        let cargo_home = temp.path().join("cargo");
        let caroot = temp.path().join("mkcert");
        let helm_config_home = temp.path().join("helm");
        let helm_repository_config = temp.path().join("repositories.yaml");
        let kubeconfig = temp.path().join("kubeconfig");
        let npm_config = temp.path().join("empty-npmrc");
        let uv_credentials_dir = temp.path().join("uv");
        fs::create_dir_all(&uv_credentials_dir).unwrap();
        fs::write(&npm_config, "").unwrap();
        fs::write(&kubeconfig, "").unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            (
                "AWS_SHARED_CREDENTIALS_FILE",
                aws_credentials.to_str().unwrap(),
            ),
            ("CARGO_HOME", cargo_home.to_str().unwrap()),
            ("CAROOT", caroot.to_str().unwrap()),
            ("HELM_CONFIG_HOME", helm_config_home.to_str().unwrap()),
            (
                "HELM_REPOSITORY_CONFIG",
                helm_repository_config.to_str().unwrap(),
            ),
            ("KUBECONFIG", kubeconfig.to_str().unwrap()),
            ("NPM_CONFIG_USERCONFIG", npm_config.to_str().unwrap()),
            ("UV_CREDENTIALS_DIR", uv_credentials_dir.to_str().unwrap()),
        ]);

        let report = run_secret_scan(&SecretScannerRequest {
            path: Some(scan_root.clone()),
            skip_paths: Vec::new(),
            output: OutputMode::Human,
            isotopes_only: false,
        })
        .unwrap();

        assert_eq!(report.summary.isotope_detectors, 0);
        assert!(
            report
                .findings
                .iter()
                .all(|finding| !finding.source.starts_with("isotope:"))
        );
        assert!(report.summary.scanned_files >= 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.source == "file-probe")
        );

        let default_report = run_secret_scan(&SecretScannerRequest {
            path: None,
            skip_paths: Vec::new(),
            output: OutputMode::Human,
            isotopes_only: false,
        })
        .unwrap();
        let has_aws_cli_detector = detect_isotope_install_reasons("aws-cli").is_some();
        if has_aws_cli_detector {
            assert!(default_report.summary.isotope_detectors > 0);
            assert!(
                default_report
                    .findings
                    .iter()
                    .any(|finding| finding.source == "isotope:aws-cli")
            );
        }

        let isotope_only_report = run_secret_scan(&SecretScannerRequest {
            path: Some(scan_root),
            skip_paths: Vec::new(),
            output: OutputMode::Human,
            isotopes_only: true,
        })
        .unwrap();

        assert_eq!(isotope_only_report.summary.scanned_files, 0);
        assert_eq!(isotope_only_report.summary.file_probes, 0);
        assert_eq!(isotope_only_report.summary.isotope_detectors, 0);
        assert!(isotope_only_report.findings.is_empty());
        assert!(
            isotope_only_report
                .findings
                .iter()
                .all(|finding| !finding.source.starts_with("file-probe"))
        );
    }

    #[test]
    fn secret_scanner_helpers_cover_default_paths_and_token_shapes() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let _env = TestEnvGuard::set(&[("HOME", home.to_str().unwrap())]);

        let paths = default_secret_scan_paths();
        assert!(paths.iter().any(|path| path.ends_with(".env")));
        assert!(paths.iter().any(|path| path == &home.join(".bashrc")));
        assert!(paths.iter().any(|path| path == &home.join(".zshrc")));
        assert!(
            paths
                .iter()
                .any(|path| path == &home.join(".aws/credentials"))
        );

        let stripe_live = ["sk", "live", "abcdefghijklmnopqrstuvwxyz"].join("_");
        for token in [
            "ghp_abcdefghijklmnopqrstuvwxyz",
            "gho_abcdefghijklmnopqrstuvwxyz",
            "ghs_abcdefghijklmnopqrstuvwxyz",
            "github_pat_abcdefghijklmnopqrstuvwxyz",
            "glpat-abcdefghijklmnopqrstuvwxyz",
            "xoxb-abcdefghijklmnopqrstuvwxyz",
            "xoxp-abcdefghijklmnopqrstuvwxyz",
            stripe_live.as_str(),
            "npm_abcdefghijklmnop",
            "sk-abcdefghijklmnopqrstuv",
            "AKIAABCDEFGHIJKLMNOP",
        ] {
            assert!(secret_value_has_known_token_shape(token), "{token}");
        }
        assert!(!secret_value_has_known_token_shape("npm_short"));
        assert!(!secret_value_has_known_token_shape("plain-secret-value"));
    }

    #[test]
    fn is_list_subcommand_accepts_both_aliases() {
        assert!(is_list_subcommand("list"));
        assert!(is_list_subcommand("ls"));
        assert!(!is_list_subcommand("outdated"));
    }

    #[test]
    fn is_info_subcommand_accepts_info_only() {
        assert!(is_info_subcommand("info"));
        assert!(!is_info_subcommand("list"));
    }

    #[test]
    fn is_update_subcommand_accepts_update_only() {
        assert!(is_update_subcommand("update"));
        assert!(!is_update_subcommand("outdated"));
        assert!(!is_update_subcommand("install"));
    }

    #[test]
    fn installed_package_names_skip_hidden_entries_and_files() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("deno")).unwrap();
        fs::create_dir_all(temp.path().join("npm/openclaw")).unwrap();
        fs::create_dir_all(temp.path().join("npm/@tobilu/qmd")).unwrap();
        fs::create_dir_all(temp.path().join("pip/psycopg2")).unwrap();
        fs::create_dir_all(temp.path().join(".tmp")).unwrap();
        fs::write(temp.path().join("README"), b"not a package").unwrap();
        write_package_receipt(
            &temp.path().join("npm/openclaw").join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "npm:openclaw".to_string(),
                version: "1.2.3".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "openclaw".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &temp.path().join("npm/@tobilu/qmd").join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "npm:@tobilu/qmd".to_string(),
                version: "0.1.0".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "@tobilu/qmd".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &temp.path().join("pip/psycopg2").join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "pip:psycopg2".to_string(),
                version: "2.9.10".to_string(),
                source: PackageReceiptSource::Pip {
                    package_name: "psycopg2".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let mut installed = installed_package_names(temp.path()).unwrap();
        installed.sort();
        assert_eq!(
            installed,
            vec![
                "deno".to_string(),
                "npm:@tobilu/qmd".to_string(),
                "npm:openclaw".to_string(),
                "pip:psycopg2".to_string()
            ]
        );
    }

    #[test]
    fn installed_package_names_include_isotopes_from_subdir() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("iso/gh")).unwrap();
        fs::create_dir_all(temp.path().join("iso/.tmp")).unwrap();

        let mut names = installed_package_names(temp.path()).unwrap();
        names.sort();

        assert_eq!(names, vec!["isotope:gh".to_string()]);
    }

    #[test]
    fn gh_isotope_migration_updates_keychain_without_login_subprocess() {
        let isotope = isotope_package_data("gh").unwrap();
        let script = isotope.migrate.as_deref().unwrap();

        assert_eq!(script, "/opt/iso/gh/bin/gh auth av-migrate \"$@\"");
        assert!(!script.contains("auth login"));
        assert!(!script.contains("--with-token"));
    }

    #[test]
    fn custom_isotope_migration_runs_rewritten_script_and_reports_failures() {
        if is_root() {
            return;
        }

        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("install");
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&install_root).unwrap();
        fs::create_dir_all(&tmp_root).unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("USER", "coverage-user"),
            ("LOGNAME", "coverage-logname"),
        ]);
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "isotope:coverage-migrate".to_string(),
            root_formula: "isotope:coverage-migrate".to_string(),
            stable_root: install_root.clone(),
            install_root: install_root.clone(),
            tmp_root,
        };
        let isotope = IsotopePackageData {
            name: "coverage-migrate".to_string(),
            replaces: Some("brew:coverage-replaced".to_string()),
            modifies: None,
            migrate: Some(
                "#!/bin/sh\nprintf '%s\\n%s\\n%s\\n' \"$ISOTOPE_NAME\" \"$ISOTOPE_PREFIX\" \"$USER\" > /opt/iso/repository-leaf/migration.out\n"
                    .to_string(),
            ),
            _repository: Some("example/repository-leaf".to_string()),
            _upstream_repository: None,
            version: "1.0.0".to_string(),
            release_url: None,
            archive_url: None,
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        let progress = InstallProgress::with_callback("coverage-migrate", None);

        run_isotope_migration(&plan, &isotope, Some(&progress)).unwrap();

        let output = fs::read_to_string(install_root.join("migration.out")).unwrap();
        assert!(output.contains("coverage-migrate"));
        assert!(output.contains(install_root.to_str().unwrap()));
        assert!(output.contains("coverage-user"));

        let failing = IsotopePackageData {
            migrate: Some("echo migration-broke >&2\nexit 9\n".to_string()),
            ..isotope
        };
        let err = run_isotope_migration(&plan, &failing, Some(&progress)).unwrap_err();
        assert!(err.contains("migration failed for coverage-migrate"));
        assert!(err.contains("exit code 9"));
        assert!(err.contains("migration-broke"));
    }

    #[test]
    fn gh_isotope_migration_plan_reports_replacement_package() {
        let plan = ops::isotope_migration_plan("gh").unwrap();

        assert_eq!(plan.isotope_name, "gh");
        assert_eq!(plan.replaces_package, Some("gh".to_string()));
        assert_eq!(plan.modifies_package, None);
        assert!(!plan.is_radioisotope);
        assert!(plan.has_migration);
    }

    #[test]
    fn aws_cli_radioisotope_plan_reports_modified_formula() {
        let plan = ops::isotope_migration_plan("aws-cli").unwrap();

        assert_eq!(plan.isotope_name, "aws-cli");
        assert_eq!(plan.replaces_package, None);
        assert_eq!(plan.modifies_package, Some("awscli".to_string()));
        assert_eq!(
            plan.is_radioisotope,
            isotope_has_post_install("isotope:aws-cli")
        );
        assert!(plan.has_migration);
        assert!(!isotope_has_post_install("isotope:gh"));
    }

    #[test]
    fn node_versioned_radioisotope_plan_reports_versioned_formula() {
        let plan = ops::isotope_migration_plan("node@24").unwrap();

        assert_eq!(plan.isotope_name, "node@24");
        assert_eq!(plan.replaces_package, None);
        assert_eq!(plan.modifies_package, Some("node@24".to_string()));
        assert!(plan.is_radioisotope);
        assert!(plan.has_migration);
    }

    #[test]
    fn explicit_homebrew_formula_install_uses_radioisotope_when_available() {
        assert_eq!(
            radioisotope_name_for_homebrew_formula_install("node@24").unwrap(),
            Some("node@24".to_string())
        );
        assert_eq!(
            radioisotope_name_for_homebrew_formula_install("ripgrep").unwrap(),
            None
        );
    }

    #[test]
    fn terraform_radioisotope_plan_reports_modified_vendor_package() {
        let isotope = isotope_package_data("terraform").unwrap();
        let plan = ops::isotope_migration_plan("terraform").unwrap();

        assert_eq!(isotope.modifies.as_deref(), Some("av:terraform"));
        assert_eq!(
            isotope_modified_package_name(isotope).unwrap(),
            Some("terraform".to_string())
        );
        assert_eq!(plan.isotope_name, "terraform");
        assert_eq!(plan.replaces_package, None);
        assert_eq!(plan.modifies_package, Some("terraform".to_string()));
        assert!(plan.is_radioisotope);
    }

    #[test]
    fn auto_install_prefers_installable_isotopes_for_matching_targets() {
        assert_eq!(
            preferred_auto_isotope_name("terraform").unwrap(),
            Some("terraform".to_string())
        );
        assert_eq!(
            installable_isotope_name_for_target(&PackageAliasTarget::HomebrewFormula(
                "awscli".to_string()
            ))
            .unwrap(),
            Some("aws-cli".to_string())
        );
        assert_eq!(
            installable_isotope_name_for_target(&PackageAliasTarget::HomebrewFormula(
                "node@24".to_string()
            ))
            .unwrap(),
            Some("node@24".to_string())
        );
        assert_eq!(
            installable_isotope_name_for_target(&PackageAliasTarget::HomebrewFormula(
                "curl".to_string()
            ))
            .unwrap(),
            None
        );
    }

    #[test]
    fn isotope_installability_distinguishes_payloads_from_detector_only_records() {
        assert!(isotope_is_installable(
            isotope_package_data("terraform").unwrap()
        ));
        let curl = isotope_package_data("curl").unwrap();
        assert_eq!(curl.version, "detector-only");
        assert!(!isotope_is_installable(curl));

        let archive_backed = IsotopePackageData {
            name: "isotope:archive-backed".to_string(),
            replaces: Some("brew:archive-backed".to_string()),
            modifies: None,
            migrate: None,
            _repository: None,
            _upstream_repository: None,
            version: "1.0.0".to_string(),
            release_url: None,
            archive_url: Some("https://example.test/archive.tgz".to_string()),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        assert!(isotope_is_installable(&archive_backed));

        let config = Config {
            bottle_tag: "all".to_string(),
        };
        let metadata_only = IsotopePackageData {
            name: "isotope:metadata-only".to_string(),
            replaces: None,
            modifies: None,
            ..archive_backed.clone()
        };
        assert!(
            isotope_dependency_graph(&metadata_only, &config)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            isotope_stub_executables(
                &metadata_only,
                &[(
                    "metadata-tool".to_string(),
                    PathBuf::from("bin/metadata-tool")
                )],
            )
            .unwrap(),
            ["metadata-tool".to_string()]
        );

        let npm_replacement = IsotopePackageData {
            name: "isotope:npm-replacement".to_string(),
            replaces: Some("npm:not-radio".to_string()),
            modifies: None,
            ..archive_backed.clone()
        };
        assert!(
            isotope_dependency_graph(&npm_replacement, &config)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            isotope_replaced_package_target(&npm_replacement).unwrap(),
            None
        );
        assert_eq!(
            isotope_modified_or_replaced_package_name(&npm_replacement).unwrap(),
            Some("npm:not-radio".to_string())
        );

        let invalid_modification = IsotopePackageData {
            name: "isotope:invalid-modification".to_string(),
            replaces: None,
            modifies: Some("npm:not-radio".to_string()),
            ..archive_backed.clone()
        };
        assert!(
            isotope_modified_package_target(&invalid_modification)
                .unwrap_err()
                .contains("radioisotopes may only modify")
        );
        assert!(
            radioisotope_modified_install_name(&PackageAliasTarget::NpmPackage(
                "not-radio".to_string()
            ))
            .unwrap_err()
            .contains("radioisotopes may only modify")
        );
    }

    #[test]
    fn run_i_package_dispatches_current_cask_isotope_and_error_paths() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let config = Config {
            bottle_tag: "all".to_string(),
        };
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();

        for package_name in ["codex", "terraform", "isotope:gh", "isotope:terraform"] {
            let install_root = package_install_root(&opt_root, package_name).unwrap();
            if fs::symlink_metadata(&install_root).is_ok() {
                remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
            }
        }

        let cask = embedded_cask("codex").unwrap();
        let cask_plan = InstallPlan::for_i("codex".to_string(), "codex".to_string());
        fs::create_dir_all(&cask_plan.install_root).unwrap();
        write_package_receipt(
            &cask_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "codex".to_string(),
                version: cask.version.clone(),
                source: PackageReceiptSource::Cask {
                    cask_name: "codex".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_root_ownership_manifest(&cask_plan, Vec::new()).unwrap();

        run_i_package(
            &config,
            RequestedPackage::HomebrewCask("codex".to_string()),
            InstallOptions {
                intent: InstallIntent::Update,
            },
        )
        .unwrap();
        assert_eq!(
            load_package_receipt(&cask_plan.root_receipt_path())
                .unwrap()
                .unwrap()
                .source,
            PackageReceiptSource::Cask {
                cask_name: "codex".to_string()
            }
        );

        let isotope_name = "gh";
        let isotope_package = isotope_qualified_name(isotope_name);
        let isotope = isotope_package_data(isotope_name).unwrap();
        let isotope_plan = InstallPlan::for_i_isotope(isotope_package.clone(), isotope_name);
        let gh_binary = isotope_plan.install_root.join("bin/gh");
        fs::create_dir_all(gh_binary.parent().unwrap()).unwrap();
        fs::write(&gh_binary, b"#!/bin/sh\nprintf gh\n").unwrap();
        let mut permissions = fs::metadata(&gh_binary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&gh_binary, permissions).unwrap();
        write_root_executable_manifest(
            &isotope_plan.root_executables_manifest_path(),
            &["gh".to_string()],
        )
        .unwrap();
        write_root_ownership_manifest(&isotope_plan, Vec::new()).unwrap();
        write_package_receipt(
            &isotope_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: isotope_package.clone(),
                version: isotope.version.clone(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: isotope_name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let mut bottle_server = start_counting_test_http_server(vec![(
            "/gh.tar.gz".to_string(),
            b"not a bottle".to_vec(),
        )]);
        let formula_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "2.80.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": "0".repeat(64),
                            "url": format!("{}/gh.tar.gz", bottle_server.base_url),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let mut formula_server =
            start_counting_test_http_server(vec![("/gh.json".to_string(), formula_json)]);
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(formula_server.base_url.clone()),
            ..Default::default()
        });

        run_i_package(
            &config,
            RequestedPackage::Isotope("gh".to_string()),
            InstallOptions {
                intent: InstallIntent::Update,
            },
        )
        .unwrap();
        let auto_err = run_i_package(
            &config,
            RequestedPackage::Auto("gh".to_string()),
            InstallOptions {
                intent: InstallIntent::Update,
            },
        )
        .unwrap_err();
        assert!(auto_err.contains("gh"));
        assert!(is_executable(&bin_root.join("gh")));
        assert_eq!(
            load_package_receipt(&isotope_plan.root_receipt_path())
                .unwrap()
                .unwrap()
                .version,
            isotope.version
        );
        assert!(formula_server.request_count() >= 1);
        assert!(bottle_server.request_count() >= 1);
        bottle_server.stop().unwrap();
        formula_server.stop().unwrap();

        if fs::symlink_metadata(bin_root.join("gh")).is_ok() {
            remove_path(&bin_root.join("gh")).unwrap();
        }
        let stub_paths = install_isotope_stubs(isotope_name, None).unwrap();
        assert_eq!(stub_paths, vec![bin_root.join("gh").display().to_string()]);
        assert!(is_executable(&bin_root.join("gh")));

        let non_radio = run_i_radioisotope(
            &config,
            isotope_package,
            isotope_name.to_string(),
            InstallIntent::Install,
            None,
        )
        .unwrap_err();
        assert!(non_radio.contains("isotope:gh is not a radioisotope"));

        let invalid_modified_target = run_i_modified_package(
            &config,
            "npm:not-radio".to_string(),
            &PackageAliasTarget::NpmPackage("not-radio".to_string()),
            InstallIntent::Install,
            None,
        )
        .unwrap_err();
        assert!(invalid_modified_target.contains("radioisotopes may only modify"));

        let invalid_vendor_modification = run_i_modified_package(
            &config,
            "missing-vendor".to_string(),
            &PackageAliasTarget::VendorPackage("not-a-registered-vendor".to_string()),
            InstallIntent::Install,
            None,
        )
        .unwrap_err();
        assert!(invalid_vendor_modification.contains("not-a-registered-vendor is not registered"));

        let terraform_launcher = Path::new("/opt/terraform/bin/terraform");
        if !terraform_launcher.exists() {
            let terraform_record = isotope_package_data("terraform").unwrap();
            let terraform_package = isotope_modified_package_name(terraform_record)
                .unwrap()
                .unwrap();
            let seed_terraform_install = || {
                let terraform_plan = InstallPlan::for_i_radioisotope(
                    "isotope:terraform".to_string(),
                    terraform_package.clone(),
                );
                fs::create_dir_all(&terraform_plan.install_root).unwrap();
                write_root_ownership_manifest(&terraform_plan, Vec::new()).unwrap();
                write_package_receipt(
                    &terraform_plan.root_receipt_path(),
                    &PackageReceipt {
                        package_name: terraform_package.clone(),
                        version: "1.2.3".to_string(),
                        source: PackageReceiptSource::Vendor {
                            vendor_name: terraform_package.clone(),
                        },
                        metadata: PackageMetadata::default(),
                    },
                )
                .unwrap();
            };

            for requested in [
                RequestedPackage::Auto("terraform".to_string()),
                RequestedPackage::Isotope("terraform".to_string()),
            ] {
                seed_terraform_install();
                let result = run_i_package(
                    &config,
                    requested,
                    InstallOptions {
                        intent: InstallIntent::Install,
                    },
                );
                if using_radioisotope_fixture_integrations() {
                    result.unwrap();
                } else {
                    let err = result.unwrap_err();
                    assert!(
                        err.contains("terraform"),
                        "expected terraform install error, got: {err}"
                    );
                }
            }
        }

        let err = run_i_package(
            &config,
            RequestedPackage::VendorPackage("not-a-registered-vendor".to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap_err();
        assert!(err.contains("not-a-registered-vendor is not registered"));

        let err = run_i_package(
            &config,
            RequestedPackage::VendorPackage("not-a-registered-vendor".to_string()),
            InstallOptions {
                intent: InstallIntent::Update,
            },
        )
        .unwrap_err();
        assert!(err.contains("not-a-registered-vendor is not registered"));

        let err = run_i_package(
            &config,
            RequestedPackage::NpmPackage {
                package: "@scope".to_string(),
                version: None,
            },
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap_err();
        assert!(err.contains("scoped npm package names"));

        let err = run_i_package(
            &config,
            RequestedPackage::PipPackage("bad/name".to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap_err();
        assert!(err.contains("pip package names must not contain path separators"));

        let err = run_i_package(
            &config,
            RequestedPackage::Isotope("bad/name".to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap_err();
        assert!(err.contains("qualified package name must not contain"));

        assert!(
            run_i_cask(
                &config,
                "missing-cask".to_string(),
                "missing-cask".to_string(),
                InstallIntent::Install,
                None,
            )
            .unwrap_err()
            .contains("no embedded cask metadata found")
        );
        assert!(
            run_i_isotope(
                &config,
                "isotope:missing-isotope".to_string(),
                "missing-isotope".to_string(),
                true,
                InstallIntent::Install,
                None,
            )
            .unwrap_err()
            .contains("unknown isotope")
        );
        assert!(
            run_i_radioisotope(
                &config,
                "isotope:missing-radio".to_string(),
                "missing-radio".to_string(),
                InstallIntent::Install,
                None,
            )
            .unwrap_err()
            .contains("unknown isotope")
        );
        assert!(
            run_i_isotope_root_only(
                &config,
                "isotope:missing-root".to_string(),
                "missing-root".to_string(),
                None,
            )
            .unwrap_err()
            .contains("unknown isotope")
        );

        for package_name in ["codex", "terraform", "isotope:gh", "isotope:terraform"] {
            remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
        }
    }

    #[test]
    fn radioisotope_update_refreshes_modified_formula() {
        assert_eq!(
            radioisotope_modified_formula_intent(InstallIntent::Install),
            None
        );
        assert_eq!(
            radioisotope_modified_formula_intent(InstallIntent::Update),
            Some(InstallIntent::Update)
        );
        assert_eq!(
            radioisotope_modified_formula_intent(InstallIntent::Reinstall),
            Some(InstallIntent::Reinstall)
        );
    }

    #[test]
    fn aws_cli_radioisotope_info_uses_modified_formula_description() {
        let _env_lock = test_env_lock().lock().unwrap();
        let (base, _server) = start_test_http_server(
            vec![(
                "/awscli.json".to_string(),
                serde_json::to_vec(&serde_json::json!({
                    "desc": "Official Amazon AWS command-line interface",
                    "homepage": "https://aws.amazon.com/cli/",
                    "license": "Apache-2.0",
                    "versions": {"stable": "2.32.0"},
                    "revision": 0,
                    "dependencies": ["python@3.14"],
                    "bottle": {
                        "stable": {
                            "files": {
                                "arm64_tahoe": {
                                    "sha256": "awscli-sha",
                                    "url": "https://example.invalid/awscli.tar.gz"
                                }
                            }
                        }
                    },
                    "disabled": false,
                    "post_install_defined": false
                }))
                .unwrap(),
            )],
            1,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(base),
            ..Default::default()
        });
        let mut info = PackageInfo {
            package_name: "isotope:aws-cli".to_string(),
            qualified_name: "isotope:aws-cli".to_string(),
            install_root: PathBuf::from("/opt/awscli"),
            installed: true,
            source: Some(PackageReceiptSource::Isotope {
                isotope_name: "aws-cli".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: Some("2.31.0".to_string()),
            latest_version: None,
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        populate_package_info_metadata(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &mut info,
        );

        assert_eq!(info.latest_version, Some("2.32.0".to_string()));
        assert_eq!(
            info.homebrew_info,
            Some(HomebrewPackageInfo {
                formula: "awscli".to_string(),
                description: Some("Official Amazon AWS command-line interface".to_string()),
                homepage: Some("https://aws.amazon.com/cli/".to_string()),
                repository: None,
                upstream_docs: None,
                docs: Vec::new(),
                license: Some("Apache-2.0".to_string()),
                dependencies: vec!["python@3.14".to_string()],
            })
        );
    }

    #[test]
    fn isotope_migration_script_is_executable_shell_script() {
        let isotope = isotope_package_data("gh").unwrap();
        let script = isotope.migrate.as_deref().unwrap();
        let plan = InstallPlan::for_i_isotope("isotope:gh".to_string(), "gh");
        let executable = executable_isotope_migration_script(script, &plan, isotope).unwrap();

        assert!(executable.starts_with("#!/bin/sh\n"));
        assert!(executable.contains("isotope migration must not run as root"));
        assert!(executable.contains("exit 77"));
    }

    #[test]
    fn isotope_migration_script_rewrites_repository_named_install_root() {
        let isotope = IsotopePackageData {
            name: "isotope:supabase".to_string(),
            replaces: Some("brew:supabase".to_string()),
            modifies: None,
            migrate: None,
            _repository: Some("automic-vault/supabase-cli".to_string()),
            _upstream_repository: None,
            version: "2.102.0".to_string(),
            release_url: Some("https://example.test/isotopes/supabase".to_string()),
            archive_url: Some("https://example.test/supabase-cli.tgz".to_string()),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        let plan = InstallPlan::for_i_isotope("isotope:supabase".to_string(), "supabase");
        let executable = executable_isotope_migration_script(
            "/opt/iso/supabase-cli/bin/supabase-go av-migrate \"$@\"",
            &plan,
            &isotope,
        )
        .unwrap();

        assert!(
            executable.contains(
                &plan
                    .install_root
                    .join("bin/supabase-go")
                    .display()
                    .to_string()
            )
        );
        assert!(!executable.contains("/opt/iso/supabase-cli"));
        assert!(!executable.contains("/tmp/opt/iso/supabase-cli"));
    }

    #[test]
    fn isotope_migration_script_rewrites_legacy_isotopes_install_root() {
        let isotope = isotope_package_data("gh").unwrap();
        let plan = InstallPlan::for_i_isotope("isotope:gh".to_string(), "gh");
        let executable = executable_isotope_migration_script(
            "/opt/isotopes/gh/bin/gh auth av-migrate \"$@\"",
            &plan,
            isotope,
        )
        .unwrap();

        assert!(executable.contains(&plan.install_root.join("bin/gh").display().to_string()));
        assert!(!executable.contains("/opt/isotopes/gh"));
        assert!(!executable.contains("/tmp/opt/isotopes/gh"));
    }

    #[test]
    fn isotope_stub_executables_use_replaced_formula_metadata() {
        let isotope = isotope_package_data("aws-cli").unwrap();
        let discovered = vec![
            ("aws".to_string(), PathBuf::from("/opt/iso/aws-cli/bin/aws")),
            (
                "aws_completer".to_string(),
                PathBuf::from("/opt/iso/aws-cli/bin/aws_completer"),
            ),
            (
                "python3.14".to_string(),
                PathBuf::from("/opt/iso/aws-cli/bin/python3.14"),
            ),
        ];

        assert_eq!(
            isotope_stub_executables(isotope, &discovered).unwrap(),
            vec!["aws".to_string(), "aws_completer".to_string()]
        );
    }

    #[test]
    fn progress_log_event_serializes_for_helper_bridge() {
        let event = ProgressEvent::Log {
            package: "isotope:gh".to_string(),
            message: "migrating secrets".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();

        assert_eq!(
            json,
            r#"{"Log":{"package":"isotope:gh","message":"migrating secrets"}}"#
        );
    }

    #[test]
    fn install_progress_reports_download_fraction_without_terminal_bar() {
        let events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let callback_events = Arc::clone(&events);
        let callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                callback_events.lock().unwrap().push(event);
            })));
        let progress = InstallProgress::with_callback("brew:sqlite", Some(callback));

        progress.begin_download_phase();
        progress.add_download_total(Some(100));
        progress.advance_download(25);
        progress.advance_download(25);

        let download_progress = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                ProgressEvent::Downloading { progress, .. } => Some(*progress),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            download_progress
                .iter()
                .any(|progress| (*progress - 0.25).abs() < f32::EPSILON),
            "expected 25% download progress, got {download_progress:?}"
        );
        assert!(
            download_progress
                .iter()
                .any(|progress| (*progress - 0.50).abs() < f32::EPSILON),
            "expected 50% download progress, got {download_progress:?}"
        );
    }

    #[test]
    fn install_progress_tracks_transitive_downloads_individually() {
        let events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let callback_events = Arc::clone(&events);
        let callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                callback_events.lock().unwrap().push(event);
            })));
        let progress = InstallProgress::with_callback("brew:yt-dlp", Some(callback));

        progress.begin_download_phase();
        progress.begin_download_for("yt-dlp");
        progress.add_download_total_for("yt-dlp", Some(100));
        progress.advance_download_for("yt-dlp", 100);
        progress.begin_download_for("python@3.14");
        progress.add_download_total_for("python@3.14", Some(300));
        progress.advance_download_for("python@3.14", 150);

        let download_events = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                ProgressEvent::Downloading {
                    package, progress, ..
                } => Some((package.clone(), *progress)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            download_events
                .iter()
                .any(|(package, progress)| package == "yt-dlp"
                    && (*progress - 1.0).abs() < f32::EPSILON),
            "expected yt-dlp to reach 100%, got {download_events:?}"
        );
        assert!(
            download_events
                .iter()
                .any(|(package, progress)| package == "python@3.14"
                    && (*progress - 0.50).abs() < f32::EPSILON),
            "expected python@3.14 to report 50%, got {download_events:?}"
        );
    }

    #[test]
    fn install_progress_emits_fallback_download_state_without_package_entry() {
        let events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let callback_events = Arc::clone(&events);
        let callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                callback_events.lock().unwrap().push(event);
            })));
        let progress = InstallProgress::with_callback("brew:sqlite", Some(callback));
        *progress.bytes_downloaded.lock().unwrap() = 75;
        *progress.total_bytes.lock().unwrap() = Some(100);
        *progress.download_started_at.lock().unwrap() =
            Some(Instant::now() - Duration::from_secs(3));

        progress.emit_downloading_for("sqlite");

        let event = events
            .lock()
            .unwrap()
            .iter()
            .find_map(|event| match event {
                ProgressEvent::Downloading {
                    package,
                    bytes_per_sec,
                    progress,
                } => Some((package.clone(), *bytes_per_sec, *progress)),
                _ => None,
            })
            .expect("fallback download event should be emitted");
        assert_eq!(event.0, "sqlite");
        assert!(event.1 > 0);
        assert!((event.2 - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn install_progress_helpers_cover_empty_no_callback_and_style_paths() {
        let progress = InstallProgress::with_callback("coverage-progress", None);

        progress.emit(ProgressEvent::Resolving);
        progress.begin_download_phase();
        progress.add_download_total(None);
        progress.add_download_total(Some(0));
        progress.advance_download(0);
        progress.emit_downloading_for("missing-package");
        progress.begin_install_phase();
        progress.begin_install_phase();
        progress.log("\n\r");
        progress.log("first line\nsecond line");
        progress.finish_with_paths(&[]);
        progress.finish_with_paths(&["/tmp/av".to_string(), "/tmp/nuke-helper".to_string()]);
        progress.clear();

        let _ = download_progress_style();
        let _ = install_progress_style();
        let _ = final_progress_style();
    }

    #[test]
    fn installed_package_summary_serializes_source() {
        let summary = core::InstalledPackageSummary {
            name: "isotope:gh".to_string(),
            source: PackageReceiptSource::Isotope {
                isotope_name: "gh".to_string(),
            },
            version: "2.80.0".to_string(),
            description: None,
            homepage: None,
            repository: None,
            upstream_docs: None,
            docs: Vec::new(),
            category: None,
            security_state: None,
            installed_versions: Vec::new(),
            install_package_names: Vec::new(),
        };
        let json = serde_json::to_string(&summary).unwrap();

        assert_eq!(
            json,
            r#"{"name":"isotope:gh","source":{"kind":"isotope","isotope_name":"gh"},"version":"2.80.0","description":null,"securityState":null}"#
        );
    }

    #[test]
    fn package_security_state_runs_detect_for_installed_isotopes() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let aws_dir = temp.path().join(".aws");
        fs::create_dir_all(&aws_dir).unwrap();
        fs::write(
            aws_dir.join("credentials"),
            "[default]\naws_secret_access_key = secret\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[("HOME", temp.path().to_str().unwrap())]);

        let state = package_security_state_for_identifiers(["awscli".to_string()]);

        if detect_isotope_install_reasons("aws-cli").is_some() {
            let state = state.expect("aws-cli should have security state");
            assert_eq!(state.isotope_name, "aws-cli");
            assert!(state.install_is_insecure);
            assert!(state.remediation_available);
            assert!(
                state
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("AWS shared credentials file")),
                "expected credentials reason, got {:?}",
                state.reasons
            );
            assert_eq!(state.error, None);
        } else {
            assert_eq!(state, None);
        }
    }

    #[test]
    fn package_security_state_runs_detect_for_uninstalled_package_info() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let aws_dir = temp.path().join(".aws");
        fs::create_dir_all(&aws_dir).unwrap();
        fs::write(
            aws_dir.join("credentials"),
            "[default]\naws_secret_access_key = secret\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[("HOME", temp.path().to_str().unwrap())]);

        let info = PackageInfo {
            package_name: "awscli".to_string(),
            qualified_name: "brew:awscli".to_string(),
            install_root: PathBuf::from("/opt/awscli"),
            installed: false,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "awscli".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: None,
            latest_version: None,
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let state = package_security_state(&info);

        if detect_isotope_install_reasons("aws-cli").is_some() {
            let state = state.expect("aws-cli should have security state");
            assert_eq!(state.isotope_name, "aws-cli");
            assert!(state.install_is_insecure);
            assert!(state.remediation_available);
            assert!(
                state
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("AWS shared credentials file")),
                "expected credentials reason, got {:?}",
                state.reasons
            );
            assert_eq!(state.error, None);
        } else {
            assert_eq!(state, None);
        }
    }

    #[test]
    fn gh_security_state_reports_manifest_migration_remediation() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("hosts.yml"),
            "github.com:\n    oauth_token: ghp_secret\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[
            ("GH_CONFIG_DIR", temp.path().to_str().unwrap()),
            ("HOME", temp.path().to_str().unwrap()),
        ]);

        let state = package_security_state_for_identifiers(["brew:gh".to_string()])
            .expect("gh should have security state");

        assert_eq!(state.isotope_name, "gh");
        assert!(state.install_is_insecure);
        assert!(state.remediation_available);
        assert!(
            state
                .reasons
                .iter()
                .any(|reason| reason.contains("GitHub CLI hosts file")),
            "expected hosts file reason, got {:?}",
            state.reasons
        );
        assert_eq!(state.error, None);
    }

    #[test]
    fn hf_security_state_reports_huggingface_cli_remediation() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let token_dir = temp.path().join(".cache/huggingface");
        fs::create_dir_all(&token_dir).unwrap();
        fs::write(token_dir.join("token"), "hf_secret\n").unwrap();
        let _env = TestEnvGuard::set(&[("HOME", temp.path().to_str().unwrap())]);

        let state = package_security_state_for_identifiers(["brew:hf".to_string()])
            .expect("brew:hf should have security state");

        assert_eq!(state.isotope_name, "huggingface-cli");
        assert!(state.install_is_insecure);
        assert!(state.remediation_available);
        assert!(
            state
                .reasons
                .iter()
                .any(|reason| reason.contains("Hugging Face token file")),
            "expected Hugging Face token reason, got {:?}",
            state.reasons
        );
        assert_eq!(state.error, None);
    }

    #[test]
    fn package_security_state_prefers_versioned_node_radioisotope() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join(".npmrc"),
            "//registry.npmjs.org/:_authToken=npm_secret\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[("HOME", temp.path().to_str().unwrap())]);

        let info = PackageInfo {
            package_name: "node".to_string(),
            qualified_name: "brew:node@24".to_string(),
            install_root: PathBuf::from("/opt/homebrew/Cellar/node@24"),
            installed: true,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "node@24".to_string(),
            }),
            source_error: None,
            aliases: Vec::new(),
            aliases_error: None,
            installed_version: Some("24.16.0".to_string()),
            latest_version: Some("24.16.0".to_string()),
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let state =
            package_security_state(&info).expect("brew:node@24 should have node@24 security state");

        assert_eq!(state.isotope_name, "node@24");
        assert!(state.install_is_insecure);
        assert!(state.remediation_available);
        assert!(
            state
                .reasons
                .iter()
                .any(|reason| reason.contains("npm user config")),
            "expected npm user config reason, got {:?}",
            state.reasons
        );
        assert_eq!(state.error, None);

        for identifier in ["node@24", "isotope:node@24", "brew:node@24"] {
            let state = package_security_state_for_identifiers([identifier.to_string()])
                .unwrap_or_else(|| panic!("{identifier} should map to node@24"));
            assert_eq!(isotope_unqualified_name(&state.isotope_name), "node@24");
            assert!(state.install_is_insecure);
        }
    }

    #[test]
    fn package_security_state_reports_detector_only_radioisotopes_without_remediation() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(home.join(".ssh")).unwrap();
        fs::create_dir_all(home.join(".gem")).unwrap();
        fs::create_dir_all(home.join(".cpan/CPAN")).unwrap();
        fs::create_dir_all(home.join(".ssl")).unwrap();
        fs::write(
            home.join(".git-credentials"),
            "https://user:supersecret@example.com/repo.git\n",
        )
        .unwrap();
        fs::write(
            home.join(".netrc"),
            "machine example.com login user password supersecret\n",
        )
        .unwrap();
        fs::write(home.join(".rsync_pass"), "supersecret\n").unwrap();
        fs::write(
            home.join(".gem/credentials"),
            ":rubygems_api_key: rubygems_secret\n",
        )
        .unwrap();
        fs::write(
            home.join(".cpan/CPAN/MyConfig.pm"),
            "'proxy_pass' => 'supersecret',\n",
        )
        .unwrap();
        fs::write(
            home.join(".ssh/id_rsa"),
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::write(
            home.join(".ssl/key.pem"),
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::write(
            home.join(".bash_profile"),
            "export BASH_SERVICE_TOKEN=secret_secret\n",
        )
        .unwrap();
        fs::write(
            home.join(".zshrc"),
            "export OPENAI_API_KEY=\"sk-proj-THIS_IS_A_FAKE_KEY_FOR_TESTING_ONLY_1234567890abcdefghijklmnopqrstuvwxyz\"\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[("HOME", home.to_str().unwrap())]);

        for (package, isotope, reason) in [
            ("brew:bash", "bash", "Bash startup file"),
            ("brew:git", "git", "Git credential store"),
            ("brew:curl", "curl", "curl netrc"),
            ("brew:zsh", "zsh", "Zsh startup file"),
            ("brew:rsync", "rsync", "rsync password file"),
            ("brew:ruby", "ruby", "RubyGems credentials"),
            ("brew:perl", "perl", "CPAN config"),
            ("brew:openssh", "openssh", "SSH private key"),
            ("brew:openssl@3", "openssl@3", "OpenSSL private key"),
        ] {
            if detect_isotope_install_reasons(isotope).is_none() {
                continue;
            }
            let state = package_security_state_for_identifiers([package.to_string()])
                .unwrap_or_else(|| panic!("{package} should have security state"));
            assert_eq!(state.isotope_name, isotope);
            assert!(state.install_is_insecure, "{package}");
            assert!(!state.remediation_available, "{package}");
            assert!(
                state
                    .reasons
                    .iter()
                    .any(|candidate| candidate.contains(reason)),
                "expected {reason:?} in {:?}",
                state.reasons
            );
        }
    }

    #[test]
    fn git_security_state_reports_credential_fill_learn_more_guidance() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let bin = temp.path().join("bin");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&bin).unwrap();
        let git = bin.join("git");
        fs::write(
            &git,
            "#!/bin/sh\n\
             if [ \"$1\" != credential ] || [ \"$2\" != fill ]; then exit 2; fi\n\
             cat >/dev/null\n\
             printf 'protocol=https\\nhost=github.com\\nusername=x-access-token\\npassword=github_pat_fake\\n\\n'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&git).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&git, permissions).unwrap();

        let path = match env::var_os("PATH") {
            Some(existing) if !existing.is_empty() => {
                format!("{}:{}", bin.display(), existing.to_string_lossy())
            }
            _ => bin.display().to_string(),
        };
        let _unset_disable =
            TestEnvGuard::unset(&["AUTOMIC_VAULT_DISABLE_GIT_CREDENTIAL_FILL_DETECTOR"]);
        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            ("PATH", &path),
            ("AUTOMIC_VAULT_TEST_GIT_CREDENTIAL_FILL_DETECTOR", "1"),
        ]);

        let state = package_security_state_for_identifiers(["brew:git".to_string()])
            .expect("git should have security state");

        assert_eq!(state.isotope_name, "git");
        assert!(state.install_is_insecure);
        assert!(!state.remediation_available);
        assert!(
            state
                .reasons
                .iter()
                .any(|reason| reason.contains("git credential fill")
                    && reason.contains("Click Learn More")
                    && !reason.contains("git credential reject")
                    && !reason.contains("Keychain Access")),
            "expected credential-fill hazard to point to Learn More, got {:?}",
            state.reasons
        );
    }

    fn using_radioisotope_fixture_integrations() -> bool {
        Path::new(env!("AUTOMIC_VAULT_GENERATED_RADIOISOTOPES_REPO"))
            == Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib/rs/fixtures/radioisotopes")
    }

    #[test]
    fn generated_isotope_integrations_tolerate_empty_home() {
        let _lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let cwd = temp.path().join("cwd");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        fs::write(
            cwd.join("hosts.yml"),
            "github.com:\n  oauth_token: cwd-token\n",
        )
        .unwrap();
        let _cwd = CurrentDirGuard::set(&cwd);
        let missing_path = temp.path().join("missing");
        let missing = missing_path.to_str().unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            ("AKAMAI_EDGERC", missing),
            ("ARGOCD_CONFIG_DIR", missing),
            ("AWS_SHARED_CREDENTIALS_FILE", missing),
            ("BITWARDENCLI_APPDATA_DIR", missing),
            ("CARGO_HOME", missing),
            ("CAROOT", missing),
            ("CIVO_CONFIG", missing),
            ("COMPOSER_HOME", missing),
            ("CX_CONFIG_FILE_PATH", missing),
            ("DCOS_DIR", missing),
            ("DIGITALOCEAN_CONFIG", missing),
            ("DOCKER_CONFIG", missing),
            ("GH_CONFIG_DIR", ""),
            ("GLAB_CONFIG_DIR", missing),
            ("HCLOUD_CONFIG", missing),
            ("HELM_CONFIG_HOME", missing),
            ("HELM_REPOSITORY_CONFIG", missing),
            ("KUBECONFIG", missing),
            ("MCP_REMOTE_CONFIG_DIR", missing),
            ("NETRC", missing),
            ("NPM_CONFIG_USERCONFIG", missing),
            ("OCI_CLI_CONFIG_FILE", missing),
            ("PULUMI_CREDENTIALS_PATH", missing),
            ("PULUMI_HOME", missing),
            ("RCLONE_CONFIG", missing),
            ("REGISTRY_AUTH_FILE", missing),
            ("SUPABASE_HOME", missing),
            ("TALOSCONFIG", missing),
            ("TALOS_HOME", missing),
            ("UV_CREDENTIALS_DIR", missing),
            ("VAGRANT_HOME", missing),
            ("XDG_CACHE_HOME", missing),
            ("XDG_CONFIG_HOME", missing),
            ("XDG_RUNTIME_DIR", missing),
            ("XDG_STATE_HOME", missing),
        ]);

        for integration in isotope_integrations::INTEGRATIONS {
            if let Some(detect) = integration.detect {
                assert!(
                    !detect()
                        .unwrap_or_else(|err| panic!("{} detect failed: {err}", integration.name)),
                    "{} should not detect secrets in an empty home",
                    integration.name
                );
            }
            if let Some(detect_reasons) = integration.detect_reasons {
                let reasons = detect_reasons().unwrap_or_else(|err| {
                    panic!("{} detect reasons failed: {err}", integration.name)
                });
                assert!(
                    reasons.is_empty(),
                    "{} should not report reasons in an empty home: {reasons:?}",
                    integration.name
                );
            }
            if let Some(migrate) = integration.migrate {
                migrate()
                    .unwrap_or_else(|err| panic!("{} migrate failed: {err}", integration.name));
            }
        }
    }

    #[test]
    fn generated_isotope_detectors_report_seeded_secret_files() {
        let _lock = test_env_lock().lock().unwrap();
        if isotope_integrations::INTEGRATIONS
            .iter()
            .all(|integration| integration.detect_reasons.is_none())
        {
            return;
        }

        fn write_fixture(path: &Path, contents: impl AsRef<[u8]>) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let xdg_config = temp.path().join("xdg-config");
        let xdg_cache = temp.path().join("xdg-cache");
        let xdg_data = temp.path().join("xdg-data");
        let xdg_state = temp.path().join("xdg-state");
        let xdg_runtime = temp.path().join("xdg-runtime");
        let missing = temp.path().join("missing");
        let akamai_edgerc = temp.path().join("akamai.edgerc");
        let argocd_config = temp.path().join("argocd");
        let aws_credentials = temp.path().join("aws-credentials");
        let bitwarden_appdata = temp.path().join("bitwarden");
        let cargo_home = temp.path().join("cargo");
        let caroot = temp.path().join("mkcert");
        let civo_config = temp.path().join("civo.json");
        let composer_home = temp.path().join("composer");
        let checkmarx_config = temp.path().join("checkmarx.yaml");
        let dcos_dir = temp.path().join("dcos");
        let doctl_config = temp.path().join("doctl.yaml");
        let docker_config = temp.path().join("docker");
        let gh_config = temp.path().join("gh");
        let glab_config = temp.path().join("glab");
        let hcloud_config = temp.path().join("hcloud.toml");
        let helm_config_home = temp.path().join("helm");
        let helm_repository_config = temp.path().join("repositories.yaml");
        let kubeconfig = temp.path().join("kubeconfig");
        let netrc = temp.path().join("netrc");
        let npmrc = temp.path().join("npmrc");
        let oci_config = temp.path().join("oci-config");
        let pulumi_credentials_dir = temp.path().join("pulumi-credentials");
        let pulumi_credentials = pulumi_credentials_dir.join("credentials.json");
        let pulumi_home = temp.path().join("pulumi-home");
        let rclone_config = temp.path().join("rclone.conf");
        let registry_auth = temp.path().join("containers-auth.json");
        let supabase_home = temp.path().join("supabase");
        let talosconfig = temp.path().join("talosconfig");
        let talos_home = temp.path().join("talos");
        let uv_credentials_dir = temp.path().join("uv");
        let vagrant_home = temp.path().join("vagrant");

        write_fixture(
            &home.join(".config/acli/jira_config.yaml"),
            "token: atlassian\n",
        );
        write_fixture(
            &home.join(".bash_profile"),
            "export BASH_SERVICE_TOKEN=secret_secret\n",
        );
        write_fixture(
            &home.join(".zshrc"),
            "export OPENAI_API_KEY=\"sk-proj-THIS_IS_A_FAKE_KEY_FOR_TESTING_ONLY_1234567890abcdefghijklmnopqrstuvwxyz\"\n",
        );
        write_fixture(&xdg_data.join("atuin/key"), "atuin-secret\n");
        write_fixture(
            &xdg_config.join("atuin/config.toml"),
            "session_path = \"~/atuin-session\"\n",
        );
        write_fixture(&home.join("atuin-session"), "atuin-session-secret\n");
        write_fixture(
            &home.join(".config/letsencrypt/live/example/privkey.pem"),
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n",
        );
        write_fixture(
            &akamai_edgerc,
            "[default]\nhost = akamai.example\nclient_token = tok\nclient_secret = sec\naccess_token = acc\n",
        );
        write_fixture(
            &xdg_config.join("algolia/config.toml"),
            "[default]\napplication_id = \"app\"\napi_key = \"algolia\"\n",
        );
        write_fixture(
            &home.join(".aliyun/config.json"),
            r#"{"profiles":[{"access_key_secret":"aliyun-secret"}]}"#,
        );
        write_fixture(
            &argocd_config.join("config"),
            "users:\n- auth-token: argocd\n",
        );
        write_fixture(&checkmarx_config, "cx_apikey: ast-secret\n");
        write_fixture(
            &bitwarden_appdata.join("data.json"),
            r#"{"accessToken":"bw"}"#,
        );
        write_fixture(&home.join(".bridgecrew/credentials"), "bridgecrew-token\n");
        write_fixture(&home.join(".circleci/cli.yml"), "token: circleci-token\n");
        write_fixture(&civo_config, r#"{"apikey":"civo-token"}"#);
        write_fixture(
            &composer_home.join("auth.json"),
            r#"{"github-oauth":{"github.com":"composer-token"}}"#,
        );
        write_fixture(
            &dcos_dir.join("clusters/prod/dcos.toml"),
            "dcos_acs_token = \"dcos-token\"\n",
        );
        write_fixture(
            &doctl_config,
            "context: default\naccess-token: doctl-token\n",
        );
        write_fixture(
            &docker_config.join("config.json"),
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}},"credsStore":"osxkeychain","credHelpers":{"ghcr.io":"desktop"}}"#,
        );
        write_fixture(
            &home.join(".docker/machine/machines/default/id_rsa"),
            "-----BEGIN RSA PRIVATE KEY-----\nkey\n-----END RSA PRIVATE KEY-----\n",
        );
        write_fixture(
            &home.join(".fastlane/spaceship/default/cookie"),
            "---\n- !ruby/object:HTTP::Cookie\n  name: myacinfo\n  value: secret\n",
        );
        write_fixture(
            &xdg_config.join("fastly/config.toml"),
            "token = \"fastly\"\n",
        );
        write_fixture(
            &home.join(".cloudflared/cert.pem"),
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n",
        );
        write_fixture(
            &xdg_config.join("cloudflared/credentials.json"),
            r#"{"TunnelSecret":"cloudflared-secret"}"#,
        );
        write_fixture(
            &xdg_config.join(".wrangler/config/default.toml"),
            "oauth_token = \"wrangler-oauth\"\nrefresh_token = \"wrangler-refresh\"\n",
        );
        write_fixture(&home.join(".fly/config.yml"), "access_token: FlyV1 token\n");
        write_fixture(
            &gh_config.join("hosts.yml"),
            "github.com:\n  oauth_token: ghp_secret\n",
        );
        write_fixture(
            &glab_config.join("config.yml"),
            "hosts:\n  gitlab.com:\n    token: glpat\n",
        );
        write_fixture(&xdg_config.join("gotify/cli.json"), r#"{"token":"gotify"}"#);
        write_fixture(
            &xdg_config.join("graphite/auth"),
            r#"{"authToken":"graphite"}"#,
        );
        write_fixture(&hcloud_config, "token = \"hcloud\"\n");
        write_fixture(&home.join(".cache/huggingface/token"), "hf_secret\n");
        write_fixture(
            &kubeconfig,
            "users:\n- name: prod\n  user:\n    token: kube-token\n",
        );
        write_fixture(
            &home.join("Library/Preferences/netlify/config.json"),
            r#"{"users":{"u":{"auth":{"token":"netlify"}}}}"#,
        );
        write_fixture(
            &xdg_config.join("NuGet/NuGet.Config"),
            r#"<configuration><apikeys><add key="feed" value="nuget-secret" /></apikeys></configuration>"#,
        );
        write_fixture(
            &home.join(".nuget/NuGet/NuGet.Config"),
            r#"<configuration></configuration>"#,
        );
        write_fixture(
            &xdg_config.join("openvpn/prod.auth"),
            "openvpn-user\nopenvpn-password\n",
        );
        write_fixture(
            &xdg_config.join("openvpn/prod.ovpn"),
            "auth-user-pass prod.auth\n<tls-crypt>\nline1\nline2\n</tls-crypt>\n",
        );
        write_fixture(&npmrc, "_authToken=npm-token\n");
        write_fixture(
            &xdg_config.join("containers/auth.json"),
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}}}"#,
        );
        write_fixture(
            &registry_auth,
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}}}"#,
        );
        write_fixture(
            &pulumi_credentials,
            r#"{"accessTokens":{"https://api.pulumi.com":"pulumi-token"}}"#,
        );
        write_fixture(
            &rclone_config,
            "[remote]\ntoken = {\"access_token\":\"rclone\"}\n",
        );
        write_fixture(&home.join(".sentryclirc"), "[auth]\ntoken=sentry-token\n");
        write_fixture(&home.join(".shodan/api_key"), "shodan-key\n");
        write_fixture(
            &xdg_config.join("configstore/snyk.json"),
            r#"{"api":"snyk-token"}"#,
        );
        write_fixture(
            &supabase_home.join("access-token"),
            format!("sbp_{}\n", "a".repeat(40)),
        );
        write_fixture(
            &home.join(".terraform.d/credentials.tfrc.json"),
            r#"{"credentials":{"app.terraform.io":{"token":"tf-token"}}}"#,
        );
        write_fixture(
            &xdg_config.join("todoist/config.json"),
            r#"{"token":"todoist"}"#,
        );
        write_fixture(
            &home.join(".travis/config.yml"),
            "access_token: travis-token\n",
        );
        write_fixture(&home.join(".pypirc"), "[pypi]\npassword = twine-token\n");
        write_fixture(
            &vagrant_home.join("data/vagrant_login_token"),
            "vagrant-token\n",
        );
        write_fixture(
            &xdg_data.join("com.vercel.cli/auth.json"),
            r#"{"token":"vercel-token","refreshToken":"vercel-refresh"}"#,
        );
        write_fixture(&home.join(".vault-token"), "hvs.secret\n");
        write_fixture(&home.join(".vt.toml"), "apikey=\"vt-key\"\n");
        write_fixture(&home.join(".vultr-cli.yaml"), "api-key: vultr-key\n");
        write_fixture(
            &home.join(".wakatime.cfg"),
            "[settings]\napi_key = wakatime\n",
        );
        write_fixture(&home.join(".wskprops"), "AUTH=fake-uuid:fake-secret\n");
        write_fixture(
            &talosconfig,
            "contexts:\n  prod:\n    endpoints: []\n    ca: talos-ca\n",
        );

        let netrc_contents = "\
machine buf.build login alice password buf-token
machine api.heroku.com login user password heroku-token
machine example.com login user password netrc-token
";
        write_fixture(&home.join(".netrc"), netrc_contents);
        write_fixture(&netrc, netrc_contents);

        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
            ("XDG_CACHE_HOME", xdg_cache.to_str().unwrap()),
            ("XDG_DATA_HOME", xdg_data.to_str().unwrap()),
            ("XDG_STATE_HOME", xdg_state.to_str().unwrap()),
            ("XDG_RUNTIME_DIR", xdg_runtime.to_str().unwrap()),
            ("AKAMAI_EDGERC", akamai_edgerc.to_str().unwrap()),
            ("ARGOCD_CONFIG_DIR", argocd_config.to_str().unwrap()),
            (
                "AWS_SHARED_CREDENTIALS_FILE",
                aws_credentials.to_str().unwrap(),
            ),
            (
                "BITWARDENCLI_APPDATA_DIR",
                bitwarden_appdata.to_str().unwrap(),
            ),
            ("CARGO_HOME", cargo_home.to_str().unwrap()),
            ("CAROOT", caroot.to_str().unwrap()),
            ("CIVO_CONFIG", civo_config.to_str().unwrap()),
            ("COMPOSER_HOME", composer_home.to_str().unwrap()),
            ("CX_CONFIG_FILE_PATH", checkmarx_config.to_str().unwrap()),
            ("DCOS_DIR", dcos_dir.to_str().unwrap()),
            ("DIGITALOCEAN_CONFIG", doctl_config.to_str().unwrap()),
            ("DOCKER_CONFIG", docker_config.to_str().unwrap()),
            ("GH_CONFIG_DIR", gh_config.to_str().unwrap()),
            ("GLAB_CONFIG_DIR", glab_config.to_str().unwrap()),
            ("HCLOUD_CONFIG", hcloud_config.to_str().unwrap()),
            ("HELM_CONFIG_HOME", helm_config_home.to_str().unwrap()),
            (
                "HELM_REPOSITORY_CONFIG",
                helm_repository_config.to_str().unwrap(),
            ),
            ("KUBECONFIG", kubeconfig.to_str().unwrap()),
            ("MCP_REMOTE_CONFIG_DIR", missing.to_str().unwrap()),
            ("NETRC", netrc.to_str().unwrap()),
            ("NPM_CONFIG_USERCONFIG", npmrc.to_str().unwrap()),
            ("OCI_CLI_CONFIG_FILE", oci_config.to_str().unwrap()),
            (
                "PULUMI_CREDENTIALS_PATH",
                pulumi_credentials_dir.to_str().unwrap(),
            ),
            ("PULUMI_HOME", pulumi_home.to_str().unwrap()),
            ("RCLONE_CONFIG", rclone_config.to_str().unwrap()),
            ("REGISTRY_AUTH_FILE", registry_auth.to_str().unwrap()),
            ("SUPABASE_HOME", supabase_home.to_str().unwrap()),
            ("TALOSCONFIG", talosconfig.to_str().unwrap()),
            ("TALOS_HOME", talos_home.to_str().unwrap()),
            ("UV_CREDENTIALS_DIR", uv_credentials_dir.to_str().unwrap()),
            ("VAGRANT_HOME", vagrant_home.to_str().unwrap()),
        ]);

        let mut triggered = Vec::new();
        for integration in isotope_integrations::INTEGRATIONS {
            let Some(detect_reasons) = integration.detect_reasons else {
                continue;
            };
            let reasons = detect_reasons()
                .unwrap_or_else(|err| panic!("{} detect reasons failed: {err}", integration.name));
            if !reasons.is_empty() {
                triggered.push(integration.name);
            }
        }

        let expected: &[&str] = if using_radioisotope_fixture_integrations() {
            &["gh", "huggingface-cli", "node@24", "terraform"]
        } else {
            &[
                "acli",
                "akamai",
                "algolia",
                "argocd",
                "atuin",
                "bash",
                "bitwarden-cli",
                "certbot",
                "cloudflare-wrangler",
                "cloudflared",
                "docker",
                "docker-machine",
                "fastlane",
                "gh",
                "kubernetes-cli",
                "openvpn",
                "supabase",
                "terraform",
                "vercel-cli",
                "zsh",
            ]
        };

        for expected in expected {
            assert!(
                triggered.contains(expected),
                "expected {expected} to report seeded secrets, got {triggered:?}"
            );
        }
        if !using_radioisotope_fixture_integrations() {
            assert!(
                triggered.len() >= 30,
                "expected broad generated detector coverage, got {triggered:?}"
            );
        }
    }

    #[test]
    fn generated_isotope_migrations_scrub_seeded_secret_files() {
        let _lock = test_env_lock().lock().unwrap();
        if using_radioisotope_fixture_integrations() {
            return;
        }
        if isotope_integrations::INTEGRATIONS
            .iter()
            .all(|integration| integration.migrate.is_none())
        {
            return;
        }

        fn write_fixture(path: &Path, contents: impl AsRef<[u8]>) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let xdg_config = temp.path().join("xdg-config");
        let xdg_cache = temp.path().join("xdg-cache");
        let xdg_state = temp.path().join("xdg-state");
        let xdg_runtime = temp.path().join("xdg-runtime");
        let missing = temp.path().join("missing");
        let akamai_edgerc = temp.path().join("akamai.edgerc");
        let argocd_config = temp.path().join("argocd");
        let bitwarden_appdata = temp.path().join("bitwarden");
        let civo_config = temp.path().join("civo.json");
        let composer_home = temp.path().join("composer");
        let checkmarx_config = temp.path().join("checkmarx.yaml");
        let dcos_dir = temp.path().join("dcos");
        let doctl_config = temp.path().join("doctl.yaml");
        let glab_config = temp.path().join("glab");
        let hcloud_config = temp.path().join("hcloud.toml");
        let kubeconfig = temp.path().join("kubeconfig");
        let netrc = temp.path().join("netrc");
        let npmrc = temp.path().join("npmrc");
        let pulumi_credentials_dir = temp.path().join("pulumi-credentials");
        let pulumi_credentials = pulumi_credentials_dir.join("credentials.json");
        let rclone_config = temp.path().join("rclone.conf");
        let registry_auth = temp.path().join("containers-auth.json");
        let talosconfig = temp.path().join("talosconfig");
        let uv_credentials_dir = temp.path().join("uv");
        let vagrant_home = temp.path().join("vagrant");

        write_fixture(
            &home.join(".config/acli/jira_config.yaml"),
            "token: atlassian\n",
        );
        write_fixture(
            &akamai_edgerc,
            "[default]\nhost = akamai.example\nclient_token = tok\nclient_secret = sec\naccess_token = acc\n",
        );
        write_fixture(
            &xdg_config.join("algolia/config.toml"),
            "[default]\napplication_id = \"app\"\napi_key = \"algolia\"\n",
        );
        write_fixture(
            &home.join(".aliyun/config.json"),
            r#"{"profiles":[{"access_key_secret":"aliyun-secret"}]}"#,
        );
        write_fixture(
            &argocd_config.join("config"),
            "users:\n- auth-token: argocd\n",
        );
        write_fixture(
            &home.join(".aws/credentials"),
            "[default]\naws_access_key_id = AKIAEXAMPLE\naws_secret_access_key = aws-secret\n",
        );
        write_fixture(&checkmarx_config, "cx_apikey: ast-secret\n");
        write_fixture(
            &bitwarden_appdata.join("data.json"),
            r#"{"accessToken":"bw"}"#,
        );
        write_fixture(
            &home.join(".bridgecrew/credentials"),
            "access_key::secret_key\n",
        );
        write_fixture(
            &home.join(".circleci/cli.yml"),
            "host: https://circleci.com\ntoken: circleci-token\n",
        );
        write_fixture(&civo_config, r#"{"apikey":"civo-token"}"#);
        write_fixture(
            &composer_home.join("auth.json"),
            r#"{"github-oauth":{"github.com":"composer-token"}}"#,
        );
        write_fixture(
            &dcos_dir.join("clusters/prod/dcos.toml"),
            "dcos_acs_token = \"dcos-token\"\n",
        );
        write_fixture(
            &doctl_config,
            "context: default\naccess-token: doctl-token\n",
        );
        write_fixture(
            &home.join(".config/configstore/firebase-tools.json"),
            r#"{"tokens":{"refresh_token":"firebase-refresh","access_token":"firebase-access"}}"#,
        );
        write_fixture(
            &xdg_config.join("fastly/config.toml"),
            "token = \"fastly\"\n",
        );
        write_fixture(&home.join(".fly/config.yml"), "access_token: FlyV1 token\n");
        write_fixture(
            &xdg_config.join("gallery-dl/config.json"),
            r#"{"extractor":{"example":{"api-key":"gallery-secret"}}}"#,
        );
        write_fixture(
            &home.join(".config/gptcommit/config.toml"),
            "[openai]\napi_key = \"gptcommit-secret\"\n",
        );
        write_fixture(
            &glab_config.join("config.yml"),
            "hosts:\n  gitlab.com:\n    token: glpat\n",
        );
        write_fixture(
            &xdg_config.join("grafanactl/config.yaml"),
            "contexts:\n  default:\n    grafana:\n      server: https://grafana.example.com\n      token: grafana-token\n",
        );
        write_fixture(&xdg_config.join("gotify/cli.json"), r#"{"token":"gotify"}"#);
        write_fixture(
            &xdg_config.join("graphite/auth"),
            r#"{"authToken":"graphite"}"#,
        );
        write_fixture(&hcloud_config, "token = \"hcloud\"\n");
        write_fixture(&home.join(".cache/huggingface/token"), "hf_secret\n");
        write_fixture(
            &kubeconfig,
            "users:\n- name: prod\n  user:\n    token: kube-token\n",
        );
        write_fixture(
            &home.join("Library/Preferences/netlify/config.json"),
            r#"{"users":{"u":{"auth":{"token":"netlify"}}}}"#,
        );
        write_fixture(
            &xdg_config.join("luarocks/upload_config.lua"),
            "return { key = \"luarocks-secret\", server = \"https://luarocks.org\" }\n",
        );
        write_fixture(
            &home.join(".m2/settings.xml"),
            "<settings><servers><server><password>maven-secret</password></server></servers></settings>\n",
        );
        write_fixture(
            &xdg_config.join("NuGet/NuGet.Config"),
            r#"<configuration><apikeys><add key="feed" value="nuget-secret" /></apikeys></configuration>"#,
        );
        write_fixture(
            &home.join(".nuget/NuGet/NuGet.Config"),
            r#"<configuration></configuration>"#,
        );
        write_fixture(&npmrc, "_authToken=npm-token\n");
        write_fixture(
            &home.join(".config/openstack/clouds.yaml"),
            "clouds:\n  dev:\n    auth:\n      password: openstack-password\n",
        );
        write_fixture(
            &xdg_config.join("openhue/config.yaml"),
            "Bridge: 192.0.2.10\nKey: openhue-secret\n",
        );
        write_fixture(
            &home.join(".runpod/config.toml"),
            "apiKey = \"runpod-secret\"\napiUrl = \"https://api.runpod.io/graphql\"\n",
        );
        write_fixture(
            &home.join(".cargo/credentials.toml"),
            "[registry]\ntoken = \"cargo-secret\"\n",
        );
        write_fixture(
            &home.join(".s3cfg"),
            "access_key = AKIAEXAMPLE\nsecret_key = s3-secret\naccess_token = s3-session\n",
        );
        write_fixture(
            &home.join(".sbt/.credentials"),
            "realm=Repo\nhost=repo.example.com\nuser=me\npassword=sbt-secret\n",
        );
        write_fixture(
            &home.join(".snowflake/config.toml"),
            "[connections.default]\npassword = \"snowflake-secret\"\n",
        );
        write_fixture(
            &pulumi_credentials,
            r#"{"accessTokens":{"https://api.pulumi.com":"pulumi-token"}}"#,
        );
        write_fixture(
            &rclone_config,
            "[remote]\ntoken = {\"access_token\":\"rclone\"}\n",
        );
        write_fixture(
            &registry_auth,
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}}}"#,
        );
        write_fixture(&home.join(".sentryclirc"), "[auth]\ntoken=sentry-token\n");
        write_fixture(&home.join(".shodan/api_key"), "shodan-key\n");
        write_fixture(
            &xdg_config.join("configstore/snyk.json"),
            r#"{"api":"snyk-token"}"#,
        );
        write_fixture(
            &talosconfig,
            "contexts:\n  prod:\n    endpoints: []\n    ca: talos-ca\n",
        );
        write_fixture(
            &home.join(".terraform.d/credentials.tfrc.json"),
            r#"{"credentials":{"app.terraform.io":{"token":"tf-token"}}}"#,
        );
        write_fixture(
            &xdg_config.join("todoist/config.json"),
            r#"{"token":"todoist"}"#,
        );
        write_fixture(
            &home.join(".travis/config.yml"),
            "access_token: travis-token\n",
        );
        write_fixture(&home.join(".pypirc"), "[pypi]\npassword = twine-token\n");
        write_fixture(
            &home.join(".uaa/config.json"),
            r#"{"Token":{"access_token":"uaa-access","refresh_token":"uaa-refresh"}}"#,
        );
        write_fixture(
            &uv_credentials_dir.join("credentials.toml"),
            "[[credentials]]\npassword = \"uv-secret\"\n",
        );
        write_fixture(
            &vagrant_home.join("data/vagrant_login_token"),
            "vagrant-token\n",
        );
        write_fixture(&home.join(".vault-token"), "hvs.secret\n");
        write_fixture(&home.join(".vt.toml"), "apikey=\"vt-key\"\n");
        write_fixture(&home.join(".vultr-cli.yaml"), "api-key: vultr-key\n");
        write_fixture(
            &home.join(".wakatime.cfg"),
            "[settings]\napi_key = wakatime\n",
        );
        write_fixture(&home.join(".wskprops"), "AUTH=fake-uuid:fake-secret\n");
        let netrc_contents = "\
machine buf.build login alice password buf-token
machine api.heroku.com login user password heroku-token
machine example.com login user password netrc-token
";
        write_fixture(&home.join(".netrc"), netrc_contents);
        write_fixture(&netrc, netrc_contents);

        let _env = TestEnvGuard::set(&[
            ("HOME", home.to_str().unwrap()),
            ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
            ("XDG_CACHE_HOME", xdg_cache.to_str().unwrap()),
            ("XDG_STATE_HOME", xdg_state.to_str().unwrap()),
            ("XDG_RUNTIME_DIR", xdg_runtime.to_str().unwrap()),
            ("AKAMAI_EDGERC", akamai_edgerc.to_str().unwrap()),
            ("ARGOCD_CONFIG_DIR", argocd_config.to_str().unwrap()),
            ("AWS_SHARED_CREDENTIALS_FILE", ""),
            (
                "BITWARDENCLI_APPDATA_DIR",
                bitwarden_appdata.to_str().unwrap(),
            ),
            ("CARGO_HOME", ""),
            ("CAROOT", missing.to_str().unwrap()),
            ("CIVO_CONFIG", civo_config.to_str().unwrap()),
            ("COMPOSER_HOME", composer_home.to_str().unwrap()),
            ("CX_CONFIG_FILE_PATH", checkmarx_config.to_str().unwrap()),
            ("DCOS_DIR", dcos_dir.to_str().unwrap()),
            ("DIGITALOCEAN_CONFIG", doctl_config.to_str().unwrap()),
            ("DOCKER_CONFIG", missing.to_str().unwrap()),
            ("GH_CONFIG_DIR", missing.to_str().unwrap()),
            ("GLAB_CONFIG_DIR", glab_config.to_str().unwrap()),
            ("HCLOUD_CONFIG", hcloud_config.to_str().unwrap()),
            ("HELM_CONFIG_HOME", missing.to_str().unwrap()),
            ("HELM_REPOSITORY_CONFIG", missing.to_str().unwrap()),
            ("KUBECONFIG", kubeconfig.to_str().unwrap()),
            ("MCP_REMOTE_CONFIG_DIR", missing.to_str().unwrap()),
            ("NETRC", netrc.to_str().unwrap()),
            ("NPM_CONFIG_USERCONFIG", npmrc.to_str().unwrap()),
            ("OCI_CLI_CONFIG_FILE", missing.to_str().unwrap()),
            ("PULUMI_CREDENTIALS_PATH", ""),
            ("PULUMI_HOME", pulumi_credentials_dir.to_str().unwrap()),
            ("RCLONE_CONFIG", rclone_config.to_str().unwrap()),
            ("REGISTRY_AUTH_FILE", registry_auth.to_str().unwrap()),
            ("SUPABASE_HOME", missing.to_str().unwrap()),
            ("TALOSCONFIG", talosconfig.to_str().unwrap()),
            ("TALOS_HOME", missing.to_str().unwrap()),
            ("UV_CREDENTIALS_DIR", uv_credentials_dir.to_str().unwrap()),
            ("VAGRANT_HOME", vagrant_home.to_str().unwrap()),
        ]);

        let migration_targets = [
            "acli",
            "akamai",
            "algolia",
            "aliyun-cli",
            "argocd",
            "ast-cli",
            "aws-cli",
            "bitwarden-cli",
            "buf",
            "checkov",
            "circleci",
            "civo",
            "composer",
            "dcos-cli",
            "doctl",
            "firebase-cli",
            "fastly",
            "flyctl",
            "gallery-dl",
            "gptcommit",
            "glab",
            "grafanactl",
            "gotify",
            "graphite",
            "hcloud",
            "heroku",
            "huggingface-cli",
            "kubernetes-cli",
            "luarocks",
            "maven",
            "netlify-cli",
            "nuget",
            "openhue-cli",
            "openstackclient",
            "pulumi",
            "rclone",
            "runpodctl",
            "rust",
            "s3cmd",
            "sbt",
            "sentry-cli",
            "shodan",
            "snowflake-cli",
            "snyk",
            "talosctl",
            "terraform",
            "todoist-cli",
            "travis",
            "twine",
            "uaa-cli",
            "uv",
            "vagrant",
            "vault",
            "virustotal-cli",
            "vultr",
            "wakatime-cli",
            "wsk",
        ];

        for name in migration_targets {
            let integration = isotope_integrations::INTEGRATIONS
                .iter()
                .find(|integration| integration.name == name)
                .unwrap_or_else(|| panic!("missing generated integration {name}"));
            let migrate = integration
                .migrate
                .unwrap_or_else(|| panic!("missing generated migration {name}"));
            let detects_seeded_secret = || -> Result<bool, String> {
                if let Some(detect_reasons) = integration.detect_reasons {
                    return detect_reasons().map(|reasons| !reasons.is_empty());
                }
                if let Some(detect) = integration.detect {
                    return detect();
                }
                Ok(false)
            };
            assert!(
                detects_seeded_secret()
                    .unwrap_or_else(|err| panic!("{name} detect failed before migration: {err}")),
                "{name} should report its seeded secret before migration"
            );
            match migrate() {
                Ok(()) => assert!(
                    !detects_seeded_secret().unwrap_or_else(|err| panic!(
                        "{name} detect failed after migration: {err}"
                    )),
                    "{name} migration left its seeded secret detectable"
                ),
                Err(err) if err.contains("isotope keychain integration is only available") => {}
                Err(err) => panic!("{name} migration failed: {err}"),
            }
        }
    }

    #[test]
    fn generated_radioisotope_migrations_cover_additional_default_paths() {
        let _lock = test_env_lock().lock().unwrap();
        if using_radioisotope_fixture_integrations() {
            return;
        }
        if isotope_integrations::INTEGRATIONS
            .iter()
            .all(|integration| integration.migrate.is_none())
        {
            return;
        }

        fn write_fixture(path: &Path, contents: impl AsRef<[u8]>) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn detects_seeded_secret(
            integration: &isotope_integrations::IsotopeIntegration,
        ) -> Result<bool, String> {
            if let Some(detect_reasons) = integration.detect_reasons {
                return detect_reasons().map(|reasons| !reasons.is_empty());
            }
            if let Some(detect) = integration.detect {
                return detect();
            }
            Ok(false)
        }

        fn run_case(name: &str, seed: fn(&Path, &Path, &Path, &Path, &Path, &Path)) {
            let temp = TempDir::new().unwrap();
            let home = temp.path().join("home");
            let xdg_config = temp.path().join("xdg-config");
            let xdg_cache = temp.path().join("xdg-cache");
            let xdg_state = temp.path().join("xdg-state");
            let xdg_runtime = temp.path().join("xdg-runtime");
            let npmrc = temp.path().join("npmrc");
            let oci_config = home.join(".oci/config");
            let mcp_remote_config = home.join(".mcp-auth");

            seed(
                &home,
                &xdg_config,
                &xdg_cache,
                &xdg_state,
                &xdg_runtime,
                &npmrc,
            );

            let _env = TestEnvGuard::set(&[
                ("HOME", home.to_str().unwrap()),
                ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
                ("XDG_CACHE_HOME", xdg_cache.to_str().unwrap()),
                ("XDG_STATE_HOME", xdg_state.to_str().unwrap()),
                ("XDG_RUNTIME_DIR", xdg_runtime.to_str().unwrap()),
                ("NPM_CONFIG_USERCONFIG", npmrc.to_str().unwrap()),
                ("OCI_CLI_CONFIG_FILE", oci_config.to_str().unwrap()),
                ("MCP_REMOTE_CONFIG_DIR", mcp_remote_config.to_str().unwrap()),
            ]);

            let integration = isotope_integrations::INTEGRATIONS
                .iter()
                .find(|integration| integration.name == name)
                .unwrap_or_else(|| panic!("missing generated integration {name}"));
            let migrate = integration
                .migrate
                .unwrap_or_else(|| panic!("missing generated migration {name}"));

            assert!(
                detects_seeded_secret(integration)
                    .unwrap_or_else(|err| panic!("{name} detect failed before migration: {err}")),
                "{name} should report its seeded secret before migration"
            );
            match migrate() {
                Ok(()) => assert!(
                    !detects_seeded_secret(integration).unwrap_or_else(|err| panic!(
                        "{name} detect failed after migration: {err}"
                    )),
                    "{name} migration left its seeded secret detectable"
                ),
                Err(err) if err.contains("isotope keychain integration is only available") => {}
                Err(err) => panic!("{name} migration failed: {err}"),
            }
        }

        type MigrationFixtureWriter = fn(&Path, &Path, &Path, &Path, &Path, &Path);

        let cases: &[(&str, MigrationFixtureWriter)] = &[
            ("astra", |_, xdg_config, _, _, _, _| {
                write_fixture(
                    &xdg_config.join("astra/.astrarc"),
                    "default=prod\ntoken=AstraCS:astra-secret\n",
                );
            }),
            ("censys", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".config/censys/censys.cfg"),
                    "[DEFAULT]\napi_id = fake-censys-id\napi_secret = fake-censys-secret\n",
                );
            }),
            ("cloudsmith-cli", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".cloudsmith/credentials.ini"),
                    "[default]\napi_key=fake-cloudsmith-key\n",
                );
            }),
            ("dropbox-uploader", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".dropbox_uploader"),
                    "APPKEY=fake-app\nOAUTH_ACCESS_TOKEN=fake-token\n",
                );
            }),
            ("gcli", |_, xdg_config, _, _, _, _| {
                write_fixture(
                    &xdg_config.join("gcli/config"),
                    "[github]\ntoken = fake-gcli-token\n",
                );
            }),
            ("goat", |_, _, _, xdg_state, _, _| {
                write_fixture(
                    &xdg_state.join("goat/auth-session.json"),
                    r#"{"password":"fake-app-password","access_token":"fake-access"}"#,
                );
            }),
            ("imap-backup", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".imap-backup/config.json"),
                    r#"{"accounts":[{"username":"a@example.com","password":"fake-password"}]}"#,
                );
            }),
            ("jfrog-cli", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".jfrog/jfrog-cli.conf.v6"),
                    r#"[{"serverId":"prod","url":"https://example.test","accessToken":"secret"}]"#,
                );
            }),
            ("mcp-remote", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".mcp-auth/server_tokens.json"),
                    r#"{"access_token":"mcp-access","refresh_token":"mcp-refresh"}"#,
                );
            }),
            ("minio-mc", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".mc/config.json"),
                    r#"{"aliases":{"minio":{"url":"https://minio.example.test","accessKey":"access","secretKey":"secret","sessionToken":"session"}}}"#,
                );
            }),
            ("mysql", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".my.cnf"),
                    "[client]\nuser = deploy\npassword = secret\n",
                );
            }),
            ("mysql-client", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".my.cnf"),
                    "[client]\nuser = deploy\npassword = secret\n",
                );
            }),
            ("mysql@8.0", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".my.cnf"),
                    "[client]\nuser = deploy\npassword = secret\n",
                );
            }),
            ("mysql@8.4", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".my.cnf"),
                    "[client]\nuser = deploy\npassword = secret\n",
                );
            }),
            ("node@18", |_, _, _, _, _, npmrc| {
                write_fixture(npmrc, "//registry.npmjs.org/:_authToken=npm_secret\n");
            }),
            ("oci-cli", |home, _, _, _, _, _| {
                write_fixture(&home.join(".oci/key.pem"), "private-key\n");
                write_fixture(
                    &home.join(".oci/config"),
                    "[DEFAULT]\nuser=ocid1.user\nkey_file=~/.oci/key.pem\n",
                );
            }),
            ("ossutil", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".ossutilconfig"),
                    "[Credentials]\naccessKeyID = LTAIEXAMPLE\naccessKeySecret = very-secret\n",
                );
            }),
            ("oxide-cli", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".config/oxide/credentials.toml"),
                    "[profile.prod]\nhost = \"https://oxide.example\"\ntoken = \"fake-oxide-token\"\n",
                );
            }),
            ("phylum-cli", |_, xdg_config, _, _, _, _| {
                write_fixture(
                    &xdg_config.join("phylum/settings.yaml"),
                    "offline_access: ph0_fake-token\n",
                );
            }),
            ("plumber", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".batchsh/plumber.json"),
                    r#"{"token":"plumber-token"}"#,
                );
            }),
            ("pnpm", |_, _, _, _, _, npmrc| {
                write_fixture(npmrc, "//registry.npmjs.org/:_authToken=pnpm_secret\n");
            }),
            ("qwen-code", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".qwen/settings.json"),
                    r#"{"env":{"DASHSCOPE_API_KEY":"sk-test"}}"#,
                );
            }),
            ("railway", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".railway/config.json"),
                    r#"{"user":{"token":"rw_legacy"}}"#,
                );
            }),
            ("soracom-cli", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".soracom/default.json"),
                    r#"{"authKeyId":"keyId-example","authKey":"secret-example"}"#,
                );
            }),
            ("sqlcmd", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".sqlcmd/sqlconfig"),
                    "users:\n- user:\n    username: sa\n    password: c2VjcmV0\n",
                );
            }),
            ("terraform-core", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".terraform.d/credentials.tfrc.json"),
                    r#"{"credentials":{"app.terraform.io":{"token":"secret"}}}"#,
                );
            }),
            ("transifex-cli", |home, _, _, _, _, _| {
                write_fixture(
                    &home.join(".transifexrc"),
                    "[https://app.transifex.com]\nrest_hostname = https://rest.api.transifex.com\ntoken = fake-token\n",
                );
            }),
        ];

        for (name, seed) in cases {
            run_case(name, *seed);
        }
    }

    #[test]
    fn generated_credential_helpers_cover_help_and_reject_bad_tokens() {
        struct MissingCredentialStore;

        impl isotope::CredentialHelperSecretStore for MissingCredentialStore {
            fn load_secret(&self, key: &str) -> Result<String, String> {
                Err(format!("missing stub credential {key}"))
            }

            fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
                Ok(())
            }
        }

        fn helper_args(name: &str) -> Vec<std::ffi::OsString> {
            match name {
                "aws" => Vec::new(),
                "cargo" => vec![std::ffi::OsString::from("--cargo-plugin")],
                "kubernetes" => vec![std::ffi::OsString::from("prod")],
                "nuget" => vec![
                    std::ffi::OsString::from("-Uri"),
                    std::ffi::OsString::from("https://api.nuget.org/v3/index.json"),
                ],
                "opentofu" | "terraform" => vec![
                    std::ffi::OsString::from("get"),
                    std::ffi::OsString::from("app.terraform.io"),
                ],
                "podman" | "skopeo" => vec![std::ffi::OsString::from("list")],
                "wakatime" => Vec::new(),
                other => panic!("unexpected credential helper {other}"),
            }
        }

        fn invocation<'a>(
            args: Vec<std::ffi::OsString>,
            token: Option<&str>,
            parent_executable_path: Option<&str>,
            store: &'a MissingCredentialStore,
        ) -> isotope::CredentialHelperInvocation<'a> {
            isotope::CredentialHelperInvocation {
                args,
                caller: isotope::CredentialHelperCallerContext {
                    token: token.map(str::to_string),
                    parent_executable_path: parent_executable_path.map(str::to_string),
                    parent_command: None,
                },
                store,
            }
        }

        let store = MissingCredentialStore;
        let helpers = isotope_integrations::INTEGRATIONS
            .iter()
            .filter_map(|integration| {
                Some((
                    integration.credential_helper_name?,
                    integration.credential_helper?,
                ))
            })
            .collect::<Vec<_>>();
        if helpers.is_empty() {
            return;
        }

        for (name, helper) in &helpers {
            helper(invocation(
                vec![std::ffi::OsString::from("--help")],
                None,
                None,
                &store,
            ))
            .unwrap();
            helper(invocation(
                vec![std::ffi::OsString::from("--version")],
                None,
                None,
                &store,
            ))
            .unwrap();

            let missing = helper(invocation(helper_args(name), None, None, &store)).unwrap_err();
            assert!(
                missing.to_ascii_lowercase().contains("token"),
                "expected missing token error for {name}, got {missing}"
            );
            let invalid =
                helper(invocation(helper_args(name), Some("short"), None, &store)).unwrap_err();
            assert!(
                invalid.to_ascii_lowercase().contains("token"),
                "expected invalid token error for {name}, got {invalid}"
            );
            let valid_token = "x".repeat(32);
            let missing_parent = helper(invocation(
                helper_args(name),
                Some(&valid_token),
                None,
                &store,
            ))
            .unwrap_err();
            assert!(
                missing_parent.to_ascii_lowercase().contains("parent"),
                "expected missing parent error for {name}, got {missing_parent}"
            );
            let wrong_parent = helper(invocation(
                helper_args(name),
                Some(&valid_token),
                Some("/tmp/not-the-approved-launcher"),
                &store,
            ))
            .unwrap_err();
            let wrong_parent = wrong_parent.to_ascii_lowercase();
            assert!(
                wrong_parent.contains("invoked")
                    || wrong_parent.contains("launcher")
                    || wrong_parent.contains("kubectl"),
                "expected wrong parent error for {name}, got {wrong_parent}"
            );
        }
        assert_eq!(
            helpers.len(),
            if using_radioisotope_fixture_integrations() {
                1
            } else {
                9
            }
        );
    }

    #[test]
    fn generated_isotope_helpers_return_none_without_compiled_integrations() {
        for name in ["gh", "aws-cli"] {
            let integration = isotope_integration(name);
            assert_eq!(
                integration.is_some(),
                isotope_integration(&format!("isotope:{name}")).is_some()
            );
            assert_eq!(
                isotope_has_migration(name),
                integration.and_then(|it| it.migrate).is_some()
            );
            assert_eq!(
                isotope_has_post_install(name),
                integration.and_then(|it| it.post_install).is_some()
            );

            if integration.is_none() {
                assert_eq!(run_generated_isotope_migration(name), None);
                assert_eq!(run_generated_isotope_post_install(name), None);
                assert_eq!(detect_isotope_install_reasons(name), None);
                assert_eq!(package_security_state_for_isotope(name), None);
            } else {
                if integration.and_then(|it| it.migrate).is_none() {
                    assert_eq!(run_generated_isotope_migration(name), None);
                }
                if integration.and_then(|it| it.post_install).is_none() {
                    assert_eq!(run_generated_isotope_post_install(name), None);
                }
            }
        }

        assert_eq!(
            package_security_state_for_identifiers(vec!["unrelated".to_string()]),
            None
        );
    }

    #[test]
    fn post_install_dispatcher_covers_supported_and_default_paths() {
        assert!(post_install_hooks::supports("python@3.14"));
        assert!(post_install_hooks::supports("openssl@3"));
        assert!(post_install_hooks::supports_dependency("openssl@3"));
        assert!(!post_install_hooks::supports_dependency("python@3.14"));
        assert!(!post_install_hooks::supports("ripgrep"));

        let temp = TempDir::new().unwrap();
        let prefix = temp.path().join("opt/python@3.14");
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(prefix.parent().unwrap().join("python@3.14")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("python3.14"), b"").unwrap();
        fs::write(bin_dir.join("pip3.14"), b"").unwrap();

        let python = post_install_hooks::run("python@3.14", &prefix, &bin_dir).unwrap();
        assert_eq!(
            python.managed_stubs,
            vec![
                "pip".to_string(),
                "pip3".to_string(),
                "python".to_string(),
                "python3".to_string(),
            ]
        );

        let openssl = post_install_hooks::run("openssl@3", temp.path(), &bin_dir).unwrap();
        assert_eq!(openssl, post_install_hooks::PostInstallOutcome::default());

        let unsupported = post_install_hooks::run("ripgrep", temp.path(), &bin_dir).unwrap();
        assert_eq!(
            unsupported,
            post_install_hooks::PostInstallOutcome::default()
        );
    }

    #[test]
    fn package_security_state_uses_source_and_alias_identifiers_without_integrations() {
        let info = PackageInfo {
            package_name: "gh".to_string(),
            qualified_name: "brew:gh".to_string(),
            install_root: PathBuf::from("/opt/gh"),
            installed: false,
            source: Some(PackageReceiptSource::Formula {
                root_formula: "gh".to_string(),
            }),
            source_error: None,
            aliases: vec!["GH".to_string(), "GitHub".to_string()],
            aliases_error: None,
            installed_version: None,
            latest_version: None,
            latest_version_error: None,
            executable_paths: Vec::new(),
            executable_paths_error: None,
            popularity: None,
            last_updated_at: None,
            homebrew_info: None,
            homebrew_info_error: None,
            npm_homepage: None,
            npm_package_info_error: None,
            security_state: None,
            version_options: Vec::new(),
        };

        let expected = package_security_state_for_isotope("gh");
        assert_eq!(package_security_state(&info), expected);
        assert_eq!(
            package_security_state_for_identifiers(info.aliases.clone()),
            expected
        );
    }

    #[test]
    fn resolve_scanned_package_statuses_warns_for_other_dirs() {
        let mut warnings = Vec::new();
        let statuses = resolve_scanned_package_statuses(
            vec![
                InstalledPackageRef {
                    package_name: "deno".to_string(),
                    install_root: PathBuf::from("/opt/deno"),
                },
                InstalledPackageRef {
                    package_name: "scratch".to_string(),
                    install_root: PathBuf::from("/opt/scratch"),
                },
            ],
            |package| match package.package_name.as_str() {
                "deno" => Ok(PackageStatus {
                    package_name: "deno".to_string(),
                    source: PackageReceiptSource::Vendor {
                        vendor_name: "deno".to_string(),
                    },
                    installed_version: "2.7.7".to_string(),
                    latest_version: "2.7.8".to_string(),
                }),
                _ => Err(format!(
                    "package {} is installed but missing package metadata",
                    package.package_name
                )),
            },
            |warning| warnings.push(warning),
        )
        .unwrap();

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].package_name, "deno");
        assert_eq!(
            warnings,
            vec!["warning: skipping /opt/scratch: package scratch is installed but missing package metadata".to_string()]
        );
    }

    #[test]
    fn resolve_scanned_package_records_warns_for_other_dirs() {
        let mut warnings = Vec::new();
        let records = resolve_scanned_package_records(
            vec![
                InstalledPackageRef {
                    package_name: "deno".to_string(),
                    install_root: PathBuf::from("/opt/deno"),
                },
                InstalledPackageRef {
                    package_name: "scratch".to_string(),
                    install_root: PathBuf::from("/opt/scratch"),
                },
            ],
            |package| match package.package_name.as_str() {
                "deno" => Ok(InstalledPackageRecord {
                    package_name: "deno".to_string(),
                    source: PackageReceiptSource::Vendor {
                        vendor_name: "deno".to_string(),
                    },
                    installed_version: "2.7.7".to_string(),
                }),
                _ => Err(format!(
                    "package {} is installed but missing package metadata",
                    package.package_name
                )),
            },
            |warning| warnings.push(warning),
        )
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].package_name, "deno");
        assert_eq!(
            warnings,
            vec!["warning: skipping /opt/scratch: package scratch is installed but missing package metadata".to_string()]
        );
    }

    #[test]
    fn resolve_scanned_package_records_sort_by_name_after_known_prefixes() {
        let records = resolve_scanned_package_records(
            vec![
                InstalledPackageRef {
                    package_name: "npm:zulu".to_string(),
                    install_root: PathBuf::from("/opt/npm/zulu"),
                },
                InstalledPackageRef {
                    package_name: "deno".to_string(),
                    install_root: PathBuf::from("/opt/deno"),
                },
                InstalledPackageRef {
                    package_name: "pip:bravo".to_string(),
                    install_root: PathBuf::from("/opt/pip/bravo"),
                },
                InstalledPackageRef {
                    package_name: "npm:@tobilu/qmd".to_string(),
                    install_root: PathBuf::from("/opt/npm/@tobilu/qmd"),
                },
                InstalledPackageRef {
                    package_name: "isotope:alpha".to_string(),
                    install_root: PathBuf::from("/opt/iso/alpha"),
                },
            ],
            |package| {
                Ok(InstalledPackageRecord {
                    package_name: package.package_name.clone(),
                    source: PackageReceiptSource::Vendor {
                        vendor_name: package.package_name.clone(),
                    },
                    installed_version: "1.0.0".to_string(),
                })
            },
            |_| {},
        )
        .unwrap();

        assert_eq!(
            records
                .into_iter()
                .map(|record| record.package_name)
                .collect::<Vec<_>>(),
            vec![
                "isotope:alpha".to_string(),
                "pip:bravo".to_string(),
                "deno".to_string(),
                "npm:@tobilu/qmd".to_string(),
                "npm:zulu".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_outdated_package_statuses_filters_up_to_date_entries() {
        let statuses = vec![
            PackageStatus {
                package_name: "deno".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: "deno".to_string(),
                },
                installed_version: "2.7.7".to_string(),
                latest_version: "2.7.8".to_string(),
            },
            PackageStatus {
                package_name: "gh".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: "gh".to_string(),
                },
                installed_version: "2.80.0".to_string(),
                latest_version: "2.80.0".to_string(),
            },
        ];
        let outdated = filter_outdated_package_statuses(statuses);

        assert_eq!(outdated.len(), 1);
        assert_eq!(outdated[0].package_name, "deno");
    }

    #[test]
    fn requested_package_from_status_preserves_formula_identity() {
        let formula = PackageStatus {
            package_name: "python@3.12".to_string(),
            source: PackageReceiptSource::Formula {
                root_formula: "python@3.12".to_string(),
            },
            installed_version: "3.12.10".to_string(),
            latest_version: "3.12.11".to_string(),
        };
        let alias = PackageStatus {
            package_name: "ffmpeg".to_string(),
            source: PackageReceiptSource::Formula {
                root_formula: "ffmpeg-full".to_string(),
            },
            installed_version: "8.0".to_string(),
            latest_version: "8.1".to_string(),
        };
        let vendor = PackageStatus {
            package_name: "deno".to_string(),
            source: PackageReceiptSource::Vendor {
                vendor_name: "deno".to_string(),
            },
            installed_version: "2.7.7".to_string(),
            latest_version: "2.7.8".to_string(),
        };
        let npm = PackageStatus {
            package_name: "openclaw".to_string(),
            source: PackageReceiptSource::Npm {
                package_name: "openclaw".to_string(),
            },
            installed_version: "1.2.3".to_string(),
            latest_version: "1.2.4".to_string(),
        };
        let pip = PackageStatus {
            package_name: "psycopg2".to_string(),
            source: PackageReceiptSource::Pip {
                package_name: "psycopg2".to_string(),
            },
            installed_version: "2.9.9".to_string(),
            latest_version: "2.9.10".to_string(),
        };

        assert_eq!(
            requested_package_from_status(&formula),
            RequestedPackage::HomebrewFormula("python@3.12".to_string())
        );
        assert_eq!(
            requested_package_from_status(&alias),
            RequestedPackage::Auto("ffmpeg".to_string())
        );
        assert_eq!(
            requested_package_from_status(&vendor),
            RequestedPackage::VendorPackage("deno".to_string())
        );
        assert_eq!(
            requested_package_from_status(&npm),
            RequestedPackage::NpmPackage {
                package: "openclaw".to_string(),
                version: None,
            }
        );
        assert_eq!(
            requested_package_from_status(&pip),
            RequestedPackage::PipPackage("psycopg2".to_string())
        );
    }

    #[test]
    fn load_or_resolve_package_receipt_requires_root_receipt() {
        let temp = TempDir::new().unwrap();
        let receipts_dir = temp.path().join(RECEIPTS_DIR);
        fs::create_dir_all(&receipts_dir).unwrap();
        fs::write(
            receipts_dir.join("ffmpeg-full.json"),
            serde_json::to_vec_pretty(&InstallReceipt {
                formula: "ffmpeg-full".to_string(),
                version: "8.1".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_tag: "arm64_tahoe".to_string(),
                owned_paths: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap();

        let error = load_or_resolve_package_receipt("ffmpeg", temp.path()).unwrap_err();
        assert_eq!(
            error,
            "package ffmpeg is installed but missing package metadata"
        );
    }

    #[test]
    fn formula_version_string_appends_revision_suffix() {
        let info = FormulaInfo {
            versions: FormulaVersions {
                stable: "2.53.0".to_string(),
            },
            revision: 1,
            ..formula_info(false)
        };

        assert_eq!(formula_version_string(&info), "2.53.0_1");
    }

    #[test]
    fn extract_semver_from_text_handles_v_prefix() {
        assert_eq!(
            extract_semver_from_text("node v22.18.0").unwrap(),
            semver::Version::parse("22.18.0").unwrap()
        );
    }

    #[cfg(feature = "gold-release")]
    #[test]
    fn parse_self_update_version_strips_leading_v() {
        assert_eq!(
            parse_self_update_version("v0.1.0").unwrap(),
            semver::Version::parse("0.1.0").unwrap()
        );
    }

    #[cfg(feature = "gold-release")]
    #[test]
    fn self_update_asset_name_for_uses_release_naming() {
        let version = semver::Version::parse("0.1.0").unwrap();
        assert_eq!(
            self_update_asset_name_for(&version, "macos", "aarch64"),
            Some("nucleus-0.1.0-Darwin-arm64.tar.gz".to_string())
        );
        assert_eq!(
            self_update_asset_name_for(&version, "linux", "x86_64"),
            Some("nucleus-0.1.0-Linux-x86_64.tar.gz".to_string())
        );
        assert_eq!(
            self_update_asset_name_for(&version, "windows", "x86_64"),
            None
        );
    }

    #[test]
    fn rewrite_absolute_path_prefers_etc_over_keg_root() {
        let rules = vec![
            RewriteRule {
                source: "/opt/homebrew/Cellar/gum/0.17.0/etc".to_string(),
                destination: "/etc".to_string(),
            },
            RewriteRule {
                source: "/opt/homebrew/Cellar/gum/0.17.0".to_string(),
                destination: "/tmp/x/gum".to_string(),
            },
        ];
        let rewritten = rewrite_absolute_path(
            "/opt/homebrew/Cellar/gum/0.17.0/etc/bash_completion.d/gum",
            &rules,
        )
        .unwrap();
        assert_eq!(rewritten.unwrap(), "/etc/bash_completion.d/gum");
    }

    #[test]
    fn rewrite_text_rewrites_openssl_cert_pem_to_short_cert_path() {
        let plan = fixed_i_plan("curl", "curl");
        let rules = build_rewrite_rules(&plan, &[]);
        let rewritten = rewrite_text(
            "@@HOMEBREW_PREFIX@@/etc/openssl@3/cert.pem\n",
            Path::new("/tmp/curl"),
            "curl",
            &rules,
        )
        .unwrap();
        assert_eq!(rewritten, "/opt/curl/ssl/cert.pem\n");
    }

    #[test]
    fn rewrite_binary_rewrites_openssl_cert_path_to_short_cert_path() {
        let plan = fixed_i_plan("python@3.12", "python@3.12");
        let rules = build_rewrite_rules(&plan, &[]);
        let expected = binary_rewrite_destination(
            &RewriteRule {
                source: "/opt/homebrew/etc/openssl@3/cert.pem".to_string(),
                destination: "/opt/python@3.12/ssl/cert.pem".to_string(),
            },
            BinaryRewriteMode::Slash,
        );
        let mut bytes = b"prefix\0/opt/homebrew/etc/openssl@3/cert.pem\0".to_vec();
        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/libcrypto.3.dylib"),
            "openssl@3",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap();
        assert!(changed);
        assert!(find_subslice(&bytes, &expected).is_some());
        assert!(find_subslice(&bytes, b"/opt/homebrew/etc/openssl@3/cert.pem").is_none());
    }

    #[test]
    fn rewrite_binary_rewrites_paths_inside_nul_delimited_segments() {
        let rule = RewriteRule {
            source: "/opt/homebrew/Cellar/gum/0.17.0".to_string(),
            destination: "/tmp/x/gum".to_string(),
        };
        let rules = vec![rule.clone()];
        let mut bytes =
            b"prefix\0OPENSSLDIR: \"/opt/homebrew/Cellar/gum/0.17.0/bin/gum\"\0".to_vec();
        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/gum"),
            "gum",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap();
        assert!(changed);
        let mut expected = binary_rewrite_destination(&rule, BinaryRewriteMode::Slash);
        expected.extend_from_slice(b"/bin/gum");
        assert!(find_subslice(&bytes, &expected).is_some());
        assert!(find_subslice(&bytes, b"/opt/homebrew").is_none());
    }

    #[test]
    fn rewrite_binary_rewrites_paths_inside_non_utf8_segments() {
        let plan = fixed_i_plan("direnv", "direnv");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "bash".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/bash.tar.gz".to_string(),
            },
            keg_dir_name: "5.3.9".to_string(),
            archive_path: PathBuf::from("/tmp/bash.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend_from_slice(b"/opt/homebrew/opt/bash/bin/bash");
        bytes.push(0x80);
        bytes.push(0);

        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/direnv"),
            "direnv",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"/opt////////////direnv/bin/bash").is_some());
        assert!(find_subslice(&bytes, b"/opt/homebrew/opt/bash/bin/bash").is_none());
    }

    #[test]
    fn rewrite_binary_keeps_shorter_path_rewrites_nul_free() {
        let plan = fixed_i_plan("direnv", "direnv");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "bash".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/bash.tar.gz".to_string(),
            },
            keg_dir_name: "5.3.9".to_string(),
            archive_path: PathBuf::from("/tmp/bash.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);
        let mut bytes = vec![0xff];
        bytes.extend_from_slice(b"/opt/homebrew/opt/bash/bin/bash");
        bytes.push(0x80);
        bytes.push(0);

        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/direnv"),
            "direnv",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"/opt////////////direnv/bin/bash").is_some());
        assert!(find_subslice(&bytes, b"/opt/direnv/bin/bash\0").is_none());
        assert_eq!(*bytes.last().unwrap(), 0);
    }

    #[test]
    fn rewrite_binary_can_nul_pad_shorter_macho_paths() {
        let rule = RewriteRule {
            source: "/opt/homebrew/opt/node".to_string(),
            destination: "/tmp/opt/npm/flood".to_string(),
        };
        let rules = vec![rule];
        let mut bytes = b"cmd\0/opt/homebrew/opt/node/lib/libllhttp.9.3.dylib\0".to_vec();

        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/node"),
            "node",
            &rules,
            BinaryRewriteMode::Nul,
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"/tmp/opt/npm/flood/lib/libllhttp.9.3.dylib\0").is_some());
        assert!(find_subslice(&bytes, b"/tmp/opt/npm/////////////flood").is_none());
        assert!(find_subslice(&bytes, b"/opt/homebrew/opt/node").is_none());
    }

    #[test]
    fn rewrite_binary_uses_loader_path_for_macho_paths_inside_future_root() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("stage");
        let future_root = temp.path().join("opt/npm/flood");
        let path = root.join("bin/node");
        let rule = RewriteRule {
            source: "/opt/homebrew/opt/node".to_string(),
            destination: future_root.to_string_lossy().to_string(),
        };
        let rules = vec![rule];
        let mut bytes = b"cmd\0/opt/homebrew/opt/node/lib/libllhttp.9.3.dylib\0".to_vec();

        let changed = rewrite_binary(
            &mut bytes,
            &path,
            "node",
            &rules,
            BinaryRewriteMode::Macho {
                path: &path,
                root: &root,
                future_root: &future_root,
            },
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"@loader_path/../lib/libllhttp.9.3.dylib\0").is_some());
        assert!(find_subslice(&bytes, b"/tmp/opt/npm/flood/lib/libllhttp").is_none());
        assert!(find_subslice(&bytes, b"/opt/homebrew/opt/node").is_none());
    }

    #[test]
    fn rewrite_binary_prefers_loader_path_for_short_production_macho_paths() {
        let root = PathBuf::from("/opt/npm/.tmp/stage/install");
        let future_root = PathBuf::from("/opt/npm/flood");
        let path = root.join("bin/node");
        let rule = RewriteRule {
            source: "/opt/homebrew/opt/node".to_string(),
            destination: future_root.to_string_lossy().to_string(),
        };
        let rules = vec![rule];
        let mut bytes = b"cmd\0/opt/homebrew/opt/node/lib/libllhttp.9.4.dylib\0".to_vec();

        let changed = rewrite_binary(
            &mut bytes,
            &path,
            "node",
            &rules,
            BinaryRewriteMode::Macho {
                path: &path,
                root: &root,
                future_root: &future_root,
            },
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"@loader_path/../lib/libllhttp.9.4.dylib\0").is_some());
        assert!(find_subslice(&bytes, b"/opt/npm/flood/lib/libllhttp").is_none());
        assert!(find_subslice(&bytes, b"/opt/homebrew/opt/node").is_none());
    }

    #[test]
    fn rewrite_binary_uses_absolute_macho_path_when_loader_path_is_longer() {
        let root = PathBuf::from("/tmp/nucleus/.tmp08cFDL/python@3.14/3.14.4_1");
        let future_root = PathBuf::from("/tmp/opt/iso/aws-cli");
        let path = root.join(
            "Frameworks/Python.framework/Versions/3.14/lib/python3.14/lib-dynload/\
             _zstd.cpython-314-darwin.so",
        );
        let rule = RewriteRule {
            source: "@@HOMEBREW_PREFIX@@/opt/zstd".to_string(),
            destination: future_root.to_string_lossy().to_string(),
        };
        let rules = vec![rule];
        let mut bytes = b"cmd\0@@HOMEBREW_PREFIX@@/opt/zstd/lib/libzstd.1.dylib\0".to_vec();

        let changed = rewrite_binary(
            &mut bytes,
            &path,
            "python@3.14",
            &rules,
            BinaryRewriteMode::Macho {
                path: &path,
                root: &root,
                future_root: &future_root,
            },
        )
        .unwrap();

        assert!(changed);
        assert!(find_subslice(&bytes, b"/tmp/opt/iso/aws-cli/lib/libzstd.1.dylib\0").is_some());
        assert!(find_subslice(&bytes, b"@loader_path/../../../../../../../lib").is_none());
        assert!(find_subslice(&bytes, b"@@HOMEBREW_PREFIX@@/opt/zstd").is_none());
    }

    #[test]
    fn rewrite_binary_error_includes_formula_and_rewrite_details() {
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/Cellar/glow/2.1.0".to_string(),
            destination: "/opt/homebrew/Cellar/glow/2.1.0-shadow".to_string(),
        }];
        let mut bytes =
            b"prefix\0OPENSSLDIR: \"/opt/homebrew/Cellar/glow/2.1.0/share/mime/globs2\"\0".to_vec();
        let error = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/glow"),
            "glow",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap_err();
        assert!(error.contains("formula glow"));
        assert!(error.contains("binary rewrite in /tmp/glow"));
        assert!(error.contains(
            "rewrote /opt/homebrew/Cellar/glow/2.1.0/share/mime/globs2 -> /opt/homebrew/Cellar/glow/2.1.0-shadow/share/mime/globs2"
        ));
        assert!(error.contains("original segment:"));
        assert!(error.contains("rewritten segment:"));
    }

    #[test]
    fn rewrite_binary_length_error_mentions_embedded_homebrew_path() {
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/etc/openssl@3/cert.pem".to_string(),
            destination: "/opt/python@3.12/share/ca-certificates/cacert.pem".to_string(),
        }];
        let mut bytes = b"prefix\0/opt/homebrew/etc/openssl@3/cert.pem\0".to_vec();
        let error = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/libcrypto.3.dylib"),
            "openssl@3",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap_err();
        assert!(error.contains("matched embedded Homebrew path"));
        assert!(error.contains("/opt/homebrew/etc/openssl@3/cert.pem"));
        assert!(error.contains("/opt/python@3.12/share/ca-certificates/cacert.pem"));
    }

    #[test]
    fn relocatable_reference_detection_ignores_usr_local_paths() {
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/Cellar/glow/2.1.1".to_string(),
            destination: "/tmp/x/glow".to_string(),
        }];
        assert!(!contains_relocatable_homebrew_reference_text(
            "MIME database at /usr/local/share/mime/globs2",
            &rules
        ));
        assert!(!contains_relocatable_homebrew_reference_bytes(
            b"MIME database at /usr/local/share/mime/globs2",
            &rules
        ));
    }

    #[test]
    fn build_rewrite_rules_only_match_opt_homebrew_sources() {
        let plan = fixed_i_plan("glow", "glow");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "glow".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/glow.tar.gz".to_string(),
            },
            keg_dir_name: "2.1.1".to_string(),
            archive_path: PathBuf::from("/tmp/glow.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);
        assert!(rules.iter().any(|rule| {
            rule.source == "/opt/homebrew/Cellar/glow/2.1.1" && rule.destination == "/opt/glow"
        }));
        assert!(rules.iter().any(|rule| {
            rule.source == "/opt/homebrew/opt/glow" && rule.destination == "/opt/glow"
        }));
        assert!(!rules.iter().any(|rule| rule.source.contains("/usr/local")));
        assert!(
            !rules
                .iter()
                .any(|rule| rule.source.contains("/home/linuxbrew/.linuxbrew"))
        );
        assert!(rules.iter().any(|rule| {
            rule.source == HOMEBREW_PREFIX_PLACEHOLDER && rule.destination == "/opt/glow"
        }));
    }

    #[test]
    fn build_rewrite_rules_expands_perl_and_repository_placeholders() {
        let plan = fixed_i_plan("ack", "ack");
        let rules = build_rewrite_rules(&plan, &[]);
        assert!(rules.iter().any(|rule| {
            rule.source == HOMEBREW_REPOSITORY_PLACEHOLDER && rule.destination == "/opt/ack"
        }));
        assert!(rules.iter().any(|rule| {
            rule.source == HOMEBREW_LIBRARY_PLACEHOLDER && rule.destination == "/opt/ack/Library"
        }));
        assert!(rules.iter().any(|rule| {
            rule.source == HOMEBREW_PERL_PLACEHOLDER
                && rule.destination.starts_with("/usr/bin/perl")
        }));
    }

    #[test]
    fn perl_placeholder_prefers_staged_perl_dependency() {
        let plan = fixed_i_plan("ack", "ack");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "perl".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/perl.tar.gz".to_string(),
            },
            keg_dir_name: "5.40.2".to_string(),
            archive_path: PathBuf::from("/tmp/perl.tar.gz"),
        }];

        assert_eq!(
            perl_placeholder_target(&plan, &installs),
            "/opt/ack/bin/perl"
        );
    }

    #[test]
    fn java_placeholder_uses_staged_openjdk_layout() {
        let plan = fixed_i_plan("scala", "scala");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "openjdk@21".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/openjdk.tar.gz".to_string(),
            },
            keg_dir_name: "21.0.8".to_string(),
            archive_path: PathBuf::from("/tmp/openjdk.tar.gz"),
        }];

        let target = java_placeholder_target(&plan, &installs).unwrap();
        if env::consts::OS == "macos" {
            assert_eq!(
                target,
                "/opt/scala/libexec/openjdk.jdk/Contents/Home".to_string()
            );
        } else {
            assert_eq!(target, "/opt/scala/libexec".to_string());
        }
        assert_eq!(java_placeholder_target(&plan, &[]), None);
    }

    #[test]
    fn rewrite_text_rewrites_homebrew_perl_shebang() {
        let plan = fixed_i_plan("ack", "ack");
        let rules = build_rewrite_rules(&plan, &[]);
        let rewritten = rewrite_text(
            "#!@@HOMEBREW_PERL@@\n",
            Path::new("/tmp/ack"),
            "ack",
            &rules,
        )
        .unwrap();
        assert!(rewritten.starts_with("#!/usr/bin/perl"));
    }

    #[test]
    fn rewrite_text_rewrites_raw_homebrew_opt_dependency_paths() {
        let plan = fixed_i_plan("direnv", "direnv");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "bash".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/bash.tar.gz".to_string(),
            },
            keg_dir_name: "5.3.9".to_string(),
            archive_path: PathBuf::from("/tmp/bash.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);
        let rewritten = rewrite_text(
            "/opt/homebrew/opt/bash/bin/bash\n",
            Path::new("/tmp/direnv"),
            "direnv",
            &rules,
        )
        .unwrap();
        assert_eq!(rewritten, "/opt/direnv/bin/bash\n");
    }

    #[test]
    fn rewrite_text_rewrites_generic_prefix_placeholder_paths() {
        let plan = fixed_i_plan("ripgrep", "ripgrep");
        let rules = build_rewrite_rules(&plan, &[]);
        let rewritten = rewrite_text(
            "@@HOMEBREW_PREFIX@@/share/ripgrep/help.txt\n",
            Path::new("/tmp/rg"),
            "ripgrep",
            &rules,
        )
        .unwrap();
        assert_eq!(rewritten, "/opt/ripgrep/share/ripgrep/help.txt\n");
    }

    #[test]
    fn rewrite_text_rewrites_versionless_cellar_placeholder_paths() {
        let plan = fixed_i_plan("python@3.12", "python@3.12");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "python@3.12".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/python@3.12.tar.gz".to_string(),
            },
            keg_dir_name: "3.12.13".to_string(),
            archive_path: PathBuf::from("/tmp/python@3.12.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);
        let rewritten = rewrite_text(
            "if os.path.realpath(sys.executable).startswith('@@HOMEBREW_CELLAR@@/python@3.12'):\n\
long_prefix = re.compile(r'@@HOMEBREW_CELLAR@@/python@3.12/[0-9\\._abrc]+')\n",
            Path::new("/tmp/sitecustomize.py"),
            "python@3.12",
            &rules,
        )
        .unwrap();
        assert_eq!(
            rewritten,
            "if os.path.realpath(sys.executable).startswith('/opt/python@3.12'):\n\
long_prefix = re.compile(r'/opt/python@3.12/[0-9\\._abrc]+')\n"
        );
    }

    #[test]
    fn relocate_file_skips_documentation_with_homebrew_placeholders() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("libssh2").join("1.11.1_1");
        let path = root.join("NEWS");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &path,
            "Changelog for the libssh2 project. Generated with git2news.pl\n\
@@HOMEBREW_PREFIX@@/include -> @@HOMEBREW_CELLAR@@/autoconf/2.72/bin/autoconf\n",
        )
        .unwrap();

        let rules = vec![
            RewriteRule {
                source: "@@HOMEBREW_PREFIX@@".to_string(),
                destination: "/opt/bat".to_string(),
            },
            RewriteRule {
                source: "@@HOMEBREW_CELLAR@@/autoconf/2.72".to_string(),
                destination: "/opt/bat".to_string(),
            },
        ];

        relocate_file(&path, &root, Path::new("/opt/bat"), "libssh2", &rules, None).unwrap();

        let unchanged = fs::read_to_string(&path).unwrap();
        assert!(unchanged.contains("@@HOMEBREW_PREFIX@@/include"));
        assert!(unchanged.contains("@@HOMEBREW_CELLAR@@/autoconf/2.72/bin/autoconf"));
    }

    #[test]
    fn relocate_file_rewrites_non_utf8_binary_paths() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("ripgrep").join("14.1.1");
        let path = root.join("bin/rg");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            b"\xff/opt/homebrew/opt/pcre2/lib/libpcre2-8.dylib\0tail",
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o444);
        fs::set_permissions(&path, permissions).unwrap();
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/opt/pcre2".to_string(),
            destination: "/opt/rg".to_string(),
        }];

        relocate_file(&path, &root, Path::new("/opt/rg"), "ripgrep", &rules, None).unwrap();

        let rewritten = fs::read(&path).unwrap();
        assert!(rewritten.starts_with(b"\xff/opt/"));
        assert!(
            !rewritten
                .windows(b"/opt/homebrew".len())
                .any(|window| window == b"/opt/homebrew")
        );
        assert!(
            rewritten
                .windows(b"lib/libpcre2-8.dylib".len())
                .any(|window| window == b"lib/libpcre2-8.dylib")
        );
        assert!(fs::metadata(&path).unwrap().permissions().mode() & 0o200 != 0);
    }

    #[test]
    fn relocate_file_rewrites_utf8_text_paths_and_skips_static_archives() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("ripgrep").join("14.1.1");
        let path = root.join("lib/pkgconfig/libpcre2.pc");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "prefix=/opt/homebrew/opt/pcre2\n").unwrap();
        let archive = root.join("lib/libpcre2.a");
        fs::write(&archive, b"/opt/homebrew/opt/pcre2").unwrap();
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/opt/pcre2".to_string(),
            destination: "/opt/rg".to_string(),
        }];

        relocate_file(&path, &root, Path::new("/opt/rg"), "ripgrep", &rules, None).unwrap();
        relocate_file(
            &archive,
            &root,
            Path::new("/opt/rg"),
            "ripgrep",
            &rules,
            None,
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "prefix=/opt/rg\n");
        assert_eq!(
            fs::read_to_string(&archive).unwrap(),
            "/opt/homebrew/opt/pcre2"
        );
    }

    #[test]
    fn documentation_detection_covers_share_doc_and_changelog_names() {
        let root = Path::new("/tmp/keg");
        assert!(is_documentation_text_path(
            Path::new("/tmp/keg/share/doc/foo/config.example"),
            root
        ));
        assert!(is_documentation_text_path(
            Path::new("/tmp/keg/CHANGELOG.md"),
            root
        ));
        assert!(!is_documentation_text_path(
            Path::new("/tmp/keg/lib/pkgconfig/foo.pc"),
            root
        ));
    }

    #[test]
    fn relocate_symlink_rewrites_cross_keg_relative_target_for_i_installs() {
        let temp = TempDir::new().unwrap();
        let keg_root = temp.path().join("aws").join("2.0.0");
        let link = keg_root.join("libexec/bin/python3.13");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../../../../../opt/python@3.13/bin/python3.13", &link).unwrap();

        let plan = InstallPlan::for_i("aws".to_string(), "aws".to_string());
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "python@3.13".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/python@3.13.tar.gz".to_string(),
            },
            keg_dir_name: "3.13.2".to_string(),
            archive_path: temp.path().join("python@3.13.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);

        relocate_symlink(&link, &keg_root, &plan.install_root, &rules).unwrap();

        assert_eq!(
            fs::read_link(&link).unwrap(),
            PathBuf::from("../../bin/python3.13")
        );
    }

    #[test]
    fn relocate_tree_rewrites_isotope_archive_cross_keg_relative_targets() {
        let temp = TempDir::new().unwrap();
        let isotope_root = temp.path().join("aws-cli");
        let link = isotope_root.join("libexec/bin/python3.14");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../../../../../opt/python@3.14/bin/python3.14", &link).unwrap();

        let plan = InstallPlan::for_i_isotope("isotope:aws-cli".to_string(), "aws-cli");
        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "python@3.14".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/python@3.14.tar.gz".to_string(),
            },
            keg_dir_name: "3.14.0".to_string(),
            archive_path: temp.path().join("python@3.14.tar.gz"),
        }];
        let rules = build_rewrite_rules(&plan, &installs);

        relocate_tree(
            &isotope_root,
            &plan.stable_root,
            &plan.package_name,
            &rules,
            None,
        )
        .unwrap();

        assert_eq!(
            fs::read_link(&link).unwrap(),
            PathBuf::from("../../bin/python3.14")
        );
    }

    #[test]
    fn relocate_symlink_keeps_relative_targets_within_the_same_keg() {
        let temp = TempDir::new().unwrap();
        let keg_root = temp.path().join("aws").join("2.0.0");
        let link = keg_root.join("libexec/bin/aws");
        fs::create_dir_all(keg_root.join("bin")).unwrap();
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../../bin/aws", &link).unwrap();

        let plan = InstallPlan::for_i("aws".to_string(), "aws".to_string());

        relocate_symlink(&link, &keg_root, &plan.install_root, &[]).unwrap();

        assert_eq!(
            fs::read_link(&link).unwrap(),
            PathBuf::from("../../bin/aws")
        );
    }

    #[test]
    fn rewrite_binary_ignores_unmatched_opt_homebrew_include_paths() {
        let rules = vec![RewriteRule {
            source: "/opt/homebrew/Cellar/abseil/20260107.1".to_string(),
            destination: "/tmp/x/abseil".to_string(),
        }];
        let mut bytes = b"prefix\0/opt/homebrew/include/gtest/gtest-matchers.h\0".to_vec();
        let changed = rewrite_binary(
            &mut bytes,
            Path::new("/tmp/abseil"),
            "abseil",
            &rules,
            BinaryRewriteMode::Slash,
        )
        .unwrap();
        assert!(!changed);
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "prefix\0/opt/homebrew/include/gtest/gtest-matchers.h\0"
        );
    }

    #[test]
    fn pkg_allow_value_contains_matches_colon_separated_flags() {
        // Keep parsing tolerant so additional allow flags can coexist.
        assert!(pkg_allow_value_contains(
            "other:relocation-failures unsupported-formulas",
            "relocation-failures"
        ));
        assert!(pkg_allow_value_contains(
            "other:relocation-failures unsupported-formulas",
            "unsupported-formulas"
        ));
    }

    #[test]
    fn configure_debug_install_environment_adds_debug_allow_flags() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _env = TestEnvGuard::set(&[("PKG_ALLOW", "other-flag")]);

        configure_debug_install_environment();

        let value = env::var("PKG_ALLOW").unwrap();
        if cfg!(debug_assertions) {
            assert!(pkg_allow_value_contains(&value, "other-flag"));
            assert!(pkg_allow_value_contains(&value, "unsupported-formulas"));
            assert!(pkg_allow_value_contains(&value, "relocation-failures"));
        } else {
            assert_eq!(value, "other-flag");
        }
    }

    #[test]
    fn configure_debug_install_environment_preserves_existing_debug_flags() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _env = TestEnvGuard::set(&[("PKG_ALLOW", "unsupported-formulas:relocation-failures")]);

        configure_debug_install_environment();

        let value = env::var("PKG_ALLOW").unwrap();
        assert_eq!(value, "unsupported-formulas:relocation-failures");
    }

    #[test]
    fn pkg_allow_runtime_override_stays_debug_only() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _env = TestEnvGuard::set(&[("PKG_ALLOW", "relocation-failures")]);

        assert_eq!(
            pkg_allow_contains("relocation-failures"),
            cfg!(debug_assertions)
        );
    }

    #[test]
    fn handle_allowed_failure_writes_to_stderr_when_allowed() {
        let mut stderr = Vec::new();
        handle_allowed_failure("relocation failed".to_string(), true, &mut stderr).unwrap();
        assert_eq!(String::from_utf8(stderr).unwrap(), "relocation failed\n");
    }

    #[test]
    fn relocate_tree_with_options_allows_failures_and_reports_them() {
        let temp = TempDir::new().unwrap();
        let keg_root = temp.path().join("foo").join("1.0.0");
        let path = keg_root.join("share/foo/config.txt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "/opt/homebrew/Cellar/foo/1.0.0/share/foo/config\n").unwrap();

        let rules = vec![RewriteRule {
            source: "/opt/homebrew/Cellar/foo/1.0.0".to_string(),
            destination: "/opt/homebrew/Cellar/foo/1.0.0-shadow".to_string(),
        }];
        let mut stderr = Vec::new();

        relocate_tree_with_options(
            &keg_root,
            temp.path(),
            "foo",
            &rules,
            None,
            true,
            &mut stderr,
        )
        .unwrap();

        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("unsupported Homebrew path remains after text rewrite"));
        assert!(stderr.contains(path.to_string_lossy().as_ref()));
    }

    #[test]
    fn macos_release_name_maps_supported_versions() {
        assert_eq!(macos_release_name(14), Some("sonoma"));
        assert_eq!(macos_release_name(15), Some("sequoia"));
        assert_eq!(macos_release_name(26), Some("tahoe"));
    }

    #[test]
    fn ghcr_repo_from_blob_url_extracts_repository() {
        let repo =
            ghcr_repo_from_blob_url("https://ghcr.io/v2/homebrew/core/zopfli/blobs/sha256:abc123");
        assert_eq!(repo, Some("homebrew/core/zopfli"));
    }

    #[test]
    fn unsupported_install_hooks_allow_openssl3_post_install() {
        let info = formula_info(true);
        assert!(!formula_skips_unknown_post_install(
            "openssl@3",
            &info,
            false,
        ));
    }

    #[test]
    fn unsupported_install_hooks_allow_ca_certificates_post_install() {
        let info = formula_info(true);
        assert!(!formula_skips_unknown_post_install(
            "ca-certificates",
            &info,
            false,
        ));
    }

    #[test]
    fn unsupported_install_hooks_allow_service_formulae() {
        let info: FormulaInfo = serde_json::from_value(serde_json::json!({
            "versions": {
                "stable": "1.0.0"
            },
            "revision": 0,
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {}
                }
            },
            "disabled": false,
            "service": {
                "run": ["bin/exampled"]
            },
            "post_install_defined": false
        }))
        .unwrap();
        assert!(!formula_skips_unknown_post_install("example", &info, false));
    }

    #[test]
    fn unsupported_install_hooks_warn_for_other_post_install_formulae() {
        let info = formula_info(true);
        assert!(formula_skips_unknown_post_install("gettext", &info, false));

        let mut stderr = Vec::new();
        warn_skipped_post_install("gettext", &mut stderr);
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "warning: skipping Homebrew post_install for gettext; install may be incomplete\n"
        );
    }

    #[test]
    fn unsupported_install_hooks_allow_python_formula_when_enabled() {
        let info = formula_info(true);
        assert!(formula_skips_unknown_post_install(
            "python@3.12",
            &info,
            false,
        ));
        assert!(!formula_skips_unknown_post_install(
            "python@3.12",
            &info,
            true,
        ));
    }

    #[test]
    fn prepare_vendor_root_area_clears_existing_contents() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "codex".to_string(),
            root_formula: "codex".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("install"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("pkgs/node")).unwrap();
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(plan.install_root.join(".pkg")).unwrap();
        fs::write(plan.install_root.join("bin/codex"), b"old").unwrap();
        fs::write(plan.install_root.join(STUB_MANIFEST), b"old").unwrap();

        prepare_vendor_root_area(&plan).unwrap();

        assert!(!plan.install_root.join("pkgs").exists());
        assert!(!plan.install_root.join("bin/codex").exists());
        assert!(!plan.install_root.join(STUB_MANIFEST).exists());
    }

    #[test]
    fn partition_dependency_names_prefers_vendor_packages() {
        let (formulas, vendors) = partition_dependency_names(&["bun", "ripgrep"]).unwrap();

        assert_eq!(formulas, vec!["ripgrep".to_string()]);
        assert_eq!(vendors, vec!["bun".to_string()]);
    }

    #[test]
    fn partition_dependency_names_handles_vendor_packages_without_formula_dependencies() {
        let (formulas, vendors) = partition_dependency_names(&["bun"]).unwrap();

        assert!(formulas.is_empty());
        assert_eq!(vendors, vec!["bun".to_string()]);
    }

    #[test]
    fn npm_package_homebrew_dependencies_support_exact_and_leaf_matches() {
        assert_eq!(
            npm_package_homebrew_dependencies("@tobilu/qmd"),
            vec!["sqlite".to_string()]
        );
        assert_eq!(
            npm_package_homebrew_dependencies("openclaw"),
            vec!["sqlite".to_string()]
        );
    }

    #[test]
    fn pip_package_install_data_supports_dependencies_and_python_formula() {
        assert_eq!(
            pip_package_homebrew_dependencies("Psycopg2"),
            vec!["libpq".to_string()]
        );
        assert_eq!(pip_package_python_formula("psycopg2"), "python@3.12");
        assert_eq!(pip_package_python_formula("unknown"), "python");
    }

    #[test]
    fn append_vendor_npm_homebrew_dependencies_uses_vendor_install_strategy() {
        let qmd = VendorInstall {
            package: vendor::VendorPackage {
                name: "qmd",
                dependencies: &[],
                executables: &["qmd"],
                version: fake_vendor_version,
                download_url: None,
                install: fake_qmd_install_strategy,
            },
            version: Version::parse("1.2.3").unwrap(),
        };
        let mut formulas = Vec::new();

        append_vendor_npm_homebrew_dependencies(&mut formulas, &[qmd]);

        assert_eq!(formulas, vec!["sqlite".to_string()]);
    }

    #[test]
    fn append_pip_package_homebrew_dependencies_uses_embedded_data() {
        let mut formulas = vec!["python@3.12".to_string()];

        append_pip_package_homebrew_dependencies(&mut formulas, "psycopg2");

        assert_eq!(
            formulas,
            vec!["python@3.12".to_string(), "libpq".to_string()]
        );
    }

    #[test]
    fn vendor_dependency_is_current_requires_matching_receipt_and_executable() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "npm-openclaw".to_string(),
            root_formula: "npm-openclaw".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("install"),
            tmp_root: temp.path().join("tmp"),
        };
        let install = fake_vendor_install("bun", &["bun"], "1.2.3");

        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        write_package_receipt(
            &plan.receipt_path("bun"),
            &PackageReceipt {
                package_name: "bun".to_string(),
                version: "1.2.3".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: "bun".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        assert!(!vendor_dependency_is_current(&plan, &install).unwrap());

        write_executable(&plan.install_root.join("bin/bun"));

        assert!(vendor_dependency_is_current(&plan, &install).unwrap());
        assert!(vendor_dependencies_are_current(&plan, std::slice::from_ref(&install)).unwrap());

        let missing = fake_vendor_install("codex", &["codex"], "0.2.0");
        assert!(!vendor_dependencies_are_current(&plan, &[missing]).unwrap());
        assert!(vendor_dependencies_are_current(&plan, &[]).unwrap());
        assert!(
            install_vendor_copy_tree(&plan, &install, "pkg", None)
                .unwrap_err()
                .contains("has no download URL")
        );
        assert!(
            install_vendor_copy_file(
                &plan,
                &[],
                &install,
                "pkg/bin/codex",
                "bin",
                None,
                0o755,
                &[],
                None
            )
            .unwrap_err()
            .contains("has no download URL")
        );
    }

    #[test]
    fn npm_install_sandbox_profile_denies_users_and_library() {
        let profile = npm_install_sandbox_profile(Path::new("/opt/.tmp/pkg"));

        assert!(profile.contains(r#"(deny file-read* (subpath "/Users"))"#));
        assert!(profile.contains(r#"(deny file-write* (subpath "/Users"))"#));
        assert!(profile.contains(r#"(deny file-read* (subpath "/Library"))"#));
        assert!(profile.contains(r#"(deny file-write* (subpath "/Library"))"#));
        assert!(profile.contains(r#"(allow file-read* (subpath "/opt/.tmp/pkg"))"#));
        assert!(profile.contains(r#"(allow file-write* (subpath "/opt/.tmp/pkg"))"#));
    }

    #[test]
    fn render_npm_probe_error_reports_exit_codes_and_signals() {
        use std::os::unix::process::ExitStatusExt;

        let exit_error = render_npm_probe_error(
            "openclaw",
            NpmProbeError {
                status: ExitStatus::from_raw(2 << 8),
                lines: vec!["npm ERR! denied".to_string()],
            },
        );
        assert!(exit_error.contains("exit code 2"));
        assert!(exit_error.contains("npm ERR! denied"));

        let signal_error = render_npm_probe_error(
            "openclaw",
            NpmProbeError {
                status: ExitStatus::from_raw(9),
                lines: Vec::new(),
            },
        );
        assert!(signal_error.contains("terminated by signal"));
    }

    #[test]
    fn build_sandboxed_npm_install_command_uses_isolated_env() {
        let _lock = test_env_lock().lock().unwrap();
        let _env = TestEnvGuard::set(&[("CODEX_CI", "1")]);
        let temp = TempDir::new().unwrap();
        let sandbox_root = TempDir::new_in(temp.path()).unwrap();
        let install_root = temp.path().join("install");
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();

        let command = build_sandboxed_npm_install_command(
            "/usr/bin/sandbox-exec",
            "/opt/pkg/bin/npm",
            "https://registry.npmjs.org/openclaw/-/openclaw-1.2.3.tgz",
            &install_root,
            &tmp_root,
            &sandbox_root,
            OsString::from("/opt/pkg/bin"),
            false,
        )
        .unwrap();

        let args: Vec<_> = command.get_args().collect();
        assert!(should_bypass_npm_install_sandbox());
        assert_eq!(command.get_program(), OsStr::new("/opt/pkg/bin/npm"));
        assert_eq!(args[0], OsStr::new("install"));
        assert_eq!(args[1], OsStr::new("-g"));
        assert_eq!(args[2], OsStr::new("--prefix"));
        assert_eq!(args[3], install_root.as_os_str());
        assert_eq!(
            args[4],
            OsStr::new("https://registry.npmjs.org/openclaw/-/openclaw-1.2.3.tgz")
        );
        assert_eq!(
            *args.last().unwrap(),
            OsStr::new("https://registry.npmjs.org/openclaw/-/openclaw-1.2.3.tgz")
        );
        assert_eq!(command.get_current_dir().unwrap(), sandbox_root.path());

        let envs: HashMap<_, _> = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_owned(),
                    value.map(|value| value.to_owned()).unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(
            envs.get(OsStr::new("PATH")).unwrap(),
            &OsString::from("/opt/pkg/bin")
        );
        assert_eq!(
            envs.get(OsStr::new("TMPDIR")).unwrap(),
            tmp_root.as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("HOME")).unwrap(),
            sandbox_root.path().join("home").as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("XDG_CONFIG_HOME")).unwrap(),
            sandbox_root.path().join("xdg-config").as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("XDG_CACHE_HOME")).unwrap(),
            sandbox_root.path().join("xdg-cache").as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("NPM_CONFIG_CACHE")).unwrap(),
            sandbox_root.path().join("npm-cache").as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("NPM_CONFIG_USERCONFIG")).unwrap(),
            sandbox_root.path().join("npmrc").as_os_str()
        );
        assert_eq!(
            envs.get(OsStr::new("NPM_CONFIG_CAFILE")).unwrap(),
            OsStr::new("/opt/pkg/ssl/cert.pem")
        );
        assert_eq!(
            envs.get(OsStr::new("NODE_EXTRA_CA_CERTS")).unwrap(),
            OsStr::new("/opt/pkg/ssl/cert.pem")
        );

        let profile_path = sandbox_root.path().join("sandbox.sb");
        assert!(profile_path.is_file());
        assert!(sandbox_root.path().join("npmrc").is_file());
        assert_eq!(
            fs::read_to_string(sandbox_root.path().join("npmrc")).unwrap(),
            ""
        );
    }

    #[test]
    fn build_sandboxed_npm_install_command_uses_sandbox_when_codex_bypass_is_absent() {
        let _lock = test_env_lock().lock().unwrap();
        let _env = TestEnvGuard::unset(&["CODEX_CI"]);
        let temp = TempDir::new().unwrap();
        let sandbox_root = TempDir::new_in(temp.path()).unwrap();
        let install_root = temp.path().join("install");
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();

        let command = build_sandboxed_npm_install_command(
            "/usr/bin/sandbox-exec",
            "/opt/pkg/bin/npm",
            "coverage-npm",
            &install_root,
            &tmp_root,
            &sandbox_root,
            OsString::from("/opt/pkg/bin"),
            true,
        )
        .unwrap();

        let args = command.get_args().collect::<Vec<_>>();
        assert_eq!(command.get_program(), OsStr::new("/usr/bin/sandbox-exec"));
        assert_eq!(args[0], OsStr::new("-f"));
        assert_eq!(args[2], OsStr::new("/opt/pkg/bin/npm"));
        assert!(args.contains(&OsStr::new("--dry-run")));
        assert_eq!(*args.last().unwrap(), OsStr::new("coverage-npm"));
    }

    #[test]
    fn normalize_bundled_npm_extension_dependencies_links_missing_root_packages() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("opt/npm/openclaw");
        let package_root = install_root.join("lib/node_modules/openclaw");
        let nested_carbon = package_root.join("dist/extensions/discord/node_modules/@buape/carbon");
        fs::create_dir_all(&nested_carbon).unwrap();

        normalize_bundled_npm_extension_dependencies(&install_root).unwrap();

        let root_carbon = package_root.join("node_modules/@buape/carbon");
        let metadata = fs::symlink_metadata(&root_carbon).unwrap();
        assert!(metadata.file_type().is_symlink());
        let target = fs::read_link(&root_carbon).unwrap();
        assert!(target.is_relative());
        assert_eq!(
            fs::canonicalize(root_carbon.parent().unwrap().join(&target)).unwrap(),
            fs::canonicalize(&nested_carbon).unwrap()
        );
    }

    #[test]
    fn normalize_bundled_npm_extension_dependencies_preserves_existing_root_packages() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("opt/npm/openclaw");
        let package_root = install_root.join("lib/node_modules/openclaw");
        let nested_carbon = package_root.join("dist/extensions/discord/node_modules/@buape/carbon");
        let root_carbon = package_root.join("node_modules/@buape/carbon");
        fs::create_dir_all(&nested_carbon).unwrap();
        fs::create_dir_all(&root_carbon).unwrap();
        fs::write(root_carbon.join("package.json"), "{}").unwrap();

        normalize_bundled_npm_extension_dependencies(&install_root).unwrap();

        let metadata = fs::symlink_metadata(&root_carbon).unwrap();
        assert!(metadata.is_dir());
        assert!(!metadata.file_type().is_symlink());
    }

    #[test]
    fn normalize_bundled_npm_extension_dependencies_finds_scoped_package_roots() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("opt/npm/widget");
        let package_root = install_root.join("lib/node_modules/@scope/widget");
        let nested_carbon = package_root.join("dist/extensions/discord/node_modules/@buape/carbon");
        fs::create_dir_all(&nested_carbon).unwrap();

        normalize_bundled_npm_extension_dependencies(&install_root).unwrap();

        let root_carbon = package_root.join("node_modules/@buape/carbon");
        let metadata = fs::symlink_metadata(&root_carbon).unwrap();
        assert!(metadata.file_type().is_symlink());
        let target = fs::read_link(&root_carbon).unwrap();
        assert!(target.is_relative());
        assert_eq!(
            fs::canonicalize(root_carbon.parent().unwrap().join(&target)).unwrap(),
            fs::canonicalize(&nested_carbon).unwrap()
        );
    }

    #[test]
    fn normalize_bundled_npm_extension_dependencies_finds_nested_dist_node_modules() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("opt/npm/openclaw");
        let package_root = install_root.join("lib/node_modules/openclaw");
        let nested_carbon = package_root.join("dist/ui/runtime/node_modules/@buape/carbon");
        fs::create_dir_all(&nested_carbon).unwrap();

        normalize_bundled_npm_extension_dependencies(&install_root).unwrap();

        let root_carbon = package_root.join("node_modules/@buape/carbon");
        let metadata = fs::symlink_metadata(&root_carbon).unwrap();
        assert!(metadata.file_type().is_symlink());
        let target = fs::read_link(&root_carbon).unwrap();
        assert!(target.is_relative());
        assert_eq!(
            fs::canonicalize(root_carbon.parent().unwrap().join(&target)).unwrap(),
            fs::canonicalize(&nested_carbon).unwrap()
        );
    }

    #[test]
    fn build_pip_commands_use_isolated_env() {
        let temp = TempDir::new().unwrap();
        let sandbox_root = temp.path().join("sandbox");
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "pip:psycopg2".to_string(),
            root_formula: "pip:psycopg2".to_string(),
            stable_root: temp.path().join("opt/pip/psycopg2"),
            install_root: temp.path().join("opt/pip/psycopg2"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.tmp_root).unwrap();

        let venv_command = build_pip_venv_command(
            "/opt/pip/psycopg2/bin/python3",
            &plan.install_root.join("venv"),
            &sandbox_root,
            &plan,
            &[],
        )
        .unwrap();
        assert_eq!(
            venv_command.get_program(),
            OsStr::new("/opt/pip/psycopg2/bin/python3")
        );
        let venv_args: Vec<_> = venv_command.get_args().collect();
        assert_eq!(
            venv_args,
            vec![
                OsStr::new("-m"),
                OsStr::new("venv"),
                OsStr::new("--copies"),
                plan.install_root.join("venv").as_os_str(),
            ]
        );

        let pip_command = build_pip_install_command(
            &plan.install_root.join("venv/bin/pip"),
            "psycopg2",
            "2.9.10",
            &sandbox_root,
            &plan,
            &[],
        )
        .unwrap();
        assert_eq!(
            pip_command.get_program(),
            plan.install_root.join("venv/bin/pip").as_os_str()
        );
        let pip_args: Vec<_> = pip_command.get_args().collect();
        assert_eq!(
            pip_args,
            vec![
                OsStr::new("install"),
                OsStr::new("--disable-pip-version-check"),
                OsStr::new("--no-input"),
                OsStr::new("psycopg2==2.9.10"),
            ]
        );

        for command in [&venv_command, &pip_command] {
            let envs: HashMap<_, _> = command
                .get_envs()
                .map(|(key, value)| {
                    (
                        key.to_owned(),
                        value.map(|value| value.to_owned()).unwrap_or_default(),
                    )
                })
                .collect();
            assert_eq!(
                envs.get(OsStr::new("TMPDIR")).unwrap(),
                plan.tmp_root.as_os_str()
            );
            assert_eq!(
                envs.get(OsStr::new("HOME")).unwrap(),
                sandbox_root.join("home").as_os_str()
            );
            assert_eq!(
                envs.get(OsStr::new("XDG_CACHE_HOME")).unwrap(),
                sandbox_root.join("xdg-cache").as_os_str()
            );
            assert_eq!(
                envs.get(OsStr::new("PIP_CACHE_DIR")).unwrap(),
                sandbox_root.join("pip-cache").as_os_str()
            );
            assert_eq!(
                envs.get(OsStr::new("PYTHONNOUSERSITE")).unwrap(),
                OsStr::new("1")
            );
        }
    }

    #[test]
    fn pip_entrypoint_discovery_script_has_indented_function_body() {
        let script = pip_entrypoint_discovery_script();

        assert!(script.contains("def norm(value):\n    out = []\n"));
        assert!(script.contains("\nfor dist in md.distributions():\n"));
    }

    #[test]
    fn collect_declared_root_executables_finds_bin_and_sbin() {
        let temp = TempDir::new().unwrap();
        let bin_dir = temp.path().join("bin");
        let sbin_dir = temp.path().join("sbin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&sbin_dir).unwrap();
        let foo = bin_dir.join("foo");
        let bar = sbin_dir.join("bar");
        fs::write(&foo, b"#!/bin/sh\n").unwrap();
        fs::write(&bar, b"#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(&foo).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&foo, permissions.clone()).unwrap();
        fs::set_permissions(&bar, permissions).unwrap();

        let found = collect_declared_root_executables(temp.path(), ["foo", "bar"]).unwrap();
        assert_eq!(
            found,
            vec![("bar".to_string(), bar), ("foo".to_string(), foo)]
        );
    }

    #[test]
    fn filter_stub_executables_omits_excluded_names() {
        let executables = vec![
            ("bash".to_string(), PathBuf::from("/tmp/bin/bash")),
            ("bashbug".to_string(), PathBuf::from("/tmp/bin/bashbug")),
        ];
        let excluded = HashSet::from(["bashbug".to_string()]);

        assert_eq!(
            filter_stub_executables(executables, &excluded),
            vec![("bash".to_string(), PathBuf::from("/tmp/bin/bash"))]
        );
    }

    #[test]
    fn formula_stub_exclusions_load_bashbug() {
        assert_eq!(
            formula_stub_exclusions("bash"),
            HashSet::from(["bashbug".to_string()])
        );
    }

    #[test]
    fn formula_stub_exclusions_alias_ffmpeg_to_ffmpeg_full() {
        assert_eq!(
            formula_stub_exclusions("ffmpeg"),
            formula_stub_exclusions("ffmpeg-full")
        );
    }

    #[test]
    fn formula_stub_exclusions_cover_dead_python_tools() {
        let exclusions = formula_stub_exclusions("python@3.12");

        for name in [
            "2to3",
            "2to3-3.12",
            "idle3",
            "idle3.12",
            "pydoc3",
            "pydoc3.12",
            "python3-config",
            "python3.12-config",
            "wheel",
            "wheel3",
            "wheel3.12",
        ] {
            assert!(exclusions.contains(name), "missing exclusion for {name}");
        }

        assert!(!exclusions.contains("python3.12"));
        assert!(!exclusions.contains("pip3.12"));
    }

    #[test]
    fn imagemagick_full_only_stubs_magick() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "imagemagick-full".to_string(),
            root_formula: "imagemagick-full".to_string(),
            stable_root: temp.path().join("opt/imagemagick-full"),
            install_root: temp.path().join("opt/imagemagick-full"),
            tmp_root: temp.path().join("tmp"),
        };
        let current = vec![
            ("convert".to_string(), PathBuf::from("/tmp/bin/convert")),
            ("magick".to_string(), PathBuf::from("/tmp/bin/magick")),
            ("identify".to_string(), PathBuf::from("/tmp/bin/identify")),
        ];

        assert_eq!(
            imagemagick_stub_exclusions(&plan, &current),
            HashSet::from(["convert".to_string(), "identify".to_string()])
        );
    }

    #[test]
    fn imagemagick_v7_only_stubs_magick() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "imagemagick".to_string(),
            root_formula: "imagemagick".to_string(),
            stable_root: temp.path().join("opt/imagemagick"),
            install_root: temp.path().join("opt/imagemagick"),
            tmp_root: temp.path().join("tmp"),
        };
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "imagemagick".to_string(),
                version: "7.1.2_3".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "imagemagick".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        let current = vec![
            ("convert".to_string(), PathBuf::from("/tmp/bin/convert")),
            ("magick".to_string(), PathBuf::from("/tmp/bin/magick")),
            ("mogrify".to_string(), PathBuf::from("/tmp/bin/mogrify")),
        ];

        assert_eq!(
            imagemagick_stub_exclusions(&plan, &current),
            HashSet::from(["convert".to_string(), "mogrify".to_string()])
        );
    }

    #[test]
    fn imagemagick_v6_keeps_legacy_stubs() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "imagemagick".to_string(),
            root_formula: "imagemagick".to_string(),
            stable_root: temp.path().join("opt/imagemagick"),
            install_root: temp.path().join("opt/imagemagick"),
            tmp_root: temp.path().join("tmp"),
        };
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "imagemagick".to_string(),
                version: "6.9.13_7".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "imagemagick".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        let current = vec![
            ("convert".to_string(), PathBuf::from("/tmp/bin/convert")),
            ("magick".to_string(), PathBuf::from("/tmp/bin/magick")),
        ];

        assert!(imagemagick_stub_exclusions(&plan, &current).is_empty());
    }

    #[test]
    fn stage_formula_merges_dependency_into_i_root() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "direnv".to_string(),
            root_formula: "direnv".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("direnv-2.37.1"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("existing")).unwrap();
        fs::write(plan.install_root.join("existing/root.txt"), b"root").unwrap();

        let keg_root = temp.path().join("ncurses/6.6");
        fs::create_dir_all(keg_root.join(".brew")).unwrap();
        fs::create_dir_all(keg_root.join(".bottle")).unwrap();
        fs::create_dir_all(keg_root.join("share")).unwrap();
        fs::write(keg_root.join("README"), b"dependency docs").unwrap();
        fs::write(
            keg_root.join(".brew/formula.rb"),
            b"class Ncurses < Formula",
        )
        .unwrap();
        fs::write(keg_root.join(".bottle/metadata.json"), b"{}").unwrap();
        fs::write(keg_root.join("share/term.info"), b"dep").unwrap();

        let install = InstalledFormula {
            spec: FormulaSpec {
                name: "ncurses".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/ncurses.tar.gz".to_string(),
            },
            keg_dir_name: "6.6".to_string(),
            archive_path: temp.path().join("ncurses.tar.gz"),
        };

        stage_formula(&plan, &install, &keg_root).unwrap();

        assert!(plan.install_root.join("existing/root.txt").is_file());
        assert!(plan.install_root.join("share/term.info").is_file());
        assert!(!plan.install_root.join("README").exists());
        assert!(!plan.install_root.join(".brew").exists());
        assert!(plan.install_root.join(".bottle/metadata.json").is_file());
        assert!(!keg_root.join("share/term.info").exists());
        assert!(!keg_root.join("README").exists());
        assert!(!keg_root.join(".brew").exists());
    }

    #[test]
    fn stage_formula_keeps_root_formula_docs_but_drops_brew_dir() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "direnv".to_string(),
            root_formula: "direnv".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("direnv-2.37.1"),
            tmp_root: temp.path().join("tmp"),
        };

        let keg_root = temp.path().join("direnv/2.37.1");
        fs::create_dir_all(keg_root.join(".brew")).unwrap();
        fs::create_dir_all(keg_root.join(".bottle")).unwrap();
        fs::create_dir_all(keg_root.join("bin")).unwrap();
        fs::write(keg_root.join("README"), b"root docs").unwrap();
        fs::write(keg_root.join(".brew/formula.rb"), b"class Direnv < Formula").unwrap();
        fs::write(keg_root.join(".bottle/metadata.json"), b"{}").unwrap();
        fs::write(keg_root.join("bin/direnv"), b"#!/bin/sh\n").unwrap();

        let install = InstalledFormula {
            spec: FormulaSpec {
                name: "direnv".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/direnv.tar.gz".to_string(),
            },
            keg_dir_name: "2.37.1".to_string(),
            archive_path: temp.path().join("direnv.tar.gz"),
        };

        stage_formula(&plan, &install, &keg_root).unwrap();

        assert!(plan.install_root.join("README").is_file());
        assert!(plan.install_root.join(".bottle/metadata.json").is_file());
        assert!(plan.install_root.join("bin/direnv").is_file());
        assert!(!plan.install_root.join(".brew").exists());
    }

    #[test]
    fn build_install_path_entries_dedupes_root_and_skips_missing_sbin() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "direnv".to_string(),
            root_formula: "direnv".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("direnv-2.37.1"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();

        let graph = vec![FormulaSpec {
            name: "ncurses".to_string(),
            bottle_sha256: "sha256".to_string(),
            bottle_url: "https://example.invalid/ncurses.tar.gz".to_string(),
        }];

        let entries = build_install_path_entries(&plan, &graph);
        assert_eq!(entries, vec![plan.install_root.join("bin")]);

        fs::create_dir_all(plan.install_root.join("sbin")).unwrap();
        let entries = build_install_path_entries(&plan, &graph);
        assert_eq!(
            entries,
            vec![
                plan.install_root.join("bin"),
                plan.install_root.join("sbin")
            ]
        );
    }

    #[test]
    fn resolve_command_in_path_entries_finds_executable() {
        let temp = TempDir::new().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        write_executable(&second.join("npm"));

        let resolved = resolve_command_in_path_entries(&[first, second.clone()], "npm").unwrap();

        assert_eq!(resolved, second.join("npm"));
    }

    #[test]
    fn install_time_command_probes_cover_success_failure_and_missing_tools() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "coverage-runtime".to_string(),
            root_formula: "coverage-runtime".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("install"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(&plan.tmp_root).unwrap();
        write_executable_with_body(&plan.install_root.join("bin/ok-runtime"), "exit 0\n");
        write_executable_with_body(&plan.install_root.join("bin/bad-runtime"), "exit 7\n");
        let progress = InstallProgress::with_callback("coverage-runtime", None);

        assert!(
            install_time_commands_are_usable(&plan, &[], ["ok-runtime"], Some(&progress)).unwrap()
        );
        assert!(
            !install_time_commands_are_usable(&plan, &[], ["ok-runtime", "bad-runtime"], None)
                .unwrap()
        );
        assert!(!install_time_commands_are_usable(&plan, &[], ["missing-runtime"], None).unwrap());
    }

    #[test]
    fn merge_path_into_recursively_merges_directories_and_replaces_files() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(source.join("nested/source.txt"), b"source").unwrap();
        fs::write(target.join("nested/target.txt"), b"target").unwrap();

        merge_path_into(&source, &target).unwrap();

        assert!(!source.exists());
        assert_eq!(
            fs::read(target.join("nested/source.txt")).unwrap(),
            b"source"
        );
        assert_eq!(
            fs::read(target.join("nested/target.txt")).unwrap(),
            b"target"
        );

        let source_file = temp.path().join("replacement");
        let target_file = target.join("nested/target.txt");
        fs::write(&source_file, b"replacement").unwrap();
        merge_path_into(&source_file, &target_file).unwrap();

        assert_eq!(fs::read(target_file).unwrap(), b"replacement");
    }

    #[test]
    fn passwd_entry_returns_current_user_when_available() {
        let uid = unsafe { libc::getuid() };
        let (home, name) = passwd_entry(uid);

        assert!(home.is_some() || name.is_some());

        if !is_root() {
            let identity = current_user_identity().unwrap();
            assert_eq!(identity.uid, uid);
            assert_eq!(identity.gid, unsafe { libc::getgid() });
        }
    }

    #[test]
    fn copy_path_preserves_symlinks_directories_and_file_modes() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination/tree");
        fs::create_dir_all(source.join("bin")).unwrap();
        fs::write(source.join("bin/tool"), b"tool").unwrap();
        symlink("../bin/tool", source.join("tool-link")).unwrap();
        let mut dir_permissions = fs::metadata(source.join("bin")).unwrap().permissions();
        dir_permissions.set_mode(0o755);
        fs::set_permissions(source.join("bin"), dir_permissions).unwrap();
        let mut file_permissions = fs::metadata(source.join("bin/tool")).unwrap().permissions();
        file_permissions.set_mode(0o700);
        fs::set_permissions(source.join("bin/tool"), file_permissions).unwrap();

        copy_path(&source, &destination).unwrap();

        assert_eq!(fs::read(destination.join("bin/tool")).unwrap(), b"tool");
        assert_eq!(
            fs::read_link(destination.join("tool-link")).unwrap(),
            PathBuf::from("../bin/tool")
        );
        assert_eq!(
            fs::metadata(destination.join("bin"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(destination.join("bin/tool"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }

    #[test]
    fn sanitize_progress_message_uses_latest_non_empty_line() {
        let message = "\rfirst line\n\n replacing existing signature \n";
        assert_eq!(
            sanitize_progress_message(message),
            "replacing existing signature"
        );
    }

    #[test]
    fn installed_stub_paths_use_usr_local_bin_prefix() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "python".to_string(),
            root_formula: "python".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("python"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.install_root).unwrap();
        write_stub_manifest(
            &plan.package_manifest_path(),
            &StubManifest {
                stubs: vec!["pip3".to_string(), "python".to_string()],
            },
        )
        .unwrap();

        assert_eq!(
            installed_stub_paths(&plan).unwrap(),
            vec![
                managed_bin_root().join("pip3").display().to_string(),
                managed_bin_root().join("python").display().to_string()
            ]
        );
    }

    #[test]
    fn sync_stubs_writes_root_executables_and_removes_stale_entries() {
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let temp = TempDir::new().unwrap();
        let package_name = "coverage-sync-stubs";
        let bin_root = managed_bin_root();
        let stale_stub = bin_root.join("coverage-stale");
        let foo_stub = bin_root.join("coverage-sync-foo");
        let bar_stub = bin_root.join("coverage-sync-bar");

        for path in [&stale_stub, &foo_stub, &bar_stub] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        let plan = InstallPlan {
            mode: Mode::I,
            package_name: package_name.to_string(),
            root_formula: package_name.to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join(package_name),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(plan.install_root.join("sbin")).unwrap();
        write_executable(&plan.install_root.join("bin/coverage-sync-foo"));
        write_executable(&plan.install_root.join("sbin/coverage-sync-bar"));
        write_executable(&stale_stub);

        sync_stubs(&plan, &[], &["coverage-stale".to_string()]).unwrap();

        assert!(is_executable(&foo_stub));
        assert!(is_executable(&bar_stub));
        assert!(fs::symlink_metadata(&stale_stub).is_err());
        assert_eq!(
            load_stub_manifest(&plan.package_manifest_path())
                .unwrap()
                .stubs,
            vec![
                "coverage-sync-bar".to_string(),
                "coverage-sync-foo".to_string(),
            ]
        );

        for path in [&foo_stub, &bar_stub] {
            remove_path(path).unwrap();
        }
    }

    #[test]
    fn sync_stubs_respects_declared_root_executable_manifest() {
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let temp = TempDir::new().unwrap();
        let package_name = "coverage-sync-manifest";
        let bin_root = managed_bin_root();
        let kept_stub = bin_root.join("coverage-keep");
        let skipped_stub = bin_root.join("coverage-skip");

        for path in [&kept_stub, &skipped_stub] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        let plan = InstallPlan {
            mode: Mode::I,
            package_name: package_name.to_string(),
            root_formula: package_name.to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join(package_name),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        write_executable(&plan.install_root.join("bin/coverage-keep"));
        write_executable(&plan.install_root.join("bin/coverage-skip"));
        write_root_executable_manifest(
            &plan.root_executables_manifest_path(),
            &["coverage-keep".to_string()],
        )
        .unwrap();

        sync_stubs(&plan, &[], &[]).unwrap();

        assert!(is_executable(&kept_stub));
        assert!(fs::symlink_metadata(&skipped_stub).is_err());
        assert_eq!(
            load_stub_manifest(&plan.package_manifest_path())
                .unwrap()
                .stubs,
            vec!["coverage-keep".to_string()]
        );

        remove_path(&kept_stub).unwrap();
    }

    #[test]
    fn stub_helpers_cover_missing_and_invalid_manifest_cases() {
        let temp = TempDir::new().unwrap();
        let manifest_path = temp.path().join("stub-manifest.json");
        fs::write(&manifest_path, b"{not json").unwrap();
        assert!(
            load_stub_manifest(&manifest_path)
                .unwrap_err()
                .contains("failed to parse")
        );

        assert!(!stub_belongs_to_package(&temp.path().join("missing-stub"), "coverage").unwrap());

        let empty_bin_dir = temp.path().join("missing-bin");
        refresh_post_uninstall_stubs(temp.path(), &empty_bin_dir).unwrap();

        let parent = temp.path().join("nested");
        fs::create_dir_all(&parent).unwrap();
        fs::write(parent.join("keep"), b"keep").unwrap();
        remove_empty_parent_dirs(&parent.join("missing/child"), temp.path()).unwrap();
        assert!(parent.exists());
    }

    #[test]
    fn stub_helpers_cover_non_not_found_io_errors() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("bin"), b"not a directory").unwrap();
        assert!(
            collect_root_executables(&root)
                .unwrap_err()
                .contains("failed to read")
        );

        let manifest_dir = temp.path().join("manifest-dir");
        fs::create_dir_all(&manifest_dir).unwrap();
        assert!(
            load_stub_manifest(&manifest_dir)
                .unwrap_err()
                .contains("failed to read")
        );
        assert!(
            stub_belongs_to_package(&manifest_dir, "coverage")
                .unwrap_err()
                .contains("failed to read")
        );

        let blocking_file = temp.path().join("blocking-file");
        fs::write(&blocking_file, b"file").unwrap();
        assert!(
            remove_empty_parent_dirs(&blocking_file.join("child"), temp.path())
                .unwrap_err()
                .contains("failed to remove")
        );

        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        remove_existing_package_install(&opt_root, "missing", &bin_dir).unwrap();
    }

    #[test]
    fn sync_declared_stubs_filters_exclusions_and_removes_stale_entries() {
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let temp = TempDir::new().unwrap();
        let package_name = "coverage-declared-stubs";
        let bin_root = managed_bin_root();
        let kept_stub = bin_root.join("coverage-declared-keep");
        let stale_stub = bin_root.join("coverage-declared-stale");
        let excluded_stub = bin_root.join("coverage-declared-skip");

        for path in [&kept_stub, &stale_stub, &excluded_stub] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        let plan = InstallPlan {
            mode: Mode::I,
            package_name: package_name.to_string(),
            root_formula: package_name.to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join(package_name),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        write_executable(&plan.install_root.join("bin/coverage-declared-keep"));
        write_executable(&plan.install_root.join("bin/coverage-declared-skip"));
        write_executable(&stale_stub);

        let excluded = HashSet::from(["coverage-declared-skip".to_string()]);
        sync_declared_stubs(
            &plan,
            &[],
            ["coverage-declared-keep", "coverage-declared-skip"],
            &excluded,
            &["coverage-declared-stale".to_string()],
        )
        .unwrap();

        assert!(is_executable(&kept_stub));
        assert!(fs::symlink_metadata(&stale_stub).is_err());
        assert!(fs::symlink_metadata(&excluded_stub).is_err());
        assert_eq!(
            load_stub_manifest(&plan.package_manifest_path())
                .unwrap()
                .stubs,
            vec!["coverage-declared-keep".to_string()]
        );

        remove_path(&kept_stub).unwrap();
    }

    #[test]
    fn run_package_post_install_returns_early_without_supported_formulas() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "coverage-no-post-install".to_string(),
            root_formula: "coverage-no-post-install".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("coverage-no-post-install"),
            tmp_root: temp.path().join("tmp"),
        };
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        run_package_post_install(&plan, &[], &bin_dir).unwrap();

        assert!(fs::symlink_metadata(plan.package_manifest_path()).is_err());
    }

    #[test]
    fn run_package_post_install_creates_python_dispatchers_and_openssl_cert_path() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "python@3.12".to_string(),
            root_formula: "python@3.12".to_string(),
            stable_root: opt_root.join("python@3.12"),
            install_root: opt_root.join("python@3.12"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.install_root).unwrap();
        fs::create_dir_all(opt_root.join("python@3.13")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(plan.install_root.join(OPENSSL_CA_CERTIFICATES_DIR)).unwrap();
        fs::write(
            plan.install_root.join(OPENSSL_CA_CERTIFICATES_CERT),
            b"cert bundle",
        )
        .unwrap();
        fs::write(
            plan.install_root
                .join(OPENSSL_CA_CERTIFICATES_DIR)
                .join("extra.pem"),
            b"extra cert",
        )
        .unwrap();
        write_stub_manifest(
            &plan.package_manifest_path(),
            &StubManifest {
                stubs: vec!["pip3.12".to_string(), "python3.12".to_string()],
            },
        )
        .unwrap();
        for name in ["python3.12", "pip3.12", "python3.13", "pip3.13"] {
            let path = bin_dir.join(name);
            fs::write(&path, b"#!/bin/sh\n").unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }

        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "openssl@3".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/openssl@3.tar.gz".to_string(),
            },
            keg_dir_name: "3.6.1".to_string(),
            archive_path: temp.path().join("openssl@3.tar.gz"),
        }];

        run_package_post_install(&plan, &installs, &bin_dir).unwrap();

        assert_eq!(
            fs::read_link(bin_dir.join("python")).unwrap(),
            PathBuf::from("python3")
        );
        assert_eq!(
            fs::read_link(bin_dir.join("pip")).unwrap(),
            PathBuf::from("pip3")
        );

        assert_eq!(
            fs::read_link(bin_dir.join("python3")).unwrap(),
            PathBuf::from("python3.13")
        );
        assert_eq!(
            fs::read_link(bin_dir.join("pip3")).unwrap(),
            PathBuf::from("pip3.13")
        );
        assert_eq!(
            fs::read_to_string(plan.install_root.join(OPENSSL_CERT_PEM_DESTINATION)).unwrap(),
            "cert bundle"
        );
        assert_eq!(
            fs::read_to_string(
                plan.install_root
                    .join(OPENSSL_CERT_PEM_DESTINATION_DIR)
                    .join("extra.pem")
            )
            .unwrap(),
            "extra cert"
        );
        assert!(!plan.install_root.join(OPENSSL_CA_CERTIFICATES_DIR).exists());

        let manifest = load_stub_manifest(&plan.package_manifest_path()).unwrap();
        assert_eq!(
            manifest.stubs,
            vec![
                "pip".to_string(),
                "pip3".to_string(),
                "pip3.12".to_string(),
                "python".to_string(),
                "python3".to_string(),
                "python3.12".to_string(),
            ]
        );
    }

    #[test]
    fn reinstall_vendor_dependency_tree_restores_formula_dependencies() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "demo".to_string(),
            root_formula: "demo".to_string(),
            stable_root: temp.path().join("demo"),
            install_root: temp.path().join("demo"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.tmp_root).unwrap();
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::write(plan.install_root.join("bin/demo"), b"#!/bin/sh\n").unwrap();

        let sqlite_archive = temp.path().join("sqlite.tar.gz");
        write_test_bottle_archive(
            &sqlite_archive,
            "sqlite",
            "3.49.1",
            &[("bin/sqlite3", b"#!/bin/sh\n")],
        );

        let installs = vec![InstalledFormula {
            spec: FormulaSpec {
                name: "sqlite".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/sqlite.tar.gz".to_string(),
            },
            keg_dir_name: "3.49.1".to_string(),
            archive_path: sqlite_archive,
        }];

        reinstall_vendor_dependency_tree(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &plan,
            &installs,
            &[],
            &[],
            None,
        )
        .unwrap();

        assert!(plan.install_root.join("bin/sqlite3").is_file());
        assert!(plan.receipt_path("sqlite").is_file());
        assert!(!plan.install_root.join("bin/demo").exists());
    }

    #[test]
    fn reinstall_vendor_dependency_tree_keeps_downloaded_bottles_alive_until_extract() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "demo".to_string(),
            root_formula: "demo".to_string(),
            stable_root: temp.path().join("demo"),
            install_root: temp.path().join("demo"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.tmp_root).unwrap();
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::write(plan.install_root.join("bin/demo"), b"#!/bin/sh\n").unwrap();

        let sqlite_archive = temp.path().join("sqlite.tar.gz");
        write_test_bottle_archive(
            &sqlite_archive,
            "sqlite",
            "3.49.1",
            &[("bin/sqlite3", b"#!/bin/sh\n")],
        );
        let sqlite_bytes = fs::read(&sqlite_archive).unwrap();
        let sqlite_sha = format!("{:x}", Sha256::digest(&sqlite_bytes));
        let bottle_server =
            start_counting_test_http_server(vec![("/sqlite.tar.gz".to_string(), sqlite_bytes)]);
        let graph = vec![FormulaSpec {
            name: "sqlite".to_string(),
            bottle_sha256: sqlite_sha,
            bottle_url: format!("{}/sqlite.tar.gz", bottle_server.base_url),
        }];

        reinstall_vendor_dependency_tree(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &plan,
            &[],
            &graph,
            &[],
            None,
        )
        .unwrap();

        assert!(plan.install_root.join("bin/sqlite3").is_file());
        assert!(plan.receipt_path("sqlite").is_file());
        assert!(!plan.install_root.join("bin/demo").exists());
    }

    #[test]
    fn remove_package_stubs_preserves_shared_entries() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let python312 = opt_root.join("python@3.12");
        let python313 = opt_root.join("python@3.13");

        fs::create_dir_all(&python312).unwrap();
        fs::create_dir_all(&python313).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();

        write_stub_manifest(
            &python312.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec![
                    "pip".to_string(),
                    "python".to_string(),
                    "python3.12".to_string(),
                ],
            },
        )
        .unwrap();
        write_stub_manifest(
            &python313.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec![
                    "pip".to_string(),
                    "python".to_string(),
                    "python3.13".to_string(),
                ],
            },
        )
        .unwrap();

        for name in ["pip", "python", "python3.12", "python3.13"] {
            write_executable(&bin_dir.join(name));
        }

        remove_package_stubs_from_bin(&opt_root, "python@3.13", &bin_dir).unwrap();

        assert!(bin_dir.join("pip").exists());
        assert!(bin_dir.join("python").exists());
        assert!(fs::symlink_metadata(bin_dir.join("python3.12")).is_ok());
        assert!(fs::symlink_metadata(bin_dir.join("python3.13")).is_err());
    }

    #[test]
    fn remove_existing_package_install_removes_prefix_and_stubs() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let foo = opt_root.join("foo");

        fs::create_dir_all(foo.join("bin")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        write_stub_manifest(
            &foo.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["foo".to_string(), "bar".to_string()],
            },
        )
        .unwrap();

        write_executable(&foo.join("bin/foo"));
        write_executable(&bin_dir.join("foo"));
        write_executable(&bin_dir.join("bar"));

        remove_existing_package_install(&opt_root, "foo", &bin_dir).unwrap();

        assert!(fs::symlink_metadata(&foo).is_err());
        assert!(fs::symlink_metadata(bin_dir.join("foo")).is_err());
        assert!(fs::symlink_metadata(bin_dir.join("bar")).is_err());
    }

    #[test]
    fn remove_existing_scoped_npm_install_removes_empty_scope_dir() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let qmd = opt_root.join("npm/@tobilu/qmd");

        fs::create_dir_all(qmd.join("bin")).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        write_stub_manifest(
            &qmd.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["qmd".to_string()],
            },
        )
        .unwrap();

        write_executable(&qmd.join("bin/qmd"));
        write_executable(&bin_dir.join("qmd"));

        remove_existing_package_install(&opt_root, "npm:@tobilu/qmd", &bin_dir).unwrap();

        assert!(fs::symlink_metadata(&qmd).is_err());
        assert!(fs::symlink_metadata(opt_root.join("npm/@tobilu")).is_err());
        assert!(fs::symlink_metadata(bin_dir.join("qmd")).is_err());
    }

    #[test]
    fn refresh_post_uninstall_stubs_updates_python_dispatchers() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let python312 = opt_root.join("python@3.12");
        let python313 = opt_root.join("python@3.13");

        fs::create_dir_all(&python312).unwrap();
        fs::create_dir_all(&python313).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();

        write_package_receipt(
            &python312.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "python@3.12".to_string(),
                version: "3.12.10".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "python@3.12".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &python313.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "python@3.13".to_string(),
                version: "3.13.3".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "python@3.13".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        write_stub_manifest(
            &python312.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec![
                    "pip".to_string(),
                    "pip3".to_string(),
                    "pip3.12".to_string(),
                    "python".to_string(),
                    "python3".to_string(),
                    "python3.12".to_string(),
                ],
            },
        )
        .unwrap();
        write_stub_manifest(
            &python313.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec![
                    "pip".to_string(),
                    "pip3".to_string(),
                    "pip3.13".to_string(),
                    "python".to_string(),
                    "python3".to_string(),
                    "python3.13".to_string(),
                ],
            },
        )
        .unwrap();

        for name in ["python3.12", "pip3.12", "python3.13", "pip3.13"] {
            write_executable(&bin_dir.join(name));
        }

        post_install_hooks::run("python@3.13", &python313, &bin_dir).unwrap();
        remove_package_stubs_from_bin(&opt_root, "python@3.13", &bin_dir).unwrap();
        remove_path(&python313).unwrap();
        refresh_post_uninstall_stubs(&opt_root, &bin_dir).unwrap();

        assert_eq!(
            fs::read_link(bin_dir.join("python")).unwrap(),
            PathBuf::from("python3")
        );
        assert_eq!(
            fs::read_link(bin_dir.join("python3")).unwrap(),
            PathBuf::from("python3.12")
        );

        assert_eq!(
            fs::read_link(bin_dir.join("pip")).unwrap(),
            PathBuf::from("pip3")
        );
        assert_eq!(
            fs::read_link(bin_dir.join("pip3")).unwrap(),
            PathBuf::from("pip3.12")
        );
        assert!(fs::symlink_metadata(bin_dir.join("python3.13")).is_err());
        assert!(fs::symlink_metadata(bin_dir.join("pip3.13")).is_err());
    }

    #[test]
    fn refresh_post_uninstall_stubs_ignores_python_dependency_receipts() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("usr-local-bin");
        let python312 = opt_root.join("python@3.12");
        let foo = opt_root.join("foo");

        fs::create_dir_all(python312.join(RECEIPTS_DIR)).unwrap();
        fs::create_dir_all(foo.join(RECEIPTS_DIR)).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();

        fs::write(python312.join(RECEIPTS_DIR).join("python@3.12.json"), b"{}").unwrap();
        write_package_receipt(
            &python312.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "python@3.12".to_string(),
                version: "3.12.10".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "python@3.12".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        fs::write(foo.join(RECEIPTS_DIR).join("python@3.12.json"), b"{}").unwrap();
        write_package_receipt(
            &foo.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "foo".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "foo".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        write_stub_manifest(
            &python312.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec![
                    "pip".to_string(),
                    "pip3".to_string(),
                    "pip3.12".to_string(),
                    "python".to_string(),
                    "python3".to_string(),
                    "python3.12".to_string(),
                ],
            },
        )
        .unwrap();

        for name in ["python", "python3", "python3.12", "pip", "pip3", "pip3.12"] {
            write_executable(&bin_dir.join(name));
        }

        remove_package_stubs_from_bin(&opt_root, "python@3.12", &bin_dir).unwrap();
        remove_path(&python312).unwrap();
        refresh_post_uninstall_stubs(&opt_root, &bin_dir).unwrap();

        for name in ["python", "python3", "python3.12", "pip", "pip3", "pip3.12"] {
            assert!(fs::symlink_metadata(bin_dir.join(name)).is_err());
        }
    }

    #[test]
    fn write_stub_double_quotes_entire_path_assignment() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "python@3.12".to_string(),
            root_formula: "python@3.12".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("python"),
            tmp_root: temp.path().join("tmp"),
        };
        let stub_path = temp.path().join("python3");
        let actual_path = PathBuf::from("/opt/python@3.12/bin/python3");
        let env_entries = vec![
            PathBuf::from("/opt/python@3.12/bin"),
            PathBuf::from("/opt/tools/$special/bin"),
        ];

        write_stub(&plan, &stub_path, &actual_path, &env_entries).unwrap();

        let script = fs::read_to_string(&stub_path).unwrap();
        assert!(script.starts_with("#!/bin/sh\n# generated by av python@3.12\n"));
        assert!(script.contains("PATH=\"/opt/python@3.12/bin:/opt/tools/\\$special/bin:$PATH\"\n"));
        assert!(script.contains("exec '/opt/python@3.12/bin/python3' \"$@\"\n"));
    }

    #[test]
    fn write_venv_stub_exports_virtualenv_before_execing_entrypoint() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "pip:psycopg2".to_string(),
            root_formula: "pip:psycopg2".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("psycopg2"),
            tmp_root: temp.path().join("tmp"),
        };
        let stub_path = temp.path().join("psql-tool");
        let venv_root = PathBuf::from("/opt/pip/psycopg2/venv");
        let actual_path = venv_root.join("bin/psql-tool");

        write_venv_stub(&plan, &stub_path, &actual_path, &venv_root).unwrap();

        let script = fs::read_to_string(&stub_path).unwrap();
        assert!(script.contains("VIRTUAL_ENV='/opt/pip/psycopg2/venv'\n"));
        assert!(script.contains("unset PYTHONHOME\n"));
        assert!(script.contains("PATH=\"/opt/pip/psycopg2/venv/bin:$PATH\"\n"));
        assert!(script.contains("exec '/opt/pip/psycopg2/venv/bin/psql-tool' \"$@\"\n"));
    }

    #[test]
    fn write_stub_execs_target_without_fork_bomb_guard() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "python@3.12".to_string(),
            root_formula: "python@3.12".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("python"),
            tmp_root: temp.path().join("tmp"),
        };
        let stub_path = temp.path().join("python3");
        let target_path = temp.path().join("actual-python3");

        fs::write(&target_path, "#!/bin/sh\nprintf 'ok\\n'\n").unwrap();
        let mut permissions = fs::metadata(&target_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target_path, permissions).unwrap();

        write_stub(&plan, &stub_path, &target_path, &[]).unwrap();

        let output = Command::new(&stub_path).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "ok\n");
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
    }

    #[test]
    fn prepare_install_target_requires_force_for_existing_install() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("already-installed");
        fs::create_dir_all(&install_root).unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "already-installed".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "already-installed".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let err = prepare_install_target(
            temp.path(),
            "already-installed",
            InstallIntent::Install,
            temp.path(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "package already-installed is already installed; use --force/-f to reinstall"
        );
        prepare_install_target(
            temp.path(),
            "already-installed",
            InstallIntent::Reinstall,
            temp.path(),
        )
        .unwrap();
        assert!(!install_root.exists());
        prepare_install_target(
            temp.path(),
            "not-installed",
            InstallIntent::Install,
            temp.path(),
        )
        .unwrap();
    }

    #[test]
    fn prepare_install_target_preserves_valid_roots_for_update() {
        let temp = TempDir::new().unwrap();
        let install_root = temp.path().join("already-installed");
        fs::create_dir_all(&install_root).unwrap();
        fs::write(install_root.join("sentinel"), b"keep").unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "already-installed".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "already-installed".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        prepare_install_target(
            temp.path(),
            "already-installed",
            InstallIntent::Update,
            temp.path(),
        )
        .unwrap();

        assert!(install_root.join("sentinel").is_file());
    }

    #[test]
    fn prepare_i_install_plan_skips_seed_for_missing_formula_ownership() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "foo".to_string(),
            root_formula: "foo".to_string(),
            stable_root: temp.path().join("opt/foo"),
            install_root: temp.path().join("opt/foo"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::write(plan.install_root.join("bin/foo"), b"old").unwrap();
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "foo".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "foo".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_receipt(
            &plan.receipt_path("foo"),
            &InstalledFormula {
                spec: FormulaSpec {
                    name: "foo".to_string(),
                    bottle_sha256: "oldsha".to_string(),
                    bottle_url: "https://example.invalid/foo.tar.gz".to_string(),
                },
                keg_dir_name: "1.0.0".to_string(),
                archive_path: PathBuf::new(),
            },
            "arm64_tahoe",
        )
        .unwrap();

        let prepared = prepare_i_install_plan(&plan, InstallIntent::Update).unwrap();

        assert!(!prepared.plan.install_root.join("bin/foo").exists());
    }

    #[test]
    fn incremental_update_and_copy_helpers_cover_seed_edges() {
        let temp = TempDir::new().unwrap();
        let shared_file = temp.path().join("shared-file");
        fs::write(&shared_file, b"not a directory").unwrap();
        assert!(!shared_tmp_root_is_writable(&shared_file.join("child")));

        let missing_receipt = InstallPlan {
            mode: Mode::I,
            package_name: "missing-receipt".to_string(),
            root_formula: "missing-receipt".to_string(),
            stable_root: temp.path().join("opt/missing-receipt"),
            install_root: temp.path().join("opt/missing-receipt"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&missing_receipt.install_root).unwrap();
        assert!(!install_root_supports_incremental_update(&missing_receipt).unwrap());

        let unreadable_receipts = InstallPlan {
            package_name: "bad-receipts".to_string(),
            root_formula: "bad-receipts".to_string(),
            stable_root: temp.path().join("opt/bad-receipts"),
            install_root: temp.path().join("opt/bad-receipts"),
            ..missing_receipt.clone()
        };
        fs::create_dir_all(&unreadable_receipts.install_root).unwrap();
        write_package_receipt(
            &unreadable_receipts.root_receipt_path(),
            &PackageReceipt {
                package_name: unreadable_receipts.package_name.clone(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: unreadable_receipts.root_formula.clone(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        fs::write(unreadable_receipts.install_root.join(RECEIPTS_DIR), b"file").unwrap();
        assert!(
            formula_receipts_support_incremental_update(&unreadable_receipts)
                .unwrap_err()
                .contains("failed to read")
        );

        let invalid_receipts = InstallPlan {
            package_name: "invalid-receipts".to_string(),
            root_formula: "invalid-receipts".to_string(),
            stable_root: temp.path().join("opt/invalid-receipts"),
            install_root: temp.path().join("opt/invalid-receipts"),
            ..missing_receipt.clone()
        };
        fs::create_dir_all(invalid_receipts.install_root.join(RECEIPTS_DIR)).unwrap();
        write_package_receipt(
            &invalid_receipts.root_receipt_path(),
            &PackageReceipt {
                package_name: invalid_receipts.package_name.clone(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: invalid_receipts.root_formula.clone(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        fs::write(
            invalid_receipts
                .install_root
                .join(RECEIPTS_DIR)
                .join("invalid.json"),
            b"{not-json",
        )
        .unwrap();
        assert!(
            formula_receipts_support_incremental_update(&invalid_receipts)
                .unwrap_err()
                .contains("failed to parse")
        );

        let seeded = InstallPlan {
            mode: Mode::I,
            package_name: "seeded".to_string(),
            root_formula: "seeded".to_string(),
            stable_root: temp.path().join("opt/seeded"),
            install_root: temp.path().join("opt/seeded"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(seeded.install_root.join("bin")).unwrap();
        fs::create_dir_all(seeded.install_root.join("share/doc")).unwrap();
        fs::write(seeded.install_root.join("bin/tool"), b"old tool").unwrap();
        fs::write(seeded.install_root.join("share/doc/readme"), b"docs").unwrap();
        symlink("bin/tool", seeded.install_root.join("tool-link")).unwrap();
        fs::create_dir_all(seeded.install_root.join(RECEIPTS_DIR)).unwrap();
        fs::write(
            seeded.install_root.join(RECEIPTS_DIR).join("notes.txt"),
            b"skip",
        )
        .unwrap();
        write_receipt_with_owned_paths(
            &seeded.receipt_path("seeded"),
            &InstalledFormula {
                spec: FormulaSpec {
                    name: "seeded".to_string(),
                    bottle_sha256: "sha".to_string(),
                    bottle_url: "https://example.invalid/seeded.tar.gz".to_string(),
                },
                keg_dir_name: "1.0.0".to_string(),
                archive_path: PathBuf::new(),
            },
            "arm64_tahoe",
            vec!["bin/tool".to_string()],
        )
        .unwrap();
        write_package_receipt(
            &seeded.root_receipt_path(),
            &PackageReceipt {
                package_name: seeded.package_name.clone(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: seeded.root_formula.clone(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let prepared = prepare_i_install_plan(&seeded, InstallIntent::Update).unwrap();
        assert_eq!(
            fs::read(prepared.plan.install_root.join("bin/tool")).unwrap(),
            b"old tool"
        );
        assert_eq!(
            fs::read_link(prepared.plan.install_root.join("tool-link")).unwrap(),
            PathBuf::from("bin/tool")
        );
        assert!(prepared.plan.install_root.join("share/doc/readme").exists());

        let source_file = temp.path().join("source-file");
        let destination_file = temp.path().join("destination-file");
        fs::write(&source_file, b"replacement").unwrap();
        fs::write(&destination_file, b"existing").unwrap();
        let metadata = fs::metadata(&source_file).unwrap();
        copy_file_preserving_metadata(&source_file, &destination_file, &metadata).unwrap();
        assert_eq!(fs::read(&destination_file).unwrap(), b"replacement");
    }

    #[test]
    fn package_install_root_uses_iso_prefix() {
        let temp = TempDir::new().unwrap();

        let install_root = package_install_root(temp.path(), "isotope:gh").unwrap();

        assert_eq!(install_root, temp.path().join("iso/gh"));
    }

    #[test]
    fn package_name_validation_and_normalization_cover_error_paths() {
        assert_eq!(
            validate_npm_package_name(""),
            Err("package qualifier 'npm:' is missing a package name".to_string())
        );
        assert_eq!(
            validate_npm_package_name("@scope"),
            Err("scoped npm package names must be in the form @scope/name".to_string())
        );
        assert_eq!(
            validate_npm_package_name("@scope/name/extra"),
            Err("scoped npm package names must be in the form @scope/name".to_string())
        );
        assert_eq!(
            validate_npm_package_name("foo/bar"),
            Err("npm package names must not contain path separators".to_string())
        );
        assert_eq!(
            parse_npm_package_request("@scope/name@1.2.3").unwrap(),
            ("@scope/name".to_string(), Some("1.2.3".to_string()))
        );
        assert_eq!(
            parse_npm_package_request("openclaw@").unwrap_err(),
            "npm package version must not be empty".to_string()
        );
        assert!(
            parse_npm_package_request("openclaw@nope")
                .unwrap_err()
                .contains("invalid npm package version nope")
        );

        assert_eq!(
            validate_pip_package_name(""),
            Err("package qualifier 'pip:' is missing a package name".to_string())
        );
        assert_eq!(
            validate_pip_package_name("foo/bar"),
            Err("pip package names must not contain path separators".to_string())
        );
        assert_eq!(
            validate_pip_package_name("bad!name"),
            Err(
                "pip package names may only contain ASCII letters, numbers, '.', '-' and '_'"
                    .to_string()
            )
        );
        assert_eq!(
            normalize_pip_package_name("Py_Proj...Tool"),
            "py-proj-tool".to_string()
        );
    }

    #[test]
    fn package_alias_and_embedded_provider_parsing_cover_variants() {
        assert_eq!(
            parse_package_alias_target("brew:").unwrap_err(),
            "package qualifier 'brew:' is missing a formula name".to_string()
        );
        assert_eq!(
            parse_package_alias_target("brew:foo/bar").unwrap_err(),
            "qualified package name must not contain additional path separators".to_string()
        );
        assert_eq!(
            parse_package_alias_target("cask:").unwrap_err(),
            "package qualifier 'cask:' is missing a cask name".to_string()
        );
        assert_eq!(
            parse_package_alias_target("av:terraform").unwrap(),
            PackageAliasTarget::VendorPackage("terraform".to_string())
        );
        assert_eq!(
            parse_package_alias_target("npm:@scope/tool").unwrap(),
            PackageAliasTarget::NpmPackage("@scope/tool".to_string())
        );
        assert_eq!(
            parse_package_alias_target("pip:Py_Proj").unwrap(),
            PackageAliasTarget::PipPackage("py-proj".to_string())
        );
        assert_eq!(
            parse_package_alias_target("tool").unwrap_err(),
            "alias targets must use a package qualifier".to_string()
        );

        assert_eq!(
            parse_embedded_provider("npm:").unwrap_err(),
            "package qualifier 'npm:' is missing a package name".to_string()
        );
        assert_eq!(
            parse_embedded_provider("cask:").unwrap_err(),
            "package qualifier 'cask:' is missing a cask name".to_string()
        );
        assert_eq!(parse_embedded_provider("brew:git").unwrap(), None);
        assert_eq!(
            parse_embedded_provider("ripgrep").unwrap(),
            Some(EmbeddedPackage::Formula("ripgrep".to_string()))
        );
    }

    #[test]
    fn package_install_root_and_formula_recommendations_cover_edge_cases() {
        let temp = TempDir::new().unwrap();

        assert_eq!(
            package_install_root(temp.path(), "npm:@scope/tool").unwrap(),
            temp.path().join("npm/@scope/tool")
        );
        assert_eq!(
            package_install_root(temp.path(), "pip:Py_Proj...Tool").unwrap(),
            temp.path().join("pip/py-proj-tool")
        );
        assert_eq!(
            package_install_root(temp.path(), "isotope:").unwrap_err(),
            "package qualifier 'isotope:' is missing an isotope name".to_string()
        );
        assert_eq!(
            package_install_root(temp.path(), "isotope:foo/bar").unwrap_err(),
            "qualified package name must not contain additional path separators".to_string()
        );
        assert_eq!(
            package_install_root(temp.path(), "npm:@scope").unwrap_err(),
            "scoped npm package names must be in the form @scope/name".to_string()
        );
        assert_eq!(
            package_install_root(temp.path(), "pip:bad/name").unwrap_err(),
            "pip package names must not contain path separators".to_string()
        );

        let mut stderr = Vec::new();
        write_full_formula_recommendation("ffmpeg", &mut stderr).unwrap();
        assert!(String::from_utf8(stderr).unwrap().contains("ffmpeg-full"));

        let mut stderr = Vec::new();
        write_full_formula_recommendation("ripgrep", &mut stderr).unwrap();
        assert!(stderr.is_empty());
    }

    #[test]
    fn prepare_install_target_removes_incomplete_install() {
        let temp = TempDir::new().unwrap();
        let bin_dir = temp.path().join("bin");
        let install_root = package_install_root(temp.path(), "npm:openclaw").unwrap();
        fs::create_dir_all(&install_root).unwrap();
        fs::write(install_root.join("stale"), b"old").unwrap();
        write_stub_manifest(
            &install_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["openclaw".to_string()],
            },
        )
        .unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("openclaw"), b"#!/bin/sh\n").unwrap();

        prepare_install_target(
            temp.path(),
            "npm:openclaw",
            InstallIntent::Install,
            &bin_dir,
        )
        .unwrap();

        assert!(!install_root.exists());
        assert!(!bin_dir.join("openclaw").exists());
    }

    #[test]
    fn rollback_failed_install_removes_partial_root_and_stubs() {
        let temp = TempDir::new().unwrap();
        let bin_dir = temp.path().join("bin");
        let install_root = package_install_root(temp.path(), "npm:openclaw").unwrap();
        fs::create_dir_all(&install_root).unwrap();
        fs::write(install_root.join("stale"), b"old").unwrap();
        write_stub_manifest(
            &install_root.join(STUB_MANIFEST),
            &StubManifest {
                stubs: vec!["openclaw".to_string()],
            },
        )
        .unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("openclaw"), b"#!/bin/sh\n").unwrap();

        rollback_failed_install(temp.path(), "npm:openclaw", &bin_dir).unwrap();

        assert!(!install_root.exists());
        assert!(!bin_dir.join("openclaw").exists());
    }

    #[test]
    fn write_full_formula_recommendation_suggests_full_variants() {
        let mut stderr = Vec::new();
        write_full_formula_recommendation("ffmpeg", &mut stderr).unwrap();
        write_full_formula_recommendation("imagemagick", &mut stderr).unwrap();
        write_full_formula_recommendation("ripgrep", &mut stderr).unwrap();

        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "info: requested `ffmpeg`; `brew:ffmpeg-full` is recommended instead\n\
info: requested `imagemagick`; `brew:imagemagick-full` is recommended instead\n"
        );
    }

    #[test]
    fn prepare_i_install_plan_stages_under_tmp_root() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "caddy".to_string(),
            root_formula: "caddy".to_string(),
            stable_root: temp.path().join("opt/caddy"),
            install_root: temp.path().join("opt/caddy"),
            tmp_root: temp.path().join("opt/.tmp"),
        };

        let prepared = prepare_i_install_plan(&plan, InstallIntent::Install).unwrap();
        let staged_plan = prepared.plan;

        assert_eq!(staged_plan.stable_root, plan.stable_root);
        assert_ne!(staged_plan.install_root, plan.install_root);
        assert!(staged_plan.install_root.starts_with(&plan.tmp_root));
    }

    #[test]
    fn preserve_temp_dir_in_debug_keeps_debug_workspaces() {
        let temp = TempDir::new().unwrap();
        let workspace = TempDir::new_in(temp.path()).unwrap();
        let workspace_path = workspace.path().to_path_buf();

        preserve_temp_dir_in_debug(workspace);

        assert_eq!(workspace_path.exists(), cfg!(debug_assertions));
        if workspace_path.exists() {
            fs::remove_dir_all(&workspace_path).unwrap();
        }
    }

    #[test]
    fn activate_install_replaces_existing_root_with_staged_tree() {
        let temp = TempDir::new().unwrap();
        let stable_root = temp.path().join("opt/caddy");
        let install_root = temp.path().join("opt/.tmp/staged/install");
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "caddy".to_string(),
            root_formula: "caddy".to_string(),
            stable_root: stable_root.clone(),
            install_root: install_root.clone(),
            tmp_root: temp.path().join("opt/.tmp"),
        };

        fs::create_dir_all(stable_root.join("bin")).unwrap();
        fs::write(stable_root.join("bin/caddy"), b"old").unwrap();
        fs::create_dir_all(install_root.join("bin")).unwrap();
        fs::write(install_root.join("bin/caddy"), b"new").unwrap();

        activate_install(&plan).unwrap();

        assert_eq!(fs::read(stable_root.join("bin/caddy")).unwrap(), b"new");
        assert!(!install_root.exists());
    }

    #[test]
    fn temp_root_for_target_root_prefers_shared_tmp_root_on_same_device() {
        let temp = TempDir::new().unwrap();
        let target_root = temp.path().join("opt");
        let system_tmp_root = temp.path().join("tmp");
        let shared_tmp_root = temp.path().join("nucleus");

        assert_eq!(
            temp_root_for_target_root(&target_root, &system_tmp_root, &shared_tmp_root),
            shared_tmp_root
        );
    }

    #[test]
    fn temp_root_for_target_root_falls_back_when_shared_root_is_not_writable() {
        let temp = TempDir::new().unwrap();
        let target_root = temp.path().join("opt");
        let system_tmp_root = temp.path().join("tmp");
        let shared_tmp_root = temp.path().join("nucleus");
        fs::create_dir_all(&shared_tmp_root).unwrap();
        let mut permissions = fs::metadata(&shared_tmp_root).unwrap().permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&shared_tmp_root, permissions).unwrap();

        assert_eq!(
            temp_root_for_target_root(&target_root, &system_tmp_root, &shared_tmp_root),
            target_root.join(".tmp")
        );
    }

    #[test]
    fn temp_root_for_target_root_falls_back_when_target_root_has_no_existing_ancestor() {
        let temp = TempDir::new().unwrap();
        let target_root = PathBuf::from("relative/opt");
        let system_tmp_root = temp.path().join("tmp");
        let shared_tmp_root = temp.path().join("nucleus");

        assert_eq!(
            temp_root_for_target_root(&target_root, &system_tmp_root, &shared_tmp_root),
            target_root.join(".tmp")
        );
    }

    #[test]
    fn install_plan_for_i_uses_detected_tmp_root() {
        let plan = InstallPlan::for_i("caddy".to_string(), "caddy".to_string());

        assert_eq!(plan.stable_root, opt_pkg_root().join("caddy"));
        assert_eq!(plan.install_root, opt_pkg_root().join("caddy"));
        assert_eq!(
            plan.tmp_root,
            temp_root_for_target_root(
                &opt_pkg_root(),
                Path::new(SYSTEM_TMP_ROOT),
                Path::new(TMP_TOOL_ROOT),
            )
        );
    }

    #[test]
    fn debug_build_uses_tmp_install_roots() {
        assert_eq!(opt_pkg_root(), PathBuf::from("/tmp/opt"));
        assert_eq!(managed_bin_root(), PathBuf::from("/tmp/usr/local/bin"));
        assert!(!install_requires_root());
    }

    #[test]
    fn install_plan_for_i_npm_uses_dedicated_opt_root() {
        let plan = InstallPlan::for_i_npm(
            "npm:openclaw".to_string(),
            "npm:openclaw".to_string(),
            "openclaw",
        );

        assert_eq!(plan.stable_root, opt_npm_root().join("openclaw"));
        assert_eq!(plan.install_root, opt_npm_root().join("openclaw"));
        assert_eq!(
            plan.tmp_root,
            temp_root_for_target_root(
                &opt_npm_root(),
                Path::new(SYSTEM_TMP_ROOT),
                Path::new(TMP_TOOL_ROOT),
            )
        );
    }

    #[test]
    fn install_plan_for_i_scoped_npm_preserves_scope_in_opt_root() {
        let plan = InstallPlan::for_i_npm(
            "npm:@tobilu/qmd".to_string(),
            "npm:@tobilu/qmd".to_string(),
            "@tobilu/qmd",
        );

        assert_eq!(plan.stable_root, opt_npm_root().join("@tobilu/qmd"));
        assert_eq!(plan.install_root, opt_npm_root().join("@tobilu/qmd"));
    }

    #[test]
    fn install_plan_for_i_pip_uses_dedicated_opt_root() {
        let plan = InstallPlan::for_i_pip(
            "pip:psycopg2".to_string(),
            "pip:psycopg2".to_string(),
            "psycopg2",
        );

        assert_eq!(plan.stable_root, opt_pip_root().join("psycopg2"));
        assert_eq!(plan.install_root, opt_pip_root().join("psycopg2"));
        assert_eq!(
            plan.tmp_root,
            temp_root_for_target_root(
                &opt_pip_root(),
                Path::new(SYSTEM_TMP_ROOT),
                Path::new(TMP_TOOL_ROOT),
            )
        );
    }

    #[test]
    fn install_plan_for_i_radioisotope_uses_formula_root() {
        let plan =
            InstallPlan::for_i_radioisotope("isotope:aws-cli".to_string(), "awscli".to_string());

        assert_eq!(plan.package_name, "isotope:aws-cli");
        assert_eq!(plan.root_formula, "awscli");
        assert_eq!(plan.stable_root, opt_pkg_root().join("awscli"));
        assert_eq!(plan.install_root, opt_pkg_root().join("awscli"));
        assert_eq!(
            plan.tmp_root,
            temp_root_for_target_root(
                &opt_pkg_root(),
                Path::new(SYSTEM_TMP_ROOT),
                Path::new(TMP_TOOL_ROOT),
            )
        );
    }

    #[test]
    fn install_plan_paths_cover_dependency_layout_and_receipts() {
        let plan = InstallPlan::for_i("rg".to_string(), "rg".to_string());

        assert_eq!(plan.actual_target_dir("rg"), opt_pkg_root().join("rg"));
        assert_eq!(plan.actual_target_dir("pcre2"), opt_pkg_root().join("rg"));
        assert_eq!(plan.stable_target_dir("rg"), opt_pkg_root().join("rg"));
        assert_eq!(plan.stable_target_dir("pcre2"), opt_pkg_root().join("rg"));
        assert_eq!(
            plan.receipt_path("rg"),
            opt_pkg_root().join("rg/.pkg/receipts/rg.json")
        );
        assert_eq!(
            plan.receipt_path("pcre2"),
            opt_pkg_root().join("rg/.pkg/receipts/pcre2.json")
        );
        assert_eq!(
            plan.package_manifest_path(),
            opt_pkg_root().join("rg/.pkg/stubs.json")
        );
        assert_eq!(
            plan.root_receipt_path(),
            opt_pkg_root().join("rg/.pkg/root-receipt.json")
        );
        assert_eq!(
            plan.root_executables_manifest_path(),
            opt_pkg_root().join("rg/.pkg/root-executables.json")
        );
    }

    #[test]
    fn metadata_probe_path_and_device_helpers_use_existing_ancestors() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("a/b/c");

        assert_eq!(metadata_probe_path(&nested).unwrap(), temp.path());
        assert!(paths_share_device(temp.path(), &nested).unwrap());
    }

    #[test]
    fn acquire_package_mutation_lock_uses_flock() {
        let temp = TempDir::new().unwrap();
        let lock = acquire_package_mutation_lock_at(temp.path()).unwrap();
        let path = temp.path().join(PKG_STATE_LOCK);
        let second = File::options().read(true).write(true).open(&path).unwrap();

        let result = unsafe { libc::flock(second.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(result, -1);
        let err = std::io::Error::last_os_error().raw_os_error().unwrap();
        assert!(err == libc::EWOULDBLOCK || err == libc::EAGAIN);

        drop(lock);

        let result = unsafe { libc::flock(second.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(result, 0);
        unsafe {
            libc::flock(second.as_raw_fd(), libc::LOCK_UN);
        }
    }

    #[test]
    fn load_db_and_schema_checks_embedded_inventory() {
        let db = load_db().unwrap();
        ensure_db_schema(&db).unwrap();
        assert_eq!(db.schema, DB_SCHEMA_VERSION);

        let old = Db {
            schema: DB_SCHEMA_VERSION - 1,
            generated_at: String::new(),
            entries: HashMap::new(),
            formulas: HashMap::new(),
            casks: HashMap::new(),
            npms: HashMap::new(),
        };
        ensure_db_schema(&old).unwrap();

        let future = Db {
            schema: DB_SCHEMA_VERSION + 1,
            generated_at: String::new(),
            entries: HashMap::new(),
            formulas: HashMap::new(),
            casks: HashMap::new(),
            npms: HashMap::new(),
        };
        assert_eq!(
            ensure_db_schema(&future).unwrap_err(),
            format!(
                "unsupported db schema {} (maximum supported {})",
                DB_SCHEMA_VERSION + 1,
                DB_SCHEMA_VERSION
            )
        );
    }

    #[test]
    fn embedded_coverage_fixture_carries_test_contract_data() {
        let data = embedded_combined_data();
        let db = &data.sources.db;

        assert_eq!(data.generated_at, "2026-05-05T00:00:00Z");
        assert_eq!(db.schema, DB_SCHEMA_VERSION);
        assert_eq!(
            db.formulas
                .get("ripgrep")
                .expect("coverage fixture should include ripgrep")
                .aliases,
            vec!["rg".to_string()]
        );
        assert_eq!(
            db.formulas
                .get("node")
                .expect("coverage fixture should include node")
                .aliases,
            vec!["node@25".to_string()]
        );
        assert_eq!(
            db.casks
                .get("codex")
                .expect("coverage fixture should include codex cask")
                .version,
            "1.0.0"
        );
        assert_eq!(
            data.sources
                .isotopes
                .get("gh")
                .expect("coverage fixture should include gh isotope")
                .replaces
                .as_deref(),
            Some("brew:gh")
        );
        assert_eq!(
            data.sources
                .pip
                .get("coverage-pip")
                .expect("coverage fixture should include coverage pip package")
                .python_formula
                .as_deref(),
            Some("python@3.14")
        );
    }

    #[test]
    fn trusted_remote_combined_data_loads_readable_root_cache_shape() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db.json");
        fs::write(&path, EMBEDDED_COMBINED_DATA).unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        let data = load_trusted_remote_combined_data_from(temp.path(), &path, false).unwrap();

        assert_eq!(data.sources.db.schema, DB_SCHEMA_VERSION);
        assert!(data.sources.isotopes.contains_key("aws-cli"));
    }

    #[test]
    fn combined_data_freshness_rejects_older_remote_cache() {
        let embedded = test_combined_data_with_generated_at("2026-05-17T13:12:55Z");
        let older_remote = test_combined_data_with_generated_at("2026-05-17T12:42:37Z");
        let same_remote = test_combined_data_with_generated_at("2026-05-17T13:12:55Z");
        let newer_remote = test_combined_data_with_generated_at("2026-05-17T13:12:56Z");
        let invalid_remote = test_combined_data_with_generated_at("not-rfc3339");
        let invalid_embedded = test_combined_data_with_generated_at("not-rfc3339");

        assert!(!combined_data_is_at_least_as_new(&older_remote, &embedded));
        assert!(combined_data_is_at_least_as_new(&same_remote, &embedded));
        assert!(combined_data_is_at_least_as_new(&newer_remote, &embedded));
        assert!(!combined_data_is_at_least_as_new(
            &invalid_remote,
            &embedded
        ));
        assert!(combined_data_is_at_least_as_new(
            &same_remote,
            &invalid_embedded
        ));
    }

    #[test]
    fn trusted_remote_combined_data_rejects_world_writable_cache_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db.json");
        fs::write(&path, EMBEDDED_COMBINED_DATA).unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();

        assert!(load_trusted_remote_combined_data_from(temp.path(), &path, false).is_none());
    }

    #[test]
    fn trusted_remote_combined_data_rejects_future_schema_cache_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db.json");
        fs::write(
            &path,
            test_combined_data_json_with_db_schema(DB_SCHEMA_VERSION + 1),
        )
        .unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(load_trusted_remote_combined_data_from(temp.path(), &path, false).is_none());
    }

    #[test]
    fn refresh_remote_combined_data_uses_etags_and_validates_json() {
        let temp = TempDir::new().unwrap();
        let data_path = temp.path().join("db.json");
        let meta_path = temp.path().join("db.meta.json");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (base, server) = start_test_etag_server(requests.clone(), test_combined_data_json());
        let url = format!("{base}/db.json");

        assert!(
            refresh_remote_combined_data_with(&url, temp.path(), &data_path, &meta_path, 0)
                .unwrap()
        );
        assert!(
            !refresh_remote_combined_data_with(&url, temp.path(), &data_path, &meta_path, 0)
                .unwrap()
        );

        server.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("accept-encoding: gzip, br")
        );
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("accept-encoding: gzip, br")
        );
        assert!(!requests[0].contains("If-None-Match"));
        assert!(requests[1].contains("If-None-Match: \"test-etag\""));
        let metadata = read_remote_combined_data_metadata(&meta_path);
        assert_eq!(metadata.etag.as_deref(), Some("\"test-etag\""));
    }

    #[test]
    fn refresh_remote_combined_data_rejects_future_schema_without_replacing_cache() {
        let temp = TempDir::new().unwrap();
        let data_path = temp.path().join("db.json");
        let meta_path = temp.path().join("db.meta.json");
        let cached_data = test_combined_data_json();
        fs::write(&data_path, &cached_data).unwrap();
        let (base, server) = start_test_http_server(
            vec![(
                "/db.json".to_string(),
                test_combined_data_json_with_db_schema(DB_SCHEMA_VERSION + 1),
            )],
            1,
        );
        let url = format!("{base}/db.json");

        let err = refresh_remote_combined_data_with(&url, temp.path(), &data_path, &meta_path, 0)
            .unwrap_err();

        server.join().unwrap();
        assert!(err.contains("unsupported remote database"));
        assert!(err.contains("unsupported db schema"));
        assert_eq!(fs::read(&data_path).unwrap(), cached_data);
        assert!(!meta_path.exists());
    }

    #[test]
    fn refresh_remote_combined_data_skips_recent_check_and_invalid_metadata_defaults() {
        let temp = TempDir::new().unwrap();
        let data_path = temp.path().join("db.json");
        let meta_path = temp.path().join("db.meta.json");

        fs::write(&meta_path, b"not-json").unwrap();
        let parsed = read_remote_combined_data_metadata(&meta_path);
        assert!(parsed.etag.is_none());
        assert!(parsed.checked_at.is_none());

        let metadata = RemoteCombinedDataMetadata {
            etag: Some("\"cached-etag\"".to_string()),
            checked_at: Some(current_unix_timestamp().unwrap()),
        };
        fs::write(&meta_path, serde_json::to_vec(&metadata).unwrap()).unwrap();

        let refreshed = refresh_remote_combined_data_with(
            "http://127.0.0.1:9/db.json",
            temp.path(),
            &data_path,
            &meta_path,
            u64::MAX,
        )
        .unwrap();

        assert!(!refreshed);
        let parsed = read_remote_combined_data_metadata(&meta_path);
        assert_eq!(parsed.etag, metadata.etag);
        assert_eq!(parsed.checked_at, metadata.checked_at);
        assert!(!data_path.exists());
    }

    #[test]
    fn trusted_remote_data_helpers_reject_bad_shapes_and_permissions() {
        let temp = TempDir::new().unwrap();
        let dir_file = temp.path().join("not-a-dir");
        let data_path = temp.path().join("db.json");
        fs::write(&dir_file, b"file").unwrap();
        fs::write(&data_path, EMBEDDED_COMBINED_DATA).unwrap();
        fs::set_permissions(&data_path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(!trusted_remote_data_path(&dir_file, &data_path, false));
        assert!(!trusted_remote_data_path(
            temp.path(),
            &temp.path().join("missing.json"),
            false
        ));

        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o777)).unwrap();
        assert!(!trusted_remote_data_path(temp.path(), &data_path, false));

        let metadata = fs::metadata(&data_path).unwrap();
        assert!(trusted_remote_data_metadata(&metadata, false));
        assert!(!trusted_remote_data_metadata(&metadata, true));
    }

    #[test]
    fn remote_combined_data_writers_persist_cache_and_metadata() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let data_path = cache_dir.join("db.json");
        let meta_path = cache_dir.join("db.meta.json");
        let bytes = test_combined_data_json();
        let metadata = RemoteCombinedDataMetadata {
            etag: Some("\"next-etag\"".to_string()),
            checked_at: Some(current_unix_timestamp().unwrap()),
        };

        write_remote_combined_data(&cache_dir, &data_path, &bytes).unwrap();
        write_remote_combined_data_metadata(&cache_dir, &meta_path, &metadata).unwrap();

        assert_eq!(fs::read(&data_path).unwrap(), bytes);
        let parsed = read_remote_combined_data_metadata(&meta_path);
        assert_eq!(parsed.etag, metadata.etag);
        assert_eq!(parsed.checked_at, metadata.checked_at);
        assert_eq!(
            fs::metadata(&cache_dir).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&data_path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(
            fs::metadata(&meta_path).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert!(current_unix_timestamp().unwrap() > 0);
    }

    #[test]
    fn help_and_version_parse_paths_return_none() {
        let invocation = Invocation {
            binary_name: "av".to_string(),
            name: "av update".to_string(),
            mode: None,
        };

        assert_eq!(
            parse_i_request_from_iter(&invocation, vec![OsString::from("-h")].into_iter()).unwrap(),
            None
        );
        assert_eq!(
            parse_uninstall_request_from_iter(
                &invocation,
                vec![OsString::from("--help")].into_iter()
            )
            .unwrap(),
            None
        );
        assert_eq!(
            parse_update_request_from_iter(&invocation, vec![OsString::from("-V")].into_iter())
                .unwrap(),
            None
        );
        assert_eq!(
            parse_package_status_request_from_iter(
                &invocation,
                vec![OsString::from("--help")].into_iter(),
                print_list_usage,
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn flag_and_subcommand_helpers_accept_supported_aliases() {
        assert!(is_help_flag(&OsString::from("-h")));
        assert!(is_help_flag(&OsString::from("--help")));
        assert!(is_version_flag(&OsString::from("-V")));
        assert!(is_version_flag(&OsString::from("--version")));
        assert!(is_force_flag(&OsString::from("-f")));
        assert!(is_force_flag(&OsString::from("--force")));
        assert!(is_no_self_update_flag(&OsString::from(
            SELF_UPDATE_DISABLE_FLAG
        )));
        assert!(is_uninstall_subcommand("rm"));
        assert!(is_uninstall_subcommand("uninstall"));
        assert!(is_outdated_subcommand("outdated"));
        assert!(!is_outdated_subcommand("list"));
    }

    #[test]
    fn package_receipts_and_stub_manifests_round_trip() {
        let temp = TempDir::new().unwrap();
        let receipt_path = temp.path().join("pkg/root.json");
        let stub_manifest_path = temp.path().join("pkg/stubs.json");
        let root_manifest_path = temp.path().join("pkg/root-executables.json");
        let receipt = PackageReceipt {
            package_name: "deno".to_string(),
            version: "2.7.7".to_string(),
            source: PackageReceiptSource::Vendor {
                vendor_name: "deno".to_string(),
            },
            metadata: PackageMetadata::default(),
        };

        assert!(load_package_receipt(&receipt_path).unwrap().is_none());
        write_package_receipt(&receipt_path, &receipt).unwrap();
        assert_eq!(load_package_receipt(&receipt_path).unwrap(), Some(receipt));

        assert_eq!(
            load_stub_manifest(&stub_manifest_path).unwrap(),
            StubManifest { stubs: Vec::new() }
        );
        write_stub_manifest(
            &stub_manifest_path,
            &StubManifest {
                stubs: vec!["deno".to_string()],
            },
        )
        .unwrap();
        assert_eq!(
            load_stub_manifest(&stub_manifest_path).unwrap(),
            StubManifest {
                stubs: vec!["deno".to_string()],
            }
        );

        write_root_executable_manifest(&root_manifest_path, &["deno".to_string()]).unwrap();
        assert_eq!(
            load_root_executable_manifest(&root_manifest_path).unwrap(),
            StubManifest {
                stubs: vec!["deno".to_string()],
            }
        );
    }

    #[test]
    fn package_receipts_without_metadata_remain_readable() {
        let temp = TempDir::new().unwrap();
        let receipt_path = temp.path().join("pkg/root.json");
        fs::create_dir_all(receipt_path.parent().unwrap()).unwrap();
        fs::write(
            &receipt_path,
            br#"{
                "package_name": "ripgrep",
                "version": "14.1.1",
                "source": {
                    "kind": "formula",
                    "root_formula": "ripgrep"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            load_package_receipt(&receipt_path).unwrap(),
            Some(PackageReceipt {
                package_name: "ripgrep".to_string(),
                version: "14.1.1".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "ripgrep".to_string(),
                },
                metadata: PackageMetadata::default(),
            })
        );
    }

    #[test]
    fn package_status_helpers_cover_current_and_missing_cases() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "sqlite".to_string(),
            root_formula: "sqlite".to_string(),
            stable_root: temp.path().join("opt/sqlite"),
            install_root: temp.path().join("opt/sqlite"),
            tmp_root: temp.path().join("tmp"),
        };
        let install = InstalledFormula {
            spec: FormulaSpec {
                name: "sqlite".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/sqlite.tar.gz".to_string(),
            },
            keg_dir_name: "3.49.1".to_string(),
            archive_path: temp.path().join("sqlite.tar.gz"),
        };

        assert!(!receipt_is_current(&plan, &install, "arm64_tahoe").unwrap());
        assert!(!package_is_current(&plan, std::slice::from_ref(&install), "arm64_tahoe").unwrap());

        write_receipt(&plan.receipt_path("sqlite"), &install, "arm64_tahoe").unwrap();
        fs::create_dir_all(&plan.install_root).unwrap();
        assert!(receipt_is_current(&plan, &install, "arm64_tahoe").unwrap());
        assert!(package_is_current(&plan, &[install], "arm64_tahoe").unwrap());
    }

    #[test]
    fn install_dependency_formulas_with_empty_graph_prepares_vendor_root() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "npm-openclaw".to_string(),
            root_formula: "npm-openclaw".to_string(),
            stable_root: temp.path().join("opt/npm-openclaw"),
            install_root: temp.path().join("opt/npm-openclaw"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(&plan.install_root).unwrap();
        fs::write(plan.install_root.join("stale"), b"old").unwrap();

        install_dependency_formulas(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &plan,
            &[],
            &[],
            None,
        )
        .unwrap();

        assert!(plan.install_root.is_dir());
        assert!(!plan.install_root.join("stale").exists());
    }

    #[test]
    fn dependency_current_checks_cover_empty_and_vendor_roots() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "codex".to_string(),
            root_formula: "codex".to_string(),
            stable_root: temp.path().join("opt/codex"),
            install_root: temp.path().join("opt/codex"),
            tmp_root: temp.path().join("tmp"),
        };
        let config = Config {
            bottle_tag: "arm64_tahoe".to_string(),
        };

        assert!(!dependencies_are_current(&plan, &[], &[], &config).unwrap());
        fs::create_dir_all(&plan.install_root).unwrap();
        assert!(dependencies_are_current(&plan, &[], &[], &config).unwrap());

        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        write_executable(&plan.install_root.join("bin/codex"));
        let vendor_install = fake_vendor_install("codex", &["codex"], "0.1.0");
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "codex".to_string(),
                version: "0.1.0".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: "codex".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(vendor_root_is_current(&plan, &vendor_install, &[], &config.bottle_tag).unwrap());

        remove_path(&plan.install_root.join("bin/codex")).unwrap();
        assert!(!vendor_root_is_current(&plan, &vendor_install, &[], &config.bottle_tag).unwrap());
    }

    #[test]
    fn dependency_current_checks_cover_npm_pip_cask_and_isotope_roots() {
        let temp = TempDir::new().unwrap();
        let config = Config {
            bottle_tag: "arm64_tahoe".to_string(),
        };

        let npm_plan = InstallPlan {
            mode: Mode::I,
            package_name: "npm:coverage-npm".to_string(),
            root_formula: "coverage-npm".to_string(),
            stable_root: temp.path().join("opt/npm/coverage-npm"),
            install_root: temp.path().join("opt/npm/coverage-npm"),
            tmp_root: temp.path().join("tmp"),
        };
        assert!(
            !npm_root_is_current(
                &npm_plan,
                "coverage-npm",
                &Version::parse("1.2.3").unwrap(),
                &[],
                &config.bottle_tag
            )
            .unwrap()
        );
        fs::create_dir_all(npm_plan.install_root.join("bin")).unwrap();
        write_executable(&npm_plan.install_root.join("bin/coverage-npm"));
        write_package_receipt(
            &npm_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "npm:coverage-npm".to_string(),
                version: "1.2.3".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "coverage-npm".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(
            npm_root_is_current(
                &npm_plan,
                "coverage-npm",
                &Version::parse("1.2.3").unwrap(),
                &[],
                &config.bottle_tag,
            )
            .unwrap()
        );
        remove_path(&npm_plan.install_root.join("bin/coverage-npm")).unwrap();
        assert!(
            !npm_root_is_current(
                &npm_plan,
                "coverage-npm",
                &Version::parse("1.2.3").unwrap(),
                &[],
                &config.bottle_tag,
            )
            .unwrap()
        );

        let pip_plan = InstallPlan {
            mode: Mode::I,
            package_name: "pip:coverage-pip".to_string(),
            root_formula: "coverage-pip".to_string(),
            stable_root: temp.path().join("opt/pip/coverage-pip"),
            install_root: temp.path().join("opt/pip/coverage-pip"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(pip_plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(pip_plan.install_root.join("venv")).unwrap();
        fs::write(pip_plan.install_root.join("venv/pyvenv.cfg"), b"").unwrap();
        write_executable(&pip_plan.install_root.join("bin/coverage-pip"));
        write_root_executable_manifest(
            &pip_plan.root_executables_manifest_path(),
            &["coverage-pip".to_string()],
        )
        .unwrap();
        write_package_receipt(
            &pip_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "pip:coverage-pip".to_string(),
                version: "2.3.4".to_string(),
                source: PackageReceiptSource::Pip {
                    package_name: "coverage-pip".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(pip_root_is_current(&pip_plan, "2.3.4", &[], &config.bottle_tag).unwrap());
        remove_path(&pip_plan.install_root.join("venv/pyvenv.cfg")).unwrap();
        assert!(!pip_root_is_current(&pip_plan, "2.3.4", &[], &config.bottle_tag).unwrap());

        let cask_plan = InstallPlan {
            mode: Mode::I,
            package_name: "cask:codex".to_string(),
            root_formula: "codex".to_string(),
            stable_root: temp.path().join("opt/cask/codex"),
            install_root: temp.path().join("opt/cask/codex"),
            tmp_root: temp.path().join("tmp"),
        };
        let cask = EmbeddedCaskMetadata {
            version: "1.0.0".to_string(),
            binaries: vec![EmbeddedCaskBinary {
                source: "Codex.app/Contents/MacOS/codex".to_string(),
                target: Some("codex".to_string()),
            }],
            ..Default::default()
        };
        fs::create_dir_all(cask_plan.install_root.join("bin")).unwrap();
        write_executable(&cask_plan.install_root.join("bin/codex"));
        write_package_receipt(
            &cask_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "cask:codex".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Cask {
                    cask_name: "codex".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(cask_root_is_current(&cask_plan, &cask, &[], &config.bottle_tag).unwrap());
        remove_path(&cask_plan.install_root.join("bin/codex")).unwrap();
        assert!(!cask_root_is_current(&cask_plan, &cask, &[], &config.bottle_tag).unwrap());

        let isotope_plan = InstallPlan {
            mode: Mode::I,
            package_name: "isotope:gh".to_string(),
            root_formula: "gh".to_string(),
            stable_root: temp.path().join("opt/iso/gh"),
            install_root: temp.path().join("opt/iso/gh"),
            tmp_root: temp.path().join("tmp"),
        };
        let isotope = IsotopePackageData {
            name: "isotope:gh".to_string(),
            replaces: Some("brew:gh".to_string()),
            modifies: None,
            migrate: None,
            _repository: None,
            _upstream_repository: None,
            version: "2.80.0".to_string(),
            release_url: Some("https://example.test/isotopes/gh".to_string()),
            archive_url: Some("https://example.test/isotopes/gh.tar.gz".to_string()),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        fs::create_dir_all(isotope_plan.install_root.join("bin")).unwrap();
        write_executable(&isotope_plan.install_root.join("bin/gh"));
        write_root_executable_manifest(
            &isotope_plan.root_executables_manifest_path(),
            &["gh".to_string()],
        )
        .unwrap();
        write_package_receipt(
            &isotope_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "isotope:gh".to_string(),
                version: "2.80.0".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "gh".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(isotope_root_is_current(&isotope_plan, &isotope).unwrap());
        remove_path(&isotope_plan.install_root.join("bin/gh")).unwrap();
        assert!(!isotope_root_is_current(&isotope_plan, &isotope).unwrap());
    }

    #[test]
    fn find_supported_post_install_prefixes_filters_supported_formula_receipts() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let python = opt_root.join("python@3.12");
        let openssl = opt_root.join("openssl@3");
        let deno = opt_root.join("deno");
        fs::create_dir_all(&python).unwrap();
        fs::create_dir_all(&openssl).unwrap();
        fs::create_dir_all(&deno).unwrap();

        write_package_receipt(
            &python.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "python@3.12".to_string(),
                version: "3.12.10".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "python@3.12".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &openssl.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "openssl@3".to_string(),
                version: "3.6.1".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "openssl@3".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_package_receipt(
            &deno.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: "deno".to_string(),
                version: "2.7.7".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: "deno".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let mut prefixes = find_supported_post_install_prefixes(&opt_root).unwrap();
        prefixes.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            prefixes,
            vec![
                ("openssl@3".to_string(), openssl),
                ("python@3.12".to_string(), python),
            ]
        );
        assert_eq!(installed_post_install_formula(&deno).unwrap(), None);
    }

    #[test]
    fn post_install_helpers_cover_python_and_openssl_branches() {
        let temp = TempDir::new().unwrap();
        let opt_root = temp.path().join("opt");
        let bin_dir = temp.path().join("bin");
        let python312 = opt_root.join("python@3.12");
        let python313 = opt_root.join("python@3.13");
        fs::create_dir_all(&python312).unwrap();
        fs::create_dir_all(&python313).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        write_executable(&bin_dir.join("python3.12"));
        write_executable(&bin_dir.join("pip3.12"));
        fs::write(bin_dir.join("python3"), b"old").unwrap();
        assert!(post_install_hooks::supports("python@3.12"));
        assert!(!post_install_hooks::supports("python@3.12.1"));
        let outcome = post_install_hooks::run("python@3.12", &python312, &bin_dir).unwrap();
        assert_eq!(
            outcome.managed_stubs,
            vec![
                "pip".to_string(),
                "pip3".to_string(),
                "python".to_string(),
                "python3".to_string(),
            ]
        );
        assert_eq!(
            fs::read_link(bin_dir.join("python3")).unwrap(),
            PathBuf::from("python3.12")
        );

        let openssl_prefix = temp.path().join("openssl");
        let source_dir = openssl_prefix.join(OPENSSL_CA_CERTIFICATES_DIR);
        let target_dir = openssl_prefix.join(OPENSSL_CERT_PEM_DESTINATION_DIR);
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(source_dir.join("cacert.pem"), b"source").unwrap();
        fs::write(source_dir.join("extra.pem"), b"extra").unwrap();
        fs::write(target_dir.join("cert.pem"), b"old").unwrap();

        assert!(post_install_hooks::supports_dependency("openssl@3"));
        post_install_hooks::run("openssl@3", &openssl_prefix, &bin_dir).unwrap();

        assert_eq!(
            fs::read_to_string(target_dir.join("cert.pem")).unwrap(),
            "source"
        );
        assert_eq!(
            fs::read_to_string(target_dir.join("extra.pem")).unwrap(),
            "extra"
        );
        assert!(!source_dir.exists());
    }

    #[test]
    fn vendor_registry_helpers_cover_install_strategies_and_parse_errors() {
        assert!(get("missing").is_none());
        assert_eq!(
            github_release_url("foo/bar", "v1.2.3", "tool.tar.gz"),
            "https://github.com/foo/bar/releases/download/v1.2.3/tool.tar.gz"
        );
        assert!(parse_semver("nope", "test").is_err());

        match bun::install(&Version::parse("1.2.3").unwrap()) {
            vendor::InstallStrategy::CopyFile {
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
            _ => panic!("bun should install a single binary"),
        }
    }

    #[test]
    fn formula_api_helpers_resolve_aliases_and_specs_from_fixture_server() {
        let _env_lock = test_env_lock().lock().unwrap();
        let (formula_alias, formula_name) = formula_index_entries()
            .unwrap()
            .iter()
            .find_map(|entry| {
                entry
                    .aliases
                    .first()
                    .cloned()
                    .map(|alias| (alias, entry.name.clone()))
            })
            .expect("embedded db should carry at least one formula alias");
        let (base, _server) = start_test_http_server(
            vec![
                (
                    "/formula.json".to_string(),
                    serde_json::to_vec(&vec![
                        serde_json::json!({
                            "name": formula_name,
                            "aliases": [formula_alias],
                            "oldnames": ["python3.12"],
                        }),
                        serde_json::json!({
                            "name": "openssl@3",
                            "aliases": [],
                            "oldnames": [],
                        }),
                    ])
                    .unwrap(),
                ),
                (
                    format!("/{formula_name}.json"),
                    serde_json::to_vec(&serde_json::json!({
                        "versions": {"stable": "3.12.10"},
                        "revision": 1,
                        "dependencies": ["openssl@3"],
                        "bottle": {
                            "stable": {
                                "files": {
                                    "arm64_tahoe": {
                                        "sha256": "python-sha",
                                        "url": "https://example.invalid/python.tar.gz"
                                    }
                                }
                            }
                        },
                        "disabled": false,
                        "post_install_defined": false
                    }))
                    .unwrap(),
                ),
                (
                    "/openssl@3.json".to_string(),
                    serde_json::to_vec(&serde_json::json!({
                        "versions": {"stable": "3.6.1"},
                        "revision": 0,
                        "dependencies": [],
                        "bottle": {
                            "stable": {
                                "files": {
                                    "arm64_tahoe": {
                                        "sha256": "openssl-sha",
                                        "url": "https://example.invalid/openssl.tar.gz"
                                    }
                                }
                            }
                        },
                        "disabled": false,
                        "post_install_defined": true
                    }))
                    .unwrap(),
                ),
            ],
            20,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(base.clone()),
            ..Default::default()
        });

        assert_eq!(
            canonical_formula_name(&formula_alias).unwrap(),
            formula_name
        );
        assert!(formula_metadata_exists(&formula_alias).unwrap());
        let fetched_info = fetch_formula_info(&formula_alias).unwrap();
        assert_eq!(formula_version_string(&fetched_info), "3.12.10_1");
        assert_eq!(
            resolve_formula_latest_version(
                &Config {
                    bottle_tag: "arm64_tahoe".to_string(),
                },
                &formula_alias,
            )
            .unwrap(),
            "3.12.10_1"
        );
        let specs = resolve_formula_specs(
            std::slice::from_ref(&formula_alias),
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            true,
        )
        .unwrap();
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.name.as_str())
                .collect::<Vec<_>>(),
            vec!["openssl@3", formula_name.as_str()]
        );
    }

    #[test]
    fn resolve_package_search_results_matches_formula_names_and_aliases() {
        let _env_lock = test_env_lock().lock().unwrap();
        let formula_index = formula_index_entries().unwrap();
        let rg_formula = formula_alias_index()
            .unwrap()
            .get("rg")
            .cloned()
            .expect("embedded db should carry the rg alias");
        let rg_summary = formula_index
            .iter()
            .find(|entry| entry.name == rg_formula)
            .and_then(|entry| string_or_none(&entry.summary))
            .expect("embedded db should carry rg summary");

        let results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            "rg",
        )
        .unwrap();
        assert!(results.iter().any(|result| {
            result.package_name == rg_formula
                && result.source
                    == PackageReceiptSource::Formula {
                        root_formula: rg_formula.clone(),
                    }
                && result.summary == Some(rg_summary.clone())
                && result.latest_version.is_none()
                && result.homepage.is_none()
                && result.dependencies.is_empty()
        }));

        let alias_results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            "rg",
        )
        .unwrap();
        assert!(alias_results.iter().any(|result| {
            result.package_name == rg_formula && result.summary == Some(rg_summary.clone())
        }));

        let vendor_results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            "av:bun",
        )
        .unwrap();
        assert!(vendor_results.iter().any(|result| {
            result.package_name == "av:bun"
                && result.source
                    == PackageReceiptSource::Vendor {
                        vendor_name: "bun".to_string(),
                    }
        }));

        let npm_results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            "coverage-npm",
        )
        .unwrap();
        assert!(npm_results.iter().any(|result| {
            result.package_name == "npm:coverage-npm"
                && result.source
                    == PackageReceiptSource::Npm {
                        package_name: "coverage-npm".to_string(),
                    }
                && result.summary == Some("Coverage npm tool".to_string())
                && result.latest_version == Some("1.2.3".to_string())
        }));
    }

    #[test]
    fn package_search_relevance_prefers_exact_name_over_scoped_and_summary_matches() {
        let mut results = [
            package_search_result(
                "npm:@askjo/camofox-browser",
                PackageReceiptSource::Npm {
                    package_name: "@askjo/camofox-browser".to_string(),
                },
                Some("Headless browser automation server and OpenClaw plugin"),
                Some(1),
            ),
            package_search_result(
                "npm:@qingchencloud/openclaw-zh",
                PackageReceiptSource::Npm {
                    package_name: "@qingchencloud/openclaw-zh".to_string(),
                },
                Some("OpenClaw localized release"),
                Some(2),
            ),
            package_search_result(
                "npm:openclaw",
                PackageReceiptSource::Npm {
                    package_name: "openclaw".to_string(),
                },
                Some("Multi-channel AI gateway"),
                None,
            ),
            package_search_result(
                "openclaw-cli",
                PackageReceiptSource::Formula {
                    root_formula: "openclaw-cli".to_string(),
                },
                Some("Your own personal AI assistant"),
                None,
            ),
        ];

        results.sort_by(|left, right| {
            compare_package_search_results_for_query("openclaw", left, right)
        });

        assert_eq!(results[0].package_name, "npm:openclaw");
        assert!(
            results
                .iter()
                .position(|result| result.package_name == "npm:@askjo/camofox-browser")
                > results
                    .iter()
                    .position(|result| result.package_name == "openclaw-cli")
        );
        assert!(
            results
                .iter()
                .position(|result| result.package_name == "npm:@askjo/camofox-browser")
                > results
                    .iter()
                    .position(|result| result.package_name == "npm:@qingchencloud/openclaw-zh")
        );
    }

    #[test]
    fn resolve_package_search_results_do_not_surface_isotopes() {
        let _env_lock = test_env_lock().lock().unwrap();
        let results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            "isotope:gh",
        )
        .unwrap();

        assert!(
            results
                .iter()
                .all(|result| result.package_name != "isotope:gh")
        );
    }

    #[test]
    fn resolve_package_search_results_surfaces_versioned_formula_aliases() {
        let _env_lock = test_env_lock().lock().unwrap();
        let (name, alias) = formula_index_entries()
            .unwrap()
            .iter()
            .find_map(|entry| {
                entry
                    .aliases
                    .iter()
                    .find(|alias| formula_versioned_base(alias).is_some())
                    .map(|alias| (entry.name.clone(), alias.clone()))
            })
            .expect("embedded db should carry at least one versioned formula alias");

        let results = resolve_package_search_results(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &name,
        )
        .unwrap();
        assert!(
            results.iter().any(|result| result.package_name == alias),
            "search should include the versioned formula alias display name"
        );
        assert!(
            results.iter().any(|result| {
                result.package_name == "node@24"
                    && result.source
                        == PackageReceiptSource::Formula {
                            root_formula: "node@24".to_string(),
                        }
            }),
            "search should include versioned formula catalog entries"
        );
        assert!(
            results
                .iter()
                .all(|result| result.package_name != "brew:node@24"),
            "search should not synthesize a duplicate recommendation row when the formula catalog has the versioned formula"
        );
    }

    #[test]
    fn formula_search_results_preserve_versioned_display_names() {
        let versioned =
            formula_search_results_for_query(&formula_index_entry("gcc@15", &[], &[]), "gcc");
        assert_eq!(
            versioned
                .iter()
                .map(|result| (
                    result.package_name.as_str(),
                    package_source_qualified_name(&result.source)
                ))
                .collect::<Vec<_>>(),
            vec![("gcc@15", "brew:gcc@15".to_string())]
        );
        let aliased = formula_search_results_for_query(
            &formula_index_entry("node", &["node@25"], &[]),
            "node@25",
        );
        assert_eq!(
            aliased
                .iter()
                .map(|result| (
                    result.package_name.as_str(),
                    package_source_qualified_name(&result.source)
                ))
                .collect::<Vec<_>>(),
            vec![("node@25", "brew:node@25".to_string())]
        );
        assert_eq!(aliased[0].install_package_names, ["node@25"]);
        let family = formula_search_results_for_query(
            &formula_index_entry("node", &["node@25"], &[]),
            "node",
        );
        assert_eq!(
            family
                .iter()
                .map(|result| result.package_name.as_str())
                .collect::<Vec<_>>(),
            vec!["node", "node@25"]
        );
    }

    #[test]
    fn formula_display_aliases_cover_major_and_minor_version_families() {
        let python = formula_index_entry("python@3.14", &["python@3"], &[]);
        assert_eq!(
            formula_display_alias(&python, "python", "3.14.1"),
            Some("python@3.14".to_string())
        );

        let node = formula_index_entry("node", &["node@25"], &[]);
        assert_eq!(
            formula_display_alias(&node, "node", "26.0.0"),
            Some("node@26".to_string())
        );
    }

    #[test]
    fn search_packages_paginates_results() {
        let _env_lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("GH_CONFIG_DIR", temp.path().to_str().unwrap()),
        ]);
        let formula_index = formula_index_entries().unwrap();
        let query = (1..=3)
            .find_map(|prefix_length| {
                let mut prefix_counts = std::collections::BTreeMap::new();
                for entry in formula_index {
                    if entry.name.len() < prefix_length {
                        continue;
                    }
                    let prefix = entry.name[..prefix_length].to_ascii_lowercase();
                    let count = prefix_counts.entry(prefix.clone()).or_insert(0usize);
                    *count += 1;
                    if *count >= 2 {
                        return Some(prefix);
                    }
                }
                None
            })
            .expect("embedded db should carry at least one shared prefix");

        let first_page = ops::search_packages(&query, 0, 1).unwrap();
        assert_eq!(first_page.packages.len(), 1);
        assert!(first_page.total_count >= 2);
        assert_eq!(first_page.next_offset, Some(1));

        let second_page = ops::search_packages(&query, 1, 1).unwrap();
        assert_eq!(second_page.packages.len(), 1);
        assert_eq!(second_page.total_count, first_page.total_count);
        assert_ne!(first_page.packages[0].name, second_page.packages[0].name);

        let vendor_page = ops::search_packages("av:bun", 0, 10).unwrap();
        let vendor_package = vendor_page
            .packages
            .iter()
            .find(|package| package.name == "av:bun")
            .expect("search should include qualified vendor packages");
        assert_eq!(
            vendor_package.source,
            PackageReceiptSource::Vendor {
                vendor_name: "bun".to_string(),
            }
        );
    }

    #[test]
    fn list_available_packages_paginates_results_and_requires_rank_metadata() {
        let _env_lock = test_env_lock().lock().unwrap();
        let db = crate::cli::load_db().unwrap();
        crate::cli::ensure_db_schema(&db).unwrap();

        let mut ranked = db
            .formulas
            .into_iter()
            .filter_map(|(name, metadata)| {
                metadata
                    .popularity
                    .map(|popularity| (popularity.rank, name))
            })
            .chain(db.casks.into_iter().filter_map(|(name, metadata)| {
                metadata
                    .popularity
                    .map(|popularity| (popularity.rank, name))
            }))
            .chain(db.npms.into_iter().filter_map(|(name, metadata)| {
                metadata
                    .popularity
                    .map(|popularity| (popularity.rank, npm_package_display_name(&name)))
            }))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        ranked.dedup_by(|left, right| left.1 == right.1);
        assert!(
            ranked.len() >= 2,
            "embedded db should carry ranked packages"
        );

        let ranked_page = ops::list_available_packages_matching_category(0, 1, None, None).unwrap();
        assert_eq!(ranked_page.packages.len(), 1);
        assert!(
            !ranked_page.category_counts.is_empty(),
            "ranked catalog response should include category counts"
        );
        assert!(
            !ranked_page.source_counts.is_empty(),
            "ranked catalog response should include package manager counts"
        );

        let first_page =
            ops::list_available_packages_matching_category(0, 1, None, Some("az")).unwrap();
        assert_eq!(first_page.packages.len(), 1);
        assert_eq!(
            first_page.total_count,
            ops::list_available_packages_matching_category(0, 0, None, Some("az"))
                .unwrap()
                .total_count
        );
        assert_eq!(first_page.next_offset, Some(1));

        let second_page =
            ops::list_available_packages_matching_category(1, 1, None, Some("az")).unwrap();
        assert_eq!(second_page.packages.len(), 1);
        assert_eq!(second_page.total_count, first_page.total_count);
        assert_eq!(second_page.source_counts, first_page.source_counts);

        let category = first_page
            .category_counts
            .keys()
            .find(|category| category.as_str() != "other")
            .or_else(|| first_page.category_counts.keys().next())
            .expect("available package response should include category counts")
            .to_string();
        let category_page =
            ops::list_available_packages_matching_category(0, 2, Some(&category), Some("az"))
                .unwrap();
        assert_eq!(
            category_page.total_count,
            first_page.category_counts[&category]
        );
        assert!(category_page.packages.iter().all(|package| {
            package
                .category
                .as_deref()
                .map(str::trim)
                .filter(|category| !category.is_empty())
                .unwrap_or("other")
                == category
        }));
        let alphabetical_category_page = ops::list_available_packages_matching_category(
            0,
            4,
            Some("developer-tools"),
            Some("az"),
        )
        .unwrap();
        let alphabetical_names = alphabetical_category_page
            .packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>();
        let mut sorted_names = alphabetical_names.clone();
        sorted_names.sort_by(|left, right| compare_package_names_for_search_order(left, right));
        assert_eq!(alphabetical_names, sorted_names);

        let available_packages = resolve_available_package_results(&Config {
            bottle_tag: "arm64_tahoe".to_string(),
        })
        .unwrap();
        assert!(available_packages.iter().any(|package| {
            package.package_name == "av:bun"
                && package.source
                    == PackageReceiptSource::Vendor {
                        vendor_name: "bun".to_string(),
                    }
        }));
        assert!(available_packages.iter().any(|package| {
            package.package_name == "npm:coverage-npm"
                && package.source
                    == PackageReceiptSource::Npm {
                        package_name: "coverage-npm".to_string(),
                    }
        }));
    }

    #[test]
    fn list_pulse_packages_paginates_recent_results() {
        let _env_lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("GH_CONFIG_DIR", temp.path().to_str().unwrap()),
        ]);
        let db = crate::cli::load_db().unwrap();
        crate::cli::ensure_db_schema(&db).unwrap();
        let pulse_reference_time = OffsetDateTime::parse(&db.generated_at, &Rfc3339).unwrap();

        let mut recent = db
            .formulas
            .into_iter()
            .filter_map(|(name, metadata)| {
                metadata.last_updated_at.and_then(|last_updated_at| {
                    OffsetDateTime::parse(&last_updated_at, &Rfc3339)
                        .ok()
                        .map(|parsed| {
                            let pulse_kind = metadata.pulse_kind.and_then(|kind| {
                                if kind.eq_ignore_ascii_case("new")
                                    && pulse_reference_time.unix_timestamp()
                                        - parsed.unix_timestamp()
                                        > 7 * 24 * 60 * 60
                                {
                                    None
                                } else {
                                    Some(kind)
                                }
                            });
                            (pulse_kind, parsed, name)
                        })
                })
            })
            .chain(db.casks.into_iter().filter_map(|(name, metadata)| {
                metadata.last_updated_at.and_then(|last_updated_at| {
                    OffsetDateTime::parse(&last_updated_at, &Rfc3339)
                        .ok()
                        .map(|parsed| {
                            let pulse_kind = metadata.pulse_kind.and_then(|kind| {
                                if kind.eq_ignore_ascii_case("new")
                                    && pulse_reference_time.unix_timestamp()
                                        - parsed.unix_timestamp()
                                        > 7 * 24 * 60 * 60
                                {
                                    None
                                } else {
                                    Some(kind)
                                }
                            });
                            (pulse_kind, parsed, name)
                        })
                })
            }))
            .chain(db.npms.into_iter().filter_map(|(name, metadata)| {
                metadata.last_updated_at.and_then(|last_updated_at| {
                    OffsetDateTime::parse(&last_updated_at, &Rfc3339)
                        .ok()
                        .map(|parsed| {
                            let pulse_kind = metadata.pulse_kind.and_then(|kind| {
                                if kind.eq_ignore_ascii_case("new")
                                    && pulse_reference_time.unix_timestamp()
                                        - parsed.unix_timestamp()
                                        > 7 * 24 * 60 * 60
                                {
                                    None
                                } else {
                                    Some(kind)
                                }
                            });
                            (pulse_kind, parsed, npm_package_display_name(&name))
                        })
                })
            }))
            .collect::<Vec<_>>();
        recent.sort_by(|left, right| left.2.cmp(&right.2));
        recent.dedup_by(|left, right| left.2 == right.2);
        recent.sort_by(|left, right| {
            let left_pulse_key = match left.0.as_deref() {
                Some(kind) if kind.eq_ignore_ascii_case("new") => 0,
                _ => 1,
            };
            let right_pulse_key = match right.0.as_deref() {
                Some(kind) if kind.eq_ignore_ascii_case("new") => 0,
                _ => 1,
            };
            left_pulse_key
                .cmp(&right_pulse_key)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        assert!(
            recent.len() >= 2,
            "embedded db should carry recent packages"
        );

        let first_page = ops::list_pulse_packages(0, 1).unwrap();
        assert_eq!(first_page.packages.len(), 1);
        assert_eq!(
            first_page.total_count,
            ops::list_pulse_packages(0, 0).unwrap().total_count
        );
        assert_eq!(first_page.next_offset, Some(1));
        assert_eq!(first_page.packages[0].name, recent[0].2);
        assert!(matches!(
            first_page.packages[0].pulse_kind.as_deref(),
            Some("new" | "updated")
        ));

        let second_page = ops::list_pulse_packages(1, 1).unwrap();
        assert_eq!(second_page.packages.len(), 1);
        assert_eq!(second_page.total_count, first_page.total_count);
        assert_eq!(second_page.packages[0].name, recent[1].2);

        let stale_new = ops::list_pulse_packages(0, 10)
            .unwrap()
            .packages
            .into_iter()
            .find(|package| package.name == "portable-libffi")
            .expect("coverage fixture should include a stale new formula");
        assert_eq!(stale_new.pulse_kind.as_deref(), Some("updated"));
    }

    #[test]
    fn list_pulse_packages_preserves_pulse_order_for_active_security_hazards() {
        let _env_lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let fly_dir = temp.path().join(".fly");
        fs::create_dir_all(&fly_dir).unwrap();
        fs::write(fly_dir.join("config.yml"), "access_token: FlyV1 secret\n").unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("GH_CONFIG_DIR", temp.path().to_str().unwrap()),
        ]);

        let Some(state) = package_security_state_for_identifiers(["brew:flyctl".to_string()])
        else {
            return;
        };
        assert!(state.install_is_insecure);

        let expected = resolve_pulse_package_results(&Config {
            bottle_tag: String::new(),
        })
        .unwrap();
        assert_ne!(
            expected
                .first()
                .map(|package| package.package_name.as_str()),
            Some("flyctl"),
            "fixture must distinguish natural pulse order from hazard promotion"
        );

        let page = ops::list_pulse_packages(0, 3).unwrap();
        let expected_names = expected
            .iter()
            .take(3)
            .map(|package| package.package_name.as_str())
            .collect::<Vec<_>>();
        let actual_names = page
            .packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actual_names, expected_names);
    }

    #[test]
    fn list_geiger_packages_returns_actionable_detector_hits() {
        let _env_lock = test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("hosts.yml"),
            "github.com:\n    user: monalisa\n    oauth_token: ghp_secret\n",
        )
        .unwrap();
        let _env = TestEnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("GH_CONFIG_DIR", temp.path().to_str().unwrap()),
        ]);

        let page = ops::list_geiger_packages(0, 25).unwrap();
        assert!(page.packages.iter().any(|package| {
            package.security_state.as_ref().is_some_and(|state| {
                state.isotope_name == "gh" && (state.install_is_insecure || state.error.is_some())
            })
        }));
    }

    #[test]
    fn protocol_method_parses_list_pulse() {
        assert_eq!(
            core::ProtocolMethod::parse("packages.listPulse"),
            Some(core::ProtocolMethod::PackagesListPulse)
        );
        assert_eq!(
            core::ProtocolMethod::parse("packages.listGeiger"),
            Some(core::ProtocolMethod::PackagesListGeiger)
        );
    }

    #[test]
    fn vendor_npm_and_pip_version_fetchers_use_fixture_metadata_servers() {
        let _env_lock = test_env_lock().lock().unwrap();
        let (base, _server) = start_test_http_server(
            vec![
                (
                    "/repos/oven-sh/bun/releases/latest".to_string(),
                    br#"{"tag_name":"bun-v1.2.3"}"#.to_vec(),
                ),
                (
                    "/openclaw".to_string(),
                    br#"{
                        "description":"A test npm package",
                        "homepage":"https://example.test/openclaw",
                        "dist-tags":{"latest":"4.5.6"},
                        "versions":{
                            "4.5.6":{
                                "dist":{"tarball":"https://registry.npmjs.org/openclaw/-/openclaw-4.5.6.tgz"}
                            }
                        }
                    }"#
                    .to_vec(),
                ),
                (
                    "/psycopg2/json".to_string(),
                    br#"{
                        "info":{
                            "version":"2.9.10",
                            "summary":"A test PyPI package",
                            "home_page":"https://example.test/psycopg2"
                        }
                    }"#
                    .to_vec(),
                ),
            ],
            20,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            github_api_root: Some(base.clone()),
            npm_registry_root: Some(base.clone()),
            pypi_root: Some(base.clone()),
            ..Default::default()
        });

        assert_eq!(bun::version().unwrap(), Version::parse("1.2.3").unwrap());
        assert_eq!(
            resolve_npm_latest_version("openclaw").unwrap(),
            "4.5.6".to_string()
        );
        assert_eq!(
            vendor::npm_tarball_url("openclaw", &Version::parse("4.5.6").unwrap()).unwrap(),
            "https://registry.npmjs.org/openclaw/-/openclaw-4.5.6.tgz".to_string()
        );
        assert_eq!(
            resolve_pip_latest_version("psycopg2").unwrap(),
            "2.9.10".to_string()
        );
        assert_eq!(
            resolve_npm_package_metadata("openclaw").unwrap(),
            PackageMetadata {
                description: Some("A test npm package".to_string()),
                homepage: Some("https://example.test/openclaw".to_string()),
            }
        );
        assert_eq!(
            resolve_pip_package_metadata("psycopg2").unwrap(),
            PackageMetadata {
                description: Some("A test PyPI package".to_string()),
                homepage: Some("https://example.test/psycopg2".to_string()),
            }
        );
    }

    #[test]
    fn download_and_install_helpers_handle_local_archives() {
        let temp = TempDir::new().unwrap();
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();

        let bottle_archive = temp.path().join("sqlite.tar.gz");
        write_test_bottle_archive(
            &bottle_archive,
            "sqlite",
            "3.49.1",
            &[("bin/sqlite3", b"#!/bin/sh\n")],
        );
        let bottle_bytes = fs::read(&bottle_archive).unwrap();
        let bottle_sha = format!("{:x}", Sha256::digest(&bottle_bytes));
        let (bottle_base, bottle_server) = start_test_http_server(
            vec![("/sqlite.tar.gz".to_string(), bottle_bytes.clone())],
            1,
        );
        let bottle_spec = FormulaSpec {
            name: "sqlite".to_string(),
            bottle_sha256: bottle_sha,
            bottle_url: format!("{bottle_base}/sqlite.tar.gz"),
        };

        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "sqlite".to_string(),
            root_formula: "sqlite".to_string(),
            stable_root: temp.path().join("opt/sqlite"),
            install_root: temp.path().join("opt/sqlite"),
            tmp_root: tmp_root.clone(),
        };
        let state = resolve_dependency_install_state(
            std::slice::from_ref(&bottle_spec),
            &plan,
            "all",
            &tmp_root,
            None,
        )
        .unwrap();
        bottle_server.join().unwrap();
        assert_eq!(state.installs.len(), 1);
        assert_eq!(state.installs[0].keg_dir_name, "3.49.1");

        let vendor_archive = temp.path().join("vendor.tar.gz");
        write_test_archive(
            &vendor_archive,
            &[
                ("pkg/bin/tool", b"#!/bin/sh\n"),
                ("pkg/share/doc.txt", b"hello\n"),
            ],
        );
        let vendor_bytes = fs::read(&vendor_archive).unwrap();
        let (vendor_base, vendor_server) =
            start_test_http_server(vec![("/vendor.tar.gz".to_string(), vendor_bytes)], 2);

        let copy_file_version = Version::parse("9.8.7").unwrap();
        register_test_download_url(&copy_file_version, format!("{vendor_base}/vendor.tar.gz"));
        let copy_tree_version = Version::parse("9.8.8").unwrap();
        register_test_download_url(&copy_tree_version, format!("{vendor_base}/vendor.tar.gz"));

        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "tool".to_string(),
            root_formula: "tool".to_string(),
            stable_root: temp.path().join("opt/tool"),
            install_root: temp.path().join("opt/tool"),
            tmp_root: tmp_root.clone(),
        };
        fs::create_dir_all(&plan.install_root).unwrap();

        let copy_file_install = VendorInstall {
            package: vendor::VendorPackage {
                name: "tool",
                dependencies: &[],
                executables: &["tool"],
                version: fake_vendor_version,
                download_url: Some(test_download_url),
                install: fake_vendor_install_strategy,
            },
            version: copy_file_version,
        };
        install_vendor_copy_file(
            &plan,
            &[],
            &copy_file_install,
            "pkg/bin/tool",
            "bin",
            Some("tool"),
            0o755,
            &["bin".to_string()],
            None,
        )
        .unwrap();
        assert!(is_executable(&plan.install_root.join("bin/tool")));

        let tree_plan = InstallPlan {
            mode: Mode::I,
            package_name: "tree".to_string(),
            root_formula: "tree".to_string(),
            stable_root: temp.path().join("opt/tree"),
            install_root: temp.path().join("opt/tree"),
            tmp_root,
        };
        fs::create_dir_all(&tree_plan.install_root).unwrap();
        let copy_tree_install = VendorInstall {
            package: vendor::VendorPackage {
                name: "tree",
                dependencies: &[],
                executables: &["tool"],
                version: fake_vendor_version,
                download_url: Some(test_download_url),
                install: fake_vendor_install_strategy,
            },
            version: copy_tree_version,
        };
        install_vendor_copy_tree(&tree_plan, &copy_tree_install, "pkg", None).unwrap();
        vendor_server.join().unwrap();
        assert!(tree_plan.install_root.join("bin/tool").is_file());
        assert!(tree_plan.install_root.join("share").is_dir());
    }

    #[test]
    fn run_i_package_keeps_downloaded_bottles_alive_until_extract() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let package_name = "archive-lifetime-test";
        let auto_package_name = "auto-formula-dispatch-test";
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        let install_root = opt_root.join(package_name);
        let auto_install_root = opt_root.join(auto_package_name);
        let stub_path = bin_root.join(package_name);
        let auto_stub_path = bin_root.join(auto_package_name);
        for path in [
            &install_root,
            &auto_install_root,
            &stub_path,
            &auto_stub_path,
        ] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        let temp = TempDir::new().unwrap();
        let bottle_archive = temp.path().join("archive-lifetime-test.tar.gz");
        write_test_bottle_archive(
            &bottle_archive,
            package_name,
            "1.0.0",
            &[("bin/archive-lifetime-test", b"#!/bin/sh\nprintf ok\n")],
        );
        let bottle_bytes = fs::read(&bottle_archive).unwrap();
        let bottle_sha = format!("{:x}", Sha256::digest(&bottle_bytes));
        let auto_bottle_archive = temp.path().join("auto-formula-dispatch-test.tar.gz");
        write_test_bottle_archive(
            &auto_bottle_archive,
            auto_package_name,
            "1.0.0",
            &[(
                "bin/auto-formula-dispatch-test",
                b"#!/bin/sh\nprintf auto\n",
            )],
        );
        let auto_bottle_bytes = fs::read(&auto_bottle_archive).unwrap();
        let auto_bottle_sha = format!("{:x}", Sha256::digest(&auto_bottle_bytes));
        let bottle_server = start_counting_test_http_server(vec![
            ("/bottle.tar.gz".to_string(), bottle_bytes),
            ("/auto-bottle.tar.gz".to_string(), auto_bottle_bytes),
        ]);
        let formula_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": bottle_sha,
                            "url": format!("{}/bottle.tar.gz", bottle_server.base_url),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let auto_formula_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": auto_bottle_sha,
                            "url": format!("{}/auto-bottle.tar.gz", bottle_server.base_url),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let formula_server = start_counting_test_http_server(vec![
            (format!("/{package_name}.json"), formula_json),
            (format!("/{auto_package_name}.json"), auto_formula_json),
        ]);
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(formula_server.base_url.clone()),
            ..Default::default()
        });

        run_i_package(
            &Config {
                bottle_tag: "all".to_string(),
            },
            RequestedPackage::HomebrewFormula(package_name.to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap();
        run_i_package(
            &Config {
                bottle_tag: "all".to_string(),
            },
            RequestedPackage::Auto(auto_package_name.to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap();

        assert!(is_executable(
            &install_root.join("bin/archive-lifetime-test")
        ));
        assert!(is_executable(&stub_path));
        assert!(is_executable(
            &auto_install_root.join("bin/auto-formula-dispatch-test")
        ));
        assert!(is_executable(&auto_stub_path));

        for path in [
            &install_root,
            &auto_install_root,
            &stub_path,
            &auto_stub_path,
        ] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }
    }

    #[test]
    fn run_i_formula_update_keeps_dependency_bottles_alive_until_parallel_extract() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let package_name = "ripgrep-lifetime-test";
        let dependency_name = "pcre2-lifetime-test";
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        let install_root = opt_root.join(package_name);
        let stub_path = bin_root.join(package_name);
        for path in [&install_root, &stub_path] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        fs::create_dir_all(install_root.join("bin")).unwrap();
        fs::write(install_root.join("bin/ripgrep-lifetime-test"), b"old").unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: package_name.to_string(),
                version: "0.9.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: package_name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_install_receipt(
            &install_root
                .join(RECEIPTS_DIR)
                .join(format!("{dependency_name}.json")),
            &InstallReceipt {
                formula: dependency_name.to_string(),
                version: "0.9.0".to_string(),
                bottle_sha256: "old-dep-sha".to_string(),
                bottle_tag: "all".to_string(),
                owned_paths: vec!["lib/libpcre2-test.dylib".to_string()],
            },
        )
        .unwrap();
        write_install_receipt(
            &install_root
                .join(RECEIPTS_DIR)
                .join(format!("{package_name}.json")),
            &InstallReceipt {
                formula: package_name.to_string(),
                version: "0.9.0".to_string(),
                bottle_sha256: "old-root-sha".to_string(),
                bottle_tag: "all".to_string(),
                owned_paths: vec!["bin/ripgrep-lifetime-test".to_string()],
            },
        )
        .unwrap();

        let temp = TempDir::new().unwrap();
        let dep_archive = temp.path().join("pcre2-lifetime-test.tar.gz");
        write_test_bottle_archive(
            &dep_archive,
            dependency_name,
            "1.0.0",
            &[("lib/libpcre2-test.dylib", b"dep")],
        );
        let root_archive = temp.path().join("ripgrep-lifetime-test.tar.gz");
        write_test_bottle_archive(
            &root_archive,
            package_name,
            "1.0.0",
            &[("bin/ripgrep-lifetime-test", b"#!/bin/sh\nprintf rg\n")],
        );
        let dep_bytes = fs::read(&dep_archive).unwrap();
        let root_bytes = fs::read(&root_archive).unwrap();
        let dep_sha = format!("{:x}", Sha256::digest(&dep_bytes));
        let root_sha = format!("{:x}", Sha256::digest(&root_bytes));
        let bottle_server = start_counting_test_http_server(vec![
            ("/pcre2.tar.gz".to_string(), dep_bytes),
            ("/ripgrep.tar.gz".to_string(), root_bytes),
        ]);
        let dep_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": dep_sha,
                            "url": format!("{}/pcre2.tar.gz", bottle_server.base_url),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let root_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [dependency_name],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": root_sha,
                            "url": format!("{}/ripgrep.tar.gz", bottle_server.base_url),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let formula_server = start_counting_test_http_server(vec![
            (format!("/{dependency_name}.json"), dep_json),
            (format!("/{package_name}.json"), root_json),
        ]);
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(formula_server.base_url.clone()),
            ..Default::default()
        });

        run_i_formula(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            package_name.to_string(),
            InstallIntent::Update,
            None,
        )
        .unwrap();

        assert!(is_executable(
            &install_root.join("bin/ripgrep-lifetime-test")
        ));
        assert!(install_root.join("lib/libpcre2-test.dylib").is_file());

        for path in [&install_root, &stub_path] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }
    }

    #[test]
    fn run_i_vendor_installs_from_local_archive_and_writes_receipts_and_stubs() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let package_name = "coverage-vendor";
        let opt_root = opt_pkg_root();
        let install_root = opt_root.join(package_name);
        let bin_root = managed_bin_root();
        let stub_path = bin_root.join(package_name);
        for path in [&install_root, &stub_path] {
            if fs::symlink_metadata(path).is_ok() {
                remove_path(path).unwrap();
            }
        }

        let temp = TempDir::new().unwrap();
        let vendor_archive = temp.path().join("coverage-vendor.tar.gz");
        write_test_archive(
            &vendor_archive,
            &[("pkg/bin/coverage-vendor", b"#!/bin/sh\nprintf coverage\n")],
        );
        let vendor_bytes = fs::read(&vendor_archive).unwrap();
        let mut vendor_server =
            start_counting_test_http_server(vec![("/vendor.tar.gz".to_string(), vendor_bytes)]);
        let vendor_base = vendor_server.base_url.clone();
        let version = Version::parse("0.0.0").unwrap();
        register_test_download_url(&version, format!("{vendor_base}/vendor.tar.gz"));

        let events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let callback_events = Arc::clone(&events);
        let callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                callback_events.lock().unwrap().push(event);
            })));

        run_i_vendor(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            vendor::VendorPackage {
                name: package_name,
                dependencies: &[],
                executables: &["coverage-vendor"],
                version: fake_vendor_version,
                download_url: Some(test_download_url),
                install: coverage_vendor_install_strategy,
            },
            InstallIntent::Install,
            Some(callback),
        )
        .unwrap();

        let receipt = load_package_receipt(&install_root.join(ROOT_RECEIPT))
            .unwrap()
            .unwrap();
        assert_eq!(receipt.package_name, package_name);
        assert_eq!(receipt.version, "0.0.0");
        assert_eq!(
            receipt.source,
            PackageReceiptSource::Vendor {
                vendor_name: package_name.to_string(),
            }
        );
        assert!(is_executable(&install_root.join("bin/coverage-vendor")));
        assert!(is_executable(&stub_path));
        assert_eq!(
            load_stub_manifest(&install_root.join(STUB_MANIFEST))
                .unwrap()
                .stubs,
            vec![package_name.to_string()]
        );
        assert!(events.lock().unwrap().iter().any(
            |event| matches!(event, ProgressEvent::Completed { package } if package == package_name)
        ));

        fs::remove_file(install_root.join("bin/coverage-vendor")).unwrap();
        let reinstall_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let reinstall_callback_events = Arc::clone(&reinstall_events);
        let reinstall_callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                reinstall_callback_events.lock().unwrap().push(event);
            })));

        run_i_vendor(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            vendor::VendorPackage {
                name: package_name,
                dependencies: &[],
                executables: &["coverage-vendor"],
                version: fake_vendor_version,
                download_url: Some(test_download_url),
                install: coverage_vendor_install_strategy,
            },
            InstallIntent::Install,
            Some(reinstall_callback),
        )
        .unwrap();
        assert!(is_executable(&install_root.join("bin/coverage-vendor")));
        assert!(reinstall_events.lock().unwrap().iter().any(
            |event| matches!(event, ProgressEvent::Completed { package } if package == package_name)
        ));

        let current_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let current_callback_events = Arc::clone(&current_events);
        let current_callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                current_callback_events.lock().unwrap().push(event);
            })));
        run_i_vendor(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            vendor::VendorPackage {
                name: package_name,
                dependencies: &[],
                executables: &["coverage-vendor"],
                version: fake_vendor_version,
                download_url: Some(test_download_url),
                install: coverage_vendor_install_strategy,
            },
            InstallIntent::Install,
            Some(current_callback),
        )
        .unwrap();
        let current_events = current_events.lock().unwrap();
        assert!(current_events.iter().any(
            |event| matches!(event, ProgressEvent::Completed { package } if package == package_name)
        ));
        vendor_server.stop().unwrap();
        let vendor_requests = vendor_server.request_count();
        assert_eq!(vendor_requests, 3);
        remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
    }

    #[test]
    fn run_i_vendor_reports_missing_download_url() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        let package_name = "coverage-vendor-missing-url";
        let install_root = package_install_root(&opt_root, package_name).unwrap();
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
        }
        let package = fake_vendor_install(package_name, &["coverage-vendor"], "1.2.3").package;

        let err = run_i_vendor(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            package,
            InstallIntent::Install,
            None,
        )
        .unwrap_err();

        assert!(err.contains("has no download URL"));
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
        }
    }

    #[test]
    fn run_i_vendor_skips_current_root_and_syncs_stubs() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        let package_name = "coverage-vendor-current";
        let install_root = package_install_root(&opt_root, package_name).unwrap();
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
        }
        let stub = bin_root.join(package_name);
        if fs::symlink_metadata(&stub).is_ok() {
            remove_path(&stub).unwrap();
        }

        let plan = InstallPlan::for_i(package_name.to_string(), package_name.to_string());
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        let executable = plan.install_root.join("bin").join(package_name);
        fs::write(&executable, b"#!/bin/sh\nprintf vendor\n").unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).unwrap();
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: package_name.to_string(),
                version: "0.0.0".to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: package_name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_root_ownership_manifest(&plan, vec![package_name.to_string()]).unwrap();

        let package = fake_vendor_install(
            "coverage-vendor-current",
            &["coverage-vendor-current"],
            "0.0.0",
        )
        .package;
        run_i_vendor(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            package,
            InstallIntent::Update,
            None,
        )
        .unwrap();

        assert!(is_executable(&stub));
        remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
    }

    #[test]
    fn run_i_npm_and_pip_install_with_local_formula_tools() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        for package_name in ["npm:coverage-npm", "pip:coverage-pip"] {
            let install_root = package_install_root(&opt_root, package_name).unwrap();
            if fs::symlink_metadata(&install_root).is_ok() {
                remove_path(&install_root).unwrap();
            }
        }
        for stub in ["coverage-npm", "coverage-pip"] {
            let path = bin_root.join(stub);
            if fs::symlink_metadata(&path).is_ok() {
                remove_path(&path).unwrap();
            }
        }

        let temp = TempDir::new().unwrap();
        let node_archive = temp.path().join("node.tar.gz");
        let fake_npm = br#"#!/bin/sh
set -eu
prefix=
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--prefix" ]; then
    prefix="$2"
    shift 2
  else
    shift
  fi
done
/bin/mkdir -p "$prefix/bin" "$prefix/lib/node_modules/coverage-npm"
/bin/cat > "$prefix/bin/coverage-npm" <<'EOF'
#!/bin/sh
printf 'coverage-npm\n'
EOF
/bin/chmod +x "$prefix/bin/coverage-npm"
"#;
        write_test_bottle_archive(&node_archive, "node", "1.0.0", &[("bin/npm", fake_npm)]);
        let node_bytes = fs::read(&node_archive).unwrap();
        let node_sha = format!("{:x}", Sha256::digest(&node_bytes));

        let python_archive = temp.path().join("python.tar.gz");
        let fake_python = br#"#!/bin/sh
set -eu
if [ "${1:-}" = "-m" ] && [ "${2:-}" = "venv" ]; then
  for last do :; done
  /bin/mkdir -p "$last/bin"
  /bin/cat > "$last/bin/python" <<'PY'
#!/bin/sh
if [ "${1:-}" = "-c" ]; then
  printf '["coverage-pip"]\n'
fi
PY
  /bin/chmod +x "$last/bin/python"
  /bin/cat > "$last/bin/pip" <<'PIP'
#!/bin/sh
dir=$(/usr/bin/dirname "$0")
/bin/cat > "$dir/coverage-pip" <<'ENTRY'
#!/bin/sh
printf 'coverage-pip\n'
ENTRY
/bin/chmod +x "$dir/coverage-pip"
PIP
  /bin/chmod +x "$last/bin/pip"
  /usr/bin/touch "$last/pyvenv.cfg"
fi
"#;
        write_test_bottle_archive(
            &python_archive,
            "python@3.14",
            "3.14.0",
            &[("bin/python3", fake_python)],
        );
        let python_bytes = fs::read(&python_archive).unwrap();
        let python_sha = format!("{:x}", Sha256::digest(&python_bytes));

        let (bottle_base, bottle_server) = start_test_http_server(
            vec![
                ("/node.tar.gz".to_string(), node_bytes),
                ("/python.tar.gz".to_string(), python_bytes),
            ],
            15,
        );
        let node_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": node_sha,
                            "url": format!("{bottle_base}/node.tar.gz"),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let python_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "3.14.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": python_sha,
                            "url": format!("{bottle_base}/python.tar.gz"),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();

        let (base, server) = start_test_http_server(
            vec![
                ("/node.json".to_string(), node_json),
                ("/python@3.14.json".to_string(), python_json),
                (
                    "/coverage-npm".to_string(),
                    br#"{
                        "description":"Coverage npm package",
                        "homepage":"https://example.test/coverage-npm",
                        "dist-tags":{"latest":"1.2.3"},
                        "versions":{
                            "1.2.3":{
                                "dist":{"tarball":"https://example.test/coverage-npm.tgz"}
                            }
                        }
                    }"#
                    .to_vec(),
                ),
                (
                    "/coverage-pip/json".to_string(),
                    br#"{
                        "info":{
                            "version":"2.3.4",
                            "summary":"Coverage pip package",
                            "home_page":"https://example.test/coverage-pip"
                        }
                    }"#
                    .to_vec(),
                ),
            ],
            30,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(base.clone()),
            npm_registry_root: Some(base.clone()),
            pypi_root: Some(base.clone()),
            ..Default::default()
        });

        run_i_package(
            &Config {
                bottle_tag: "all".to_string(),
            },
            RequestedPackage::Auto("coverage-npm".to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap();
        run_i_package(
            &Config {
                bottle_tag: "all".to_string(),
            },
            RequestedPackage::PipPackage("coverage-pip".to_string()),
            InstallOptions {
                intent: InstallIntent::Install,
            },
        )
        .unwrap();

        let npm_root = opt_root.join("npm/coverage-npm");
        let pip_root = opt_root.join("pip/coverage-pip");
        assert!(is_executable(&npm_root.join("bin/coverage-npm")));
        assert!(is_executable(&pip_root.join("bin/coverage-pip")));
        assert!(is_executable(&bin_root.join("coverage-npm")));
        assert!(is_executable(&bin_root.join("coverage-pip")));
        assert_eq!(
            load_package_receipt(&npm_root.join(ROOT_RECEIPT))
                .unwrap()
                .unwrap()
                .version,
            "1.2.3"
        );
        assert_eq!(
            load_package_receipt(&pip_root.join(ROOT_RECEIPT))
                .unwrap()
                .unwrap()
                .version,
            "2.3.4"
        );

        fs::remove_file(npm_root.join("bin/coverage-npm")).unwrap();
        fs::remove_file(pip_root.join("bin/coverage-pip")).unwrap();

        let reinstall_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let npm_reinstall_events = Arc::clone(&reinstall_events);
        let npm_callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                npm_reinstall_events.lock().unwrap().push(event);
            })));
        run_i_npm(
            &Config {
                bottle_tag: "all".to_string(),
            },
            "npm:coverage-npm".to_string(),
            "coverage-npm".to_string(),
            None,
            InstallOptions {
                intent: InstallIntent::Install,
            },
            InstallIntent::Install,
            Some(npm_callback),
        )
        .unwrap();

        let pip_reinstall_events = Arc::clone(&reinstall_events);
        let pip_callback: Arc<Mutex<Box<ProgressCallback>>> =
            Arc::new(Mutex::new(Box::new(move |event| {
                pip_reinstall_events.lock().unwrap().push(event);
            })));
        run_i_pip(
            &Config {
                bottle_tag: "all".to_string(),
            },
            "pip:coverage-pip".to_string(),
            "coverage-pip".to_string(),
            InstallIntent::Install,
            Some(pip_callback),
        )
        .unwrap();

        assert!(is_executable(&npm_root.join("bin/coverage-npm")));
        assert!(is_executable(&pip_root.join("bin/coverage-pip")));
        let reinstall_events = reinstall_events.lock().unwrap();
        assert!(reinstall_events.iter().any(
            |event| matches!(event, ProgressEvent::Completed { package } if package == "npm:coverage-npm")
        ));
        assert!(reinstall_events.iter().any(
            |event| matches!(event, ProgressEvent::Completed { package } if package == "pip:coverage-pip")
        ));

        run_i_npm(
            &Config {
                bottle_tag: "all".to_string(),
            },
            "npm:coverage-npm".to_string(),
            "coverage-npm".to_string(),
            None,
            InstallOptions {
                intent: InstallIntent::Update,
            },
            InstallIntent::Update,
            None,
        )
        .unwrap();
        run_i_pip(
            &Config {
                bottle_tag: "all".to_string(),
            },
            "pip:coverage-pip".to_string(),
            "coverage-pip".to_string(),
            InstallIntent::Update,
            None,
        )
        .unwrap();

        drain_test_server(&base, "/coverage-npm", 30);
        drain_test_server(&bottle_base, "/node.tar.gz", 15);
        server.join().unwrap();
        bottle_server.join().unwrap();
        remove_existing_package_install(&opt_root, "npm:coverage-npm", &bin_root).unwrap();
        remove_existing_package_install(&opt_root, "pip:coverage-pip", &bin_root).unwrap();
    }

    #[test]
    fn run_i_npm_repairs_current_but_unlaunchable_node_runtime() {
        let _env_lock = test_env_lock().lock().unwrap();
        let _package_lock = acquire_package_mutation_lock().unwrap();
        let opt_root = opt_pkg_root();
        let bin_root = managed_bin_root();
        let package_name = "npm:runtime-probe";
        let npm_package = "runtime-probe";
        let install_root = package_install_root(&opt_root, package_name).unwrap();
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
        }
        let stub = bin_root.join(npm_package);
        if fs::symlink_metadata(&stub).is_ok() {
            remove_path(&stub).unwrap();
        }

        fs::create_dir_all(install_root.join("bin")).unwrap();
        fs::write(
            install_root.join("bin/node"),
            b"#!/bin/sh\n# broken-node\nexit 78\n",
        )
        .unwrap();
        fs::write(
            install_root.join("bin/npm"),
            b"#!/usr/bin/env node\n# broken-npm\nexit 78\n",
        )
        .unwrap();
        fs::write(
            install_root.join("bin/runtime-probe"),
            b"#!/bin/sh\nprintf 'runtime-probe\\n'\n",
        )
        .unwrap();
        for path in [
            install_root.join("bin/node"),
            install_root.join("bin/npm"),
            install_root.join("bin/runtime-probe"),
        ] {
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }

        let temp = TempDir::new().unwrap();
        let node_archive = temp.path().join("node.tar.gz");
        let fake_node = br#"#!/bin/sh
if [ "${1:-}" = "--version" ]; then
  printf 'repaired-node\n'
  exit 0
fi
script="${1:-}"
if [ -z "$script" ]; then
  exit 0
fi
shift
exec /bin/sh "$script" "$@"
"#;
        let fake_npm = br#"#!/usr/bin/env node
set -eu
if [ "${1:-}" = "--version" ]; then
  printf 'repaired-npm\n'
  exit 0
fi
prefix=
dry_run=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix)
      prefix="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    *)
      shift
      ;;
  esac
done
if [ "$dry_run" = 1 ]; then
  exit 0
fi
/bin/mkdir -p "$prefix/bin" "$prefix/lib/node_modules/runtime-probe"
/bin/cat > "$prefix/bin/runtime-probe" <<'EOF'
#!/bin/sh
printf 'runtime-probe\n'
EOF
/bin/chmod +x "$prefix/bin/runtime-probe"
"#;
        write_test_bottle_archive(
            &node_archive,
            "node",
            "1.0.0",
            &[("bin/node", fake_node), ("bin/npm", fake_npm)],
        );
        let node_bytes = fs::read(&node_archive).unwrap();
        let node_sha = format!("{:x}", Sha256::digest(&node_bytes));
        let node_spec = InstalledFormula {
            spec: FormulaSpec {
                name: "node".to_string(),
                bottle_sha256: node_sha.clone(),
                bottle_url: "https://example.invalid/node.tar.gz".to_string(),
            },
            keg_dir_name: "1.0.0".to_string(),
            archive_path: PathBuf::new(),
        };
        write_receipt_with_owned_paths(
            &install_root.join(RECEIPTS_DIR).join("node.json"),
            &node_spec,
            "all",
            vec!["bin/node".to_string(), "bin/npm".to_string()],
        )
        .unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: package_name.to_string(),
                version: "1.2.3".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: npm_package.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let (bottle_base, bottle_server) =
            start_test_http_server(vec![("/node.tar.gz".to_string(), node_bytes)], 5);
        let node_json = serde_json::to_vec(&serde_json::json!({
            "versions": { "stable": "1.0.0" },
            "dependencies": [],
            "bottle": {
                "stable": {
                    "files": {
                        "all": {
                            "sha256": node_sha,
                            "url": format!("{bottle_base}/node.tar.gz"),
                        }
                    }
                }
            },
            "disabled": false
        }))
        .unwrap();
        let package_json = br#"{
            "description":"Runtime probe npm package",
            "homepage":"https://example.test/runtime-probe",
            "dist-tags":{"latest":"1.2.3"},
            "versions":{
                "1.2.3":{
                    "dist":{"tarball":"https://example.test/runtime-probe.tgz"}
                }
            }
        }"#
        .to_vec();
        let (base, server) = start_test_http_server(
            vec![
                ("/node.json".to_string(), node_json),
                ("/runtime-probe".to_string(), package_json),
            ],
            10,
        );
        let _endpoints = TestEndpointGuard::set(config::TestEndpointOverrides {
            formula_api_root: Some(base.clone()),
            npm_registry_root: Some(base.clone()),
            ..Default::default()
        });

        run_i_npm(
            &Config {
                bottle_tag: "all".to_string(),
            },
            package_name.to_string(),
            npm_package.to_string(),
            None,
            InstallOptions {
                intent: InstallIntent::Update,
            },
            InstallIntent::Update,
            None,
        )
        .unwrap();

        assert!(
            String::from_utf8(fs::read(install_root.join("bin/node")).unwrap())
                .unwrap()
                .contains("repaired-node")
        );
        assert!(is_executable(&install_root.join("bin/runtime-probe")));
        assert!(is_executable(&bin_root.join("runtime-probe")));

        drain_test_server(&base, "/runtime-probe", 10);
        drain_test_server(&bottle_base, "/node.tar.gz", 5);
        server.join().unwrap();
        bottle_server.join().unwrap();
        remove_existing_package_install(&opt_root, package_name, &bin_root).unwrap();
    }

    #[test]
    fn unpack_vendor_archive_accepts_plain_tar_with_tgz_extension() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join("plain.tgz");
        write_test_plain_tar_archive(
            &archive,
            &[
                ("pkg/bin/tool", b"#!/bin/sh\n"),
                ("pkg/share/doc.txt", b"hello\n"),
            ],
        );
        let destination = temp.path().join("out");
        fs::create_dir_all(&destination).unwrap();

        unpack_vendor_archive(&archive, &destination, "plain-tgz").unwrap();

        assert!(destination.join("pkg/bin/tool").is_file());
        assert!(destination.join("pkg/share/doc.txt").is_file());
    }

    #[test]
    fn unpack_vendor_archive_reports_unknown_and_zip_failures() {
        let temp = TempDir::new().unwrap();
        let destination = temp.path().join("out");
        fs::create_dir_all(&destination).unwrap();
        let unsupported = temp.path().join("payload.bin");
        fs::write(&unsupported, b"payload").unwrap();

        assert!(
            unpack_vendor_archive(&unsupported, &destination, "payload")
                .unwrap_err()
                .contains("unsupported vendor archive format")
        );

        #[cfg(target_os = "macos")]
        {
            let missing_zip = temp.path().join("missing.zip");
            assert!(
                unpack_vendor_archive(&missing_zip, &destination, "missing")
                    .unwrap_err()
                    .contains("failed to unpack vendor archive")
            );
        }
    }

    #[test]
    fn install_cask_root_accepts_direct_binary_payload() {
        let temp = TempDir::new().unwrap();
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();
        let binary_bytes = b"#!/bin/sh\necho claude\n".to_vec();
        let binary_sha = format!("{:x}", Sha256::digest(&binary_bytes));
        let (base, server) = start_test_http_server(vec![("/claude".to_string(), binary_bytes)], 1);
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "claude-code".to_string(),
            root_formula: "claude-code".to_string(),
            stable_root: temp.path().join("opt/claude-code"),
            install_root: temp.path().join("opt/claude-code"),
            tmp_root,
        };
        let cask = EmbeddedCaskMetadata {
            url: format!("{base}/claude"),
            sha256: binary_sha,
            version: "2.1.112".to_string(),
            binaries: vec![EmbeddedCaskBinary {
                source: "claude".to_string(),
                target: None,
            }],
            ..Default::default()
        };

        install_cask_root(&plan, "claude-code", &cask, None).unwrap();
        server.join().unwrap();

        let installed = plan.install_root.join("bin/claude");
        assert!(installed.is_file());
        assert!(is_executable(&installed));
        assert_eq!(
            fs::read_to_string(&installed).unwrap(),
            "#!/bin/sh\necho claude\n"
        );
    }

    #[test]
    fn cask_install_helpers_cover_tar_payload_current_receipts_and_sha_mismatch() {
        let temp = TempDir::new().unwrap();
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();
        let archive = temp.path().join("codex.tar.gz");
        write_test_archive(
            &archive,
            &[
                ("Codex.app/Contents/MacOS/codex", b"#!/bin/sh\necho codex\n"),
                ("Codex.app/Contents/MacOS/cdx", b"#!/bin/sh\necho cdx\n"),
            ],
        );
        let archive_bytes = fs::read(&archive).unwrap();
        let archive_sha = format!("{:x}", Sha256::digest(&archive_bytes));
        let (base, server) = start_test_http_server(
            vec![("/codex.tar.gz".to_string(), archive_bytes.clone())],
            2,
        );
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "cask:codex".to_string(),
            root_formula: "codex".to_string(),
            stable_root: temp.path().join("opt/cask/codex"),
            install_root: temp.path().join("opt/cask/codex"),
            tmp_root: tmp_root.clone(),
        };
        let cask = EmbeddedCaskMetadata {
            summary: "OpenAI Codex".to_string(),
            homepage: "https://example.test/codex".to_string(),
            url: format!("{base}/codex.tar.gz"),
            sha256: archive_sha,
            version: "1.0.0".to_string(),
            binaries: vec![
                EmbeddedCaskBinary {
                    source: "Codex.app/Contents/MacOS/codex".to_string(),
                    target: None,
                },
                EmbeddedCaskBinary {
                    source: "Codex.app/Contents/MacOS/cdx".to_string(),
                    target: Some("codex-chat".to_string()),
                },
            ],
            ..Default::default()
        };

        install_cask_root(&plan, "codex", &cask, None).unwrap();
        assert!(is_executable(&plan.install_root.join("bin/codex")));
        assert!(is_executable(&plan.install_root.join("bin/codex-chat")));
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "cask:codex".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Cask {
                    cask_name: "codex".to_string(),
                },
                metadata: PackageMetadata {
                    description: Some("OpenAI Codex".to_string()),
                    homepage: Some("https://example.test/codex".to_string()),
                },
            },
        )
        .unwrap();
        assert!(cask_root_is_current(&plan, &cask, &[], "all").unwrap());

        let bad_archive = temp.path().join("bad-codex.tar.gz");
        let bad_cask = EmbeddedCaskMetadata {
            sha256: "deadbeef".repeat(8),
            url: format!("{base}/codex.tar.gz"),
            version: "1.0.0".to_string(),
            binaries: vec![EmbeddedCaskBinary {
                source: "bad-codex.tar.gz".to_string(),
                target: None,
            }],
            ..Default::default()
        };
        let err = download_cask_archive("codex", &bad_cask, &bad_archive, None).unwrap_err();
        assert!(err.contains("sha256 mismatch for cask codex"));
        server.join().unwrap();
    }

    #[test]
    fn isotope_install_helpers_cover_nested_and_flat_archives() {
        let temp = TempDir::new().unwrap();
        let tmp_root = temp.path().join("tmp");
        fs::create_dir_all(&tmp_root).unwrap();

        let nested_archive = temp.path().join("gh.tar.gz");
        write_test_archive(
            &nested_archive,
            &[
                ("gh-2.80.0/bin/gh", b"#!/bin/sh\nprintf gh\\n\n"),
                ("gh-2.80.0/share/man/man1/gh.1", b"GH manual\n"),
            ],
        );
        let flat_archive = temp.path().join("aws-cli.tgz");
        write_test_archive(
            &flat_archive,
            &[
                ("bin/aws", b"#!/bin/sh\nprintf aws\\n\n"),
                ("share/doc/aws.txt", b"aws docs\n"),
            ],
        );
        let bin_only_archive = temp.path().join("supabase-cli.tgz");
        write_test_archive(
            &bin_only_archive,
            &[
                ("bin/supabase", b"#!/bin/sh\nprintf supabase\\n\n"),
                ("bin/supabase-go", b"#!/bin/sh\nprintf supabase-go\\n\n"),
            ],
        );
        let (base, server) = start_test_http_server(
            vec![
                ("/gh.tar.gz".to_string(), fs::read(&nested_archive).unwrap()),
                ("/aws-cli.tgz".to_string(), fs::read(&flat_archive).unwrap()),
                (
                    "/supabase-cli.tgz".to_string(),
                    fs::read(&bin_only_archive).unwrap(),
                ),
            ],
            3,
        );

        let isotope = IsotopePackageData {
            name: "isotope:gh".to_string(),
            replaces: Some("brew:gh".to_string()),
            modifies: None,
            migrate: None,
            _repository: None,
            _upstream_repository: None,
            version: "2.80.0".to_string(),
            release_url: Some("https://example.test/isotopes/gh".to_string()),
            archive_url: Some(format!("{base}/gh.tar.gz")),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        let gh_plan = InstallPlan {
            mode: Mode::I,
            package_name: "isotope:gh".to_string(),
            root_formula: "gh".to_string(),
            stable_root: temp.path().join("opt/iso/gh"),
            install_root: temp.path().join("opt/iso/gh"),
            tmp_root: tmp_root.clone(),
        };
        install_isotope_root(&gh_plan, &isotope, &[], None).unwrap();
        assert!(is_executable(&gh_plan.install_root.join("bin/gh")));
        assert!(gh_plan.install_root.join("share/man/man1/gh.1").is_file());
        write_root_executable_manifest(
            &gh_plan.root_executables_manifest_path(),
            &["gh".to_string()],
        )
        .unwrap();
        write_package_receipt(
            &gh_plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "isotope:gh".to_string(),
                version: "2.80.0".to_string(),
                source: PackageReceiptSource::Isotope {
                    isotope_name: "gh".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        assert!(isotope_root_is_current(&gh_plan, &isotope).unwrap());

        let radioisotope = IsotopePackageData {
            name: "isotope:aws-cli".to_string(),
            replaces: None,
            modifies: Some("brew:awscli".to_string()),
            migrate: Some("aws configure import --csv file://$1".to_string()),
            _repository: None,
            _upstream_repository: None,
            version: "1.0.0".to_string(),
            release_url: Some("https://example.test/isotopes/aws-cli".to_string()),
            archive_url: Some(format!("{base}/aws-cli.tgz")),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        let aws_plan = InstallPlan {
            mode: Mode::I,
            package_name: "isotope:aws-cli".to_string(),
            root_formula: "awscli".to_string(),
            stable_root: temp.path().join("opt/iso/aws-cli"),
            install_root: temp.path().join("opt/iso/aws-cli"),
            tmp_root: tmp_root.clone(),
        };
        install_isotope_root(&aws_plan, &radioisotope, &[], None).unwrap();
        assert!(is_executable(&aws_plan.install_root.join("bin/aws")));
        assert!(aws_plan.install_root.join("share/doc/aws.txt").is_file());

        let bin_only_isotope = IsotopePackageData {
            name: "isotope:supabase".to_string(),
            replaces: Some("brew:supabase".to_string()),
            modifies: None,
            migrate: Some("/opt/iso/supabase/bin/supabase-go av-migrate \"$@\"".to_string()),
            _repository: None,
            _upstream_repository: None,
            version: "2.101.0".to_string(),
            release_url: Some("https://example.test/isotopes/supabase-cli".to_string()),
            archive_url: Some(format!("{base}/supabase-cli.tgz")),
            published_at: None,
            applies_to_versioned_formulae: false,
        };
        let bin_only_plan = InstallPlan {
            mode: Mode::I,
            package_name: "isotope:supabase".to_string(),
            root_formula: "supabase".to_string(),
            stable_root: temp.path().join("opt/iso/supabase"),
            install_root: temp.path().join("opt/iso/supabase"),
            tmp_root: tmp_root.clone(),
        };
        install_isotope_root(&bin_only_plan, &bin_only_isotope, &[], None).unwrap();
        assert!(is_executable(
            &bin_only_plan.install_root.join("bin/supabase")
        ));
        assert!(is_executable(
            &bin_only_plan.install_root.join("bin/supabase-go")
        ));
        assert!(!bin_only_plan.install_root.join("supabase").exists());

        let missing_archive = IsotopePackageData {
            archive_url: None,
            ..radioisotope
        };
        let err = install_isotope_root(&aws_plan, &missing_archive, &[], None).unwrap_err();
        assert!(err.contains("isotope isotope:aws-cli has no archive URL"));
        server.join().unwrap();
    }

    #[test]
    fn install_package_and_command_helpers_cover_end_to_end_staging() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "sqlite".to_string(),
            root_formula: "sqlite".to_string(),
            stable_root: temp.path().join("opt/sqlite"),
            install_root: temp.path().join("opt/sqlite"),
            tmp_root: temp.path().join("tmp"),
        };
        ensure_plan_parent_dirs(&plan).unwrap();

        let archive = temp.path().join("sqlite.tar.gz");
        write_test_bottle_archive(
            &archive,
            "sqlite",
            "3.49.1",
            &[
                ("bin/sqlite3", b"#!/bin/sh\n"),
                ("share/doc.txt", b"hello\n"),
            ],
        );
        let install = InstalledFormula {
            spec: FormulaSpec {
                name: "sqlite".to_string(),
                bottle_sha256: "sha256".to_string(),
                bottle_url: "https://example.invalid/sqlite.tar.gz".to_string(),
            },
            keg_dir_name: "3.49.1".to_string(),
            archive_path: archive,
        };
        let config = Config {
            bottle_tag: "arm64_tahoe".to_string(),
        };
        let graph = vec![install.spec.clone()];
        let rewrite_rules = build_rewrite_rules(&plan, std::slice::from_ref(&install));

        install_package(
            &config,
            &plan,
            std::slice::from_ref(&install),
            std::slice::from_ref(&install),
            &rewrite_rules,
            None,
        )
        .unwrap();

        assert!(plan.install_root.join("bin/sqlite3").is_file());
        assert!(plan.install_root.join("share/doc.txt").is_file());
        assert!(package_is_current(&plan, &[install], &config.bottle_tag).unwrap());
        assert_eq!(
            build_formula_order(&plan, &graph),
            vec!["sqlite".to_string()]
        );
        assert_eq!(
            resolve_install_time_command(&plan, &graph, "sqlite3").unwrap(),
            plan.install_root.join("bin/sqlite3")
        );
    }

    #[test]
    fn install_package_incremental_reuses_current_dependency_and_replaces_changed_root() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "foo".to_string(),
            root_formula: "foo".to_string(),
            stable_root: temp.path().join("opt/foo"),
            install_root: temp.path().join("opt/foo"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(plan.install_root.join("lib")).unwrap();
        fs::create_dir_all(&plan.tmp_root).unwrap();
        fs::write(plan.install_root.join("bin/foo"), b"old").unwrap();
        fs::write(plan.install_root.join("bin/stale"), b"stale").unwrap();
        fs::write(plan.install_root.join("lib/bar.txt"), b"bar").unwrap();
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "foo".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "foo".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        let old_foo = InstalledFormula {
            spec: FormulaSpec {
                name: "foo".to_string(),
                bottle_sha256: "oldsha".to_string(),
                bottle_url: "https://example.invalid/foo-old.tar.gz".to_string(),
            },
            keg_dir_name: "1.0.0".to_string(),
            archive_path: PathBuf::new(),
        };
        write_receipt_with_owned_paths(
            &plan.receipt_path("foo"),
            &old_foo,
            "arm64_tahoe",
            vec!["bin/foo".to_string(), "bin/stale".to_string()],
        )
        .unwrap();
        let bar = InstalledFormula {
            spec: FormulaSpec {
                name: "bar".to_string(),
                bottle_sha256: "barsha".to_string(),
                bottle_url: "https://example.invalid/bar.tar.gz".to_string(),
            },
            keg_dir_name: "1.0.0".to_string(),
            archive_path: PathBuf::new(),
        };
        write_receipt_with_owned_paths(
            &plan.receipt_path("bar"),
            &bar,
            "arm64_tahoe",
            vec!["lib/bar.txt".to_string()],
        )
        .unwrap();

        let foo_archive = temp.path().join("foo-new.tar.gz");
        write_test_bottle_archive(&foo_archive, "foo", "2.0.0", &[("bin/foo", b"new")]);
        let new_foo = InstalledFormula {
            spec: FormulaSpec {
                name: "foo".to_string(),
                bottle_sha256: "newsha".to_string(),
                bottle_url: "https://example.invalid/foo-new.tar.gz".to_string(),
            },
            keg_dir_name: "2.0.0".to_string(),
            archive_path: foo_archive,
        };
        let installs = vec![bar.clone(), new_foo.clone()];
        let rewrite_rules = build_rewrite_rules(&plan, &installs);

        install_package(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &plan,
            &installs,
            std::slice::from_ref(&new_foo),
            &rewrite_rules,
            None,
        )
        .unwrap();

        assert_eq!(fs::read(plan.install_root.join("bin/foo")).unwrap(), b"new");
        assert!(plan.install_root.join("lib/bar.txt").is_file());
        assert!(!plan.install_root.join("bin/stale").exists());
        assert_eq!(
            load_install_receipt(&plan.receipt_path("bar"))
                .unwrap()
                .unwrap()
                .version,
            "1.0.0"
        );
        assert_eq!(
            load_install_receipt(&plan.receipt_path("foo"))
                .unwrap()
                .unwrap()
                .version,
            "2.0.0"
        );
    }

    #[test]
    fn install_package_incremental_removes_dropped_dependency() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "foo".to_string(),
            root_formula: "foo".to_string(),
            stable_root: temp.path().join("opt/foo"),
            install_root: temp.path().join("opt/foo"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(plan.install_root.join("share/baz")).unwrap();
        fs::write(plan.install_root.join("bin/foo"), b"foo").unwrap();
        fs::write(plan.install_root.join("share/baz/data"), b"baz").unwrap();
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "foo".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: "foo".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        let foo = InstalledFormula {
            spec: FormulaSpec {
                name: "foo".to_string(),
                bottle_sha256: "foosha".to_string(),
                bottle_url: "https://example.invalid/foo.tar.gz".to_string(),
            },
            keg_dir_name: "1.0.0".to_string(),
            archive_path: PathBuf::new(),
        };
        write_receipt_with_owned_paths(
            &plan.receipt_path("foo"),
            &foo,
            "arm64_tahoe",
            vec!["bin/foo".to_string()],
        )
        .unwrap();
        let baz = InstalledFormula {
            spec: FormulaSpec {
                name: "baz".to_string(),
                bottle_sha256: "bazsha".to_string(),
                bottle_url: "https://example.invalid/baz.tar.gz".to_string(),
            },
            keg_dir_name: "1.0.0".to_string(),
            archive_path: PathBuf::new(),
        };
        write_receipt_with_owned_paths(
            &plan.receipt_path("baz"),
            &baz,
            "arm64_tahoe",
            vec!["share/baz".to_string(), "share/baz/data".to_string()],
        )
        .unwrap();

        let rewrite_rules = build_rewrite_rules(&plan, std::slice::from_ref(&foo));
        install_package(
            &Config {
                bottle_tag: "arm64_tahoe".to_string(),
            },
            &plan,
            std::slice::from_ref(&foo),
            &[],
            &rewrite_rules,
            None,
        )
        .unwrap();

        assert!(plan.install_root.join("bin/foo").is_file());
        assert!(!plan.install_root.join("share/baz").exists());
        assert!(
            load_install_receipt(&plan.receipt_path("baz"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn root_payload_ownership_replaces_root_files_without_removing_dependencies() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "npm:tool".to_string(),
            root_formula: "npm:tool".to_string(),
            stable_root: temp.path().join("opt/npm/tool"),
            install_root: temp.path().join("opt/npm/tool"),
            tmp_root: temp.path().join("tmp"),
        };
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::create_dir_all(plan.install_root.join("lib")).unwrap();
        fs::write(plan.install_root.join("bin/tool"), b"old").unwrap();
        fs::write(plan.install_root.join("lib/dependency"), b"dep").unwrap();
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: "npm:tool".to_string(),
                version: "1.0.0".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "tool".to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();
        write_root_ownership_manifest(&plan, vec!["bin/tool".to_string()]).unwrap();

        let before = prepare_root_payload_install(&plan).unwrap();
        fs::create_dir_all(plan.install_root.join("bin")).unwrap();
        fs::write(plan.install_root.join("bin/tool"), b"new").unwrap();
        finish_root_payload_install(&plan, before).unwrap();

        assert_eq!(
            fs::read(plan.install_root.join("bin/tool")).unwrap(),
            b"new"
        );
        assert_eq!(
            fs::read(plan.install_root.join("lib/dependency")).unwrap(),
            b"dep"
        );
        assert_eq!(
            load_root_ownership_manifest(&plan.root_ownership_manifest_path())
                .unwrap()
                .unwrap()
                .stubs,
            vec!["bin/tool".to_string()]
        );
    }

    #[test]
    fn path_and_process_helpers_cover_remaining_utility_branches() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("readonly");
        fs::write(&path, b"text").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&path, permissions).unwrap();
        ensure_writable(&path).unwrap();
        assert_ne!(fs::metadata(&path).unwrap().permissions().mode() & 0o200, 0);

        assert_eq!(
            normalize_path(Path::new("/opt/homebrew/Cellar/../opt/./sqlite")),
            PathBuf::from("/opt/homebrew/opt/sqlite")
        );
        assert_eq!(
            relative_path_from(
                Path::new("/opt/sqlite/bin"),
                Path::new("/opt/sqlite/share/doc")
            ),
            PathBuf::from("../share/doc")
        );
        assert_eq!(
            relative_path_from(Path::new("relative"), Path::new("/absolute/path")),
            PathBuf::from("/absolute/path")
        );
        assert_eq!(
            source_keg_root(Path::new("/tmp/root/sqlite/3.49.1")).unwrap(),
            PathBuf::from("/opt/homebrew/Cellar/sqlite/3.49.1")
        );
        assert_eq!(
            homebrew_relative_symlink_source(Path::new("/opt/homebrew/opt/sqlite/bin/sqlite3")),
            Some("@@HOMEBREW_PREFIX@@/opt/sqlite/bin/sqlite3".to_string())
        );
        assert_eq!(
            homebrew_relative_symlink_source(Path::new(
                "/opt/homebrew/Cellar/sqlite/3.49.1/bin/sqlite3"
            )),
            Some("@@HOMEBREW_CELLAR@@/sqlite/3.49.1/bin/sqlite3".to_string())
        );
        assert!(!is_macho(b"abc"));
        assert!(codesign_if_macho(Path::new("/tmp/not-macho"), b"#!/bin/sh\n", None).is_ok());

        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("printf 'hello\\n'; printf 'warn\\n' >&2");
        let output = run_command_with_logged_output(&mut command, None, "test command").unwrap();
        assert!(output.status.success());
        assert!(output.lines.iter().any(|line| line == "hello"));
        assert!(output.lines.iter().any(|line| line == "warn"));
        assert_eq!(
            format_command_output_suffix(&["".to_string(), "warn".to_string()]),
            ": warn".to_string()
        );
    }

    fn fake_vendor_version() -> Result<semver::Version, String> {
        Ok(semver::Version::parse("0.0.0").unwrap())
    }

    fn fake_vendor_install_strategy(_version: &semver::Version) -> vendor::InstallStrategy {
        vendor::InstallStrategy::CopyTree {
            source: "ignored".to_string(),
        }
    }

    fn fake_qmd_install_strategy(_version: &semver::Version) -> vendor::InstallStrategy {
        vendor::InstallStrategy::NpmGlobal {
            package: "@tobilu/qmd".to_string(),
        }
    }

    fn coverage_vendor_install_strategy(_version: &semver::Version) -> vendor::InstallStrategy {
        vendor::InstallStrategy::CopyFile {
            source: "pkg/bin/coverage-vendor".to_string(),
            destination_dir: "bin".to_string(),
            destination_name: Some("coverage-vendor".to_string()),
            mode: 0o755,
            create_dirs: vec!["bin".to_string()],
        }
    }

    static TEST_DOWNLOAD_URLS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

    struct TestEnvGuard {
        previous: Vec<(String, Option<OsString>)>,
    }

    impl TestEnvGuard {
        fn set(values: &[(&str, &str)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, value)| {
                    let previous = env::var_os(key);
                    unsafe {
                        env::set_var(key, value);
                    }
                    ((*key).to_string(), previous)
                })
                .collect();
            Self { previous }
        }

        fn unset(keys: &[&str]) -> Self {
            let previous = keys
                .iter()
                .map(|key| {
                    let previous = env::var_os(key);
                    unsafe {
                        env::remove_var(key);
                    }
                    ((*key).to_string(), previous)
                })
                .collect();
            Self { previous }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                match previous {
                    Some(value) => unsafe {
                        env::set_var(&key, value);
                    },
                    None => unsafe {
                        env::remove_var(&key);
                    },
                }
            }
        }
    }

    struct CurrentDirGuard(PathBuf);

    impl CurrentDirGuard {
        fn set(path: &Path) -> Self {
            let previous = env::current_dir().unwrap();
            env::set_current_dir(path).unwrap();
            Self(previous)
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.0).unwrap();
        }
    }

    struct TestEndpointGuard;

    impl TestEndpointGuard {
        fn set(overrides: config::TestEndpointOverrides) -> Self {
            config::set_test_endpoint_overrides(overrides);
            Self
        }
    }

    impl Drop for TestEndpointGuard {
        fn drop(&mut self) {
            config::clear_test_endpoint_overrides();
        }
    }

    fn drain_test_server(base: &str, path: &str, attempts: usize) {
        let url = format!("{base}{path}");
        for _ in 0..attempts {
            let _ = ureq::get(&url).call();
        }
    }

    fn test_env_lock() -> &'static Mutex<()> {
        crate::global_test_env_lock()
    }

    fn test_download_urls() -> &'static Mutex<HashMap<String, String>> {
        TEST_DOWNLOAD_URLS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn register_test_download_url(version: &Version, url: String) {
        test_download_urls()
            .lock()
            .unwrap()
            .insert(version.to_string(), url);
    }

    fn test_download_url(version: &Version) -> String {
        test_download_urls()
            .lock()
            .unwrap()
            .get(&version.to_string())
            .cloned()
            .unwrap()
    }

    fn fake_vendor_install(
        name: &'static str,
        executables: &'static [&'static str],
        version: &str,
    ) -> VendorInstall {
        VendorInstall {
            package: vendor::VendorPackage {
                name,
                dependencies: &[],
                executables,
                version: fake_vendor_version,
                download_url: None,
                install: fake_vendor_install_strategy,
            },
            version: semver::Version::parse(version).unwrap(),
        }
    }

    fn write_test_bottle_archive(
        archive_path: &Path,
        formula: &str,
        keg_dir: &str,
        files: &[(&str, &[u8])],
    ) {
        let file = File::create(archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);

        for (path, contents) in files {
            let archive_path = format!("{formula}/{keg_dir}/{path}");
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive
                .append_data(&mut header, archive_path, *contents)
                .unwrap();
        }

        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn write_test_archive(archive_path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);

        for (path, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, *path, *contents).unwrap();
        }

        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn write_test_plain_tar_archive(archive_path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(archive_path).unwrap();
        let mut archive = tar::Builder::new(file);

        for (path, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, *path, *contents).unwrap();
        }

        archive.finish().unwrap();
    }

    fn start_test_http_server(
        routes: Vec<(String, Vec<u8>)>,
        requests: usize,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let routes = Arc::new(routes.into_iter().collect::<HashMap<_, _>>());
        let handle = thread::spawn(move || {
            for _ in 0..requests {
                let (stream, _) = listener.accept().unwrap();
                respond_to_test_http_request(stream, routes.as_ref());
            }
        });
        (format!("http://{address}"), handle)
    }

    struct CountingTestHttpServer {
        base_url: String,
        requests: Arc<Mutex<usize>>,
        shutdown: std::sync::mpsc::Sender<()>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl CountingTestHttpServer {
        fn request_count(&self) -> usize {
            *self.requests.lock().unwrap()
        }

        fn stop(&mut self) -> thread::Result<()> {
            let Some(handle) = self.handle.take() else {
                return Ok(());
            };
            let _ = self.shutdown.send(());
            handle.join()
        }
    }

    impl Drop for CountingTestHttpServer {
        fn drop(&mut self) {
            let _ = self.stop();
        }
    }

    fn start_counting_test_http_server(routes: Vec<(String, Vec<u8>)>) -> CountingTestHttpServer {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let routes = Arc::new(routes.into_iter().collect::<HashMap<_, _>>());
        let requests = Arc::new(Mutex::new(0));
        let thread_requests = Arc::clone(&requests);
        let (shutdown, shutdown_rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            loop {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        *thread_requests.lock().unwrap() += 1;
                        respond_to_test_http_request(stream, routes.as_ref());
                    }
                    Err(err)
                        if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) =>
                    {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(err) => panic!("failed to accept test HTTP request: {err}"),
                }
            }
        });
        CountingTestHttpServer {
            base_url: format!("http://{address}"),
            requests,
            shutdown,
            handle: Some(handle),
        }
    }

    fn respond_to_test_http_request(
        mut stream: std::net::TcpStream,
        routes: &HashMap<String, Vec<u8>>,
    ) {
        stream.set_nonblocking(false).unwrap();
        let mut buffer = [0u8; 4096];
        let count = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..count]);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap();
        let (status, body) = match routes.get(path) {
            Some(body) => ("200 OK", body.clone()),
            None => ("404 Not Found", Vec::new()),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
    }

    fn start_test_etag_server(
        requests: Arc<Mutex<Vec<String>>>,
        body: Vec<u8>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_string();
                requests.lock().unwrap().push(request);

                if index == 0 {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nETag: \"test-etag\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    stream.write_all(&body).unwrap();
                    stream.flush().unwrap();
                } else {
                    stream
                        .write_all(
                            b"HTTP/1.1 304 Not Modified\r\nETag: \"test-etag\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .unwrap();
                    stream.flush().unwrap();
                }
            }
        });
        (format!("http://{address}"), handle)
    }

    fn test_combined_data_json() -> Vec<u8> {
        test_combined_data_json_with_db_schema(DB_SCHEMA_VERSION)
    }

    fn test_combined_data_with_generated_at(generated_at: &str) -> CombinedData {
        serde_json::from_value(serde_json::json!({
            "schema": 1,
            "generated_at": generated_at,
            "sources": {
                "db": {
                    "schema": DB_SCHEMA_VERSION,
                    "generated_at": generated_at,
                    "entries": {},
                    "npms": {}
                },
                "isotopes": {},
                "npm": {},
                "pip": {},
                "stub_exclusions": {}
            }
        }))
        .unwrap()
    }

    fn test_combined_data_json_with_db_schema(db_schema: u32) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": 1,
            "generated_at": "2026-05-05T00:00:00Z",
            "sources": {
                "db": {
                    "schema": db_schema,
                    "generated_at": "2026-05-05T00:00:00Z",
                    "entries": {},
                    "npms": {}
                },
                "isotopes": {},
                "npm": {},
                "pip": {},
                "stub_exclusions": {}
            }
        }))
        .unwrap()
    }
}
