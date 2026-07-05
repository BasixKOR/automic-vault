#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_hosts_paths()? {
        if path.exists() && hosts_contains_oauth_token(&read_to_string(&path)?) {
            reasons.push(format!(
                "GitHub CLI hosts file contains plaintext OAuth tokens: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_hosts_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(dir) = std::env::var_os("GH_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(dir).join("hosts.yml")]);
    }

    let home = home_dir()?;
    let mut paths = vec![home.join(".config/gh/hosts.yml")];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("gh/hosts.yml"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn hosts_contains_oauth_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim_start();
        line.strip_prefix("oauth_token:")
            .map(str::trim)
            .is_some_and(non_empty_yaml_scalar)
    })
}

fn non_empty_yaml_scalar(value: &str) -> bool {
    let value = value.trim_matches('"').trim_matches('\'');
    !value.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_host_and_user_oauth_tokens() {
        assert!(hosts_contains_oauth_token(
            "github.com:\n  oauth_token: gho_secret\n"
        ));
        assert!(hosts_contains_oauth_token(
            "github.com:\n  users:\n    monalisa:\n      oauth_token: gho_secret\n"
        ));
    }

    #[test]
    fn ignores_empty_oauth_tokens() {
        assert!(!hosts_contains_oauth_token(
            "github.com:\n  oauth_token: ''\n"
        ));
        assert!(!hosts_contains_oauth_token(
            "github.com:\n  oauth_token: \"\"\n"
        ));
    }

    #[test]
    fn gh_config_dir_overrides_default_locations() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "{}-detect-gh-config-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let config = root.join("gh");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(
            config.join("hosts.yml"),
            "github.com:\n  oauth_token: gho_secret\n",
        )
        .unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_gh_config = std::env::var_os("GH_CONFIG_DIR");
        unsafe {
            std::env::set_var("HOME", root.join("home"));
            std::env::set_var("GH_CONFIG_DIR", &config);
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_gh_config {
                Some(value) => std::env::set_var("GH_CONFIG_DIR", value),
                None => std::env::remove_var("GH_CONFIG_DIR"),
            }
        }

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains(&config.join("hosts.yml").display().to_string()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("gh-cli", install_insecurity_reasons, home)
}
