#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut paths = common_password_paths()?;
    for config in config_paths()? {
        if config.exists() {
            paths.extend(secrets_files_from_config(&read_to_string(&config)?)?);
        }
    }
    paths.sort();
    paths.dedup();

    let mut reasons = Vec::new();
    for path in paths {
        if path.exists() && file_contains_secret(&path)? {
            reasons.push(format!(
                "rsync password file contains plaintext credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn common_password_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".rsync_pass"),
        home.join(".rsync-password"),
        home.join(".rsync.pass"),
    ])
}

fn config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".rsyncd.conf"),
        home.join(".config/rsync/rsyncd.conf"),
    ])
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn secrets_files_from_config(contents: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if key == "secrets file" || key == "password file" {
            paths.push(expand_home_path(value.trim())?);
        }
    }
    Ok(paths)
}

fn expand_home_path(value: &str) -> Result<PathBuf, String> {
    let value = value.trim_matches('"').trim_matches('\'');
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(PathBuf::from(value))
}

fn file_contains_secret(path: &Path) -> Result<bool, String> {
    let contents = read_to_string(path)?;
    Ok(contents.lines().any(|line| {
        let value = line.trim();
        !value.is_empty()
            && !value.starts_with('#')
            && value.len() >= 6
            && !value.contains("${")
            && !value.eq_ignore_ascii_case("password")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_secrets_file_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("rsync-home-{}", std::process::id()));
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let paths = secrets_files_from_config("secrets file = ~/.rsync-secrets\n").unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(paths, vec![home.join(".rsync-secrets")]);
    }

    #[test]
    fn top_level_detection_reports_common_password_file() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("rsync-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".rsync_pass"), "supersecret\n").unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("rsync", install_insecurity_reasons, home)
}
