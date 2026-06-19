use super::*;

pub(crate) const DB_SCHEMA_VERSION: u32 = 7;
#[cfg(all(not(test), feature = "packaged-db"))]
pub(crate) const EMBEDDED_COMBINED_DATA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/combined.json"));
#[cfg(test)]
pub(crate) const EMBEDDED_COMBINED_DATA: &[u8] = include_bytes!("fixtures/coverage-combined.json");
pub(crate) const EMBEDDED_POST_INSTALL_CHECK_SKIP: &str =
    include_str!("../../../data/post_install_check_skip.jsonc");
const REMOTE_COMBINED_DATA_URL: &str = "https://automicvault.com/db.json";
const REMOTE_COMBINED_DATA_DIR: &str = "/var/db/automic-vault";
const REMOTE_COMBINED_DATA_PATH: &str = "/var/db/automic-vault/db.json";
const REMOTE_COMBINED_DATA_META_PATH: &str = "/var/db/automic-vault/db.meta.json";
const REMOTE_COMBINED_DATA_CHECK_INTERVAL_SECONDS: u64 = 60 * 60;
static COMBINED_DATA: OnceLock<CombinedData> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub(crate) struct CombinedData {
    #[allow(dead_code)]
    pub(crate) schema: u32,
    #[allow(dead_code)]
    pub(crate) generated_at: String,
    pub(crate) sources: CombinedDataSources,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CombinedDataSources {
    pub(crate) db: Db,
    pub(crate) isotopes: HashMap<String, IsotopePackageData>,
    pub(crate) npm: HashMap<String, PackageInstallData>,
    pub(crate) pip: HashMap<String, PackageInstallData>,
    #[serde(default, rename = "security-recommendations")]
    pub(crate) security_recommendations: SecurityRecommendationsData,
    pub(crate) stub_exclusions: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RemoteCombinedDataMetadata {
    pub(crate) etag: Option<String>,
    pub(crate) checked_at: Option<u64>,
}

pub(crate) fn embedded_combined_data() -> &'static CombinedData {
    COMBINED_DATA.get_or_init(|| {
        #[cfg(all(not(test), feature = "packaged-db"))]
        {
            let embedded = serde_json::from_slice(EMBEDDED_COMBINED_DATA)
                .expect("failed to parse embedded combined package data JSON");
            match load_trusted_remote_combined_data() {
                Some(remote) if combined_data_is_at_least_as_new(&remote, &embedded) => remote,
                _ => embedded,
            }
        }
        #[cfg(test)]
        {
            serde_json::from_slice(EMBEDDED_COMBINED_DATA)
                .expect("failed to parse embedded combined package data JSON")
        }
        #[cfg(all(not(test), not(feature = "packaged-db")))]
        {
            load_trusted_remote_combined_data().unwrap_or_else(|| {
                panic!(
                    "non-packaged debug build requires a fetched trusted package database at {REMOTE_COMBINED_DATA_PATH}"
                )
            })
        }
    })
}

#[cfg(any(test, feature = "packaged-db"))]
pub(crate) fn combined_data_is_at_least_as_new(
    candidate: &CombinedData,
    baseline: &CombinedData,
) -> bool {
    let Ok(candidate_time) = OffsetDateTime::parse(&candidate.generated_at, &Rfc3339) else {
        return false;
    };
    let Ok(baseline_time) = OffsetDateTime::parse(&baseline.generated_at, &Rfc3339) else {
        return true;
    };
    candidate_time >= baseline_time
}

#[cfg(not(test))]
pub(crate) fn load_trusted_remote_combined_data() -> Option<CombinedData> {
    load_trusted_remote_combined_data_from(
        Path::new(REMOTE_COMBINED_DATA_DIR),
        Path::new(REMOTE_COMBINED_DATA_PATH),
        !cfg!(debug_assertions),
    )
}

pub(crate) fn load_trusted_remote_combined_data_from(
    dir: &Path,
    path: &Path,
    require_root_owner: bool,
) -> Option<CombinedData> {
    if !trusted_remote_data_path(dir, path, require_root_owner) {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let data = serde_json::from_slice::<CombinedData>(&bytes).ok()?;
    ensure_combined_data_schema(&data).ok()?;
    Some(data)
}

pub(crate) fn ensure_combined_data_schema(data: &CombinedData) -> Result<(), String> {
    ensure_db_schema(&data.sources.db)
}

pub(crate) fn trusted_remote_data_path(dir: &Path, path: &Path, require_root_owner: bool) -> bool {
    let Ok(dir_metadata) = fs::metadata(dir) else {
        return false;
    };
    if !dir_metadata.is_dir() || !trusted_remote_data_metadata(&dir_metadata, require_root_owner) {
        return false;
    }
    let Ok(file_metadata) = fs::metadata(path) else {
        return false;
    };
    file_metadata.is_file() && trusted_remote_data_metadata(&file_metadata, require_root_owner)
}

pub(crate) fn trusted_remote_data_metadata(
    metadata: &fs::Metadata,
    require_root_owner: bool,
) -> bool {
    if require_root_owner && metadata.uid() != 0 {
        return false;
    }
    metadata.mode() & 0o022 == 0
}

pub fn refresh_remote_combined_data() -> Result<bool, String> {
    refresh_remote_combined_data_with(
        REMOTE_COMBINED_DATA_URL,
        Path::new(REMOTE_COMBINED_DATA_DIR),
        Path::new(REMOTE_COMBINED_DATA_PATH),
        Path::new(REMOTE_COMBINED_DATA_META_PATH),
        REMOTE_COMBINED_DATA_CHECK_INTERVAL_SECONDS,
    )
}

pub(crate) fn refresh_remote_combined_data_with(
    url: &str,
    dir: &Path,
    data_path: &Path,
    meta_path: &Path,
    interval_seconds: u64,
) -> Result<bool, String> {
    let mut metadata = read_remote_combined_data_metadata(meta_path);
    let now = current_unix_timestamp()?;
    if metadata
        .checked_at
        .is_some_and(|checked_at| now.saturating_sub(checked_at) < interval_seconds)
    {
        return Ok(false);
    }

    let mut request = ureq::get(url).set("User-Agent", USER_AGENT);
    if let Some(etag) = metadata.etag.as_deref() {
        request = request.set("If-None-Match", etag);
    }

    let response = match request.call() {
        Ok(response) => response,
        Err(UreqError::Status(304, response)) => {
            metadata.checked_at = Some(now);
            if let Some(etag) = response.header("ETag") {
                metadata.etag = Some(etag.to_string());
            }
            write_remote_combined_data_metadata(dir, meta_path, &metadata)?;
            return Ok(false);
        }
        Err(UreqError::Status(code, _)) => {
            return Err(format!("failed to fetch {url}: http {code}"));
        }
        Err(UreqError::Transport(err)) => {
            return Err(format!("failed to fetch {url}: {err}"));
        }
    };

    if response.status() == 304 {
        metadata.checked_at = Some(now);
        if let Some(etag) = response.header("ETag") {
            metadata.etag = Some(etag.to_string());
        }
        write_remote_combined_data_metadata(dir, meta_path, &metadata)?;
        return Ok(false);
    }

    let etag = response.header("ETag").map(str::to_string);
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read {url}: {err}"))?;
    let data = serde_json::from_slice::<CombinedData>(&bytes)
        .map_err(|err| format!("failed to parse {url}: {err}"))?;
    ensure_combined_data_schema(&data)
        .map_err(|err| format!("unsupported remote database {url}: {err}"))?;
    write_remote_combined_data(dir, data_path, &bytes)?;
    metadata.etag = etag.or(metadata.etag);
    metadata.checked_at = Some(now);
    write_remote_combined_data_metadata(dir, meta_path, &metadata)?;
    Ok(true)
}

pub(crate) fn read_remote_combined_data_metadata(path: &Path) -> RemoteCombinedDataMetadata {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn write_remote_combined_data(
    dir: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    ensure_remote_combined_data_dir(dir)?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)
        .map_err(|err| format!("failed to write {}: {err}", temp_path.display()))?;
    set_root_readable_permissions(&temp_path, 0o644)?;
    fs::rename(&temp_path, path)
        .map_err(|err| format!("failed to replace {}: {err}", path.display()))?;
    set_root_readable_permissions(path, 0o644)
}

pub(crate) fn write_remote_combined_data_metadata(
    dir: &Path,
    path: &Path,
    metadata: &RemoteCombinedDataMetadata,
) -> Result<(), String> {
    ensure_remote_combined_data_dir(dir)?;
    let bytes = serde_json::to_vec(metadata)
        .map_err(|err| format!("failed to encode {}: {err}", path.display()))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)
        .map_err(|err| format!("failed to write {}: {err}", temp_path.display()))?;
    set_root_readable_permissions(&temp_path, 0o644)?;
    fs::rename(&temp_path, path)
        .map_err(|err| format!("failed to replace {}: {err}", path.display()))?;
    set_root_readable_permissions(path, 0o644)
}

pub(crate) fn ensure_remote_combined_data_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    set_root_readable_permissions(path, 0o755)
}

pub(crate) fn set_root_readable_permissions(path: &Path, mode: u32) -> Result<(), String> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|err| format!("failed to chmod {}: {err}", path.display()))?;
    set_root_owner(path)
}

pub(crate) fn set_root_owner(path: &Path) -> Result<(), String> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("path contains interior nul: {}", path.display()))?;
    let result = unsafe { libc::chown(c_path.as_ptr(), 0, 0) };
    if result == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.kind() == ErrorKind::PermissionDenied && cfg!(debug_assertions) {
        return Ok(());
    }
    Err(format!("failed to chown {}: {err}", path.display()))
}

pub(crate) fn current_unix_timestamp() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| format!("system clock is before unix epoch: {err}"))
}
