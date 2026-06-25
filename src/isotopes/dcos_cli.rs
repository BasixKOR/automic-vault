#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let dcos_dir = dcos_config_dir()?;
    if !dcos_dir.exists() {
        return Ok(Vec::new());
    }

    let mut reasons = Vec::new();
    for path in credential_files(&dcos_dir)? {
        reasons.push(format!(
            "dcos-cli cluster config contains a plaintext ACS token: {}",
            path.display()
        ));
    }
    Ok(reasons)
}

fn dcos_config_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DCOS_DIR").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".dcos"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn credential_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let clusters = root.join("clusters");
    if !clusters.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&clusters)
        .map_err(|err| format!("failed to read {}: {err}", clusters.display()))?
    {
        let entry =
            entry.map_err(|err| format!("failed to read {} entry: {err}", clusters.display()))?;
        let path = entry.path().join("dcos.toml");
        if path.is_file() && config_contains_acs_token(&path)? {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn config_contains_acs_token(path: &Path) -> Result<bool, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(contents.lines().any(token_line_has_value))
}

fn token_line_has_value(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || !trimmed.starts_with("dcos_acs_token") {
        return false;
    }
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    key.trim() == "dcos_acs_token" && !value.trim().trim_matches('"').is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nonempty_acs_token_lines() {
        assert!(token_line_has_value(r#"dcos_acs_token = "token_abc""#));
        assert!(token_line_has_value(r#"  dcos_acs_token="token_abc""#));
        assert!(!token_line_has_value(r#"dcos_acs_token = """#));
        assert!(!token_line_has_value(r#"# dcos_acs_token = "token_abc""#));
        assert!(!token_line_has_value(
            r#"dcos_url = "https://example.test""#
        ));
    }

    #[test]
    fn finds_cluster_configs_with_tokens() {
        let temp = test_dir("dcos-cli-detect");
        let cluster = temp.join("clusters/cluster-a");
        std::fs::create_dir_all(&cluster).unwrap();
        std::fs::write(
            cluster.join("dcos.toml"),
            r#"[core]
dcos_url = "https://dcos.example.test"
dcos_acs_token = "token_abc"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(temp.join("clusters/cluster-b")).unwrap();
        std::fs::write(
            temp.join("clusters/cluster-b/dcos.toml"),
            r#"[core]
dcos_url = "https://dcos.example.test"
"#,
        )
        .unwrap();

        assert_eq!(
            credential_files(&temp).unwrap(),
            vec![PathBuf::from("clusters/cluster-a/dcos.toml")]
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_dir_is_missing() {
        let home = test_dir("dcos-cli-missing");
        let previous_home = std::env::var_os("HOME");
        let previous_dcos_dir = std::env::var_os("DCOS_DIR");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("DCOS_DIR");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_dcos_dir {
                Some(value) => std::env::set_var("DCOS_DIR", value),
                None => std::env::remove_var("DCOS_DIR"),
            }
        }
        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("dcos-cli", install_insecurity_reasons, home)
}
