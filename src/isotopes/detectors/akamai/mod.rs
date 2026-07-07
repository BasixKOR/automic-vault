#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = edgerc_path()?;
    if path.exists() && config_has_edgegrid_secrets(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Akamai CLI .edgerc contains plaintext EdgeGrid credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn edgerc_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("AKAMAI_EDGERC").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".edgerc"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_edgegrid_secrets(contents: &str) -> bool {
    config_values(contents).any(|(key, value)| {
        matches!(
            key.as_str(),
            "client_token" | "client_secret" | "access_token"
        ) && !value.is_empty()
    })
}

fn config_values(contents: &str) -> impl Iterator<Item = (String, String)> + '_ {
    contents.lines().filter_map(|line| {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        let (key, value) = line.split_once('=')?;
        Some((key.trim().to_string(), unquote_ini_value(value.trim())))
    })
}

fn unquote_ini_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_edgegrid_tokens() {
        assert!(config_has_edgegrid_secrets(
            "[default]\nhost = example.luna.akamaiapis.net\nclient_token = tok\nclient_secret = sec\naccess_token = acc\n"
        ));
    }

    #[test]
    fn ignores_metadata_only_edgerc() {
        assert!(!config_has_edgegrid_secrets(
            "[default]\nhost = example.luna.akamaiapis.net\naccount_key = acct\n"
        ));
    }

    #[test]
    fn ignores_blank_secret_values() {
        assert!(!config_has_edgegrid_secrets(
            "[default]\nclient_token = \"\"\nclient_secret = ''\naccess_token =\n"
        ));
    }

    #[test]
    fn config_values_trim_comments_and_ignore_non_assignments() {
        let values = config_values(
            "client_token = tok # trailing comment\n; full comment\ninvalid\naccess_token = acc\n",
        )
        .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                ("client_token".to_string(), "tok".to_string()),
                ("access_token".to_string(), "acc".to_string()),
            ]
        );
    }

    #[test]
    fn edgerc_path_prefers_env_and_requires_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = std::env::temp_dir().join(format!("akamai-detect-path-{}", std::process::id()));
        let previous_home = std::env::var_os("HOME");
        let previous_edgerc = std::env::var_os("AKAMAI_EDGERC");
        let expected = temp.join("custom.edgerc");

        unsafe {
            std::env::set_var("AKAMAI_EDGERC", &expected);
            std::env::remove_var("HOME");
        }
        assert_eq!(edgerc_path().unwrap(), expected);

        unsafe {
            std::env::remove_var("AKAMAI_EDGERC");
        }
        assert_eq!(edgerc_path().unwrap_err(), "HOME is not set");

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_edgerc {
                Some(value) => std::env::set_var("AKAMAI_EDGERC", value),
                None => std::env::remove_var("AKAMAI_EDGERC"),
            }
        }
    }

    #[test]
    fn install_insecurity_reasons_reports_edgerc_path() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp =
            std::env::temp_dir().join(format!("akamai-detect-report-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let edgerc = temp.join(".edgerc");
        fs::write(&edgerc, "[default]\nclient_secret = sec\n").unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_edgerc = std::env::var_os("AKAMAI_EDGERC");
        unsafe {
            std::env::set_var("HOME", &temp);
            std::env::remove_var("AKAMAI_EDGERC");
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_edgerc {
                Some(value) => std::env::set_var("AKAMAI_EDGERC", value),
                None => std::env::remove_var("AKAMAI_EDGERC"),
            }
        }

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains(edgerc.to_str().unwrap()));
        fs::remove_dir_all(temp).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("akamai", install_insecurity_reasons, home)
}
