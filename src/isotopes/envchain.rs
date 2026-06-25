#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in shell_history_paths()? {
        if path.exists() && history_mentions_envchain_setup(&read_to_string(&path)?) {
            reasons.push(format!(
                "Shell history shows envchain namespaces storing environment secrets: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn shell_history_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".zsh_history"),
        home.join(".bash_history"),
        home.join(".history"),
    ])
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn history_mentions_envchain_setup(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.contains("envchain")
            && (line.contains(" --set ")
                || line.contains(" -s ")
                || line.contains(" --no-require-passphrase ")
                || line.contains(" --require-passphrase "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_envchain_setup_history() {
        assert!(history_mentions_envchain_setup(
            "envchain --set aws AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY\n"
        ));
        assert!(!history_mentions_envchain_setup("envchain aws env\n"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("envchain", install_insecurity_reasons, home)
}
