use std::path::Path;

use crate::Finding;

mod acli;
mod akamai;
mod algolia;
mod aliyun_cli;
mod ansible;
mod argocd;
mod ast_cli;
mod astra;
mod atuin;
mod aws_cli;
mod aws_sso_cli;
mod aws_vault;
mod azure_cli;
mod bash;
mod bitwarden_cli;
mod buf;
mod bun;
mod cariddi;
mod censys;
mod certbot;
mod checkov;
mod circleci;
mod civo;
mod cloudflare_wrangler;
mod cloudflared;
mod cloudsmith_cli;
pub(crate) mod codex;
mod composer;
mod curl;
mod databricks;
mod dcos_cli;
mod docker;
mod docker_credential_helper;
mod docker_machine;
mod doctl;
mod dropbox_uploader;
mod envchain;
mod fastlane;
mod fastly;
mod fauna_shell;
mod firebase_cli;
mod flyctl;
mod gallery_dl;
mod gcli;
pub(crate) mod gh_cli;
mod git;
mod glab;
mod goat;
mod gotify;
mod gptcommit;
mod grafanactl;
mod graphite;
mod hcloud;
mod helm;
mod heroku;
mod homebrew;
mod httpie;
mod huggingface_cli;
mod imap_backup;
mod jfrog_cli;
mod js_release_age;
mod k6;
mod kubernetes_cli;
mod luarocks;
mod macos;
mod maestro;
mod mariadb;
mod maven;
mod mcp_remote;
mod mercurial;
mod midnight_commander;
mod minio_mc;
mod mkcert;
mod mongodb_atlas_cli;
mod mycli;
mod mysql;
mod mysql_8_0;
mod mysql_8_4;
mod mysql_client;
mod netlify_cli;
mod node;
mod node_18;
mod npm;
mod nuget;
mod oauth2l;
mod oci_cli;
mod opencode;
mod openhue_cli;
mod openssh;
mod openssl_3;
mod openstackclient;
mod opentofu;
mod openvpn;
mod ordercli;
mod ossutil;
mod oxide_cli;
mod perl;
mod phylum_cli;
mod pianobar;
mod plumber;
mod pnpm;
mod podman;
mod poetry;
mod pulumi;
mod qwen_code;
mod radioisotope;
mod railway;
mod rclone;
mod rsync;
mod ruby;
mod runpodctl;
mod rust;
mod s3cmd;
mod sbt;
mod secretlint;
mod sentry_cli;
mod shodan;
mod sip;
mod skopeo;
mod snowflake_cli;
mod snyk;
mod soracom_cli;
mod sqlcmd;
mod sshpass;
mod sslmate;
mod stripe_cli;
pub(crate) mod sudo;
mod supabase;
mod tailscale;
mod talosctl;
mod terraform;
mod terraform_core;
mod todoist_cli;
mod transifex_cli;
mod travis;
mod twine;
mod uaa_cli;
mod uv;
mod vagrant;
mod vault;
mod vercel_cli;
mod virustotal_cli;
mod vultr;
mod wakatime_cli;
mod wget;
mod wget2;
mod wsk;
mod yarn;
mod yt_dlp;
mod zsh;

pub(crate) struct DetectorMetadata {
    pub(crate) name: String,
    pub(crate) homepage: String,
    pub(crate) docs_url: String,
    pub(crate) documentation: &'static str,
}

struct Detector {
    module: &'static str,
    findings: fn(&Path) -> Vec<Finding>,
    docs_url: &'static str,
    documentation: &'static str,
}

#[cfg(test)]
const DOCS_BASE: &str =
    "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/";

macro_rules! detector {
    ($module:ident) => {
        Detector {
            module: stringify!($module),
            findings: $module::findings,
            docs_url: concat!(
                "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/",
                stringify!($module),
                "/detector.md"
            ),
            documentation: include_str!(concat!(stringify!($module), "/detector.md")),
        }
    };
    ($package:ident::$module:ident, $name:literal) => {
        Detector {
            module: $name,
            findings: $package::$module::findings,
            docs_url: concat!(
                "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/",
                stringify!($package),
                "/",
                stringify!($module),
                ".md"
            ),
            documentation: include_str!(concat!(
                stringify!($package),
                "/",
                stringify!($module),
                ".md"
            )),
        }
    };
}

