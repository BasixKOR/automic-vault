#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let path = kubeconfig_path()?;
    if path.exists() && kubeconfig_contains_secret(&read_to_string(&path)?) {
        reasons.push(format!(
            "kubeconfig contains plaintext cluster credentials: {}",
            path.display()
        ));
    }
    Ok(reasons)
}

fn kubeconfig_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("KUBECONFIG").filter(|value| !value.is_empty()) {
        let value = path.to_string_lossy();
        if !value.contains(':') {
            return Ok(PathBuf::from(path));
        }
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".kube/config"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn kubeconfig_contains_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = trim_yaml_list_marker(line.trim_start());
        SECRET_PREFIXES
            .iter()
            .any(|prefix| line_has_non_empty_value(trimmed, prefix))
    })
}

const SECRET_PREFIXES: &[&str] = &[
    "token:",
    "password:",
    "client-key:",
    "client-key-data:",
    "access-token:",
    "refresh-token:",
    "id-token:",
    "client-secret:",
];

fn line_has_non_empty_value(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "\"\"" && value != "''")
}

fn trim_yaml_list_marker(line: &str) -> &str {
    line.strip_prefix("- ").unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bearer_token() {
        assert!(kubeconfig_contains_secret("users:\n- token: secret\n"));
    }

    #[test]
    fn detects_embedded_client_key() {
        assert!(kubeconfig_contains_secret("client-key-data: base64\n"));
    }

    #[test]
    fn ignores_empty_tokens() {
        assert!(!kubeconfig_contains_secret("token: \"\"\n"));
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
    super::radioisotope::findings("kubernetes-cli", install_insecurity_reasons, home)
}
