#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in cloudsmith_credentials_paths()? {
        if path.exists() && credentials_contain_api_key(&read_to_string(&path)?) {
            reasons.push(format!(
                "cloudsmith credentials contain a plaintext API key: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn cloudsmith_credentials_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join("Library/Application Support/cloudsmith/credentials.ini"),
        home.join(".cloudsmith/credentials.ini"),
    ])
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn credentials_contain_api_key(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        let Some((name, value)) = line.split_once('=') else {
            return false;
        };
        name.trim() == "api_key" && !value.trim().trim_matches('"').trim_matches('\'').is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nonempty_api_keys() {
        assert!(credentials_contain_api_key("[default]\napi_key=fake-key\n"));
        assert!(credentials_contain_api_key(
            "[profile:prod]\napi_key = 'fake-profile-key'\n"
        ));
    }

    #[test]
    fn ignores_empty_or_unrelated_credentials() {
        assert!(!credentials_contain_api_key("[default]\napi_key=\n"));
        assert!(!credentials_contain_api_key(
            "[default]\napi_host=api.cloudsmith.io\n"
        ));
        assert!(!credentials_contain_api_key("# api_key=fake\n"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_defaults_are_missing() {
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
    super::radioisotope::findings("cloudsmith-cli", install_insecurity_reasons, home)
}