const DETECTORS: &[Detector] = &[
    detector!(acli),
    detector!(akamai),
    detector!(algolia),
    detector!(aliyun_cli),
    detector!(ansible),
    detector!(argocd),
    detector!(ast_cli),
    detector!(astra),
    detector!(atuin),
    detector!(aws_cli::credentials_file, "aws-cli-credentials-file"),
    detector!(aws_cli::legacy_plugins, "aws-cli-legacy-plugins"),
    detector!(aws_cli::login_cache, "aws-cli-login-cache"),
    detector!(aws_sso_cli),
    detector!(aws_vault),
    detector!(azure_cli),
    detector!(bash),
    detector!(bitwarden_cli),
    detector!(buf),
    detector!(bun),
    detector!(cariddi::persisted_output, "cariddi-persisted-output"),
    detector!(cariddi::shell_history, "cariddi-shell-history"),
    detector!(censys),
    detector!(certbot),
    detector!(checkov),
    detector!(circleci),
    detector!(civo),
    detector!(cloudflare_wrangler),
    detector!(cloudflared),
    detector!(cloudsmith_cli),
    detector!(codex),
    detector!(composer),
    detector!(curl),
    detector!(databricks),
    detector!(dcos_cli),
    detector!(docker::credential_helpers, "docker-credential-helpers"),
    detector!(docker::registry_credentials, "docker-registry-credentials"),
    detector!(docker::root_access, "docker-root-access"),
    detector!(docker_credential_helper),
    detector!(docker_machine),
    detector!(doctl),
    detector!(dropbox_uploader),
    detector!(envchain),
    detector!(fastlane),
    detector!(fastly),
    detector!(fauna_shell),
    detector!(firebase_cli),
    detector!(flyctl),
    detector!(gallery_dl),
    detector!(gcli),
    detector!(gh_cli::hosts_token, "gh-cli-hosts-token"),
    detector!(gh_cli::keychain_access, "gh-cli-keychain-access"),
    detector!(git::credential_fill, "git-credential-fill"),
    detector!(git::credential_oauth, "git-credential-oauth"),
    detector!(git::credentials_file, "git-credentials-file"),
    detector!(glab),
    detector!(goat),
    detector!(gotify),
    detector!(gptcommit),
    detector!(grafanactl),
    detector!(graphite),
    detector!(hcloud),
    detector!(helm),
    detector!(heroku),
    detector!(homebrew),
    detector!(httpie),
    detector!(huggingface_cli),
    detector!(imap_backup),
    detector!(jfrog_cli),
    detector!(k6),
    detector!(kubernetes_cli),
    detector!(luarocks),
    detector!(maestro),
    detector!(macos),
    detector!(mariadb),
    detector!(maven),
    detector!(mcp_remote),
    detector!(mercurial),
    detector!(midnight_commander),
    detector!(minio_mc),
    detector!(mkcert),
    detector!(mongodb_atlas_cli),
    detector!(mycli),
    detector!(mysql),
    detector!(mysql_client),
    detector!(mysql_8_0),
    detector!(mysql_8_4),
    detector!(netlify_cli),
    detector!(node),
    detector!(node_18),
    detector!(npm),
    detector!(nuget),
    detector!(oauth2l),
    detector!(oci_cli),
    detector!(opencode),
    detector!(openhue_cli),
    detector!(openssh),
    detector!(openssl_3),
    detector!(openstackclient),
    detector!(opentofu),
    detector!(openvpn),
    detector!(ordercli),
    detector!(ossutil),
    detector!(oxide_cli),
    detector!(perl),
    detector!(phylum_cli),
    detector!(pianobar),
    detector!(plumber),
    detector!(pnpm::auth_token, "pnpm-auth-token"),
    detector!(pnpm::minimum_release_age, "pnpm-minimum-release-age"),
    detector!(podman),
    detector!(poetry),
    detector!(pulumi),
    detector!(qwen_code),
    detector!(railway),
    detector!(rclone),
    detector!(rsync),
    detector!(ruby),
    detector!(runpodctl),
    detector!(rust),
    detector!(s3cmd),
    detector!(sbt),
    detector!(secretlint::persisted_report, "secretlint-persisted-report"),
    detector!(secretlint::shell_history, "secretlint-shell-history"),
    detector!(sentry_cli),
    detector!(shodan),
    detector!(sip),
    detector!(skopeo),
    detector!(snowflake_cli),
    detector!(snyk),
    detector!(soracom_cli),
    detector!(sqlcmd),
    detector!(sshpass),
    detector!(sslmate),
    detector!(stripe_cli),
    detector!(sudo),
    detector!(supabase),
    detector!(tailscale),
    detector!(talosctl),
    detector!(terraform),
    detector!(terraform_core),
    detector!(todoist_cli),
    detector!(transifex_cli),
    detector!(travis),
    detector!(twine),
    detector!(uaa_cli),
    detector!(uv),
    detector!(vagrant),
    detector!(vault),
    detector!(vercel_cli),
    detector!(virustotal_cli),
    detector!(vultr),
    detector!(wakatime_cli),
    detector!(wget),
    detector!(wget2),
    detector!(wsk),
    detector!(yarn),
    detector!(yt_dlp),
    detector!(zsh),
];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for detector in DETECTORS {
        let mut detected = (detector.findings)(home);
        for finding in &mut detected {
            finding.homepage = detector.docs_url;
            finding.docs_url = detector.docs_url;
            if let Some(solution) = documented_solution(detector.documentation) {
                finding.solution = solution;
            }
        }
        findings.extend(detected);
    }
    merge_duplicate_shell_path_findings(findings)
}

