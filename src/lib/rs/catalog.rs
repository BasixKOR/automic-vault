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

pub(crate) static NPM_PACKAGE_DATA: OnceLock<HashMap<String, PackageInstallData>> = OnceLock::new();

pub(crate) static PIP_PACKAGE_DATA: OnceLock<HashMap<String, PackageInstallData>> = OnceLock::new();

pub(crate) static ISOTOPE_DATA: OnceLock<HashMap<String, IsotopePackageData>> = OnceLock::new();

pub(crate) static VIRTUAL_ISOTOPE_DATA: OnceLock<
    Mutex<HashMap<String, &'static IsotopePackageData>>,
> = OnceLock::new();

pub(crate) static SECURITY_RECOMMENDATIONS: OnceLock<SecurityRecommendationsData> = OnceLock::new();

pub(crate) static FORMULA_INDEX: OnceLock<Result<Vec<FormulaIndexEntry>, String>> = OnceLock::new();

pub(crate) static FORMULA_ALIAS_INDEX: OnceLock<Result<HashMap<String, String>, String>> =
    OnceLock::new();

pub(crate) static CASK_ALIAS_INDEX: OnceLock<HashMap<String, String>> = OnceLock::new();

pub(crate) static STUB_EXCLUSIONS: OnceLock<HashMap<String, HashSet<String>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Db {
    pub(crate) schema: u32,
    #[allow(dead_code)]
    pub(crate) generated_at: String,
    pub(crate) entries: HashMap<String, String>,
    #[serde(default)]
    pub(crate) formulas: HashMap<String, EmbeddedFormulaMetadata>,
    #[serde(default)]
    pub(crate) casks: HashMap<String, EmbeddedCaskMetadata>,
    #[serde(default)]
    pub(crate) npms: HashMap<String, EmbeddedNpmMetadata>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct EmbeddedFormulaMetadata {
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) oldnames: Vec<String>,
    #[serde(default)]
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) homepage: String,
    #[serde(default, alias = "repo")]
    pub(crate) repository: String,
    #[serde(default, rename = "upstreamDocs")]
    pub(crate) upstream_docs: String,
    #[serde(default)]
    pub(crate) docs: Vec<String>,
    pub(crate) popularity: Option<EmbeddedPackagePopularity>,
    pub(crate) last_updated_at: Option<String>,
    pub(crate) pulse_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct EmbeddedCaskMetadata {
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) homepage: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) sha256: String,
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) dependencies: Vec<String>,
    #[serde(default)]
    pub(crate) binaries: Vec<EmbeddedCaskBinary>,
    pub(crate) popularity: Option<EmbeddedPackagePopularity>,
    pub(crate) last_updated_at: Option<String>,
    pub(crate) pulse_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct EmbeddedCaskBinary {
    pub(crate) source: String,
    pub(crate) target: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub(crate) struct EmbeddedPackagePopularity {
    pub(crate) installs_per_365_days: u64,
    pub(crate) rank: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct EmbeddedNpmMetadata {
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) homepage: String,
    pub(crate) version: String,
    pub(crate) executable: String,
    pub(crate) popularity: Option<EmbeddedNpmPopularity>,
    pub(crate) last_updated_at: Option<String>,
    pub(crate) pulse_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct EmbeddedNpmPopularity {
    #[allow(dead_code)]
    pub(crate) downloads_per_30_days: u64,
    pub(crate) rank: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct PackageInstallData {
    #[serde(default, rename = "homebrewDeps")]
    pub(crate) homebrew_dependencies: Vec<String>,
    #[serde(default, rename = "pythonFormula")]
    pub(crate) python_formula: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct SecurityRecommendationsData {
    #[serde(default)]
    pub(crate) packages: HashMap<String, SecurityRecommendationPackage>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct SecurityRecommendationPackage {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "installPackageName")]
    pub(crate) install_package_name: String,
    #[serde(default)]
    pub(crate) priority: u32,
    #[serde(default)]
    pub(crate) signals: Vec<String>,
    #[serde(default)]
    pub(crate) reasons: Vec<String>,
    #[serde(default)]
    pub(crate) isotope: Option<String>,
    #[serde(default, rename = "isotopePackage")]
    pub(crate) isotope_package: Option<String>,
    #[serde(default, rename = "approvalGate")]
    pub(crate) approval_gate: bool,
    #[serde(default, rename = "geigerLevel")]
    pub(crate) geiger_level: Option<String>,
    #[serde(default, rename = "geigerConfidence")]
    pub(crate) geiger_confidence: Option<String>,
    #[serde(default, rename = "geigerCategory")]
    pub(crate) geiger_category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct IsotopePackageData {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) replaces: Option<String>,
    #[serde(default)]
    pub(crate) modifies: Option<String>,
    #[serde(default)]
    pub(crate) migrate: Option<String>,
    #[serde(default)]
    pub(crate) _repository: Option<String>,
    #[serde(default, rename = "upstreamRepository")]
    pub(crate) _upstream_repository: Option<String>,
    pub(crate) version: String,
    #[serde(default, rename = "releaseUrl")]
    pub(crate) release_url: Option<String>,
    #[serde(default, rename = "archiveUrl")]
    pub(crate) archive_url: Option<String>,
    #[serde(default, rename = "publishedAt")]
    pub(crate) published_at: Option<String>,
    #[serde(default, rename = "appliesToVersionedFormulae")]
    pub(crate) applies_to_versioned_formulae: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct PackageSecurityState {
    #[serde(rename = "isotopeName")]
    pub(crate) isotope_name: String,
    #[serde(rename = "installIsInsecure")]
    pub(crate) install_is_insecure: bool,
    #[serde(rename = "remediationAvailable")]
    pub(crate) remediation_available: bool,
    pub(crate) reasons: Vec<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FormulaIndexEntry {
    pub(crate) name: String,
    #[serde(default, alias = "desc")]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) oldnames: Vec<String>,
    #[serde(default)]
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) homepage: String,
    #[serde(default, alias = "repo")]
    pub(crate) repository: String,
    #[serde(default, rename = "upstreamDocs")]
    pub(crate) upstream_docs: String,
    #[serde(default)]
    pub(crate) docs: Vec<String>,
    pub(crate) popularity: Option<EmbeddedPackagePopularity>,
    pub(crate) last_updated_at: Option<String>,
    pub(crate) pulse_kind: Option<String>,
}

pub(crate) fn embedded_npm_package_data() -> &'static HashMap<String, PackageInstallData> {
    NPM_PACKAGE_DATA.get_or_init(|| embedded_combined_data().sources.npm.clone())
}

pub(crate) fn embedded_pip_package_data() -> &'static HashMap<String, PackageInstallData> {
    PIP_PACKAGE_DATA.get_or_init(|| embedded_combined_data().sources.pip.clone())
}

pub(crate) fn embedded_isotope_data() -> &'static HashMap<String, IsotopePackageData> {
    ISOTOPE_DATA.get_or_init(|| {
        embedded_combined_data()
            .sources
            .isotopes
            .clone()
            .into_values()
            .map(|record| (record.name.clone(), record))
            .collect()
    })
}

pub(crate) fn embedded_security_recommendations() -> &'static SecurityRecommendationsData {
    SECURITY_RECOMMENDATIONS.get_or_init(|| {
        embedded_combined_data()
            .sources
            .security_recommendations
            .clone()
    })
}

