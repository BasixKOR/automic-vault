#![allow(dead_code)]

const SOLUTION: &str = "Set `npmMinimalAgeGate: 1w` in the reported Yarn config file.";

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::js_release_age::policy_findings(
        "yarn",
        home,
        [home.join(".yarnrc.yml")],
        super::js_release_age::yarn_duration,
        SOLUTION,
    )
}