const SHELL_PATH_FINDING_SOURCES: &[&str] = &["bash", "zsh"];
const SHELL_PATH_FINDING_MARKER: &str =
    "PATH has a user-writable directory before protected system directories";
const MERGED_SHELL_PATH_SOURCE: &str = "bash+zsh";
const MERGED_SHELL_PATH_SOLUTION: &str = "Shell startup files contain arbitrary user programs and shared environment configuration. Automic Vault cannot rewrite them without changing shell behavior or guessing which commands need each secret. Move the reported value with `av save KEY`, then inject it only into the command that needs it. For an unsafe `PATH`, move every protected system directory before the reported user-writable directories and remove empty or relative entries.";

/// `bash` and `zsh` each detect the same process-wide `$PATH` independently,
/// so an insecure directory is reported once per shell. Collapse those
/// duplicates into a single finding that names every affected shell instead
/// of doubling the audit for anyone with both shells configured.
fn merge_duplicate_shell_path_findings(findings: Vec<Finding>) -> Vec<Finding> {
    let mut merged: Vec<Finding> = Vec::with_capacity(findings.len());
    for finding in findings {
        let existing = shell_path_entry(&finding).and_then(|entry| {
            merged.iter_mut().find(|candidate| {
                candidate.source != finding.source && shell_path_entry(candidate) == Some(entry)
            })
        });
        match existing {
            Some(existing) => merge_shell_path_finding(existing),
            None => merged.push(finding),
        }
    }
    merged
}

/// The PATH entry a shell PATH finding reports, taken from the explanation
/// rather than `affected`: `affected` keeps only entries starting with `/` or
/// `~`, so it is empty for the relative and empty entries `path_security`
/// deliberately reports, and cannot distinguish one from another.
fn shell_path_entry(finding: &Finding) -> Option<&str> {
    if !SHELL_PATH_FINDING_SOURCES.contains(&finding.source) {
        return None;
    }
    finding
        .explanation
        .split_once(SHELL_PATH_FINDING_MARKER)?
        .1
        .strip_prefix(": ")
}

fn merge_shell_path_finding(existing: &mut Finding) {
    if let Some(entry) = shell_path_entry(existing) {
        existing.explanation = format!(
            "Bash and zsh PATH have a user-writable directory before protected system directories: {entry}"
        );
    }
    existing.source = MERGED_SHELL_PATH_SOURCE;
    existing.solution = MERGED_SHELL_PATH_SOLUTION.to_string();
}

pub(crate) fn documented_solution(documentation: &str) -> Option<String> {
    if let Some(mitigation) = documentation
        .split_once("## Mitigation")
        .map(|(_, section)| section)
        .and_then(|section| section.split("\n## ").next())
    {
        if let Some(command) = mitigation.lines().find(|line| line.contains("av harden ")) {
            return Some(format!("Run `{}`.", command.trim()));
        }
        let paragraph = first_paragraph(mitigation);
        if !paragraph.is_empty() {
            return Some(paragraph);
        }
    }
    documentation
        .split_once("## Why This is not Yet Hardened")
        .map(|(_, section)| section)
        .and_then(|section| section.split("\n## ").next())
        .map(first_paragraph)
        .filter(|solution| !solution.is_empty())
}

