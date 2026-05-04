use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const DEBUG_OPT_ROOT: &str = "/tmp/opt";
const DEBUG_BIN_ROOT: &str = "/tmp/usr/local/bin";
const RELEASE_OPT_ROOT: &str = "/opt";
const RELEASE_BIN_ROOT: &str = "/usr/local/bin";
const FORMULA_API_ROOT: &str = "https://formulae.brew.sh/api/formula";
const PYPI_ROOT: &str = "https://pypi.org/pypi";
const GITHUB_API_ROOT: &str = "https://api.github.com";
const NPM_REGISTRY_ROOT: &str = "https://registry.npmjs.org";

pub(crate) fn opt_pkg_root() -> PathBuf {
    if cfg!(debug_assertions) {
        return PathBuf::from(DEBUG_OPT_ROOT);
    }
    PathBuf::from(RELEASE_OPT_ROOT)
}

pub(crate) fn opt_npm_root() -> PathBuf {
    opt_pkg_root().join("npm")
}

pub(crate) fn opt_pip_root() -> PathBuf {
    opt_pkg_root().join("pip")
}

pub(crate) fn managed_bin_root() -> PathBuf {
    if cfg!(debug_assertions) {
        return PathBuf::from(DEBUG_BIN_ROOT);
    }
    PathBuf::from(RELEASE_BIN_ROOT)
}

pub(crate) fn install_requires_root() -> bool {
    !cfg!(debug_assertions)
}

pub(crate) fn homebrew_debug_allowance_enabled() -> bool {
    cfg!(debug_assertions)
}

pub(crate) fn formula_api_root() -> String {
    endpoint_overrides()
        .formula_api_root
        .clone()
        .unwrap_or_else(|| FORMULA_API_ROOT.to_string())
}

pub(crate) fn pypi_root() -> String {
    endpoint_overrides()
        .pypi_root
        .clone()
        .unwrap_or_else(|| PYPI_ROOT.to_string())
}

pub(crate) fn github_api_root() -> String {
    endpoint_overrides()
        .github_api_root
        .clone()
        .unwrap_or_else(|| GITHUB_API_ROOT.to_string())
}

pub(crate) fn npm_registry_root() -> String {
    endpoint_overrides()
        .npm_registry_root
        .clone()
        .unwrap_or_else(|| NPM_REGISTRY_ROOT.to_string())
}

#[derive(Clone, Default)]
struct EndpointOverrides {
    formula_api_root: Option<String>,
    pypi_root: Option<String>,
    github_api_root: Option<String>,
    npm_registry_root: Option<String>,
}

static ENDPOINT_OVERRIDES: OnceLock<Mutex<EndpointOverrides>> = OnceLock::new();

fn endpoint_overrides() -> EndpointOverrides {
    ENDPOINT_OVERRIDES
        .get_or_init(|| Mutex::new(EndpointOverrides::default()))
        .lock()
        .unwrap()
        .clone()
}

#[cfg(test)]
pub(crate) fn set_test_endpoint_overrides(overrides: TestEndpointOverrides) {
    *ENDPOINT_OVERRIDES
        .get_or_init(|| Mutex::new(EndpointOverrides::default()))
        .lock()
        .unwrap() = EndpointOverrides {
        formula_api_root: overrides.formula_api_root,
        pypi_root: overrides.pypi_root,
        github_api_root: overrides.github_api_root,
        npm_registry_root: overrides.npm_registry_root,
    };
}

#[cfg(test)]
pub(crate) fn clear_test_endpoint_overrides() {
    *ENDPOINT_OVERRIDES
        .get_or_init(|| Mutex::new(EndpointOverrides::default()))
        .lock()
        .unwrap() = EndpointOverrides::default();
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct TestEndpointOverrides {
    pub(crate) formula_api_root: Option<String>,
    pub(crate) pypi_root: Option<String>,
    pub(crate) github_api_root: Option<String>,
    pub(crate) npm_registry_root: Option<String>,
}
