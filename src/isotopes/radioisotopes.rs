use std::ffi::OsString;
use std::path::Path;
use std::sync::Mutex;

use crate::{AffectedFile, Finding};

const HIGH: &str = "high";
const SOLUTION: &str =
    "Review the reported plaintext secret and move or remove it; this radioisotope is detect-only.";

static ENV_LOCK: Mutex<()> = Mutex::new(());

macro_rules! import_detector {
    ($module:ident, $slug:literal) => {
        mod $module {
            #![allow(dead_code)]

            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../radioisotopes/",
                $slug,
                "/detect.rs"
            ));
        }
    };
}

import_detector!(atuin, "atuin");
import_detector!(aws_sso_cli, "aws-sso-cli");
import_detector!(aws_vault, "aws-vault");
import_detector!(azure_cli, "azure-cli");
import_detector!(cariddi, "cariddi");
import_detector!(certbot, "certbot");
import_detector!(cloudflare_wrangler, "cloudflare-wrangler");
import_detector!(cloudflared, "cloudflared");
import_detector!(curl, "curl");
import_detector!(databricks, "databricks");
import_detector!(docker, "docker");
import_detector!(docker_credential_helper, "docker-credential-helper");
import_detector!(docker_machine, "docker-machine");
import_detector!(envchain, "envchain");
import_detector!(fastlane, "fastlane");
import_detector!(httpie, "httpie");
import_detector!(mongodb_atlas_cli, "mongodb-atlas-cli");
import_detector!(oauth2l, "oauth2l");
import_detector!(opencode, "opencode");
import_detector!(openssh, "openssh");
import_detector!(openssl_3, "openssl@3");
import_detector!(openvpn, "openvpn");
import_detector!(perl, "perl");
import_detector!(pianobar, "pianobar");
import_detector!(poetry, "poetry");
import_detector!(rsync, "rsync");
import_detector!(ruby, "ruby");
import_detector!(secretlint, "secretlint");
import_detector!(sshpass, "sshpass");
import_detector!(stripe_cli, "stripe-cli");
import_detector!(tailscale, "tailscale");
import_detector!(vercel_cli, "vercel-cli");
import_detector!(wget, "wget");
import_detector!(wget2, "wget2");
import_detector!(yt_dlp, "yt-dlp");

struct Detector {
    name: &'static str,
    docs_url: &'static str,
    reasons: fn() -> Result<Vec<String>, String>,
}

macro_rules! detector {
    ($slug:literal, $module:ident) => {
        Detector {
            name: $slug,
            docs_url: concat!(
                "https://github.com/automic-vault/radioisotopes/tree/main/",
                $slug
            ),
            reasons: $module::install_insecurity_reasons,
        }
    };
}

const DETECTORS: &[Detector] = &[
    detector!("atuin", atuin),
    detector!("aws-sso-cli", aws_sso_cli),
    detector!("aws-vault", aws_vault),
    detector!("azure-cli", azure_cli),
    detector!("cariddi", cariddi),
    detector!("certbot", certbot),
    detector!("cloudflare-wrangler", cloudflare_wrangler),
    detector!("cloudflared", cloudflared),
    detector!("curl", curl),
    detector!("databricks", databricks),
    detector!("docker", docker),
    detector!("docker-credential-helper", docker_credential_helper),
    detector!("docker-machine", docker_machine),
    detector!("envchain", envchain),
    detector!("fastlane", fastlane),
    detector!("httpie", httpie),
    detector!("mongodb-atlas-cli", mongodb_atlas_cli),
    detector!("oauth2l", oauth2l),
    detector!("opencode", opencode),
    detector!("openssh", openssh),
    detector!("openssl@3", openssl_3),
    detector!("openvpn", openvpn),
    detector!("perl", perl),
    detector!("pianobar", pianobar),
    detector!("poetry", poetry),
    detector!("rsync", rsync),
    detector!("ruby", ruby),
    detector!("secretlint", secretlint),
    detector!("sshpass", sshpass),
    detector!("stripe-cli", stripe_cli),
    detector!("tailscale", tailscale),
    detector!("vercel-cli", vercel_cli),
    detector!("wget", wget),
    detector!("wget2", wget2),
    detector!("yt-dlp", yt_dlp),
];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let _lock = ENV_LOCK.lock().expect("radioisotope env lock poisoned");
    let _home = HomeEnv::set(home);

    let mut findings = Vec::new();
    for detector in DETECTORS {
        let Ok(reasons) = (detector.reasons)() else {
            continue;
        };
        findings.extend(reasons.into_iter().map(|reason| finding(detector, reason)));
    }
    findings
}

fn finding(detector: &Detector, reason: String) -> Finding {
    Finding {
        source: detector.name,
        homepage: detector.docs_url,
        severity: HIGH,
        affected: affected(&reason),
        explanation: reason,
        solution: SOLUTION.to_string(),
        docs_url: detector.docs_url,
    }
}

fn affected(reason: &str) -> Vec<AffectedFile> {
    reason
        .rsplit_once(": ")
        .map(|(_, path)| path.trim())
        .filter(|path| path.starts_with('/') || path.starts_with('~'))
        .map(|path| {
            vec![AffectedFile {
                path: path.to_string(),
                line: 1,
            }]
        })
        .unwrap_or_default()
}

struct HomeEnv {
    previous: Option<OsString>,
}

impl HomeEnv {
    fn set(home: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        // SAFETY: these imported detectors read HOME from process env; ENV_LOCK
        // serializes scan-time HOME changes and Drop restores the prior value.
        unsafe { std::env::set_var("HOME", home) };
        Self { previous }
    }
}

impl Drop for HomeEnv {
    fn drop(&mut self) {
        // SAFETY: still protected by ENV_LOCK while restoring HOME.
        unsafe {
            match &self.previous {
                Some(previous) => std::env::set_var("HOME", previous),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_detect_only_radioisotopes() {
        assert_eq!(DETECTORS.len(), 35);
        assert!(
            DETECTORS
                .iter()
                .any(|detector| detector.name == "stripe-cli")
        );
        assert!(!DETECTORS.iter().any(|detector| detector.name == "git"));
    }

    #[test]
    fn maps_reason_paths_to_affected_file() {
        assert_eq!(
            affected("Stripe CLI config contains plaintext API keys: /tmp/config.toml"),
            vec![AffectedFile {
                path: "/tmp/config.toml".to_string(),
                line: 1,
            }]
        );
    }
}
