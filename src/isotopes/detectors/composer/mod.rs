#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    for path in candidate_auth_paths()? {
        if path.exists() && composer_auth_contains_secret(&read_to_string(&path)?) {
            return Ok(vec![format!(
                "Composer auth.json contains plaintext credentials: {}",
                path.display()
            )]);
        }
    }
    Ok(Vec::new())
}

fn candidate_auth_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(home) = std::env::var_os("COMPOSER_HOME").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(home).join("auth.json")]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = Vec::new();
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("composer/auth.json"));
    }
    paths.push(home.join(".composer/auth.json"));
    paths.push(home.join("Library/Application Support/Composer/auth.json"));
    Ok(paths)
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn composer_auth_contains_secret(contents: &str) -> bool {
    [
        "\"http-basic\"",
        "\"github-oauth\"",
        "\"gitlab-oauth\"",
        "\"gitlab-token\"",
        "\"bearer\"",
    ]
    .iter()
    .any(|key| contents.contains(key))
        && contents.contains(':')
        && contents.contains('{')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_http_basic_auth() {
        assert!(composer_auth_contains_secret(
            r#"{"http-basic":{"repo.example":{"username":"u","password":"p"}}}"#
        ));
    }

    #[test]
    fn detects_github_oauth() {
        assert!(composer_auth_contains_secret(
            r#"{"github-oauth":{"github.com":"token"}}"#
        ));
    }

    #[test]
    fn ignores_empty_object() {
        assert!(!composer_auth_contains_secret("{}"));
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
    super::radioisotope::findings("composer", install_insecurity_reasons, home)
}
