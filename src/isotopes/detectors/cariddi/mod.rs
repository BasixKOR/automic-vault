#![allow(dead_code)]

pub(crate) mod persisted_output;
pub(crate) mod shell_history;

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_output_paths()? {
        if path.exists() && file_has_real_contents(&path)? {
            reasons.push(format!(
                "cariddi default output can contain discovered secrets: {}",
                path.display()
            ));
        }
    }
    for path in shell_history_paths()? {
        if path.exists() && history_mentions_sensitive_cariddi_args(&read_to_string(&path)?) {
            reasons.push(format!(
                "Shell history contains cariddi header or custom secret-scanner arguments: {}",
                path.display()
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn reasons_matching(prefix: &str) -> Result<Vec<String>, String> {
    Ok(install_insecurity_reasons()?
        .into_iter()
        .filter(|reason| reason.starts_with(prefix))
        .collect())
}

fn candidate_output_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![home.join("output-cariddi/secrets")];
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("output-cariddi/secrets"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
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

fn file_has_real_contents(path: &Path) -> Result<bool, String> {
    Ok(read_to_string(path)?.lines().any(|line| {
        let line = line.trim();
        !line.is_empty() && !line.eq_ignore_ascii_case("secret")
    }))
}

fn history_mentions_sensitive_cariddi_args(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.contains("cariddi")
            && (line.contains("-headers")
                || line.contains("-headersfile")
                || line.contains(" -sf ")
                || line.contains(" -s -sf "))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_history_flags() {
        assert!(history_mentions_sensitive_cariddi_args(
            "cat urls | cariddi -headers 'Cookie: auth=yes'\n"
        ));
        assert!(!history_mentions_sensitive_cariddi_args(
            "cat urls | cariddi -json\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("cariddi", install_insecurity_reasons, home)
}
