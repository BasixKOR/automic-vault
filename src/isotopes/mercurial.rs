#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in mercurial_configs()? {
        if config.path.exists() && hgrc_has_secrets(&read_to_string(&config.path)?) {
            reasons.push(format!(
                "Mercurial hgrc contains credentials: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn mercurial_configs() -> Result<Vec<ConfigFile>, String> {
    let home = user_home()?;
    let config_home = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        home.join(".config")
    };
    Ok(vec![
        ConfigFile {
            path: home.join(".hgrc"),
        },
        ConfigFile {
            path: config_home.join("hg/hgrc"),
        },
    ])
}

struct ConfigFile {
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

fn hgrc_has_secrets(contents: &str) -> bool {
    contents.lines().any(line_has_secret)
}

fn line_has_secret(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return false;
    }
    if contains_password_url(trimmed) {
        return true;
    }
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    key.ends_with(".password")
        && !value.is_empty()
        && !value.eq_ignore_ascii_case("none")
        && !value.eq_ignore_ascii_case("null")
}

fn contains_password_url(value: &str) -> bool {
    let Some(scheme_index) = value.find("://") else {
        return false;
    };
    let after_scheme = &value[scheme_index + 3..];
    let Some(at_index) = after_scheme.find('@') else {
        return false;
    };
    let userinfo = &after_scheme[..at_index];
    let Some(colon_index) = userinfo.find(':') else {
        return false;
    };
    !userinfo[colon_index + 1..].is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_auth_password_fields() {
        assert!(hgrc_has_secrets("[auth]\ncorp.password = secret\n"));
    }

    #[test]
    fn detects_password_bearing_urls() {
        assert!(hgrc_has_secrets(
            "[paths]\ndefault = https://user:secret@example.com/repo\n"
        ));
    }

    #[test]
    fn ignores_empty_values_and_comments() {
        assert!(!hgrc_has_secrets("corp.password = \n"));
        assert!(!hgrc_has_secrets("# corp.password = secret\n"));
        assert!(!hgrc_has_secrets(
            "default = https://user@example.com/repo\n"
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
    super::radioisotope::findings("mercurial", install_insecurity_reasons, home)
}
