#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = transifex_root_config_path()?;
    if path.exists() && root_config_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Transifex root config contains plaintext credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn transifex_root_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".transifexrc"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn root_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(line_has_secret)
}

fn line_has_secret(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return false;
    }
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    let key = key.trim();
    let value = unquote(value.trim());
    matches!(key, "token" | "password")
        && !value.is_empty()
        && value != "__api_token__"
        && value != "__password_or_api_token__"
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tokens_and_legacy_passwords() {
        assert!(root_config_contains_secret(
            "[https://app.transifex.com]\nrest_hostname = https://rest.api.transifex.com\ntoken = fake-token\n"
        ));
        assert!(root_config_contains_secret(
            "[https://www.transifex.com]\npassword = \"fake-password\"\n"
        ));
    }

    #[test]
    fn ignores_comments_empty_values_and_placeholders() {
        assert!(!root_config_contains_secret(
            "# token = fake-token\n[host]\ntoken =\npassword = __password_or_api_token__\n"
        ));
        assert!(!root_config_contains_secret(
            "[host]\ntoken = __api_token__\n"
        ));
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
    super::radioisotope::findings("transifex-cli", install_insecurity_reasons, home)
}
