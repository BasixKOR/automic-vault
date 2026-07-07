#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = maven_settings_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if settings_xml_contains_secret(&contents) {
        return Ok(vec![format!(
            "Maven settings.xml contains plaintext server credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn maven_settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".m2/settings.xml"))
}

fn settings_xml_contains_secret(contents: &str) -> bool {
    ["password", "passphrase", "privateKey"]
        .iter()
        .any(|tag| xml_tag_has_non_empty_value(contents, tag))
}

fn xml_tag_has_non_empty_value(contents: &str, tag: &str) -> bool {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut remaining = contents;
    while let Some(start) = remaining.find(&open) {
        let value_start = start + open.len();
        let Some(end) = remaining[value_start..].find(&close) else {
            return false;
        };
        let value = remaining[value_start..value_start + end].trim();
        if !value.is_empty() {
            return true;
        }
        remaining = &remaining[value_start + end + close.len()..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct HomeGuard(Option<std::ffi::OsString>);

    impl HomeGuard {
        fn set(path: &PathBuf) -> Self {
            let previous = std::env::var_os("HOME");
            unsafe { std::env::set_var("HOME", path) };
            Self(previous)
        }

        fn unset() -> Self {
            let previous = std::env::var_os("HOME");
            unsafe { std::env::remove_var("HOME") };
            Self(previous)
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => unsafe { std::env::set_var("HOME", value) },
                None => unsafe { std::env::remove_var("HOME") },
            }
        }
    }

    #[test]
    fn detects_server_password() {
        assert!(settings_xml_contains_secret(
            "<settings><servers><server><password>secret</password></server></servers></settings>"
        ));
    }

    #[test]
    fn ignores_empty_password() {
        assert!(!settings_xml_contains_secret(
            "<settings><password> </password></settings>"
        ));
    }

    #[test]
    fn detects_private_key_and_ignores_missing_close_tag() {
        assert!(settings_xml_contains_secret(
            "<settings><privateKey> key </privateKey></settings>"
        ));
        assert!(!xml_tag_has_non_empty_value("<password>secret", "password"));
    }

    #[test]
    fn install_detection_uses_default_settings_path() {
        let temp = std::env::temp_dir().join(format!("maven-detect-{}", std::process::id()));
        let settings = temp.join(".m2/settings.xml");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let _home = HomeGuard::set(&temp);

        assert_eq!(maven_settings_path().unwrap(), settings);
        assert!(!install_is_insecure().unwrap());

        fs::write(
            &settings,
            "<settings><servers><server><passphrase>secret</passphrase></server></servers></settings>",
        )
        .unwrap();
        let reasons = install_insecurity_reasons().unwrap();
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("plaintext server credentials"));

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn settings_path_requires_home() {
        let _home = HomeGuard::unset();
        assert_eq!(maven_settings_path().unwrap_err(), "HOME is not set");
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
    super::radioisotope::findings("maven", install_insecurity_reasons, home)
}
