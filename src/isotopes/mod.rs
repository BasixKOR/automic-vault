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
