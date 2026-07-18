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

pub(crate) fn macos_gui_path() -> Option<std::ffi::OsString> {
    detectors::macos_gui_path()
}

pub(crate) fn hardener_metadata() -> Vec<hardeners::HardenerMetadata> {
    hardeners::metadata()
}
