#![allow(dead_code)]

pub(crate) mod credential_helpers;
pub(crate) mod registry_credentials;
pub(crate) mod root_access;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
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

    reasons.extend(docker_root_equivalent_access_hazards()?);

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

fn docker_root_equivalent_access_hazards() -> Result<Vec<String>, String> {
    #[cfg(unix)]
    {
        docker_root_equivalent_access_hazards_unix()
    }

    #[cfg(not(unix))]
    {
        Ok(Vec::new())
    }
}

#[cfg(unix)]
fn docker_root_equivalent_access_hazards_unix() -> Result<Vec<String>, String> {
    if current_effective_uid() == 0 {
        return Ok(Vec::new());
    }

    let group_ids = current_group_ids();
    let mut reasons = Vec::new();
    if current_groups_include_named_group("docker", &group_ids) {
        reasons.push(
            "Current user is in the docker group, which grants root-equivalent host access through root containers and writable bind mounts".to_string(),
        );
    }

    for path in docker_socket_paths() {
        if docker_socket_is_writable_by_current_user(&path, &group_ids) {
            reasons.push(format!(
                "Current user can write the Docker daemon socket, which grants root-equivalent host access through root containers and writable bind mounts: {}",
                path.display()
            ));
        }
    }

    reasons.sort();
    reasons.dedup();
    Ok(reasons)
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

#[cfg(unix)]
fn current_effective_uid() -> u32 {
    unsafe { libc::geteuid() as u32 }
}

#[cfg(unix)]
fn current_group_ids() -> Vec<u32> {
    let mut groups = vec![unsafe { libc::getgid() as u32 }, unsafe {
        libc::getegid() as u32
    }];
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count > 0 {
        let mut raw = vec![0 as libc::gid_t; count as usize];
        let read = unsafe { libc::getgroups(raw.len() as i32, raw.as_mut_ptr()) };
        if read > 0 {
            groups.extend(raw.into_iter().take(read as usize).map(|gid| gid as u32));
        }
    }
    groups.sort();
    groups.dedup();
    groups
}

#[cfg(unix)]
fn current_groups_include_named_group(name: &str, group_ids: &[u32]) -> bool {
    let Ok(contents) = std::fs::read_to_string("/etc/group") else {
        return false;
    };
    group_file_contains_named_group_id(&contents, name, group_ids)
}

#[cfg(unix)]
fn group_file_contains_named_group_id(contents: &str, name: &str, group_ids: &[u32]) -> bool {
    contents
        .lines()
        .filter_map(|line| group_file_line_name_and_gid(line))
        .any(|(group_name, gid)| group_name == name && group_ids.contains(&gid))
}

#[cfg(unix)]
fn group_file_line_name_and_gid(line: &str) -> Option<(&str, u32)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let mut parts = trimmed.split(':');
    let name = parts.next()?.trim();
    let _password = parts.next()?;
    let gid = parts.next()?.trim().parse::<u32>().ok()?;
    (!name.is_empty()).then_some((name, gid))
}

#[cfg(unix)]
fn docker_socket_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(host) = std::env::var_os("DOCKER_HOST").filter(|value| !value.is_empty()) {
        if let Some(path) = docker_host_unix_socket_path(&host.to_string_lossy()) {
            paths.push(path);
        }
    }
    paths.push(PathBuf::from("/var/run/docker.sock"));
    paths.push(PathBuf::from("/run/docker.sock"));
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(unix)]
fn docker_host_unix_socket_path(host: &str) -> Option<PathBuf> {
    let path = host.strip_prefix("unix://")?;
    (!path.trim().is_empty()).then(|| PathBuf::from(path))
}

#[cfg(unix)]
fn docker_socket_is_writable_by_current_user(path: &Path, group_ids: &[u32]) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    metadata_is_writable_by_current_user(&metadata, group_ids)
}

#[cfg(unix)]
fn metadata_is_writable_by_current_user(metadata: &std::fs::Metadata, group_ids: &[u32]) -> bool {
    let mode = metadata.mode();
    let uid = metadata.uid();
    let gid = metadata.gid();
    let euid = current_effective_uid();

    (uid == euid && mode & 0o200 != 0)
        || (group_ids.contains(&gid) && mode & 0o020 != 0)
        || mode & 0o002 != 0
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
        reasons.push(format!(
            "Docker config uses ambient credential store `{helper}`: {}",
            path.display()
        ));
    }

    for helper in credential_helper_values(contents) {
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

    #[cfg(unix)]
    #[test]
    fn detects_docker_group_as_root_equivalent_access() {
        let groups = [20, 998, 1000];
        assert!(group_file_contains_named_group_id(
            "staff:x:20:\ndocker:x:998:agent\n",
            "docker",
            &groups
        ));
        assert!(!group_file_contains_named_group_id(
            "staff:x:20:\ndocker:x:999:agent\n",
            "docker",
            &groups
        ));
    }

    #[cfg(unix)]
    #[test]
    fn parses_docker_host_unix_socket_paths() {
        assert_eq!(
            docker_host_unix_socket_path("unix:///var/run/docker.sock"),
            Some(PathBuf::from("/var/run/docker.sock"))
        );
        assert_eq!(docker_host_unix_socket_path("tcp://127.0.0.1:2375"), None);
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
