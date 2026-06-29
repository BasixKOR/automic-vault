#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = wsk_props_path()?;
    if path.exists() && wsk_auth_value(&read_to_string(&path)?).is_some() {
        return Ok(vec![format!(
            "OpenWhisk CLI properties contain a plaintext AUTH key: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn wsk_props_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".wskprops"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn wsk_auth_value(contents: &str) -> Option<String> {
    contents.lines().find_map(parse_auth_line)
}

fn parse_auth_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    if key.trim() != "AUTH" {
        return None;
    }

    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_auth_property() {
        assert_eq!(
            wsk_auth_value("APIHOST=https://openwhisk.example\nAUTH=fake-uuid:fake-secret\n"),
            Some("fake-uuid:fake-secret".to_string())
        );
    }

    #[test]
    fn ignores_empty_or_commented_auth() {
        assert_eq!(wsk_auth_value("# AUTH=fake-secret\nAUTH=\n"), None);
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_props_are_missing() {
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
    super::radioisotope::findings("wsk", install_insecurity_reasons, home)
}