fn first_paragraph(section: &str) -> String {
    section
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn metadata() -> Vec<DetectorMetadata> {
    DETECTORS
        .iter()
        .map(|detector| {
            let name = detector_name(detector.module);
            DetectorMetadata {
                documentation: detector.documentation,
                homepage: detector.docs_url.to_string(),
                docs_url: detector.docs_url.to_string(),
                name,
            }
        })
        .collect()
}

fn detector_name(module: &str) -> String {
    match module {
        "mysql_8_0" => "mysql@8.0".to_string(),
        "mysql_8_4" => "mysql@8.4".to_string(),
        "macos" => "macOS".to_string(),
        "node_18" => "node@18".to_string(),
        "openssl_3" => "openssl@3".to_string(),
        _ => module.replace('_', "-"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_runs_every_registered_isotope() {
        assert_eq!(DETECTORS.len(), 158);
    }

    #[test]
    fn metadata_names_detectors() {
        let metadata = metadata();
        let names = metadata
            .iter()
            .map(|detector| detector.name.clone())
            .collect::<Vec<_>>();

        assert!(!names.contains(&"aws".to_string()));
        assert!(!names.contains(&"aws-cli".to_string()));
        assert!(names.contains(&"aws-cli-credentials-file".to_string()));
        assert!(names.contains(&"docker-root-access".to_string()));
        assert!(names.contains(&"pnpm-minimum-release-age".to_string()));
        assert!(!names.contains(&"git".to_string()));
        assert!(names.contains(&"git-credential-fill".to_string()));
        assert!(names.contains(&"git-credential-oauth".to_string()));
        assert!(names.contains(&"git-credentials-file".to_string()));
        assert!(names.contains(&"homebrew".to_string()));
        assert!(names.contains(&"macOS".to_string()));
        assert!(names.contains(&"sip".to_string()));
        assert!(names.contains(&"mysql@8.0".to_string()));
        assert!(names.contains(&"sudo".to_string()));
        assert!(names.contains(&"terraform-core".to_string()));
        assert_eq!(
            metadata
                .iter()
                .find(|detector| detector.name == "homebrew")
                .unwrap()
                .homepage,
            format!("{DOCS_BASE}homebrew/detector.md")
        );
        assert_eq!(
            metadata
                .iter()
                .find(|detector| detector.name == "git-credential-fill")
                .unwrap()
                .docs_url,
            format!("{DOCS_BASE}git/credential_fill.md")
        );
    }

    #[test]
    fn documentation_supplies_hardening_or_deferred_solution() {
        assert_eq!(
            documented_solution("## Mitigation\n\n```sh\nsudo av harden foo\n```"),
            Some("Run `sudo av harden foo`.".to_string())
        );
        assert_eq!(
            documented_solution("## Mitigation\n\nRemove the reported token.\nThen log in again."),
            Some("Remove the reported token. Then log in again.".to_string())
        );
        assert_eq!(
            documented_solution(
                "## Why This is not Yet Hardened\n\nFoo needs a temporary secret file.\nThat is not sufficient.\n\n## Sensitive Files"
            ),
            Some("Foo needs a temporary secret file. That is not sufficient.".to_string())
        );
    }

    fn shell_path_finding(shell: &'static str, path: &str) -> Finding {
        let explanation = format!(
            "{} PATH has a user-writable directory before protected system directories: {path}",
            if shell == "bash" { "Bash" } else { "Zsh" },
        );
        Finding {
            source: shell,
            homepage: "https://example.test/",
            severity: "high",
            // Built by the same function the detectors use, so relative and
            // empty entries have no affected path here either.
            affected: super::radioisotope::affected(&explanation),
            explanation,
            solution: format!("{shell} startup files contain arbitrary user programs."),
            docs_url: "https://example.test/docs.md",
        }
    }

    #[test]
    fn merges_bash_and_zsh_findings_for_the_same_path_directory() {
        let findings = vec![
            shell_path_finding("bash", "/opt/homebrew/bin"),
            shell_path_finding("zsh", "/opt/homebrew/bin"),
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "bash+zsh");
        assert_eq!(
            merged[0].explanation,
            "Bash and zsh PATH have a user-writable directory before protected system directories: /opt/homebrew/bin"
        );
        assert_eq!(merged[0].affected[0].path, "/opt/homebrew/bin");
    }

    #[test]
    fn keeps_shell_path_findings_for_different_directories_separate() {
        let findings = vec![
            shell_path_finding("bash", "/opt/homebrew/bin"),
            shell_path_finding("zsh", "/opt/homebrew/bin"),
            shell_path_finding("bash", "/Users/tester/.bun/bin"),
            shell_path_finding("zsh", "/Users/tester/.bun/bin"),
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|finding| finding.source == "bash+zsh"));
    }

    /// Regression: `PATH="first:second:/usr/bin:/bin"`. Both relative entries
    /// have an empty `affected`, so keying the merge on `affected` collapsed
    /// them into one finding and `second` disappeared from the scan.
    #[test]
    fn keeps_relative_path_entries_separate_when_merging() {
        let findings = vec![
            shell_path_finding("bash", "first"),
            shell_path_finding("bash", "second"),
            shell_path_finding("zsh", "first"),
            shell_path_finding("zsh", "second"),
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|finding| finding.source == "bash+zsh"));
        assert_eq!(
            merged[0].explanation,
            "Bash and zsh PATH have a user-writable directory before protected system directories: first"
        );
        assert_eq!(
            merged[1].explanation,
            "Bash and zsh PATH have a user-writable directory before protected system directories: second"
        );
    }

    #[test]
    fn merges_the_empty_path_entry_reported_as_a_dot() {
        let findings = vec![
            shell_path_finding("bash", "."),
            shell_path_finding("zsh", "."),
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "bash+zsh");
        assert_eq!(
            merged[0].explanation,
            "Bash and zsh PATH have a user-writable directory before protected system directories: ."
        );
        assert!(merged[0].affected.is_empty());
    }

    #[test]
    fn does_not_merge_two_findings_from_the_same_shell() {
        let findings = vec![
            shell_path_finding("bash", "relative"),
            shell_path_finding("bash", "relative"),
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|finding| finding.source == "bash"));
    }

    #[test]
    fn does_not_merge_a_lone_shell_path_finding() {
        let findings = vec![shell_path_finding("zsh", "/opt/homebrew/bin")];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "zsh");
    }

    #[test]
    fn does_not_merge_non_path_bash_and_zsh_findings() {
        let findings = vec![
            Finding {
                source: "bash",
                homepage: "https://example.test/",
                severity: "high",
                explanation: "Bash startup file contains plaintext-looking credential assignment: /home/user/.bashrc".to_string(),
                solution: "Move the reported value with `av save KEY`.".to_string(),
                affected: vec![crate::AffectedFile {
                    path: "/home/user/.bashrc".to_string(),
                    line: None,
                }],
                docs_url: "https://example.test/docs.md",
            },
            Finding {
                source: "zsh",
                homepage: "https://example.test/",
                severity: "high",
                explanation: "Zsh startup file contains plaintext-looking credential assignment: /home/user/.zshrc".to_string(),
                solution: "Move the reported value with `av save KEY`.".to_string(),
                affected: vec![crate::AffectedFile {
                    path: "/home/user/.zshrc".to_string(),
                    line: None,
                }],
                docs_url: "https://example.test/docs.md",
            },
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].source, "bash");
        assert_eq!(merged[1].source, "zsh");
    }

    #[test]
    fn does_not_merge_the_macos_gui_path_finding_with_shell_findings() {
        let findings = vec![
            shell_path_finding("bash", "/opt/homebrew/bin"),
            shell_path_finding("zsh", "/opt/homebrew/bin"),
            Finding {
                source: "macOS",
                homepage: "https://example.test/",
                severity: "high",
                explanation: "macOS GUI PATH has a user-writable directory before protected system directories: /opt/homebrew/bin".to_string(),
                solution: "Move protected system directories before user-writable directories in the launchd PATH.".to_string(),
                affected: vec![crate::AffectedFile {
                    path: "/opt/homebrew/bin".to_string(),
                    line: None,
                }],
                docs_url: "https://example.test/docs.md",
            },
        ];

        let merged = merge_duplicate_shell_path_findings(findings);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].source, "bash+zsh");
        assert_eq!(merged[1].source, "macOS");
    }
}
