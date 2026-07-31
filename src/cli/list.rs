use std::ffi::OsString;
use std::io::Write;

pub(super) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if !args.is_empty() {
        let _ = writeln!(stderr, "usage: av list");
        return 2;
    }
    match crate::secrets::list_secret_names() {
        Ok(mut names) => {
            names.sort();
            for name in names {
                let _ = writeln!(stdout, "{name}");
            }
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "av list: {err}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_test_keychain_files_in_name_order() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("av-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ignored-directory")).unwrap();
        std::fs::write(dir.join("Z_SECRET"), "z").unwrap();
        std::fs::write(dir.join("A_SECRET"), "a").unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &dir) };

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(Vec::new(), &mut stdout, &mut stderr);

        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(stdout).unwrap(), "A_SECRET\nZ_SECRET\n");
        assert!(stderr.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
