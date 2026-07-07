#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in shell_history_paths()? {
        if path.exists() && history_mentions_sshpass_password(&read_to_string(&path)?) {
            reasons.push(format!(
                "Shell history contains sshpass password material: {}",
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

fn history_mentions_sshpass_password(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.contains("sshpass")
            && (line.contains(" -p ")
                || line.contains(" -e ")
                || line.contains("SSHPASS=")
                || line.contains("--password"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sshpass_password_history() {
        assert!(history_mentions_sshpass_password(
            "sshpass -p hunter2 ssh user@example\n"
        ));
        assert!(history_mentions_sshpass_password(
            "SSHPASS=hunter2 sshpass -e ssh user@example\n"
        ));
        assert!(!history_mentions_sshpass_password("ssh user@example\n"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("sshpass", install_insecurity_reasons, home)
}
