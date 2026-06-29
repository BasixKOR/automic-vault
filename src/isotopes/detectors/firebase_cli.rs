#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = firebase_configstore_path()?;
    if path.exists() && configstore_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Firebase CLI configstore contains plaintext tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn firebase_configstore_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".config/configstore/firebase-tools.json"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn configstore_contains_token(contents: &str) -> bool {
    ["refresh_token", "access_token"]
        .iter()
        .any(|field| json_string_field(contents, field).is_some_and(|value| !value.is_empty()))
}

fn json_string_field<'a>(contents: &'a str, field: &str) -> Option<&'a str> {
    let quoted = format!("\"{field}\"");
    let after_key = contents.split(&quoted).nth(1)?.split_once(':')?.1;
    after_key
        .trim_start()
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_refresh_token() {
        assert!(configstore_contains_token(
            r#"{"tokens":{"refresh_token":"secret","access_token":"access"}}"#
        ));
    }

    #[test]
    fn ignores_config_without_tokens() {
        assert!(!configstore_contains_token(
            r#"{"user":{"email":"me@example.com"}}"#
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("firebase-cli", install_insecurity_reasons, home)
}
