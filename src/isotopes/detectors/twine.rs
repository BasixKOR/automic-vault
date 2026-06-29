#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = pypirc_path()?;
    if path.exists() && pypirc_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Twine config contains plaintext package index credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn pypirc_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".pypirc"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn pypirc_contains_secret(contents: &str) -> bool {
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
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    key == "password" || (key == "repository" && url_contains_userinfo(value))
}

fn url_contains_userinfo(value: &str) -> bool {
    let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return false;
    };
    let Some(userinfo_end) = rest.find('@') else {
        return false;
    };
    let host_end = rest.find('/').unwrap_or(rest.len());
    userinfo_end < host_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_password_and_repository_userinfo() {
        assert!(pypirc_contains_secret(
            "[pypi]\nusername = __token__\npassword = fake-token\n"
        ));
        assert!(pypirc_contains_secret(
            "[internal]\nrepository = https://user:fake@example.invalid/simple/\n"
        ));
    }

    #[test]
    fn ignores_comments_empty_values_and_plain_repository_urls() {
        assert!(!pypirc_contains_secret(
            "# password = fake\n[pypi]\npassword =\nrepository = https://example.invalid/simple/\n"
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
    super::radioisotope::findings("twine", install_insecurity_reasons, home)
}
