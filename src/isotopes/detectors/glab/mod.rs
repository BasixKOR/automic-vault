#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    for path in candidate_config_paths()? {
        if path.exists() && glab_config_contains_token(&read_to_string(&path)?) {
            return Ok(vec![format!(
                "GLab config file contains plaintext tokens: {}",
                path.display()
            )]);
        }
    }
    Ok(Vec::new())
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(dir) = std::env::var_os("GLAB_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(dir).join("config.yml")]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = vec![home.join(".config/glab-cli/config.yml")];
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("glab-cli/config.yml"));
    }
    paths.push(home.join("Library/Application Support/glab-cli/config.yml"));
    Ok(paths)
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn glab_config_contains_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim_start();
        ["token:", "oauth2_refresh_token:"]
            .iter()
            .any(|prefix| line_has_non_empty_value(trimmed, prefix))
    })
}

fn line_has_non_empty_value(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "\"\"" && value != "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_host_token() {
        assert!(glab_config_contains_token(
            "hosts:\n  gitlab.com:\n    token: glpat-secret\n"
        ));
    }

    #[test]
    fn detects_oauth_refresh_token() {
        assert!(glab_config_contains_token(
            "hosts:\n  gitlab.com:\n    oauth2_refresh_token: refresh\n"
        ));
    }

    #[test]
    fn ignores_empty_token() {
        assert!(!glab_config_contains_token("token: ''\n"));
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
    super::radioisotope::findings("glab", install_insecurity_reasons, home)
}
