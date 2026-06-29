#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_token_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = read_to_string(&path)?;
        if galaxy_token_yaml_contains_token(&contents) {
            reasons.push(format!(
                "Ansible Galaxy token file contains a plaintext token: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_token_paths() -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    if let Some(path) =
        std::env::var_os("ANSIBLE_GALAXY_TOKEN_PATH").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(path));
    }
    paths.push(home_dir()?.join(".ansible/galaxy_token"));
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

fn galaxy_token_yaml_contains_token(contents: &str) -> bool {
    galaxy_token_value(contents).is_some()
}

fn galaxy_token_value(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        if key.trim() != "token" {
            continue;
        }
        let value = value.trim();
        if value.is_empty() || value.starts_with('#') {
            continue;
        }
        let value = value.trim_matches('"').trim_matches('\'').trim();
        if value.is_empty() || value == "null" || value == "~" {
            continue;
        }
        return Some(value.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_galaxy_token() {
        assert!(galaxy_token_yaml_contains_token("token: abc123\n"));
        assert!(galaxy_token_yaml_contains_token("token: 'abc123'\n"));
        assert!(!galaxy_token_yaml_contains_token("token: null\n"));
        assert!(!galaxy_token_yaml_contains_token("# token: abc123\n"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_token_path = std::env::var_os("ANSIBLE_GALAXY_TOKEN_PATH");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("ANSIBLE_GALAXY_TOKEN_PATH");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_token_path {
                Some(value) => std::env::set_var("ANSIBLE_GALAXY_TOKEN_PATH", value),
                None => std::env::remove_var("ANSIBLE_GALAXY_TOKEN_PATH"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("ansible", install_insecurity_reasons, home)
}
