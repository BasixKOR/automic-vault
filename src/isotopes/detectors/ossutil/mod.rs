#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = ossutil_config_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = read_to_string(&path)?;
    if config_has_sensitive_values(&contents) {
        return Ok(vec![format!(
            "ossutil config contains plaintext credentials: {}",
            path.display()
        )]);
    }

    Ok(Vec::new())
}

fn ossutil_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".ossutilconfig"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_sensitive_values(contents: &str) -> bool {
    contents.lines().any(line_has_sensitive_value)
}

fn line_has_sensitive_value(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return false;
    }

    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    let key = key.trim().to_ascii_lowercase();
    if !is_sensitive_key(&key) {
        return false;
    }

    let value = value.trim().trim_matches('"').trim_matches('\'');
    !value.is_empty()
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(key, "accesskeysecret" | "ststoken")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_access_key_secret_and_sts_token() {
        let contents = "\
[Credentials]\n\
accessKeyID = LTAIEXAMPLE\n\
accessKeySecret = very-secret\n\
stsToken = session-token\n";

        assert!(config_has_sensitive_values(contents));
    }

    #[test]
    fn ignores_empty_sensitive_values_and_comments() {
        let contents = "\
[Credentials]\n\
accessKeySecret = \"\"\n\
; stsToken = commented\n\
# accessKeySecret = commented\n";

        assert!(!config_has_sensitive_values(contents));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_location_is_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("ossutil", install_insecurity_reasons, home)
}
