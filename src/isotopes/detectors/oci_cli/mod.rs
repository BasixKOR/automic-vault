#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = config_path()?;
    if path.exists() && config_has_secret_references(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "OCI CLI config references plaintext credential material: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("OCI_CLI_CONFIG_FILE").map(PathBuf::from) {
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".oci/config"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_secret_references(contents: &str) -> bool {
    config_values(contents).any(|(key, value)| {
        matches!(
            key.as_str(),
            "key_file" | "pass_phrase" | "security_token_file" | "delegation_token_file"
        ) && !value.is_empty()
    })
}

fn config_values(contents: &str) -> impl Iterator<Item = (String, String)> + '_ {
    contents.lines().filter_map(|line| {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        let (key, value) = line.split_once('=')?;
        Some((key.trim().to_string(), value.trim().to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_key_file_reference() {
        assert!(config_has_secret_references(
            "[DEFAULT]\nuser=ocid1.user\nkey_file=~/.oci/oci_api_key.pem\n"
        ));
    }

    #[test]
    fn detects_pass_phrase() {
        assert!(config_has_secret_references(
            "[DEFAULT]\npass_phrase=hunter2\n"
        ));
    }

    #[test]
    fn ignores_non_secret_profile_metadata() {
        assert!(!config_has_secret_references(
            "[DEFAULT]\nuser=ocid1.user\nfingerprint=aa:bb\ntenancy=ocid1.tenancy\nregion=us-ashburn-1\n"
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
    super::radioisotope::findings("oci-cli", install_insecurity_reasons, home)
}
