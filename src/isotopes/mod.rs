use std::path::Path;

use crate::Finding;

mod detectors;
pub(crate) mod hardeners;

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    detectors::findings(home)
}

pub(crate) fn detector_metadata() -> Vec<detectors::DetectorMetadata> {
    detectors::metadata()
}

pub(crate) fn documented_solution(documentation: &str) -> Option<String> {
    detectors::documented_solution(documentation)
}

pub(crate) fn hardener_metadata() -> Vec<hardeners::HardenerMetadata> {
    hardeners::metadata()
}
