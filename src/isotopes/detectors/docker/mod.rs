#![allow(dead_code)]

pub(crate) mod credential_helpers;
pub(crate) mod registry_credentials;

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let config = docker_config_path()?;
    let config_contents = if config.exists() {
        Some(read_to_string(&config)?)
    } else {
        None
    };

    if let Some(contents) = config_contents.as_deref() {
        reasons.extend(docker_config_hazards(contents, &config));
    }

    let legacy = home_dir()?.join(".dockercfg");
    if legacy.exists() && docker_legacy_config_contains_secret(&read_to_string(&legacy)?) {
        reasons.push(format!(
            "Docker legacy config contains registry credentials: {}",
            legacy.display()
        ));
    }

    Ok(reasons)
}

fn reasons_matching(matches: impl Fn(&str) -> bool) -> Result<Vec<String>, String> {
    Ok(install_insecurity_reasons()?
        .into_iter()
        .filter(|reason| matches(reason))
        .collect())
}

fn docker_config_path() -> Result<PathBuf, String> {
    if let Some(config) = std::env::var_os("DOCKER_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(config).join("config.json"));
    }
    Ok(home_dir()?.join(".docker/config.json"))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn docker_config_hazards(contents: &str, path: &Path) -> Vec<String> {
    let mut reasons = Vec::new();
    if docker_config_contains_inline_secret(contents) {
        reasons.push(format!(
            "Docker config contains inline registry credentials: {}",
            path.display()
        ));
    }

    for helper in string_values_for_key(contents, "credsStore") {
        if helper == "av" {
            continue;
        }
        reasons.push(format!(
            "Docker config uses ambient credential store `{helper}`: {}",
            path.display()
        ));
    }

    for helper in credential_helper_values(contents) {
        if helper == "av" {
            continue;
        }
        reasons.push(format!(
            "Docker config uses ambient per-registry credential helper `{helper}`: {}",
            path.display()
        ));
    }

    reasons
}

fn docker_config_contains_inline_secret(contents: &str) -> bool {
    let Some(auths) = object_for_key(contents, "auths") else {
        return false;
    };
    ["auth", "identitytoken", "identityToken"]
        .iter()
        .any(|key| {
            string_values_for_key(auths, key)
                .into_iter()
                .any(|value| !value.trim().is_empty())
        })
}

fn docker_legacy_config_contains_secret(contents: &str) -> bool {
    ["auth", "identitytoken", "identityToken"]
        .iter()
        .any(|key| {
            string_values_for_key(contents, key)
                .into_iter()
                .any(|value| !value.trim().is_empty())
        })
}

fn credential_helper_values(contents: &str) -> Vec<String> {
    let Some(helpers) = object_for_key(contents, "credHelpers") else {
        return Vec::new();
    };
    string_values_after_colons(helpers).collect()
}

fn object_for_key<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    let key = format!("\"{key}\"");
    let after_key = contents.split(&key).nth(1)?;
    let after_colon = after_key.split_once(':')?.1;
    object_value(after_colon.trim_start())
}

fn object_value(value: &str) -> Option<&str> {
    let mut chars = value.char_indices();
    let (_, first) = chars.next()?;
    if first != '{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&value[..=index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn string_values_for_key(contents: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    contents
        .split(&needle)
        .skip(1)
        .filter_map(|after_key| after_key.split_once(':').map(|(_, value)| value))
        .filter_map(|value| json_string_value(value.trim_start()).map(str::to_string))
        .collect()
}

fn string_values_after_colons(contents: &str) -> impl Iterator<Item = String> + '_ {
    contents
        .split(':')
        .skip(1)
        .filter_map(|value| json_string_value(value.trim_start()).map(str::to_string))
}

fn json_string_value(value: &str) -> Option<&str> {
    let value = value.strip_prefix('"')?;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(&value[..index]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_inline_docker_auth() {
        let reasons = docker_config_hazards(
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}}}"#,
            Path::new("/tmp/config.json"),
        );

        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("inline registry credentials"))
        );
    }

    #[test]
    fn detects_ambient_credential_helpers() {
        let reasons = docker_config_hazards(
            r#"{"credsStore":"osxkeychain","credHelpers":{"registry.example":"desktop"}}"#,
            Path::new("/tmp/config.json"),
        );

        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("credential store `osxkeychain`"))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("credential helper `desktop`"))
        );
    }

    #[test]
    fn detects_legacy_dockercfg_auth() {
        assert!(docker_legacy_config_contains_secret(
            r#"{"registry.example":{"auth":"dXNlcjpwYXNz"}}"#
        ));
        assert!(!docker_legacy_config_contains_secret(
            r#"{"registry.example":{}}"#
        ));
    }

    #[test]
    fn top_level_detection_reports_docker_config() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("docker-detect-{}", std::process::id()));
        let docker = home.join(".docker");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&docker).unwrap();
        std::fs::write(docker.join("config.json"), r#"{"credsStore":"desktop"}"#).unwrap();
        let previous_home = std::env::var_os("HOME");
        let previous_docker_config = std::env::var_os("DOCKER_CONFIG");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("DOCKER_CONFIG");
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_docker_config {
                Some(value) => std::env::set_var("DOCKER_CONFIG", value),
                None => std::env::remove_var("DOCKER_CONFIG"),
            }
        }
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("credential store `desktop`"))
        );
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("docker", install_insecurity_reasons, home)
}
