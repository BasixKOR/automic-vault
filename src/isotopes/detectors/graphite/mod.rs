#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in graphite_configs()? {
        if config.detect_secrets
            && config.path.exists()
            && config_has_auth_token(&read_to_string(&config.path)?)
        {
            reasons.push(format!(
                "Graphite CLI auth token is stored in plaintext config: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn graphite_configs() -> Result<Vec<GraphiteConfigFile>, String> {
    let dir = graphite_config_dir()?;
    Ok(vec![
        GraphiteConfigFile {
            path: dir.join("auth"),
            detect_secrets: true,
        },
        GraphiteConfigFile {
            path: dir.join("user_config"),
            detect_secrets: true,
        },
    ])
}

struct GraphiteConfigFile {
    path: PathBuf,
    detect_secrets: bool,
}

fn graphite_config_dir() -> Result<PathBuf, String> {
    let config_home = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(path) => PathBuf::from(path),
        None => user_home()?.join(".config"),
    };
    Ok(config_home.join("graphite"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_auth_token(contents: &str) -> bool {
    contents.contains("\"authToken\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_auth_token_in_auth_and_user_config() {
        assert!(config_has_auth_token(r#"{"authToken":"gt_token"}"#));
        assert!(config_has_auth_token(
            r#"{"alternativeProfiles":[{"name":"work","authToken":"gt_token"}]}"#
        ));
    }

    #[test]
    fn ignores_non_auth_config() {
        assert!(!config_has_auth_token(
            r#"{"updateAutomatically":true,"promptForUpdates":true}"#
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
    super::radioisotope::findings("graphite", install_insecurity_reasons, home)
}
