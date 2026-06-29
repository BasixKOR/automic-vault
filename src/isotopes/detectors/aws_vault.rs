#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let mut reasons = Vec::new();
    let config = home.join(".aws/config");
    if config.exists() && aws_config_invokes_aws_vault(&read_to_string(&config)?) {
        reasons.push(format!(
            "AWS config invokes aws-vault as an ambient credential_process: {}",
            config.display()
        ));
    }

    let file_backend = home.join(".awsvault/keys");
    if file_backend.is_dir() && dir_has_files(&file_backend)? {
        reasons.push(format!(
            "aws-vault file backend directory contains credential vault files: {}",
            file_backend.display()
        ));
    }
    Ok(reasons)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn dir_has_files(path: &Path) -> Result<bool, String> {
    for entry in std::fs::read_dir(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if entry.path().is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn aws_config_invokes_aws_vault(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim() == "credential_process"
            && value.split_whitespace().any(|word| word == "aws-vault")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_credential_process() {
        assert!(aws_config_invokes_aws_vault(
            "[profile dev]\ncredential_process = aws-vault export --format=json dev\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("aws-vault", install_insecurity_reasons, home)
}
