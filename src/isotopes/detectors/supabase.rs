#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_access_token_paths()? {
        if path.exists() && !read_to_string(&path)?.trim().is_empty() {
            reasons.push(format!(
                "Supabase CLI fallback access-token file contains a plaintext token: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_access_token_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![home.join(".supabase/access-token")];
    if let Some(supabase_home) = std::env::var_os("SUPABASE_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(supabase_home).join("access-token"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_fallback_access_token() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-supabase-token-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let token_dir = home.join(".supabase");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&token_dir).unwrap();
        std::fs::write(token_dir.join("access-token"), "sbp_secret\n").unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_supabase_home = std::env::var_os("SUPABASE_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("SUPABASE_HOME");
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_supabase_home {
                Some(value) => std::env::set_var("SUPABASE_HOME", value),
                None => std::env::remove_var("SUPABASE_HOME"),
            }
        }

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains(&token_dir.join("access-token").display().to_string()));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn ignores_empty_access_token_file() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-empty-supabase-token-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let token_dir = home.join(".supabase");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&token_dir).unwrap();
        std::fs::write(token_dir.join("access-token"), "\n").unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_supabase_home = std::env::var_os("SUPABASE_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("SUPABASE_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_supabase_home {
                Some(value) => std::env::set_var("SUPABASE_HOME", value),
                None => std::env::remove_var("SUPABASE_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("supabase", install_insecurity_reasons, home)
}
