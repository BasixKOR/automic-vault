use std::path::{Path, PathBuf};

use crate::path_security::{user_writable_entries_before_system_paths, user_writable_path_reason};

pub(crate) fn bash_reasons() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".bash_login"),
        home.join(".profile"),
    ];
    if let Some(path) = std::env::var_os("BASH_ENV").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(path));
    }
    let mut reasons = reasons(
        paths,
        "Bash startup file contains plaintext-looking credential assignment",
    )?;
    reasons.extend(path_reasons("Bash"));
    Ok(reasons)
}

pub(crate) fn zsh_reasons() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let root = std::env::var_os("ZDOTDIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(home);
    let mut reasons = reasons(
        [".zshenv", ".zprofile", ".zshrc", ".zlogin", ".zlogout"]
            .into_iter()
            .map(|file| root.join(file)),
        "Zsh startup file contains plaintext-looking credential assignment",
    )?;
    reasons.extend(path_reasons("Zsh"));
    Ok(reasons)
}

fn path_reasons(shell: &str) -> Vec<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    user_writable_entries_before_system_paths(&path)
        .into_iter()
        .map(|entry| user_writable_path_reason(shell, &entry))
        .collect()
}

fn reasons(paths: impl IntoIterator<Item = PathBuf>, message: &str) -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in paths {
        if path.exists() && file_contains_secret_assignment(&path)? {
            reasons.push(format!("{message}: {}", path.display()));
        }
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

fn file_contains_secret_assignment(path: &Path) -> Result<bool, String> {
    Ok(read_to_string(path)?
        .lines()
        .any(line_contains_secret_assignment))
}

fn line_contains_secret_assignment(line: &str) -> bool {
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    secret_key(key.trim()) && literal_secret_value(value.trim())
}

fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASS",
        "API_KEY",
        "ACCESS_KEY",
        "PRIVATE_KEY",
        "AUTH",
    ]
    .iter()
    .any(|word| key.contains(word))
}

fn literal_secret_value(value: &str) -> bool {
    let value = value
        .split(['#', ';'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    !value.is_empty()
        && value.len() >= 6
        && !value.contains('*')
        && !value.starts_with('$')
        && !value.starts_with('`')
        && !value.starts_with("$(")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_literal_secret_assignments_only() {
        assert!(line_contains_secret_assignment("export API_TOKEN=abcdef"));
        assert!(line_contains_secret_assignment("PASSWORD='correct horse'"));
        assert!(!line_contains_secret_assignment("TOKEN=$TOKEN"));
        assert!(!line_contains_secret_assignment("TOKEN=******"));
        assert!(!line_contains_secret_assignment("# TOKEN=abcdef"));
    }
}
