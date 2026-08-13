use std::path::Path;

use crate::Finding;

mod detectors;
pub(crate) mod hardeners;

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    detectors::findings(home)
}

pub(crate) fn findings_for(
    home: &Path,
    detector_names: &[String],
) -> Result<Vec<detectors::DetectorResult>, String> {
    detectors::findings_for(home, detector_names)
}

pub(crate) fn detector_metadata(home: &Path) -> Vec<detectors::DetectorMetadata> {
    detectors::metadata(home)
}

pub(crate) fn hardener_metadata() -> Vec<hardeners::HardenerMetadata> {
    hardeners::metadata()
}
