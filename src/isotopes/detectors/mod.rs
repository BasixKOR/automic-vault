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
mod gh_cli;
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
mod sudo;
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
    documentation: &'static str,
}

macro_rules! detector {
    ($module:ident) => {
        Detector {
            module: stringify!($module),
            findings: $module::findings,
            documentation: include_str!(concat!(stringify!($module), "/detector.md")),
        }
    };
    ($package:ident::$module:ident, $name:literal) => {
        Detector {
            module: $name,
            findings: $package::$module::findings,
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
        findings.extend((detector.findings)(home));
    }
    findings
}

pub(crate) fn metadata() -> Vec<DetectorMetadata> {
    DETECTORS
        .iter()
        .map(|detector| {
            let name = detector_name(detector.module);
            DetectorMetadata {
                documentation: detector.documentation,
                homepage: detector_homepage(&name),
                docs_url: detector_docs_url(&name),
                name,
            }
        })
        .collect()
}

fn detector_name(module: &str) -> String {
    match module {
        "mysql_8_0" => "mysql@8.0".to_string(),
        "mysql_8_4" => "mysql@8.4".to_string(),
        "node_18" => "node@18".to_string(),
        "openssl_3" => "openssl@3".to_string(),
        _ => module.replace('_', "-"),
    }
}

fn detector_homepage(name: &str) -> String {
    match name {
        "git-credential-fill" | "git-credential-oauth" | "git-credentials-file" => {
            "https://git-scm.com/".to_string()
        }
        _ => detector_docs_url(name),
    }
}

fn detector_docs_url(name: &str) -> String {
    format!("https://github.com/automic-vault/radioisotopes/tree/main/{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_runs_every_registered_isotope() {
        assert_eq!(DETECTORS.len(), 156);
    }

    #[test]
    fn metadata_names_detectors() {
        let names = metadata()
            .into_iter()
            .map(|detector| detector.name)
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
        assert!(names.contains(&"sip".to_string()));
        assert!(names.contains(&"mysql@8.0".to_string()));
        assert!(names.contains(&"sudo".to_string()));
        assert!(names.contains(&"terraform-core".to_string()));
    }
}
