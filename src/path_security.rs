use std::collections::HashSet;
use std::ffi::{CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub(crate) fn user_writable_entries_before_system_paths(path: &OsStr) -> Vec<PathBuf> {
    let entries = std::env::split_paths(path)
        .map(|entry| {
            let writable = !entry.is_absolute() || is_user_writable_or_creatable(&entry);
            let displayed = if entry.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                entry
            };
            (displayed, writable)
        })
        .collect::<Vec<_>>();

    let last_system_path = entries.iter().rposition(|(_, writable)| !writable);
    let all_entries_are_writable = last_system_path.is_none();
    let mut seen = HashSet::<OsString>::new();

    entries
        .into_iter()
        .enumerate()
        .filter(|(index, (_, writable))| {
            *writable
                && (all_entries_are_writable || last_system_path.is_some_and(|last| *index < last))
        })
        .filter_map(|(_, (entry, _))| seen.insert(entry.as_os_str().to_owned()).then_some(entry))
        .collect()
}

fn is_user_writable_or_creatable(path: &Path) -> bool {
    let mut candidate = path;
    loop {
        if candidate.exists() {
            if !candidate.is_dir() {
                return candidate.parent().is_some_and(is_user_writable);
            }
            return is_user_writable(candidate);
        }
        let Some(parent) = candidate.parent() else {
            return false;
        };
        candidate = parent;
    }
}

pub(crate) fn is_user_writable(path: &Path) -> bool {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `path` is a live, NUL-terminated C string and access does not retain it.
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("av-path-security-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn reports_writable_entries_that_precede_protected_entries() {
        let root = temp_dir("order");
        let writable = root.join("writable");
        let protected = root.join("protected");
        let trailing = root.join("trailing");
        std::fs::create_dir_all(&writable).unwrap();
        std::fs::create_dir_all(&protected).unwrap();
        std::fs::create_dir_all(&trailing).unwrap();
        std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o555)).unwrap();

        let path = std::env::join_paths([&writable, &protected, &trailing]).unwrap();
        let insecure = user_writable_entries_before_system_paths(&path);

        assert_eq!(insecure, vec![writable]);
        std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_an_entirely_user_writable_path() {
        let root = temp_dir("all-writable");
        let one = root.join("one");
        let two = root.join("two");
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&two).unwrap();
        let path = std::env::join_paths([&one, &two]).unwrap();

        assert_eq!(
            user_writable_entries_before_system_paths(&path),
            vec![one, two]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn treats_creatable_relative_and_empty_entries_as_user_writable() {
        let root = temp_dir("creatable");
        let missing = root.join("missing");
        let protected = root.join("protected");
        std::fs::create_dir_all(&protected).unwrap();
        std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o555)).unwrap();
        let path = OsString::from(format!(
            ":relative:{}:{}",
            missing.display(),
            protected.display()
        ));

        assert_eq!(
            user_writable_entries_before_system_paths(&path),
            vec![PathBuf::from("."), PathBuf::from("relative"), missing]
        );
        std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