pub(crate) fn embedded_cask(cask: &str) -> Result<EmbeddedCaskMetadata, String> {
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    let canonical = canonical_cask_name(cask, &db);
    db.casks
        .get(&canonical)
        .cloned()
        .ok_or_else(|| format!("no embedded cask metadata found for {cask}"))
}

pub(crate) fn canonical_cask_name(cask: &str, db: &Db) -> String {
    cask_alias_index(db)
        .get(cask)
        .cloned()
        .unwrap_or_else(|| cask.to_string())
}

pub(crate) fn cask_alias_index(db: &Db) -> &'static HashMap<String, String> {
    CASK_ALIAS_INDEX.get_or_init(|| {
        let mut aliases = HashMap::new();
        for (name, metadata) in &db.casks {
            for alias in &metadata.aliases {
                aliases.entry(alias.clone()).or_insert_with(|| name.clone());
            }
        }
        aliases
    })
}

pub(crate) fn canonical_formula_name(formula: &str) -> Result<String, String> {
    Ok(formula_install_package_name_with_aliases(
        formula,
        formula_alias_index()?,
    ))
}

pub(crate) fn formula_install_package_name(formula: &str) -> Result<String, String> {
    Ok(canonical_formula_name_with_aliases(
        formula,
        formula_alias_index()?,
    ))
}

pub(crate) fn embedded_provider_install_package_name(
    package_name: &str,
) -> Result<Option<String>, String> {
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    let Some(provider) = db.entries.get(package_name) else {
        return Ok(None);
    };
    let Some(resolved) = crate::cli::parse_embedded_provider(provider)? else {
        return Ok(None);
    };
    Ok(Some(match resolved {
        EmbeddedPackage::Formula(formula) => formula_install_package_name(&formula)?,
        EmbeddedPackage::Cask(cask) => cask,
        EmbeddedPackage::NpmPackage(package) => npm_package_display_name(&package),
    }))
}

pub(crate) fn formula_install_package_name_with_aliases(
    formula: &str,
    aliases: &HashMap<String, String>,
) -> String {
    canonical_formula_name_with_aliases(formula, aliases)
}

pub(crate) fn canonical_formula_name_with_aliases(
    formula: &str,
    aliases: &HashMap<String, String>,
) -> String {
    aliases
        .get(formula)
        .cloned()
        .unwrap_or_else(|| formula.to_string())
}

pub(crate) fn formula_alias_index() -> Result<&'static HashMap<String, String>, String> {
    FORMULA_ALIAS_INDEX
        .get_or_init(build_formula_alias_index)
        .as_ref()
        .map_err(|err| err.clone())
}

pub(crate) fn build_formula_alias_index() -> Result<HashMap<String, String>, String> {
    Ok(collect_formula_aliases(formula_index_entries()?.clone()))
}

pub(crate) fn build_formula_index() -> Result<Vec<FormulaIndexEntry>, String> {
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    let mut entries = db
        .formulas
        .into_iter()
        .map(|(name, metadata)| FormulaIndexEntry {
            name,
            summary: metadata.summary,
            aliases: metadata.aliases,
            oldnames: metadata.oldnames,
            category: metadata.category,
            homepage: metadata.homepage,
            repository: metadata.repository,
            upstream_docs: metadata.upstream_docs,
            docs: metadata.docs,
            popularity: metadata.popularity,
            last_updated_at: metadata.last_updated_at,
            pulse_kind: metadata.pulse_kind,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

pub(crate) fn collect_formula_aliases(entries: Vec<FormulaIndexEntry>) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for entry in entries {
        for alias in entry.aliases.into_iter().chain(entry.oldnames) {
            aliases.entry(alias).or_insert_with(|| entry.name.clone());
        }
    }
    aliases
}

pub(crate) fn formula_index_entries() -> Result<&'static Vec<FormulaIndexEntry>, String> {
    FORMULA_INDEX
        .get_or_init(build_formula_index)
        .as_ref()
        .map_err(|err| err.clone())
}
