use std::path::Path;

use crate::Finding;

mod detect {
    #![allow(dead_code)]

    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../radioisotopes/node@18/detect.rs"
    ));
}

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    super::radioisotope::findings("node@18", detect::install_insecurity_reasons, home)
}
