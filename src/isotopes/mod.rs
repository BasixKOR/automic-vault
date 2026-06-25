use std::path::Path;

use crate::Finding;

pub(crate) mod git;

const DETECTORS: &[fn(&Path) -> Vec<Finding>] = &[git::findings];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for detector in DETECTORS {
        findings.extend(detector(home));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_runs_every_registered_isotope() {
        assert_eq!(DETECTORS.len(), 1);
        assert_eq!(git::NAME, "git");
    }
}
