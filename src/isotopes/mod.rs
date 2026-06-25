use std::path::Path;

use crate::Finding;

pub(crate) mod aws;
pub(crate) mod git;

const DETECTORS: &[fn(&Path) -> Vec<Finding>] = &[git::findings, aws::findings];

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
        assert_eq!(DETECTORS.len(), 2);
        assert_eq!(aws::NAME, "aws");
        assert_eq!(git::NAME, "git");
    }
}
