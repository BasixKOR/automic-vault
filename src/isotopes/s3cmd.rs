use std::path::Path;

use crate::Finding;

mod detect {
    #![allow(dead_code)]

    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../radioisotopes/s3cmd/detect.rs"
    ));
}

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    super::radioisotope::findings("s3cmd", detect::install_insecurity_reasons, home)
}
