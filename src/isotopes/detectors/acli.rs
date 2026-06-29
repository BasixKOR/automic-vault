#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in acli_configs()? {
        if config.path.exists() && config_has_secrets(&read_to_string(&config.path)?) {
            reasons.push(format!(
                "Atlassian CLI credentials are stored in plaintext config: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn acli_configs() -> Result<Vec<AcliConfigFile>, String> {
    let dir = user_home()?.join(".config/acli");
    Ok(CONFIG_FILES
        .iter()
        .map(|file_name| AcliConfigFile {
            path: dir.join(file_name),
        })
        .collect())
}

const CONFIG_FILES: &[&str] = &[
    "confluence_config.yaml",
    "jira_config.yaml",
    "assets_config.yaml",
    "rovodev_config.yaml",
    "brie_config.yaml",
    "global_auth_config.yaml",
    "global_config.yaml",
    "admin_config.yaml",
];

struct AcliConfigFile {
    path: PathBuf,
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_secrets(contents: &str) -> bool {
    contents.lines().any(line_has_secret_key)
}

fn line_has_secret_key(line: &str) -> bool {
    let trimmed_start = line.trim_start();
    let trimmed = trimmed_start
        .strip_prefix("- ")
        .unwrap_or(trimmed_start)
        .trim_start();
    [
        "token:",
        "api_token:",
        "apiToken:",
        "access_token:",
        "accessToken:",
        "refresh_token:",
        "refreshToken:",
        "client_secret:",
        "clientSecret:",
    ]
    .iter()
    .any(|key| trimmed.starts_with(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn set(values: &[(&'static str, &std::path::Path)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, value)| {
                    let previous = std::env::var_os(key);
                    unsafe { std::env::set_var(key, value) };
                    (*key, previous)
                })
                .collect();
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                match previous {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[test]
    fn detects_token_keys() {
        assert!(config_has_secrets("profiles:\n  - token: fake\n"));
        assert!(config_has_secrets("api_token: fake\n"));
        assert!(config_has_secrets(
            "access_token: fake\nrefresh_token: fake\n"
        ));
        assert!(config_has_secrets("client_secret: fake\n"));
    }

    #[test]
    fn ignores_default_auth_type_config() {
        assert!(!config_has_secrets(
            "version: 1\nprofile:\n  email: \"\"\n  accountId: \"\"\n  auth_type: \"\"\n"
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();
        let _env = EnvGuard::set(&[("HOME", &home), ("XDG_CONFIG_HOME", &xdg)]);

        let result = install_is_insecure().unwrap();

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("acli", install_insecurity_reasons, home)
}
