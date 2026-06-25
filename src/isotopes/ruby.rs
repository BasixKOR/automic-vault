#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = home_dir()?.join(".gem/credentials");
    if path.exists() && gem_credentials_contain_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "RubyGems credentials file contains plaintext API keys: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn gem_credentials_contain_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = trimmed.trim_start_matches(':').split_once(':') else {
            return false;
        };
        let key = key.trim().trim_start_matches(':').to_ascii_lowercase();
        key.contains("api_key") && secret_value_is_real(value.trim())
    })
}

fn secret_value_is_real(value: &str) -> bool {
    let value = value.trim_matches('"').trim_matches('\'').trim();
    value.len() >= 6
        && !value.contains("${")
        && !value.eq_ignore_ascii_case("example")
        && !value.eq_ignore_ascii_case("redacted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rubygems_api_keys() {
        assert!(gem_credentials_contain_secret(
            "---\n:rubygems_api_key: rubygems_secret\n"
        ));
        assert!(!gem_credentials_contain_secret(
            "# :rubygems_api_key: rubygems_secret\n:rubygems_api_key: ${TOKEN}\n"
        ));
    }

    #[test]
    fn top_level_detection_reports_credentials_file() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("ruby-detect-{}", std::process::id()));
        let credentials = home.join(".gem/credentials");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(credentials.parent().unwrap()).unwrap();
        std::fs::write(&credentials, ":rubygems_api_key: rubygems_secret\n").unwrap();
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
    super::radioisotope::findings("ruby", install_insecurity_reasons, home)
}
