use std::ffi::OsString;
use std::path::Path;
use std::sync::Mutex;

use crate::{AffectedFile, Finding};

pub(super) const DETECTORS_URL: &str =
    "https://github.com/automic-vault/automic-vault/tree/main/src/isotopes/detectors";
const HIGH: &str = "high";
const SOLUTION: &str =
    "Review the reported plaintext secret and move or remove it; this detector is detect-only.";

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn findings(
    name: &'static str,
    reasons: fn() -> Result<Vec<String>, String>,
    home: &Path,
) -> Vec<Finding> {
    let _lock = ENV_LOCK.lock().expect("detector env lock poisoned");
    let _home = HomeEnv::set(home);

    let Ok(reasons) = reasons() else {
        return Vec::new();
    };

    reasons
        .into_iter()
        .map(|reason| Finding {
            source: name,
            homepage: DETECTORS_URL,
            severity: HIGH,
            affected: affected(&reason),
            explanation: reason,
            solution: SOLUTION.to_string(),
            docs_url: DETECTORS_URL,
        })
        .collect()
}

pub(super) fn affected(reason: &str) -> Vec<AffectedFile> {
    reason
        .rsplit_once(": ")
        .map(|(_, path)| path.trim())
        .filter(|path| path.starts_with('/') || path.starts_with('~'))
        .map(|path| {
            vec![AffectedFile {
                path: path.to_string(),
                line: None,
            }]
        })
        .unwrap_or_default()
}

struct HomeEnv {
    previous: Option<OsString>,
}

impl HomeEnv {
    fn set(home: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        // SAFETY: ported detectors read HOME from process env; ENV_LOCK
        // serializes scan-time HOME changes and Drop restores it.
        unsafe { std::env::set_var("HOME", home) };
        Self { previous }
    }
}

impl Drop for HomeEnv {
    fn drop(&mut self) {
        // SAFETY: still protected by ENV_LOCK while restoring HOME.
        unsafe {
            match &self.previous {
                Some(previous) => std::env::set_var("HOME", previous),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_reason_paths_to_affected_file() {
        assert_eq!(
            affected("Stripe CLI config contains plaintext API keys: /tmp/config.toml"),
            vec![AffectedFile {
                path: "/tmp/config.toml".to_string(),
                line: None,
            }]
        );
    }
}
