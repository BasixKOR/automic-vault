#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in shell_history_paths()? {
        if path.exists() && history_mentions_unmasked_secretlint(&read_to_string(&path)?) {
            reasons.push(format!(
                concat!(
                    "Shell history contains Secretlint invocations that can expose ",
                    "unmasked secrets: {}"
                ),
                path.display()
            ));
        }
    }
    for path in candidate_report_paths()? {
        if path.exists() && secretlint_report_may_contain_findings(&read_to_string(&path)?) {
            reasons.push(format!(
                "Secretlint report may contain persisted secret findings: {}",
                path.display()
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
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

fn candidate_report_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join("secretlint-report.json"),
        home.join("secretlint-output.json"),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("secretlint-report.json"));
        paths.push(cwd.join("secretlint-output.json"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn history_mentions_unmasked_secretlint(contents: &str) -> bool {
    contents.lines().any(|line| {
        line.contains("secretlint")
            && (line.contains("--no-maskSecrets")
                || (line.contains("--format=mask-result") && line.contains("--output=")))
    })
}

fn secretlint_report_may_contain_findings(contents: &str) -> bool {
    contents.contains("\"messages\"")
        && (contents.contains("\"ruleId\"") || contents.contains("\"message\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unmasked_secretlint_history_and_reports() {
        assert!(history_mentions_unmasked_secretlint(
            "secretlint --no-maskSecrets '**/*'\n"
        ));
        assert!(secretlint_report_may_contain_findings(
            r#"{"messages":[{"ruleId":"@secretlint/rule","message":"found secret"}]}"#
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("secretlint", install_insecurity_reasons, home)
}
