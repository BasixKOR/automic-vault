use std::path::Path;
use std::process::Command;

use crate::Finding;

const DOCS_URL: &str = "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/sip/detector.md";
const ENABLED: &str = "System Integrity Protection status: enabled.";

pub(crate) fn findings(_home: &Path) -> Vec<Finding> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }

    if let Some(status) = crate::test_env_string("AUTOMIC_VAULT_TEST_SIP_STATUS") {
        return finding_for_status(&status).into_iter().collect();
    }

    let Ok(output) = Command::new("/usr/bin/csrutil").arg("status").output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    finding_for_status(&String::from_utf8_lossy(&output.stdout))
        .into_iter()
        .collect()
}

fn finding_for_status(status: &str) -> Option<Finding> {
    (!status.lines().any(|line| line.trim() == ENABLED)).then(|| Finding {
        source: "sip",
        homepage: "https://support.apple.com/102149",
        severity: "high",
        explanation: "System Integrity Protection is not fully enabled.".to_string(),
        solution: "Boot into macOS Recovery, run `csrutil enable`, then restart.".to_string(),
        affected: Vec::new(),
        docs_url: DOCS_URL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_fully_enabled_sip() {
        assert!(finding_for_status(ENABLED).is_none());

        for status in [
            "System Integrity Protection status: disabled.",
            "System Integrity Protection status: unknown (Custom Configuration).",
        ] {
            let finding = finding_for_status(status).unwrap();
            assert_eq!(finding.severity, "high");
            assert!(finding.affected.is_empty());
        }
    }
}
