#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut paths = candidate_secret_paths()?;
    paths.sort();
    paths.dedup();

    let mut reasons = Vec::new();
    for path in paths {
        if path_has_nonempty_secret(&path)? {
            reasons.push(format!(
                "Atuin sync secret is stored in plaintext: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_secret_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let data_dir = xdg_data_home().unwrap_or_else(|| home.join(".local/share"));
    let mut paths = vec![
        data_dir.join("atuin/key"),
        data_dir.join("atuin/session"),
        home.join(".local/share/atuin/key"),
        home.join(".local/share/atuin/session"),
    ];

    for config in candidate_config_files()? {
        if !config.exists() {
            continue;
        }
        let contents = read_to_string(&config)?;
        for key in ["key_path", "session_path"] {
            if let Some(value) = assignment_value(&contents, key) {
                paths.push(expand_home_path(&value)?);
            }
        }
    }
    Ok(paths)
}

fn candidate_config_files() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let config_home = xdg_config_home().unwrap_or_else(|| home.join(".config"));
    Ok(vec![
        config_home.join("atuin/config.toml"),
        home.join(".config/atuin/config.toml"),
    ])
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn xdg_data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn path_has_nonempty_secret(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let contents = read_to_string(path)?;
    Ok(!contents.trim().is_empty() && !looks_like_placeholder(contents.trim()))
}

fn assignment_value(contents: &str, name: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == name {
            return Some(trim_quotes(value.trim()).to_string());
        }
    }
    None
}

fn expand_home_path(value: &str) -> Result<PathBuf, String> {
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(PathBuf::from(value))
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

fn looks_like_placeholder(value: &str) -> bool {
    value.contains("${") || value.contains("YOUR_") || value.eq_ignore_ascii_case("changeme")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_default_key_file() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("atuin-detect-{}", std::process::id()));
        let key = home.join(".local/share/atuin/key");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(key.parent().unwrap()).unwrap();
        std::fs::write(&key, "atuin-secret\n").unwrap();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_DATA_HOME");
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg_data {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("atuin", install_insecurity_reasons, home)
}
