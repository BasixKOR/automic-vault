use std::path::{Path, PathBuf};

use crate::{Finding, GIT_SOURCE, HIGH};

mod git_config;
mod git_credential_fill;
mod git_credential_oauth;
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
