//! Security isotope registry for `av scan`.
//!
//! What this runs:
//! - The small set of Git-related detectors currently shipped in this CLI.
//! - Each detector returns high-level findings without secret values.
//!
//! Why this exists:
//! - `av scan` needs one boring place to collect detector findings.
//! - A static registry is enough while the repo has only a few built-in
//!   detectors; no generator or plugin loader is justified yet.
//!
//! Evidence flow:
//! - The caller supplies the home directory being scanned.
//! - Detectors read only their own supported files or run their own bounded
//!   checks.
//! - Findings are appended in deterministic detector order.
//!
//! Known issues:
//! - There is no severity normalization beyond the shared `high` helper.
//! - Duplicate findings across detectors are not deduplicated yet.
//!
//! Known omissions:
//! - No package-management state, Homebrew ownership, remediation, or isotope
//!   installation logic lives here.
//! - There is no runtime discovery of detectors.
//!
//! Safety notes:
//! - This module should remain a composition point, not a detector itself.
//! - Keep detector side effects explicit in the detector module docs.

use std::path::{Path, PathBuf};

use crate::{Finding, GIT_SOURCE, HIGH};

mod git_config;
#[path = "git-credential-fill.rs"]
mod git_credential_fill;
#[path = "git-credential-oauth.rs"]
mod git_credential_oauth;
#[path = "git-credentials.rs"]
pub(crate) mod git_credentials;

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = git_credentials::findings(home);
    findings.extend(git_credential_fill::findings(home));
    findings.extend(git_credential_oauth::findings(home));
    findings
}

fn high(message: impl Into<String>) -> Finding {
    Finding {
        source: GIT_SOURCE,
        severity: HIGH,
        message: message.into(),
    }
}

fn git_config_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![home.join(".gitconfig")];
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(config_home).join("git/config"));
    } else {
        paths.push(home.join(".config/git/config"));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn read_to_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}
