pub(crate) const RELOCATABLE_HOMEBREW_PREFIX: &str = "/opt/homebrew";
pub(crate) const HOMEBREW_PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
pub(crate) const HOMEBREW_CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
pub(crate) const HOMEBREW_REPOSITORY_PLACEHOLDER: &str = "@@HOMEBREW_REPOSITORY@@";
pub(crate) const HOMEBREW_LIBRARY_PLACEHOLDER: &str = "@@HOMEBREW_LIBRARY@@";
pub(crate) const HOMEBREW_PERL_PLACEHOLDER: &str = "@@HOMEBREW_PERL@@";
pub(crate) const HOMEBREW_JAVA_PLACEHOLDER: &str = "@@HOMEBREW_JAVA@@";
pub(crate) const OPENSSL_CA_CERTIFICATES_DIR: &str = "share/ca-certificates";
pub(crate) const OPENSSL_CA_CERTIFICATES_CERT: &str = "share/ca-certificates/cacert.pem";
pub(crate) const OPENSSL_CERT_PEM_PATH: &str = "/etc/openssl@3/cert.pem";
pub(crate) const OPENSSL_CERT_PEM_DESTINATION_DIR: &str = "ssl";
pub(crate) const OPENSSL_CERT_PEM_DESTINATION: &str = "ssl/cert.pem";
pub(crate) const TMP_TOOL_ROOT: &str = "/tmp/nucleus";
#[cfg(feature = "gold-release")]
pub(crate) const SELF_UPDATE_TARGET: &str = "/usr/local/bin/av";
#[cfg(feature = "gold-release")]
pub(crate) const SELF_UPDATE_REPO: &str = "mxcl/nucleus";
pub(crate) const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
pub(crate) const SAFE_BINARY_PATH_BYTES: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._+-/@";
pub(crate) const HOMEBREW_NEEDLES: [&[u8]; 6] = [
    b"@@HOMEBREW_PREFIX@@",
    b"@@HOMEBREW_CELLAR@@",
    b"@@HOMEBREW_REPOSITORY@@",
    b"@@HOMEBREW_LIBRARY@@",
    b"@@HOMEBREW_PERL@@",
    b"@@HOMEBREW_JAVA@@",
];
pub(crate) static POST_INSTALL_CHECK_SKIP: OnceLock<HashSet<String>> = OnceLock::new();

pub(crate) fn configure_debug_install_environment() {
    if !homebrew_debug_allowance_enabled() {
        return;
    }

    let mut flags = env::var("PKG_ALLOW").unwrap_or_default();
    for flag in ["unsupported-formulas", "relocation-failures"] {
        if pkg_allow_value_contains(&flags, flag) {
            continue;
        }
        if !flags.is_empty() {
            flags.push(':');
        }
        flags.push_str(flag);
    }
    // SAFETY: This runs during process startup before any worker threads are
    // spawned, so mutating the process environment here is well-defined.
    unsafe { env::set_var("PKG_ALLOW", flags) };
}

#[derive(Debug, Clone)]
pub(crate) struct FormulaSpec {
    pub(crate) name: String,
    pub(crate) bottle_sha256: String,
    pub(crate) bottle_url: String,
}

#[derive(Debug)]
pub(crate) struct DownloadedBottle {
    pub(crate) path: PathBuf,
    pub(crate) _tmp_dir: TempDir,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GhcrTokenResponse {
    pub(crate) token: String,
}

#[cfg(feature = "gold-release")]
#[derive(Debug, Deserialize)]
pub(crate) struct GithubRelease {
    pub(crate) tag_name: String,
    pub(crate) assets: Vec<GithubReleaseAsset>,
}

#[cfg(feature = "gold-release")]
#[derive(Debug, Deserialize)]
pub(crate) struct GithubReleaseAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

#[cfg(feature = "gold-release")]
#[derive(Debug)]
pub(crate) struct SelfUpdateRelease {
    pub(crate) version: semver::Version,
    pub(crate) asset_name: String,
    pub(crate) download_url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct InstalledFormula {
    pub(crate) spec: FormulaSpec,
    pub(crate) keg_dir_name: String,
    pub(crate) archive_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteRule {
    pub(crate) source: String,
    pub(crate) destination: String,
}

use super::*;

pub(crate) mod post_install_hooks {
    use super::*;

    #[derive(Debug, Default, PartialEq, Eq)]
    pub(crate) struct PostInstallOutcome {
        pub(crate) managed_stubs: Vec<String>,
    }

    mod python {
        include!("../post-install/python.rs");
    }

    mod openssl {
        include!("../post-install/openssl.rs");
    }

    pub(crate) fn supports(formula: &str) -> bool {
        python::supports(formula) || openssl::supports(formula)
    }

    pub(crate) fn supports_dependency(formula: &str) -> bool {
        openssl::supports(formula)
    }

    pub(crate) fn run(
        formula: &str,
        prefix: &Path,
        bin_dir: &Path,
    ) -> Result<PostInstallOutcome, String> {
        if python::supports(formula) {
            return python::post_install(prefix, bin_dir);
        }
        if openssl::supports(formula) {
            openssl::post_install(prefix)?;
            return Ok(PostInstallOutcome::default());
        }
        Ok(PostInstallOutcome::default())
    }
}

pub(crate) struct VendorInstall {
    pub(crate) package: vendor::VendorPackage,
    pub(crate) version: semver::Version,
}

pub(crate) struct ResolvedVendorDependencies {
    pub(crate) formula_graph: Vec<FormulaSpec>,
    pub(crate) vendor_installs: Vec<VendorInstall>,
}

#[derive(Debug)]
pub(crate) struct DependencyInstallState {
    pub(crate) _downloads: HashMap<String, DownloadedBottle>,
    pub(crate) installs: Vec<InstalledFormula>,
    pub(crate) changed_installs: Vec<InstalledFormula>,
}

pub(crate) struct PreparedInstallPlan {
    pub(crate) plan: InstallPlan,
    pub(crate) workspace: Option<TempDir>,
}

#[derive(Clone)]
pub(crate) struct InstallProgress {
    pub(crate) enabled: bool,
    pub(crate) bar: Option<ProgressBar>,
    pub(crate) state: Arc<Mutex<InstallProgressState>>,
    pub(crate) callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
    pub(crate) package_name: String,
    pub(crate) bytes_downloaded: Arc<Mutex<u64>>,
    pub(crate) total_bytes: Arc<Mutex<Option<u64>>>,
    pub(crate) download_started_at: Arc<Mutex<Option<Instant>>>,
    pub(crate) package_downloads: Arc<Mutex<HashMap<String, PackageDownloadProgress>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallProgressPhase {
    Download,
    Install,
}

#[derive(Debug)]
pub(crate) struct InstallProgressState {
    pub(crate) phase: InstallProgressPhase,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PackageDownloadProgress {
    pub(crate) bytes_downloaded: u64,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) started_at: Option<Instant>,
}

impl PackageDownloadProgress {
    pub(crate) fn started() -> Self {
        Self {
            bytes_downloaded: 0,
            total_bytes: None,
            started_at: Some(Instant::now()),
        }
    }
}

pub(crate) struct LoggedCommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) lines: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BinaryRewriteMode<'a> {
    Slash,
    #[allow(dead_code)]
    Nul,
    Macho {
        path: &'a Path,
        root: &'a Path,
        future_root: &'a Path,
    },
}

pub(crate) fn temp_root_for_target_root(
    target_root: &Path,
    system_tmp_root: &Path,
    shared_tmp_root: &Path,
) -> PathBuf {
    match paths_share_device(target_root, system_tmp_root) {
        Ok(true) if shared_tmp_root_is_writable(shared_tmp_root) => shared_tmp_root.to_path_buf(),
        Ok(false) | Err(_) => target_root.join(".tmp"),
        Ok(true) => target_root.join(".tmp"),
    }
}

pub(crate) fn shared_tmp_root_is_writable(path: &Path) -> bool {
    if fs::create_dir_all(path).is_err() {
        return false;
    }
    TempDir::new_in(path).is_ok()
}

pub(crate) fn paths_share_device(left: &Path, right: &Path) -> Result<bool, String> {
    Ok(device_id(left)? == device_id(right)?)
}

pub(crate) fn device_id(path: &Path) -> Result<u64, String> {
    let metadata_path = metadata_probe_path(path)?;
    let metadata = fs::metadata(metadata_path)
        .map_err(|err| format!("failed to stat {}: {err}", metadata_path.display()))?;
    Ok(metadata.dev())
}

pub(crate) fn metadata_probe_path(path: &Path) -> Result<&Path, String> {
    path.ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| format!("no existing ancestor for {}", path.display()))
}

pub(crate) fn prepare_i_install_plan(
    plan: &InstallPlan,
    intent: InstallIntent,
) -> Result<PreparedInstallPlan, String> {
    fs::create_dir_all(&plan.tmp_root)
        .map_err(|err| format!("failed to create {}: {err}", plan.tmp_root.display()))?;
    let workspace = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!(
            "failed to create staging dir in {}: {err}",
            plan.tmp_root.display()
        )
    })?;
    let staged_plan = InstallPlan {
        install_root: workspace.path().join("install"),
        ..plan.clone()
    };
    if intent == InstallIntent::Update {
        seed_incremental_update_root(plan, &staged_plan)?;
    }
    Ok(PreparedInstallPlan {
        plan: staged_plan,
        workspace: Some(workspace),
    })
}

pub(crate) fn preserve_temp_dir_in_debug(workspace: TempDir) {
    if !cfg!(debug_assertions) {
        return;
    }

    let path = workspace.path().to_path_buf();
    let _ = workspace.keep();
    eprintln!("info: preserved temp dir {}", path.display());
}

pub(crate) fn preserve_optional_temp_dir_on_failure(workspace: Option<TempDir>) {
    if let Some(workspace) = workspace {
        preserve_temp_dir_in_debug(workspace);
    }
}

pub(crate) fn seed_incremental_update_root(
    source_plan: &InstallPlan,
    staged_plan: &InstallPlan,
) -> Result<bool, String> {
    if !source_plan.install_root.is_dir() {
        return Ok(false);
    }
    if !install_root_supports_incremental_update(source_plan)? {
        return Ok(false);
    }
    copy_tree_contents(&source_plan.install_root, &staged_plan.install_root)?;
    Ok(true)
}

pub(crate) fn install_root_supports_incremental_update(plan: &InstallPlan) -> Result<bool, String> {
    let Some(package_receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };

    if !formula_receipts_support_incremental_update(plan)? {
        return Ok(false);
    }

    if !matches!(package_receipt.source, PackageReceiptSource::Formula { .. })
        && load_root_ownership_manifest(&plan.root_ownership_manifest_path())?.is_none()
    {
        return Ok(false);
    }

    Ok(true)
}

pub(crate) fn formula_receipts_support_incremental_update(
    plan: &InstallPlan,
) -> Result<bool, String> {
    let receipts_dir = plan.install_root.join(RECEIPTS_DIR);
    let entries = match fs::read_dir(&receipts_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(format!("failed to read {}: {err}", receipts_dir.display())),
    };

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", receipts_dir.display()))?;
        if entry.path().extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }
        let Some(receipt) = load_install_receipt(&entry.path())? else {
            return Ok(false);
        };
        if receipt.owned_paths.is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn copy_tree_contents(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;
    for entry in
        fs::read_dir(source).map_err(|err| format!("failed to read {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", source.display()))?;
        copy_path(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

pub(crate) fn copy_path(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to stat {}: {err}", source.display()))?;
    if metadata.file_type().is_symlink() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        let target = fs::read_link(source)
            .map_err(|err| format!("failed to read symlink {}: {err}", source.display()))?;
        symlink(&target, destination).map_err(|err| {
            format!(
                "failed to link {} -> {}: {err}",
                destination.display(),
                target.display()
            )
        })?;
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)
            .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;
        fs::set_permissions(destination, metadata.permissions())
            .map_err(|err| format!("failed to chmod {}: {err}", destination.display()))?;
        for entry in fs::read_dir(source)
            .map_err(|err| format!("failed to read {}: {err}", source.display()))?
        {
            let entry =
                entry.map_err(|err| format!("failed to read {}: {err}", source.display()))?;
            copy_path(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    copy_file_preserving_metadata(source, destination, &metadata)
}

pub(crate) fn copy_file_preserving_metadata(
    source: &Path,
    destination: &Path,
    metadata: &fs::Metadata,
) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    clone_file_or_copy(source, destination)?;
    fs::set_permissions(destination, metadata.permissions())
        .map_err(|err| format!("failed to chmod {}: {err}", destination.display()))
}

#[cfg(target_os = "macos")]
pub(crate) fn clone_file_or_copy(source: &Path, destination: &Path) -> Result<(), String> {
    unsafe extern "C" {
        fn clonefile(src: *const libc::c_char, dst: *const libc::c_char, flags: u32)
        -> libc::c_int;
    }

    let source_c = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| format!("path contains NUL byte: {}", source.display()))?;
    let destination_c = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| format!("path contains NUL byte: {}", destination.display()))?;
    let cloned = unsafe { clonefile(source_c.as_ptr(), destination_c.as_ptr(), 0) == 0 };
    if cloned {
        return Ok(());
    }
    fs::copy(source, destination).map(|_| ()).map_err(|err| {
        format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            destination.display()
        )
    })
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn clone_file_or_copy(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination).map(|_| ()).map_err(|err| {
        format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            destination.display()
        )
    })
}

impl InstallProgress {
    pub(crate) fn with_callback(
        label: &str,
        callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
    ) -> Self {
        if std::io::stderr().is_terminal() {
            let bar = ProgressBar::new(0);
            bar.set_prefix(label.to_string());
            bar.set_style(download_progress_style());
            bar.enable_steady_tick(Duration::from_millis(120));
            return Self {
                enabled: true,
                bar: Some(bar),
                state: Arc::new(Mutex::new(InstallProgressState {
                    phase: InstallProgressPhase::Download,
                })),
                callback,
                package_name: label.to_string(),
                bytes_downloaded: Arc::new(Mutex::new(0)),
                total_bytes: Arc::new(Mutex::new(None)),
                download_started_at: Arc::new(Mutex::new(None)),
                package_downloads: Arc::new(Mutex::new(HashMap::new())),
            };
        }

        Self {
            enabled: false,
            bar: None,
            state: Arc::new(Mutex::new(InstallProgressState {
                phase: InstallProgressPhase::Download,
            })),
            callback,
            package_name: label.to_string(),
            bytes_downloaded: Arc::new(Mutex::new(0)),
            total_bytes: Arc::new(Mutex::new(None)),
            download_started_at: Arc::new(Mutex::new(None)),
            package_downloads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn begin_download_phase(&self) {
        let mut state = self.state.lock().unwrap();
        state.phase = InstallProgressPhase::Download;
        drop(state);
        *self.bytes_downloaded.lock().unwrap() = 0;
        *self.total_bytes.lock().unwrap() = None;
        *self.download_started_at.lock().unwrap() = Some(Instant::now());
        self.package_downloads.lock().unwrap().clear();
        if let Some(bar) = &self.bar {
            bar.set_style(download_progress_style());
            bar.set_position(0);
            bar.set_length(0);
            bar.set_message(String::new());
        }
        self.emit(ProgressEvent::Resolving);
    }

    pub(crate) fn add_download_total(&self, total: Option<u64>) {
        self.add_download_total_for(&self.package_name, total);
    }

    pub(crate) fn add_download_total_for(&self, package: &str, total: Option<u64>) {
        let Some(total) = total else {
            return;
        };
        if total == 0 {
            return;
        }
        {
            let mut total_bytes = self.total_bytes.lock().unwrap();
            *total_bytes = Some(total_bytes.unwrap_or(0) + total);
        }
        {
            let mut package_downloads = self.package_downloads.lock().unwrap();
            let state = package_downloads
                .entry(package.to_string())
                .or_insert_with(PackageDownloadProgress::started);
            state.total_bytes = Some(state.total_bytes.unwrap_or(0) + total);
        }
        if let Some(bar) = &self.bar {
            bar.inc_length(total);
        }
        self.emit_downloading_for(package);
    }

    pub(crate) fn advance_download(&self, amount: u64) {
        self.advance_download_for(&self.package_name, amount);
    }

    pub(crate) fn begin_download_for(&self, package: &str) {
        let mut package_downloads = self.package_downloads.lock().unwrap();
        package_downloads
            .entry(package.to_string())
            .or_insert_with(PackageDownloadProgress::started);
        drop(package_downloads);
        self.emit_downloading_for(package);
    }

    pub(crate) fn advance_download_for(&self, package: &str, amount: u64) {
        if amount == 0 {
            return;
        }
        {
            let mut bytes_downloaded = self.bytes_downloaded.lock().unwrap();
            *bytes_downloaded += amount;
        }
        {
            let mut package_downloads = self.package_downloads.lock().unwrap();
            let state = package_downloads
                .entry(package.to_string())
                .or_insert_with(PackageDownloadProgress::started);
            state.bytes_downloaded += amount;
        }
        self.emit_downloading_for(package);
        if !self.enabled {
            return;
        }
        if let Some(bar) = &self.bar {
            bar.inc(amount);
        }
    }

    pub(crate) fn begin_install_phase(&self) {
        self.begin_install_phase_for(&self.package_name);
    }

    pub(crate) fn begin_install_phase_for(&self, package: &str) {
        let mut state = self.state.lock().unwrap();
        let already_installing = state.phase == InstallProgressPhase::Install;
        if !already_installing {
            state.phase = InstallProgressPhase::Install;
        }
        drop(state);
        if already_installing && package == self.package_name {
            return;
        }
        if let Some(bar) = &self.bar {
            bar.set_style(install_progress_style());
            bar.set_message("staging files".to_string());
        }
        self.emit(ProgressEvent::Installing {
            package: package.to_string(),
        });
    }

    pub(crate) fn log<S: AsRef<str>>(&self, message: S) {
        let message = sanitize_progress_message(message.as_ref());
        if message.is_empty() {
            return;
        }
        self.begin_install_phase();
        if let Some(bar) = &self.bar {
            bar.set_message(message.clone());
        }
        self.emit(ProgressEvent::Log {
            package: self.package_name.clone(),
            message,
        });
    }

    pub(crate) fn finish_with_paths(&self, paths: &[String]) {
        let message = format_installed_paths(paths);
        self.emit(ProgressEvent::Completed {
            package: self.package_name.clone(),
        });
        if let Some(bar) = &self.bar {
            bar.set_style(final_progress_style());
            bar.finish_with_message(message);
        } else {
            eprintln!("{message}");
        }
    }

    pub(crate) fn clear(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }

    pub(crate) fn emit_downloading_for(&self, package: &str) {
        if let Some(package_state) = self.package_downloads.lock().unwrap().get(package).copied() {
            let progress = package_state
                .total_bytes
                .filter(|total| *total > 0)
                .map(|total| package_state.bytes_downloaded as f32 / total as f32)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let bytes_per_sec = package_state
                .started_at
                .map(|started| started.elapsed())
                .filter(|elapsed| elapsed.as_secs_f32() > 0.0)
                .map(|elapsed| {
                    (package_state.bytes_downloaded as f32 / elapsed.as_secs_f32()) as u64
                })
                .unwrap_or(0);
            self.emit(ProgressEvent::Downloading {
                package: package.to_string(),
                bytes_per_sec,
                progress,
            });
            return;
        }

        let bytes_downloaded = *self.bytes_downloaded.lock().unwrap();
        let total_bytes = *self.total_bytes.lock().unwrap();
        let started_at = *self.download_started_at.lock().unwrap();
        let progress = total_bytes
            .filter(|total| *total > 0)
            .map(|total| bytes_downloaded as f32 / total as f32)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let bytes_per_sec = started_at
            .map(|started| started.elapsed())
            .filter(|elapsed| elapsed.as_secs_f32() > 0.0)
            .map(|elapsed| (bytes_downloaded as f32 / elapsed.as_secs_f32()) as u64)
            .unwrap_or(0);
        self.emit(ProgressEvent::Downloading {
            package: package.to_string(),
            bytes_per_sec,
            progress,
        });
    }

    pub(crate) fn emit(&self, event: ProgressEvent) {
        let Some(callback) = &self.callback else {
            return;
        };
        if let Ok(mut callback) = callback.lock() {
            callback(event);
        }
    }
}

pub(crate) fn download_progress_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{spinner:.cyan} {prefix:.bold} [{bar:28.cyan/blue}] {percent:>3}% {bytes}/{total_bytes}",
    )
    .unwrap()
    .progress_chars("=> ")
}

pub(crate) fn install_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {prefix:.bold} {msg}").unwrap()
}

pub(crate) fn final_progress_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}").unwrap()
}

pub(crate) fn sanitize_progress_message(message: &str) -> String {
    message
        .split(['\n', '\r'])
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(|line| {
            line.chars()
                .filter(|ch| !ch.is_control())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

pub(crate) fn format_installed_paths(paths: &[String]) -> String {
    if paths.is_empty() {
        "installed".to_string()
    } else {
        paths.join("\n")
    }
}

pub(crate) fn npm_package_homebrew_dependencies(package: &str) -> Vec<String> {
    let data = embedded_npm_package_data();
    if let Some(entry) = data.get(package) {
        return entry.homebrew_dependencies.clone();
    }
    if let Some((_, leaf_name)) = package.rsplit_once('/')
        && let Some(entry) = data.get(leaf_name)
    {
        return entry.homebrew_dependencies.clone();
    }
    Vec::new()
}

pub(crate) fn append_npm_package_homebrew_dependencies(
    formula_names: &mut Vec<String>,
    package: &str,
) {
    for dependency in npm_package_homebrew_dependencies(package) {
        push_unique_string(formula_names, dependency);
    }
}

pub(crate) fn run_i(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_i_request(invocation, &mut args)? {
        Some(request) => request,
        None => return Ok(()),
    };

    if install_requires_root() && !is_root() {
        return Err("must be run as root".to_string());
    }

    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    for package in request.packages {
        run_i_package_with_progress(
            &config,
            package,
            InstallOptions {
                intent: if request.force {
                    InstallIntent::Reinstall
                } else {
                    InstallIntent::Install
                },
            },
            None,
        )?;
    }
    Ok(())
}

pub(crate) fn run_i_package(
    config: &Config,
    requested: RequestedPackage,
    options: InstallOptions,
) -> Result<(), String> {
    run_i_package_with_progress(config, requested, options, None)
}

pub(crate) fn run_i_package_with_progress(
    config: &Config,
    requested: RequestedPackage,
    options: InstallOptions,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let mut rollback_name = requested_package_name(&requested);
    let result = match requested {
        RequestedPackage::Auto(package_name) => {
            if let Some(isotope_name) = preferred_auto_isotope_name(&package_name)? {
                let package_name = isotope_qualified_name(&isotope_name);
                rollback_name = package_name.clone();
                if isotope_has_post_install(&package_name) {
                    run_i_radioisotope(
                        config,
                        package_name,
                        isotope_name,
                        options.intent,
                        progress_callback.clone(),
                    )
                } else {
                    prepare_install_target(
                        &opt_pkg_root(),
                        &package_name,
                        options.intent,
                        &managed_bin_root(),
                    )?;
                    run_i_isotope(
                        config,
                        package_name,
                        isotope_name,
                        true,
                        options.intent,
                        progress_callback.clone(),
                    )
                }
            } else if let Some(package) = vendor::get(&package_name) {
                prepare_install_target(
                    &opt_pkg_root(),
                    &package_name,
                    options.intent,
                    &managed_bin_root(),
                )?;
                run_i_vendor(
                    config,
                    package_name.clone(),
                    package,
                    options.intent,
                    progress_callback.clone(),
                )
            } else {
                match resolve_i_root_package(&package_name)? {
                    EmbeddedPackage::Formula(root_formula) => {
                        let install_package_name = formula_install_package_name(&root_formula)?;
                        rollback_name = install_package_name.clone();
                        prepare_install_target(
                            &opt_pkg_root(),
                            &install_package_name,
                            options.intent,
                            &managed_bin_root(),
                        )?;
                        run_i_formula(
                            config,
                            install_package_name,
                            root_formula,
                            options.intent,
                            progress_callback.clone(),
                        )
                    }
                    EmbeddedPackage::Cask(cask_name) => {
                        prepare_install_target(
                            &opt_pkg_root(),
                            &package_name,
                            options.intent,
                            &managed_bin_root(),
                        )?;
                        run_i_cask(
                            config,
                            package_name.clone(),
                            cask_name,
                            options.intent,
                            progress_callback.clone(),
                        )
                    }
                    EmbeddedPackage::NpmPackage(npm_package) => run_i_package_with_progress(
                        config,
                        RequestedPackage::NpmPackage {
                            package: npm_package,
                            version: None,
                        },
                        options,
                        progress_callback.clone(),
                    ),
                }
            }
        }
        RequestedPackage::VendorPackage(package_name) => {
            let package = vendor::get(&package_name)
                .ok_or_else(|| format!("vendor package {package_name} is not registered"))?;
            prepare_install_target(
                &opt_pkg_root(),
                &package_name,
                options.intent,
                &managed_bin_root(),
            )?;
            run_i_vendor(
                config,
                package_name.clone(),
                package,
                options.intent,
                progress_callback.clone(),
            )
        }
        RequestedPackage::HomebrewFormula(formula) => {
            let package_name = formula_install_package_name(&formula)?;
            rollback_name = package_name.clone();
            if let Some(isotope_name) = radioisotope_name_for_homebrew_formula_install(&formula)? {
                run_i_radioisotope(
                    config,
                    isotope_qualified_name(&isotope_name),
                    isotope_name,
                    options.intent,
                    progress_callback.clone(),
                )
            } else {
                prepare_install_target(
                    &opt_pkg_root(),
                    &package_name,
                    options.intent,
                    &managed_bin_root(),
                )?;
                run_i_formula(
                    config,
                    package_name,
                    formula,
                    options.intent,
                    progress_callback.clone(),
                )
            }
        }
        RequestedPackage::HomebrewCask(cask) => {
            prepare_install_target(&opt_pkg_root(), &cask, options.intent, &managed_bin_root())?;
            run_i_cask(
                config,
                cask.clone(),
                cask,
                options.intent,
                progress_callback.clone(),
            )
        }
        RequestedPackage::Isotope(isotope) => {
            let package_name = isotope_qualified_name(&isotope);
            if isotope_has_post_install(&package_name) {
                run_i_radioisotope(
                    config,
                    package_name,
                    isotope,
                    options.intent,
                    progress_callback.clone(),
                )
            } else {
                prepare_install_target(
                    &opt_pkg_root(),
                    &package_name,
                    options.intent,
                    &managed_bin_root(),
                )?;
                run_i_isotope(
                    config,
                    package_name,
                    isotope,
                    true,
                    options.intent,
                    progress_callback.clone(),
                )
            }
        }
        RequestedPackage::NpmPackage {
            package: npm_package,
            version,
        } => {
            let package_name = npm_package_display_name(&npm_package);
            prepare_install_target(
                &opt_pkg_root(),
                &package_name,
                options.intent,
                &managed_bin_root(),
            )?;
            run_i_npm(
                config,
                package_name.clone(),
                npm_package,
                version,
                options,
                options.intent,
                progress_callback.clone(),
            )
        }
        RequestedPackage::PipPackage(pip_package) => {
            let package_name = pip_package_display_name(&pip_package);
            prepare_install_target(
                &opt_pkg_root(),
                &package_name,
                options.intent,
                &managed_bin_root(),
            )?;
            run_i_pip(
                config,
                package_name.clone(),
                pip_package,
                options.intent,
                progress_callback.clone(),
            )
        }
    };
    if let Err(err) = result {
        if options.intent == InstallIntent::Update {
            return Err(err);
        }
        rollback_failed_install(&opt_pkg_root(), &rollback_name, &managed_bin_root())
            .map_err(|cleanup_err| format!("{err}\ncleanup failed: {cleanup_err}"))?;
        return Err(err);
    }
    Ok(())
}

pub(crate) fn run_i_formula(
    config: &Config,
    package_name: String,
    root_formula: String,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    print_full_formula_recommendation(&root_formula)?;
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let graph = resolve_formula_specs(std::slice::from_ref(&root_formula), config, true)?;
        let root_formula = graph
            .last()
            .map(|spec| spec.name.clone())
            .ok_or_else(|| "no formula resolved".to_string())?;
        let plan = InstallPlan::for_i(package_name.clone(), root_formula);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let dependency_state = resolve_dependency_install_state(
                &graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            progress.begin_install_phase();
            let installs = &dependency_state.installs;
            let changed_installs = &dependency_state.changed_installs;
            let root_install = installs
                .iter()
                .find(|install| install.spec.name == plan.root_formula)
                .ok_or_else(|| {
                    format!(
                        "root formula {} not present in install graph",
                        plan.root_formula
                    )
                })?;

            ensure_plan_parent_dirs(&staged_plan)?;
            let rewrite_rules = build_rewrite_rules(&staged_plan, installs);
            install_package(
                config,
                &staged_plan,
                installs,
                changed_installs,
                &rewrite_rules,
                Some(&progress),
            )?;
            activate_install(&staged_plan)?;
            let metadata = formula_package_metadata(&plan.root_formula)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: root_install.keg_dir_name.clone(),
                    source: PackageReceiptSource::Formula {
                        root_formula: plan.root_formula.clone(),
                    },
                    metadata,
                },
            )?;
            sync_stubs(&plan, &graph, &previous_stubs)?;
            run_package_post_install(&plan, installs, &managed_bin_root())?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_cask(
    config: &Config,
    package_name: String,
    cask_name: String,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let cask = embedded_cask(&cask_name)?;
        ensure_cask_install_metadata(&cask_name, &cask)?;
        let dependency_graph = resolve_formula_specs(&cask.dependencies, config, true)?;
        let plan = InstallPlan::for_i(package_name.clone(), cask_name.clone());
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let dependency_state = resolve_dependency_install_state(
                &dependency_graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current =
                dependencies_are_current(&staged_plan, &dependency_state.installs, &[], config)?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependency_state.changed_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            if !cask_root_is_current(
                &staged_plan,
                &cask,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    if dependency_graph.is_empty() {
                        prepare_vendor_root_area(&staged_plan)?;
                    } else {
                        reinstall_vendor_dependency_tree(
                            config,
                            &staged_plan,
                            &dependency_state.installs,
                            &dependency_graph,
                            &[],
                            Some(&progress),
                        )?;
                    }
                }
                let root_payload_before = prepare_root_payload_install(&staged_plan)?;
                install_cask_root(&staged_plan, &cask_name, &cask, Some(&progress))?;
                finish_root_payload_install(&staged_plan, root_payload_before)?;
            }

            activate_install(&staged_plan)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: cask.version.clone(),
                    source: PackageReceiptSource::Cask {
                        cask_name: cask_name.clone(),
                    },
                    metadata: PackageMetadata {
                        description: string_or_none(&cask.summary),
                        homepage: string_or_none(&cask.homepage),
                    },
                },
            )?;
            sync_declared_stubs(
                &plan,
                &dependency_graph,
                cask_binary_names(&cask),
                &package_stub_exclusions(&plan.package_name),
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_isotope(
    config: &Config,
    package_name: String,
    isotope_name: String,
    install_stubs: bool,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let record = isotope_package_data(&isotope_name)?.clone();
        let dependency_graph = isotope_dependency_graph(&record, config)?;
        let plan = InstallPlan::for_i_isotope(package_name.clone(), &isotope_name);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let dependency_state = resolve_dependency_install_state(
                &dependency_graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;
            let dependency_current =
                dependencies_are_current(&staged_plan, &dependency_state.installs, &[], config)?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependency_state.changed_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            if !isotope_root_is_current(&staged_plan, &record)? {
                if !dependencies_reinstalled && dependency_graph.is_empty() {
                    prepare_vendor_root_area(&staged_plan)?;
                }
                let root_payload_before = prepare_root_payload_install(&staged_plan)?;
                install_isotope_root(
                    &staged_plan,
                    &record,
                    &dependency_state.installs,
                    Some(&progress),
                )?;
                finish_root_payload_install(&staged_plan, root_payload_before)?;
            }
            let executables = collect_root_executables(&staged_plan.install_root)?;
            let stub_executables = isotope_stub_executables(&record, &executables)?;
            write_root_executable_manifest(
                &staged_plan.root_executables_manifest_path(),
                &stub_executables,
            )?;
            activate_install(&staged_plan)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: record.version.clone(),
                    source: PackageReceiptSource::Isotope {
                        isotope_name: isotope_name.clone(),
                    },
                    metadata: PackageMetadata {
                        description: record
                            .replaces
                            .as_deref()
                            .map(|replaces| format!("Isotope mirror replacing {replaces}")),
                        homepage: record.release_url.clone(),
                    },
                },
            )?;
            if install_stubs {
                sync_declared_stubs(
                    &plan,
                    &dependency_graph,
                    stub_executables.iter().map(String::as_str),
                    &isotope_stub_exclusions(&plan.package_name, &record),
                    &previous_stubs,
                )?;
            }
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_isotope_root_only(
    config: &Config,
    package_name: String,
    isotope_name: String,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    run_i_isotope(
        config,
        package_name,
        isotope_name,
        false,
        InstallIntent::Install,
        progress_callback,
    )
}

pub(crate) fn run_i_radioisotope(
    config: &Config,
    package_name: String,
    isotope_name: String,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback.clone());
    let result = (|| {
        let record = isotope_package_data(&isotope_name)?.clone();
        if !isotope_has_post_install(&record.name) {
            return Err(format!("isotope:{} is not a radioisotope", isotope_name));
        }
        let modified_target = isotope_modified_package_target(&record)?
            .ok_or_else(|| format!("radioisotope:{} does not declare modifies", isotope_name))?;
        let modified_package = radioisotope_modified_install_name(&modified_target)?;
        let plan = InstallPlan::for_i_radioisotope(package_name.clone(), modified_package.clone());

        match radioisotope_modified_formula_intent(intent) {
            Some(InstallIntent::Reinstall) => {
                prepare_install_target(
                    &opt_pkg_root(),
                    &modified_package,
                    InstallIntent::Reinstall,
                    &managed_bin_root(),
                )?;
                run_i_modified_package(
                    config,
                    modified_package.clone(),
                    &modified_target,
                    InstallIntent::Reinstall,
                    progress_callback.clone(),
                )?;
            }
            Some(InstallIntent::Update) => {
                run_i_modified_package(
                    config,
                    modified_package.clone(),
                    &modified_target,
                    InstallIntent::Update,
                    progress_callback.clone(),
                )?;
            }
            Some(InstallIntent::Install) => unreachable!("install intent is handled as None"),
            None => {
                let modified_root = package_install_root(&opt_pkg_root(), &modified_package)?;
                if !modified_root.exists() {
                    prepare_install_target(
                        &opt_pkg_root(),
                        &modified_package,
                        InstallIntent::Install,
                        &managed_bin_root(),
                    )?;
                    run_i_modified_package(
                        config,
                        modified_package.clone(),
                        &modified_target,
                        InstallIntent::Install,
                        progress_callback.clone(),
                    )?;
                }
                ensure_package_installed(&opt_pkg_root(), &modified_package)?;
            }
        }

        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let formula_receipt = load_package_receipt(&plan.root_receipt_path())?
            .ok_or_else(|| format!("missing receipt for modified package {modified_package}"))?;
        progress.log("converting Homebrew install to isotope");
        match run_generated_isotope_post_install(&record.name) {
            Some(result) => result?,
            None => return Err(format!("isotope:{} has no post-install step", isotope_name)),
        }
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: package_name.clone(),
                version: formula_receipt.version,
                source: PackageReceiptSource::Isotope {
                    isotope_name: isotope_name.clone(),
                },
                metadata: PackageMetadata {
                    description: record
                        .modifies
                        .as_deref()
                        .map(|modifies| format!("Radioisotope modifying {modifies}")),
                    homepage: record.release_url.clone(),
                },
            },
        )?;
        sync_stubs(&plan, &[], &previous_stubs)?;
        installed_stub_paths(&plan)
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_modified_package(
    config: &Config,
    package_name: String,
    target: &PackageAliasTarget,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    match target {
        PackageAliasTarget::HomebrewFormula(formula) => run_i_formula(
            config,
            package_name,
            formula.clone(),
            intent,
            progress_callback,
        ),
        PackageAliasTarget::VendorPackage(vendor_name) => {
            let package = vendor::get(vendor_name)
                .ok_or_else(|| format!("vendor package {vendor_name} is not registered"))?;
            run_i_vendor(config, package_name, package, intent, progress_callback)
        }
        _ => Err(format!(
            "invalid isotope modification {}: radioisotopes may only modify Homebrew formulae or vendor packages",
            target.display_name()
        )),
    }
}

pub(crate) fn radioisotope_modified_formula_intent(intent: InstallIntent) -> Option<InstallIntent> {
    match intent {
        InstallIntent::Install => None,
        InstallIntent::Reinstall => Some(InstallIntent::Reinstall),
        InstallIntent::Update => Some(InstallIntent::Update),
    }
}

pub(crate) fn install_isotope_stubs(
    isotope_name: &str,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<Vec<String>, String> {
    let package_name = isotope_qualified_name(isotope_name);
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let record = isotope_package_data(isotope_name)?.clone();
    let plan = InstallPlan::for_i_isotope(package_name, isotope_name);
    if let Some(replaced_package) = isotope_replaced_package_name(&record)?
        && package_install_root(&opt_pkg_root(), &replaced_package)?.exists()
    {
        return Err(format!(
            "cannot install isotope stubs while replacement package is installed: \
                 {replaced_package}"
        ));
    }
    let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
    let executables_manifest =
        load_root_executable_manifest(&plan.root_executables_manifest_path())?;
    let executables = collect_declared_root_executables(
        &plan.install_root,
        executables_manifest.stubs.iter().map(String::as_str),
    )?;
    progress.log("installing isotope stubs");
    sync_declared_stubs(
        &plan,
        &[],
        executables.iter().map(|(name, _)| name.as_str()),
        &isotope_stub_exclusions(&plan.package_name, &record),
        &previous_stubs,
    )?;
    installed_stub_paths(&plan)
}

pub(crate) fn isotope_stub_executables(
    isotope: &IsotopePackageData,
    discovered: &[(String, PathBuf)],
) -> Result<Vec<String>, String> {
    if let Some(formula) = isotope_modified_or_replaced_package_name(isotope)? {
        let executables = predicted_homebrew_executables(&formula)?;
        if !executables.is_empty() {
            return Ok(executables);
        }
    }

    Ok(discovered.iter().map(|(name, _)| name.clone()).collect())
}

pub(crate) fn isotope_stub_exclusions(
    package_name: &str,
    isotope: &IsotopePackageData,
) -> HashSet<String> {
    let mut exclusions = package_stub_exclusions(package_name);
    if let Ok(Some(formula)) = isotope_modified_or_replaced_package_name(isotope) {
        exclusions.extend(formula_stub_exclusions(&formula));
    }
    exclusions
}

pub(crate) fn embedded_post_install_check_skip() -> &'static HashSet<String> {
    POST_INSTALL_CHECK_SKIP.get_or_init(|| {
        json5::from_str::<Vec<String>>(EMBEDDED_POST_INSTALL_CHECK_SKIP)
            .expect("failed to parse embedded post-install check skip list JSONC")
            .into_iter()
            .collect()
    })
}

pub(crate) fn embedded_stub_exclusions() -> &'static HashMap<String, HashSet<String>> {
    STUB_EXCLUSIONS.get_or_init(|| {
        embedded_combined_data()
            .sources
            .stub_exclusions
            .clone()
            .into_iter()
            .map(|(package, executables)| (package, executables.into_iter().collect()))
            .collect()
    })
}

pub(crate) fn formula_stub_exclusions(formula: &str) -> HashSet<String> {
    let mut exclusions = embedded_stub_exclusions()
        .get(&format!("{BREW_PACKAGE_PREFIX}{formula}"))
        .cloned()
        .unwrap_or_default();
    exclusions.extend(versioned_python_stub_exclusions(formula));
    exclusions
}

pub(crate) fn vendor_stub_exclusions(package: &vendor::VendorPackage) -> HashSet<String> {
    embedded_stub_exclusions()
        .get(&format!("vendor:{}", package.name))
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn package_stub_exclusions(package_name: &str) -> HashSet<String> {
    embedded_stub_exclusions()
        .get(package_name)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn imagemagick_stub_exclusions(
    plan: &InstallPlan,
    current: &[(String, PathBuf)],
) -> HashSet<String> {
    if !should_only_stub_magick(plan) {
        return HashSet::new();
    }

    current
        .iter()
        .filter_map(|(name, _)| (name != "magick").then_some(name.clone()))
        .collect()
}

pub(crate) fn should_only_stub_magick(plan: &InstallPlan) -> bool {
    match plan.root_formula.as_str() {
        "imagemagick-full" => true,
        "imagemagick" => installed_formula_major_version(plan).is_some_and(|major| major >= 7),
        _ => false,
    }
}

pub(crate) fn installed_formula_major_version(plan: &InstallPlan) -> Option<u64> {
    let receipt = load_package_receipt(&plan.root_receipt_path())
        .ok()
        .flatten()?;
    let PackageReceiptSource::Formula { root_formula } = receipt.source else {
        return None;
    };
    if root_formula != plan.root_formula {
        return None;
    }
    parse_homebrew_major_version(&receipt.version)
}

pub(crate) fn parse_homebrew_major_version(version: &str) -> Option<u64> {
    let trimmed = version.strip_prefix('v').unwrap_or(version);
    trimmed
        .split(|ch: char| !ch.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|major| major.parse().ok())
}

pub(crate) fn versioned_python_stub_exclusions(formula: &str) -> HashSet<String> {
    let Some((major, minor)) = parse_python_formula_version(formula) else {
        return HashSet::new();
    };

    [
        "2to3".to_string(),
        format!("2to3-{major}.{minor}"),
        format!("idle{major}"),
        format!("idle{major}.{minor}"),
        format!("pydoc{major}"),
        format!("pydoc{major}.{minor}"),
        "wheel".to_string(),
        format!("wheel{major}"),
        format!("wheel{major}.{minor}"),
        format!("python{major}-config"),
        format!("python{major}.{minor}-config"),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn parse_python_formula_version(formula: &str) -> Option<(u64, u64)> {
    let version = formula.strip_prefix("python@")?;
    let (major, minor) = version.split_once('.')?;
    if minor.contains('.') {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}

pub(crate) fn run_i_vendor(
    config: &Config,
    package_name: String,
    package: vendor::VendorPackage,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i(package_name.clone(), package.name.to_string());
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let version = (package.version)()?;
            let vendor_install = VendorInstall { package, version };
            let dependencies = resolve_vendor_dependency_specs(
                vendor_install.package.dependencies,
                config,
                false,
            )?;
            let dependency_state = resolve_dependency_install_state(
                &dependencies.formula_graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current = dependencies_are_current(
                &staged_plan,
                &dependency_state.installs,
                &dependencies.vendor_installs,
                config,
            )?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependency_state.changed_installs,
                    Some(&progress),
                )?;
                install_vendor_dependencies(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &dependencies.vendor_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            if !vendor_root_is_current(
                &staged_plan,
                &vendor_install,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    if dependencies.formula_graph.is_empty()
                        && dependencies.vendor_installs.is_empty()
                    {
                        prepare_vendor_root_area(&staged_plan)?;
                    } else {
                        reinstall_vendor_dependency_tree(
                            config,
                            &staged_plan,
                            &dependency_state.installs,
                            &dependencies.formula_graph,
                            &dependencies.vendor_installs,
                            Some(&progress),
                        )?;
                    }
                }
                let root_payload_before = prepare_root_payload_install(&staged_plan)?;
                install_vendor_root(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &vendor_install,
                    Some(&progress),
                )?;
                finish_root_payload_install(&staged_plan, root_payload_before)?;
            }

            activate_install(&staged_plan)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: vendor_install.version.to_string(),
                    source: PackageReceiptSource::Vendor {
                        vendor_name: vendor_install.package.name.to_string(),
                    },
                    metadata: PackageMetadata::default(),
                },
            )?;
            sync_vendor_stubs(
                &plan,
                &dependencies.formula_graph,
                &vendor_install.package,
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_npm(
    config: &Config,
    package_name: String,
    npm_package: String,
    requested_version: Option<String>,
    _options: InstallOptions,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i_npm(package_name.clone(), package_name.clone(), &npm_package);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let executable = npm_package_executable_name(&npm_package);
            let mut dependency_names = vec!["node".to_string()];
            append_npm_package_homebrew_dependencies(&mut dependency_names, &npm_package);
            let dependency_names = dependency_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let dependencies = resolve_vendor_dependency_specs(&dependency_names, config, false)?;
            let dependency_state = resolve_dependency_install_state(
                &dependencies.formula_graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current = dependencies_are_current(
                &staged_plan,
                &dependency_state.installs,
                &dependencies.vendor_installs,
                config,
            )?;
            let mut dependencies_reinstalled = false;
            if dependency_current
                && !install_time_commands_are_usable(
                    &staged_plan,
                    &dependencies.formula_graph,
                    ["node", "npm"],
                    Some(&progress),
                )?
            {
                progress.log("reinstalling npm runtime");
                reinstall_vendor_dependency_tree(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependencies.formula_graph,
                    &dependencies.vendor_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            } else if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependency_state.changed_installs,
                    Some(&progress),
                )?;
                install_vendor_dependencies(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &dependencies.vendor_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            let version = resolve_installable_npm_version(
                &staged_plan,
                &dependencies.formula_graph,
                &package_name,
                &npm_package,
                requested_version.as_deref(),
                Some(&progress),
            )?;

            if !npm_root_is_current(
                &staged_plan,
                &executable,
                &version,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    if dependencies.formula_graph.is_empty()
                        && dependencies.vendor_installs.is_empty()
                    {
                        prepare_vendor_root_area(&staged_plan)?;
                    } else {
                        reinstall_vendor_dependency_tree(
                            config,
                            &staged_plan,
                            &dependency_state.installs,
                            &dependencies.formula_graph,
                            &dependencies.vendor_installs,
                            Some(&progress),
                        )?;
                    }
                }
                let root_payload_before = prepare_root_payload_install(&staged_plan)?;
                install_npm_root(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &package_name,
                    &npm_package,
                    &version,
                    Some(&progress),
                )?;
                finish_root_payload_install(&staged_plan, root_payload_before)?;
            }

            activate_install(&staged_plan)?;
            let metadata = resolve_npm_package_metadata(&npm_package)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: version.to_string(),
                    source: PackageReceiptSource::Npm {
                        package_name: npm_package.clone(),
                    },
                    metadata,
                },
            )?;
            sync_declared_stubs(
                &plan,
                &dependencies.formula_graph,
                [executable.as_str()],
                &package_stub_exclusions(&plan.package_name),
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_pip(
    config: &Config,
    package_name: String,
    pip_package: String,
    intent: InstallIntent,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i_pip(package_name.clone(), package_name.clone(), &pip_package);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let prepared = prepare_i_install_plan(&plan, intent)?;
        let staged_plan = prepared.plan;
        let staging_workspace = prepared.workspace;
        let install_result = (|| {
            let version = resolve_pip_latest_version(&pip_package)?;
            let mut dependency_names = vec![pip_package_python_formula(&pip_package)];
            append_pip_package_homebrew_dependencies(&mut dependency_names, &pip_package);
            let formula_graph = resolve_formula_specs(&dependency_names, config, true)?;
            let dependency_state = resolve_dependency_install_state(
                &formula_graph,
                &staged_plan,
                &config.bottle_tag,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current =
                dependencies_are_current(&staged_plan, &dependency_state.installs, &[], config)?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    &dependency_state.changed_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            if !pip_root_is_current(
                &staged_plan,
                &version,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    reinstall_vendor_dependency_tree(
                        config,
                        &staged_plan,
                        &dependency_state.installs,
                        &[],
                        &[],
                        Some(&progress),
                    )?;
                }
                let root_payload_before = prepare_root_payload_install(&staged_plan)?;
                let entrypoints = install_pip_root(
                    &staged_plan,
                    &formula_graph,
                    &package_name,
                    &pip_package,
                    &version,
                    Some(&progress),
                )?;
                finish_root_payload_install(&staged_plan, root_payload_before)?;
                write_root_executable_manifest(
                    &staged_plan.root_executables_manifest_path(),
                    &entrypoints,
                )?;
            }

            activate_install(&staged_plan)?;
            let metadata = resolve_pip_package_metadata(&pip_package)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: version.to_string(),
                    source: PackageReceiptSource::Pip {
                        package_name: pip_package.clone(),
                    },
                    metadata,
                },
            )?;
            let root_executables =
                load_root_executable_manifest(&plan.root_executables_manifest_path())?.stubs;
            sync_declared_stubs(
                &plan,
                &formula_graph,
                &root_executables,
                &package_stub_exclusions(&plan.package_name),
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn isotope_dependency_graph(
    isotope: &IsotopePackageData,
    config: &Config,
) -> Result<Vec<FormulaSpec>, String> {
    let Some(replaces) = isotope.replaces.as_deref() else {
        return Ok(Vec::new());
    };
    let Some(formula) = replaces.strip_prefix(BREW_PACKAGE_PREFIX) else {
        return Ok(Vec::new());
    };
    let formula = canonical_formula_name(formula)?;
    let info = fetch_formula_info(&formula)?;
    resolve_formula_specs(&info.dependencies, config, true)
}

pub(crate) fn pip_package_install_data(package: &str) -> Option<&'static PackageInstallData> {
    embedded_pip_package_data().get(&normalize_pip_package_name(package))
}

pub(crate) fn pip_package_homebrew_dependencies(package: &str) -> Vec<String> {
    pip_package_install_data(package)
        .map(|entry| entry.homebrew_dependencies.clone())
        .unwrap_or_default()
}

pub(crate) fn append_pip_package_homebrew_dependencies(
    formula_names: &mut Vec<String>,
    package: &str,
) {
    for dependency in pip_package_homebrew_dependencies(package) {
        push_unique_string(formula_names, dependency);
    }
}

pub(crate) fn pip_package_python_formula(package: &str) -> String {
    pip_package_install_data(package)
        .and_then(|entry| entry.python_formula.clone())
        .unwrap_or_else(|| "python".to_string())
}

pub(crate) fn append_vendor_npm_homebrew_dependencies(
    formula_names: &mut Vec<String>,
    vendor_installs: &[VendorInstall],
) {
    for install in vendor_installs {
        if let vendor::InstallStrategy::NpmGlobal {
            package: npm_package,
        } = (install.package.install)(&install.version)
        {
            append_npm_package_homebrew_dependencies(formula_names, &npm_package);
        }
    }
}

#[cfg(not(feature = "gold-release"))]
#[allow(dead_code)]
pub(crate) fn maybe_self_update_and_restart(_request: &UpdateRequest) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "gold-release")]
#[allow(dead_code)]
pub(crate) fn maybe_self_update_and_restart(request: &UpdateRequest) -> Result<(), String> {
    if request.no_self_update || !running_from_self_update_target() {
        return Ok(());
    }

    let Some(release) = resolve_self_update_release()? else {
        return Ok(());
    };

    install_self_update(&release)?;
    exec_self_update_restart()
}

#[cfg(feature = "gold-release")]
pub(crate) fn running_from_self_update_target() -> bool {
    env::current_exe()
        .ok()
        .is_some_and(|path| path == Path::new(SELF_UPDATE_TARGET))
}

#[cfg(feature = "gold-release")]
pub(crate) fn resolve_self_update_release() -> Result<Option<SelfUpdateRelease>, String> {
    let release: GithubRelease = fetch_json(
        &format!("https://api.github.com/repos/{SELF_UPDATE_REPO}/releases/latest"),
        || format!("failed to fetch latest release for {SELF_UPDATE_REPO}"),
    )?;
    let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|err| format!("failed to parse current av version: {err}"))?;
    let latest_version = parse_self_update_version(&release.tag_name)?;
    if latest_version <= current_version {
        return Ok(None);
    }

    let asset_name = current_self_update_asset_name(&latest_version).ok_or_else(|| {
        format!(
            "self-update is unsupported on {}-{}",
            env::consts::OS,
            env::consts::ARCH
        )
    })?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| {
            format!(
                "latest av release {} does not contain asset {}",
                release.tag_name, asset_name
            )
        })?;

    Ok(Some(SelfUpdateRelease {
        version: latest_version,
        asset_name,
        download_url: asset.browser_download_url,
    }))
}

#[cfg(feature = "gold-release")]
pub(crate) fn parse_self_update_version(tag: &str) -> Result<semver::Version, String> {
    semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|err| format!("failed to parse release version {tag}: {err}"))
}

#[cfg(feature = "gold-release")]
pub(crate) fn current_self_update_asset_name(version: &semver::Version) -> Option<String> {
    self_update_asset_name_for(version, env::consts::OS, env::consts::ARCH)
}

#[cfg(feature = "gold-release")]
pub(crate) fn self_update_asset_name_for(
    version: &semver::Version,
    os: &str,
    arch: &str,
) -> Option<String> {
    let os = match os {
        "macos" => "Darwin",
        "linux" => "Linux",
        _ => return None,
    };
    let arch = match arch {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        _ => return None,
    };
    Some(format!("nucleus-{version}-{os}-{arch}.tar.gz"))
}

#[cfg(feature = "gold-release")]
pub(crate) fn install_self_update(release: &SelfUpdateRelease) -> Result<(), String> {
    let target = Path::new(SELF_UPDATE_TARGET);
    let target_permissions = fs::metadata(target)
        .map_err(|err| format!("failed to stat {}: {err}", target.display()))?
        .permissions();
    let temp_dir = TempDir::new_in(USR_LOCAL_BIN)
        .map_err(|err| format!("failed to create temp dir in {USR_LOCAL_BIN}: {err}"))?;
    let archive_path = temp_dir.path().join(&release.asset_name);
    download_vendor_asset(&release.download_url, &archive_path, "av", None)?;
    unpack_bottle(&archive_path, temp_dir.path())?;

    let extracted = temp_dir.path().join("av");
    let metadata = fs::metadata(&extracted)
        .map_err(|err| format!("failed to stat {}: {err}", extracted.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "self-update archive for av {} did not contain an av binary",
            release.version
        ));
    }

    fs::set_permissions(&extracted, target_permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", extracted.display()))?;
    fs::rename(&extracted, target).map_err(|err| {
        format!(
            "failed to replace {} with av {}: {err}",
            target.display(),
            release.version
        )
    })
}

#[cfg(feature = "gold-release")]
pub(crate) fn exec_self_update_restart() -> Result<(), String> {
    let mut command = Command::new(SELF_UPDATE_TARGET);
    for arg in env::args_os().skip(1) {
        command.arg(arg);
    }
    command.arg(SELF_UPDATE_DISABLE_FLAG);
    let err = command.exec();
    Err(format!("failed to exec {}: {err}", SELF_UPDATE_TARGET))
}

pub(crate) fn ensure_plan_parent_dirs(plan: &InstallPlan) -> Result<(), String> {
    let stable_parent = plan
        .stable_root
        .parent()
        .ok_or_else(|| format!("invalid stable root {}", plan.stable_root.display()))?;
    let install_parent = plan
        .install_root
        .parent()
        .ok_or_else(|| format!("invalid install root {}", plan.install_root.display()))?;
    fs::create_dir_all(stable_parent)
        .map_err(|err| format!("failed to create {}: {err}", stable_parent.display()))?;
    fs::create_dir_all(install_parent)
        .map_err(|err| format!("failed to create {}: {err}", install_parent.display()))?;
    fs::create_dir_all(&plan.tmp_root)
        .map_err(|err| format!("failed to create {}: {err}", plan.tmp_root.display()))?;
    Ok(())
}

pub(crate) fn install_package(
    config: &Config,
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    changed_installs: &[InstalledFormula],
    rewrite_rules: &[RewriteRule],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if package_is_current(plan, installs, &config.bottle_tag)?
        && installed_formula_receipts_match_graph(plan, installs)?
    {
        return Ok(());
    }

    if plan.install_root.exists() && incremental_root_is_seeded(plan) {
        prepare_incremental_formula_update(plan, installs, changed_installs)?;
    } else {
        prepare_clean_install_root(plan)?;
    }
    let installs_to_write = if incremental_root_is_seeded(plan) {
        changed_installs
    } else {
        installs
    };
    let results: Vec<Result<(), String>> = installs_to_write
        .par_iter()
        .map(|install| install_formula(config, plan, install, rewrite_rules, progress))
        .collect();
    for result in results {
        result?;
    }
    Ok(())
}

pub(crate) fn installed_formula_receipts_match_graph(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
) -> Result<bool, String> {
    let receipts_dir = plan.install_root.join(RECEIPTS_DIR);
    let entries = match fs::read_dir(&receipts_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(installs.is_empty()),
        Err(err) => return Err(format!("failed to read {}: {err}", receipts_dir.display())),
    };
    let expected = installs
        .iter()
        .map(|install| install.spec.name.as_str())
        .collect::<HashSet<_>>();
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", receipts_dir.display()))?;
        if entry.path().extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }
        let Some(receipt) = load_install_receipt(&entry.path())? else {
            continue;
        };
        if !expected.contains(receipt.formula.as_str()) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn install_dependency_formulas(
    config: &Config,
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    changed_installs: &[InstalledFormula],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if installs.is_empty() {
        prepare_vendor_root_area(plan)?;
        return Ok(());
    }

    let rewrite_rules = build_rewrite_rules(plan, installs);
    install_package(
        config,
        plan,
        installs,
        changed_installs,
        &rewrite_rules,
        progress,
    )?;
    run_package_post_install(plan, installs, &managed_bin_root())
}

pub(crate) fn incremental_root_is_seeded(plan: &InstallPlan) -> bool {
    plan.install_root.is_dir() && plan.root_receipt_path().is_file()
}

pub(crate) fn prepare_incremental_formula_update(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    changed_installs: &[InstalledFormula],
) -> Result<(), String> {
    let new_names = installs
        .iter()
        .map(|install| install.spec.name.as_str())
        .collect::<HashSet<_>>();
    let changed_names = changed_installs
        .iter()
        .map(|install| install.spec.name.as_str())
        .collect::<HashSet<_>>();
    let receipts_dir = plan.install_root.join(RECEIPTS_DIR);
    let entries = match fs::read_dir(&receipts_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to read {}: {err}", receipts_dir.display())),
    };
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", receipts_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }
        let Some(receipt) = load_install_receipt(&path)? else {
            continue;
        };
        if changed_names.contains(receipt.formula.as_str())
            || !new_names.contains(receipt.formula.as_str())
        {
            remove_owned_paths(&plan.install_root, &receipt.owned_paths)?;
            remove_path(&path)?;
        }
    }
    Ok(())
}

pub(crate) fn dependencies_are_current(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    vendor_installs: &[VendorInstall],
    config: &Config,
) -> Result<bool, String> {
    if installs.is_empty() && vendor_installs.is_empty() {
        return Ok(
            plan.install_root.is_dir() && installed_formula_receipts_match_graph(plan, installs)?
        );
    }

    if !installs.is_empty() && !package_is_current(plan, installs, &config.bottle_tag)? {
        return Ok(false);
    }
    if !installed_formula_receipts_match_graph(plan, installs)? {
        return Ok(false);
    }

    vendor_dependencies_are_current(plan, vendor_installs)
}

pub(crate) fn vendor_root_is_current(
    plan: &InstallPlan,
    install: &VendorInstall,
    installs: &[InstalledFormula],
    bottle_tag: &str,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }
    if !installs.is_empty() && !package_is_current(plan, installs, bottle_tag)? {
        return Ok(false);
    }
    let Some(receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };
    if receipt.package_name != plan.package_name
        || receipt.version != install.version.to_string()
        || receipt.source
            != (PackageReceiptSource::Vendor {
                vendor_name: install.package.name.to_string(),
            })
    {
        return Ok(false);
    }
    Ok(declared_root_executables_exist(
        &plan.install_root,
        install.package.executables.iter().copied(),
    ))
}

pub(crate) fn npm_root_is_current(
    plan: &InstallPlan,
    executable: &str,
    version: &semver::Version,
    installs: &[InstalledFormula],
    bottle_tag: &str,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }
    if !installs.is_empty() && !package_is_current(plan, installs, bottle_tag)? {
        return Ok(false);
    }
    let Some(receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };
    if receipt.package_name != plan.package_name
        || receipt.version != version.to_string()
        || !matches!(receipt.source, PackageReceiptSource::Npm { .. })
    {
        return Ok(false);
    }
    Ok(declared_root_executables_exist(
        &plan.install_root,
        [executable],
    ))
}

pub(crate) fn pip_root_is_current(
    plan: &InstallPlan,
    version: &str,
    installs: &[InstalledFormula],
    bottle_tag: &str,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }
    if !installs.is_empty() && !package_is_current(plan, installs, bottle_tag)? {
        return Ok(false);
    }
    let Some(receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };
    if receipt.package_name != plan.package_name
        || receipt.version != version
        || !matches!(receipt.source, PackageReceiptSource::Pip { .. })
    {
        return Ok(false);
    }
    if !plan.install_root.join("venv").join("pyvenv.cfg").is_file() {
        return Ok(false);
    }
    let manifest = load_root_executable_manifest(&plan.root_executables_manifest_path())?;
    Ok(declared_root_executables_exist(
        &plan.install_root,
        manifest.stubs.iter().map(String::as_str),
    ))
}

pub(crate) fn cask_binary_target(binary: &EmbeddedCaskBinary) -> Result<&str, String> {
    binary
        .target
        .as_deref()
        .or_else(|| {
            Path::new(&binary.source)
                .file_name()
                .and_then(OsStr::to_str)
        })
        .ok_or_else(|| format!("invalid cask binary path {}", binary.source))
}

pub(crate) fn cask_binary_names(cask: &EmbeddedCaskMetadata) -> Vec<String> {
    cask.binaries
        .iter()
        .filter_map(|binary| cask_binary_target(binary).ok().map(str::to_string))
        .collect()
}

pub(crate) fn cask_root_is_current(
    plan: &InstallPlan,
    cask: &EmbeddedCaskMetadata,
    installs: &[InstalledFormula],
    bottle_tag: &str,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }
    if !installs.is_empty() && !package_is_current(plan, installs, bottle_tag)? {
        return Ok(false);
    }
    let Some(receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };
    if receipt.package_name != plan.package_name
        || receipt.version != cask.version
        || !matches!(receipt.source, PackageReceiptSource::Cask { .. })
    {
        return Ok(false);
    }
    Ok(declared_root_executables_exist(
        &plan.install_root,
        cask_binary_names(cask).iter().map(String::as_str),
    ))
}

pub(crate) fn isotope_root_is_current(
    plan: &InstallPlan,
    isotope: &IsotopePackageData,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }
    let Some(receipt) = load_package_receipt(&plan.root_receipt_path())? else {
        return Ok(false);
    };
    if receipt.package_name != plan.package_name
        || receipt.version != isotope.version
        || !matches!(receipt.source, PackageReceiptSource::Isotope { .. })
    {
        return Ok(false);
    }
    let manifest = load_root_executable_manifest(&plan.root_executables_manifest_path())?;
    Ok(declared_root_executables_exist(
        &plan.install_root,
        manifest.stubs.iter().map(String::as_str),
    ))
}

pub(crate) fn install_cask_root(
    plan: &InstallPlan,
    cask_name: &str,
    cask: &EmbeddedCaskMetadata,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let tmp_dir = TempDir::new_in(&plan.tmp_root)
        .map_err(|err| format!("failed to create temp dir for {cask_name}: {err}"))?;
    let archive_path = tmp_dir.path().join(vendor_archive_name(&cask.url));
    download_cask_archive(cask_name, cask, &archive_path, progress)?;
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("unpacking archive");
    }
    let unpack_root = tmp_dir.path().join("unpacked");
    fs::create_dir_all(&unpack_root)
        .map_err(|err| format!("failed to create {}: {err}", unpack_root.display()))?;
    unpack_cask_payload(&archive_path, &unpack_root, cask_name, cask)?;

    let bin_root = plan.install_root.join("bin");
    fs::create_dir_all(&bin_root)
        .map_err(|err| format!("failed to create {}: {err}", bin_root.display()))?;
    for binary in &cask.binaries {
        let source_path = unpack_root.join(&binary.source);
        if !source_path.is_file() {
            return Err(format!(
                "cask {cask_name} expected {} in downloaded archive",
                source_path.display()
            ));
        }
        let destination = bin_root.join(cask_binary_target(binary)?);
        fs::copy(&source_path, &destination).map_err(|err| {
            format!(
                "failed to copy {} to {}: {err}",
                source_path.display(),
                destination.display()
            )
        })?;
        let mut permissions = fs::metadata(&destination)
            .map_err(|err| format!("failed to stat {}: {err}", destination.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions)
            .map_err(|err| format!("failed to chmod {}: {err}", destination.display()))?;
    }
    Ok(())
}

pub(crate) fn ensure_cask_install_metadata(
    cask_name: &str,
    cask: &EmbeddedCaskMetadata,
) -> Result<(), String> {
    if cask.version.trim().is_empty() {
        return Err(format!(
            "cask {cask_name} is missing version metadata in the package database"
        ));
    }
    if cask.url.trim().is_empty() {
        return Err(format!(
            "cask {cask_name} is missing archive URL metadata in the package database"
        ));
    }
    if cask.sha256.trim().is_empty() {
        return Err(format!(
            "cask {cask_name} is missing sha256 metadata in the package database"
        ));
    }
    Ok(())
}

pub(crate) fn install_isotope_root(
    plan: &InstallPlan,
    isotope: &IsotopePackageData,
    dependency_installs: &[InstalledFormula],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if isotope_root_is_current(plan, isotope)? {
        return Ok(());
    }

    let archive_url = isotope
        .archive_url
        .as_deref()
        .ok_or_else(|| format!("isotope {} has no archive URL", isotope.name))?;
    let tmp_dir = TempDir::new_in(&plan.tmp_root)
        .map_err(|err| format!("failed to create temp dir for {}: {err}", isotope.name))?;
    let archive_path = tmp_dir.path().join(vendor_archive_name(archive_url));
    download_vendor_asset(archive_url, &archive_path, &isotope.name, progress)?;
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("unpacking isotope archive");
    }
    let unpack_root = tmp_dir.path().join("unpacked");
    fs::create_dir_all(&unpack_root)
        .map_err(|err| format!("failed to create {}: {err}", unpack_root.display()))?;
    unpack_vendor_archive(&archive_path, &unpack_root, &isotope.name)?;
    let isotope_root = resolve_isotope_archive_root(&unpack_root)?;
    let rules = build_rewrite_rules(plan, dependency_installs);
    relocate_tree(
        &isotope_root,
        &plan.stable_root,
        &isotope.name,
        &rules,
        progress,
    )?;
    stage_root_formula(&plan.install_root, &isotope_root, true)
}

pub(crate) fn resolve_isotope_archive_root(unpack_root: &Path) -> Result<PathBuf, String> {
    let mut entries = fs::read_dir(unpack_root)
        .map_err(|err| format!("failed to read {}: {err}", unpack_root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", unpack_root.display()))?;
    if entries
        .iter()
        .any(|entry| isotope_archive_top_level_entry_is_install_layout(&entry.file_name()))
    {
        return Ok(unpack_root.to_path_buf());
    }
    if entries.len() == 1 {
        let path = entries.remove(0).path();
        if path.is_dir() {
            return Ok(path);
        }
    }
    Ok(unpack_root.to_path_buf())
}

pub(crate) fn isotope_archive_top_level_entry_is_install_layout(name: &OsStr) -> bool {
    matches!(
        name.as_bytes(),
        b".bottle"
            | b".pkg"
            | b"bin"
            | b"etc"
            | b"include"
            | b"lib"
            | b"libexec"
            | b"sbin"
            | b"share"
            | b"ssl"
    )
}

pub(crate) fn download_cask_archive(
    cask_name: &str,
    cask: &EmbeddedCaskMetadata,
    destination: &Path,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.begin_download_phase();
    }
    let response = ureq::get(&cask.url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| match err {
            UreqError::Status(code, _) => {
                format!("failed to download cask archive for {cask_name}: http {code}")
            }
            UreqError::Transport(err) => {
                format!("failed to download cask archive for {cask_name}: {err}")
            }
        })?;
    if let Some(progress) = progress {
        progress.add_download_total(
            response
                .header("Content-Length")
                .and_then(|value| value.parse::<u64>().ok()),
        );
    }
    let mut reader = response.into_reader();
    let mut file = File::create(destination)
        .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to read cask archive for {cask_name}: {err}"))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|err| format!("failed to write {}: {err}", destination.display()))?;
        if let Some(progress) = progress {
            progress.advance_download(count as u64);
        }
        hasher.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != cask.sha256 {
        return Err(format!(
            "sha256 mismatch for cask {cask_name}: expected {}, got {}",
            cask.sha256, actual
        ));
    }
    Ok(())
}

pub(crate) fn install_vendor_dependencies(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    vendor_installs: &[VendorInstall],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    for vendor_install in vendor_installs {
        install_vendor_root(plan, graph, vendor_install, progress)?;
        write_package_receipt(
            &plan.receipt_path(vendor_install.package.name),
            &PackageReceipt {
                package_name: vendor_install.package.name.to_string(),
                version: vendor_install.version.to_string(),
                source: PackageReceiptSource::Vendor {
                    vendor_name: vendor_install.package.name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )?;
    }
    Ok(())
}

pub(crate) fn reinstall_vendor_dependency_tree(
    config: &Config,
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    graph: &[FormulaSpec],
    vendor_installs: &[VendorInstall],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let downloads = if graph.is_empty() {
        None
    } else {
        Some(download_bottles(graph, &plan.tmp_root, progress)?)
    };
    let installs = if let Some(downloads) = downloads.as_ref() {
        inspect_keg_dirs(graph, downloads)?
    } else {
        installs.to_vec()
    };
    prepare_vendor_root_area(plan)?;
    install_dependency_formulas(config, plan, &installs, &installs, progress)?;
    drop(downloads);
    install_vendor_dependencies(plan, graph, vendor_installs, progress)
}

pub(crate) fn install_time_commands_are_usable<const N: usize>(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    executables: [&str; N],
    progress: Option<&InstallProgress>,
) -> Result<bool, String> {
    for executable in executables {
        if install_time_command_is_usable(plan, graph, executable)? {
            continue;
        }
        if let Some(progress) = progress {
            progress.log(format!("{executable} runtime probe failed"));
        }
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn install_time_command_is_usable(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    executable: &str,
) -> Result<bool, String> {
    let Some(path) = resolve_install_time_command(plan, graph, executable) else {
        return Ok(false);
    };
    let status = Command::new(&path)
        .arg("--version")
        .env("PATH", build_install_path(plan, graph))
        .env("TMPDIR", &plan.tmp_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to probe {}: {err}", path.display()))?;
    Ok(status.success())
}

pub(crate) fn resolve_dependency_install_state(
    graph: &[FormulaSpec],
    plan: &InstallPlan,
    bottle_tag: &str,
    tmp_root: &Path,
    progress: Option<&InstallProgress>,
) -> Result<DependencyInstallState, String> {
    if graph.is_empty() {
        return Ok(DependencyInstallState {
            _downloads: HashMap::new(),
            installs: Vec::new(),
            changed_installs: Vec::new(),
        });
    }

    let mut reusable_installs = Vec::new();
    let mut changed_specs = Vec::new();
    let can_reuse = incremental_root_is_seeded(plan);
    for spec in graph {
        if can_reuse && let Some(receipt) = formula_spec_receipt_is_current(plan, spec, bottle_tag)?
        {
            reusable_installs.push(InstalledFormula {
                spec: spec.clone(),
                keg_dir_name: receipt.version,
                archive_path: PathBuf::new(),
            });
            continue;
        }
        changed_specs.push(spec.clone());
    }

    let downloads = download_bottles(&changed_specs, tmp_root, progress)?;
    let changed_installs = inspect_keg_dirs(&changed_specs, &downloads)?;
    let mut installs = reusable_installs;
    installs.extend(changed_installs.iter().cloned());
    let graph_order = graph
        .iter()
        .enumerate()
        .map(|(index, spec)| (spec.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    installs.sort_by_key(|install| graph_order[install.spec.name.as_str()]);
    Ok(DependencyInstallState {
        _downloads: downloads,
        installs,
        changed_installs,
    })
}

pub(crate) fn prepare_vendor_root_area(plan: &InstallPlan) -> Result<(), String> {
    fs::create_dir_all(&plan.install_root)
        .map_err(|err| format!("failed to create {}: {err}", plan.install_root.display()))?;
    for entry in fs::read_dir(&plan.install_root)
        .map_err(|err| format!("failed to read {}: {err}", plan.install_root.display()))?
    {
        let entry = entry
            .map_err(|err| format!("failed to read {}: {err}", plan.install_root.display()))?;
        remove_path(&entry.path())?;
    }
    fs::create_dir_all(&plan.install_root)
        .map_err(|err| format!("failed to create {}: {err}", plan.install_root.display()))?;
    Ok(())
}

pub(crate) fn vendor_dependencies_are_current(
    plan: &InstallPlan,
    installs: &[VendorInstall],
) -> Result<bool, String> {
    for install in installs {
        if !vendor_dependency_is_current(plan, install)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn vendor_dependency_is_current(
    plan: &InstallPlan,
    install: &VendorInstall,
) -> Result<bool, String> {
    let Some(receipt) = load_package_receipt(&plan.receipt_path(install.package.name))? else {
        return Ok(false);
    };

    if receipt.package_name != install.package.name
        || receipt.version != install.version.to_string()
        || receipt.source
            != (PackageReceiptSource::Vendor {
                vendor_name: install.package.name.to_string(),
            })
    {
        return Ok(false);
    }

    Ok(declared_root_executables_exist(
        &plan.install_root,
        install.package.executables.iter().copied(),
    ))
}

pub(crate) fn install_vendor_root(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    vendor_install: &VendorInstall,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let strategy = (vendor_install.package.install)(&vendor_install.version);
    match strategy {
        vendor::InstallStrategy::NpmGlobal { package } => install_npm_global(
            plan,
            graph,
            vendor_install.package.name,
            &package,
            &vendor_install.version,
            progress,
        ),
        vendor::InstallStrategy::CopyFile {
            source,
            destination_dir,
            destination_name,
            mode,
            create_dirs,
        } => install_vendor_copy_file(
            plan,
            graph,
            vendor_install,
            &source,
            &destination_dir,
            destination_name.as_deref(),
            mode,
            &create_dirs,
            progress,
        ),
        vendor::InstallStrategy::CopyTree { source } => {
            install_vendor_copy_tree(plan, vendor_install, &source, progress)
        }
    }
}

pub(crate) fn install_npm_root(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    display_name: &str,
    npm_package: &str,
    version: &semver::Version,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    install_npm_global(plan, graph, display_name, npm_package, version, progress)
}

pub(crate) fn resolve_installable_npm_version(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    display_name: &str,
    package: &str,
    requested_version: Option<&str>,
    progress: Option<&InstallProgress>,
) -> Result<semver::Version, String> {
    let npm = resolve_install_time_command(plan, graph, "npm")
        .ok_or_else(|| format!("package {display_name} requires npm in PATH"))?;
    let path = build_install_path(plan, graph);
    if let Some(version) = requested_version {
        return vendor::parse_semver(version, package);
    }
    let versions = vendor::npm_versions_desc(package)?;
    let Some(latest_version) = versions.first().cloned() else {
        return Err(format!(
            "no installable npm release found for {display_name}"
        ));
    };
    let latest_error = probe_npm_install_version(
        plan,
        &npm,
        &path,
        display_name,
        package,
        &latest_version,
        progress,
    )?;
    if latest_error.is_none() {
        return Ok(latest_version);
    }
    Err(render_npm_probe_error(display_name, latest_error.unwrap()))
}

pub(crate) fn install_pip_root(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    display_name: &str,
    package: &str,
    version: &str,
    progress: Option<&InstallProgress>,
) -> Result<Vec<String>, String> {
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("creating virtualenv");
    }
    let python = resolve_install_time_command(plan, graph, "python3")
        .or_else(|| resolve_install_time_command(plan, graph, "python"))
        .ok_or_else(|| format!("package {display_name} requires python in PATH"))?;
    let env_root = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!("failed to create temp dir for pip install of {display_name}: {err}")
    })?;
    let venv_root = plan.install_root.join("venv");

    let mut venv_command =
        build_pip_venv_command(&python, &venv_root, env_root.path(), plan, graph)?;
    let output = run_command_with_logged_output(
        &mut venv_command,
        progress,
        &format!("failed to create virtualenv for {display_name}"),
    )?;
    if !output.status.success() {
        return Err(match output.status.code() {
            Some(code) => format!(
                "virtualenv creation failed for {display_name} with exit code {code}{}",
                format_command_output_suffix(&output.lines)
            ),
            None => format!(
                "virtualenv creation terminated by signal for {display_name}{}",
                format_command_output_suffix(&output.lines)
            ),
        });
    }

    if let Some(progress) = progress {
        progress.log("running pip install");
    }
    let pip = venv_root.join("bin/pip");
    let mut pip_command =
        build_pip_install_command(&pip, package, version, env_root.path(), plan, graph)?;
    let output = run_command_with_logged_output(
        &mut pip_command,
        progress,
        &format!("failed to run pip for {display_name}"),
    )?;
    if !output.status.success() {
        return Err(match output.status.code() {
            Some(code) => format!(
                "pip install failed for {display_name} with exit code {code}{}",
                format_command_output_suffix(&output.lines)
            ),
            None => format!(
                "pip install terminated by signal for {display_name}{}",
                format_command_output_suffix(&output.lines)
            ),
        });
    }

    let entrypoints = discover_pip_entrypoints(&venv_root, package)?;
    write_pip_entrypoint_stubs(plan, &venv_root, &entrypoints)?;
    Ok(entrypoints)
}

pub(crate) fn install_npm_global(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    display_name: &str,
    package: &str,
    version: &semver::Version,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("running npm install");
    }
    let npm = resolve_install_time_command(plan, graph, "npm")
        .ok_or_else(|| format!("package {display_name} requires npm in PATH"))?;
    let npm_env = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!("failed to create temp dir for npm install of {display_name}: {err}")
    })?;
    let install_spec = vendor::npm_tarball_url(package, version)?;
    let mut command = build_sandboxed_npm_install_command(
        SANDBOX_EXEC,
        &npm,
        &install_spec,
        &plan.install_root,
        &plan.tmp_root,
        &npm_env,
        build_install_path(plan, graph),
        false,
    )?;
    let output = run_command_with_logged_output(
        &mut command,
        progress,
        &format!("failed to run npm for {display_name}"),
    )?;
    preserve_temp_dir_in_debug(npm_env);
    if output.status.success() {
        normalize_bundled_npm_extension_dependencies(&plan.install_root)?;
        return Ok(());
    }

    Err(match output.status.code() {
        Some(code) => format!(
            "npm install failed for {display_name} with exit code {code}{}",
            format_command_output_suffix(&output.lines)
        ),
        None => format!(
            "npm install terminated by signal for {display_name}{}",
            format_command_output_suffix(&output.lines)
        ),
    })
}

pub(crate) fn pip_env_paths(sandbox_root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        sandbox_root.join("home"),
        sandbox_root.join("xdg-cache"),
        sandbox_root.join("pip-cache"),
    )
}

pub(crate) fn prepare_pip_env(sandbox_root: &Path) -> Result<(), String> {
    let (home, xdg_cache_home, pip_cache_dir) = pip_env_paths(sandbox_root);
    for dir in [&home, &xdg_cache_home, &pip_cache_dir] {
        fs::create_dir_all(dir)
            .map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    }
    Ok(())
}

pub(crate) fn build_pip_venv_command(
    python: impl AsRef<Path>,
    venv_root: &Path,
    sandbox_root: &Path,
    plan: &InstallPlan,
    graph: &[FormulaSpec],
) -> Result<Command, String> {
    prepare_pip_env(sandbox_root)?;
    let mut command = Command::new(python.as_ref());
    command
        .arg("-m")
        .arg("venv")
        .arg("--copies")
        .arg(venv_root)
        .current_dir(sandbox_root)
        .env("PATH", build_install_path(plan, graph))
        .env("TMPDIR", &plan.tmp_root)
        .env("HOME", sandbox_root.join("home"))
        .env("XDG_CACHE_HOME", sandbox_root.join("xdg-cache"))
        .env("PIP_CACHE_DIR", sandbox_root.join("pip-cache"))
        .env("PYTHONNOUSERSITE", "1");
    Ok(command)
}

pub(crate) fn build_pip_install_command(
    pip: &Path,
    package: &str,
    version: &str,
    sandbox_root: &Path,
    plan: &InstallPlan,
    graph: &[FormulaSpec],
) -> Result<Command, String> {
    prepare_pip_env(sandbox_root)?;
    let mut command = Command::new(pip);
    command
        .arg("install")
        .arg("--disable-pip-version-check")
        .arg("--no-input")
        .arg(format!("{package}=={version}"))
        .current_dir(sandbox_root)
        .env("PATH", build_install_path(plan, graph))
        .env("TMPDIR", &plan.tmp_root)
        .env("HOME", sandbox_root.join("home"))
        .env("XDG_CACHE_HOME", sandbox_root.join("xdg-cache"))
        .env("PIP_CACHE_DIR", sandbox_root.join("pip-cache"))
        .env("PYTHONNOUSERSITE", "1");
    Ok(command)
}

pub(crate) fn discover_pip_entrypoints(
    venv_root: &Path,
    package: &str,
) -> Result<Vec<String>, String> {
    let python = venv_root.join("bin/python");
    let mut command = Command::new(&python);
    command.arg("-c").arg(pip_entrypoint_discovery_script());
    command.arg(package);
    let output = command
        .output()
        .map_err(|err| format!("failed to inspect entrypoints for {package}: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            String::new()
        } else {
            format!(": {stderr}")
        };
        return Err(format!(
            "failed to inspect entrypoints for {package}{detail}"
        ));
    }
    let mut entrypoints: Vec<String> = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("failed to parse entrypoints for {package}: {err}"))?;
    entrypoints.retain(|entrypoint| is_executable(&venv_root.join("bin").join(entrypoint)));
    entrypoints.sort();
    entrypoints.dedup();
    Ok(entrypoints)
}

pub(crate) fn pip_entrypoint_discovery_script() -> &'static str {
    r#"import importlib.metadata as md, json, sys
def norm(value):
    out = []
    last_sep = False
    for ch in value.lower():
        if ch.isalnum():
            out.append(ch)
            last_sep = False
        elif ch in '-_.':
            if not last_sep:
                out.append('-')
                last_sep = True
    return ''.join(out).strip('-')
want = norm(sys.argv[1])
for dist in md.distributions():
    name = dist.metadata.get('Name')
    if name and norm(name) == want:
        print(json.dumps(sorted({ep.name for ep in dist.entry_points if ep.group in {'console_scripts', 'gui_scripts'}})))
        raise SystemExit(0)
print('[]')
"#
}

pub(crate) fn write_pip_entrypoint_stubs(
    plan: &InstallPlan,
    venv_root: &Path,
    entrypoints: &[String],
) -> Result<(), String> {
    let bin_dir = plan.install_root.join("bin");
    fs::create_dir_all(&bin_dir)
        .map_err(|err| format!("failed to create {}: {err}", bin_dir.display()))?;
    for entrypoint in entrypoints {
        write_venv_stub(
            plan,
            &bin_dir.join(entrypoint),
            &venv_root.join("bin").join(entrypoint),
            venv_root,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sandboxed_npm_install_command(
    sandbox_exec: impl AsRef<Path>,
    npm: impl AsRef<Path>,
    install_spec: &str,
    install_root: &Path,
    tmp_root: &Path,
    sandbox_root: &TempDir,
    path: OsString,
    dry_run: bool,
) -> Result<Command, String> {
    let sandbox_home = sandbox_root.path().join("home");
    let xdg_config_home = sandbox_root.path().join("xdg-config");
    let xdg_cache_home = sandbox_root.path().join("xdg-cache");
    let npm_cache = sandbox_root.path().join("npm-cache");
    let npm_userconfig = sandbox_root.path().join("npmrc");
    let sandbox_profile = sandbox_root.path().join("sandbox.sb");
    let ca_file = npm
        .as_ref()
        .parent()
        .and_then(Path::parent)
        .unwrap_or(install_root)
        .join(OPENSSL_CERT_PEM_DESTINATION);

    for dir in [&sandbox_home, &xdg_config_home, &xdg_cache_home, &npm_cache] {
        fs::create_dir_all(dir)
            .map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    }
    fs::write(&npm_userconfig, b"" as &[u8])
        .map_err(|err| format!("failed to create {}: {err}", npm_userconfig.display()))?;
    fs::write(&sandbox_profile, npm_install_sandbox_profile(tmp_root))
        .map_err(|err| format!("failed to write {}: {err}", sandbox_profile.display()))?;

    let mut command = if should_bypass_npm_install_sandbox() {
        Command::new(npm.as_ref())
    } else {
        let mut command = Command::new(sandbox_exec.as_ref());
        command.arg("-f").arg(&sandbox_profile).arg(npm.as_ref());
        command
    };
    command
        .arg("install")
        .arg("-g")
        .args(dry_run.then_some("--dry-run"))
        .arg("--prefix")
        .arg(install_root)
        .arg(install_spec)
        .env("PATH", path)
        .env("HOME", &sandbox_home)
        .env("XDG_CONFIG_HOME", &xdg_config_home)
        .env("XDG_CACHE_HOME", &xdg_cache_home)
        .env("NPM_CONFIG_CACHE", &npm_cache)
        .env("NPM_CONFIG_USERCONFIG", &npm_userconfig)
        .env("NPM_CONFIG_CAFILE", &ca_file)
        .env("NODE_EXTRA_CA_CERTS", &ca_file)
        .env("TMPDIR", tmp_root)
        .current_dir(sandbox_root.path());
    Ok(command)
}

pub(crate) fn should_bypass_npm_install_sandbox() -> bool {
    cfg!(test) && env::var_os("CODEX_CI").is_some()
}

pub(crate) struct NpmProbeError {
    pub(crate) status: std::process::ExitStatus,
    pub(crate) lines: Vec<String>,
}

pub(crate) fn render_npm_probe_error(display_name: &str, error: NpmProbeError) -> String {
    match error.status.code() {
        Some(code) => format!(
            "npm install failed for {display_name} with exit code {code}{}",
            format_command_output_suffix(&error.lines)
        ),
        None => format!(
            "npm install terminated by signal for {display_name}{}",
            format_command_output_suffix(&error.lines)
        ),
    }
}

pub(crate) fn probe_npm_install_version(
    plan: &InstallPlan,
    npm: &Path,
    path: &OsString,
    display_name: &str,
    package: &str,
    version: &semver::Version,
    progress: Option<&InstallProgress>,
) -> Result<Option<NpmProbeError>, String> {
    let npm_env = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!("failed to create temp dir for npm install of {display_name}: {err}")
    })?;
    let probe_root = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!("failed to create temp dir for npm install of {display_name}: {err}")
    })?;
    for dir in ["bin", "lib"] {
        fs::create_dir_all(probe_root.path().join(dir)).map_err(|err| {
            format!(
                "failed to create {} for npm install of {display_name}: {err}",
                probe_root.path().join(dir).display()
            )
        })?;
    }
    let install_spec = vendor::npm_tarball_url(package, version)?;
    let mut command = build_sandboxed_npm_install_command(
        SANDBOX_EXEC,
        npm,
        &install_spec,
        probe_root.path(),
        &plan.tmp_root,
        &npm_env,
        path.clone(),
        true,
    )?;
    let output = run_command_with_logged_output(
        &mut command,
        progress,
        &format!("failed to run npm for {display_name}"),
    )?;
    preserve_temp_dir_in_debug(npm_env);
    preserve_temp_dir_in_debug(probe_root);
    if output.status.success() {
        return Ok(None);
    }
    Ok(Some(NpmProbeError {
        status: output.status,
        lines: output.lines,
    }))
}

pub(crate) fn normalize_bundled_npm_extension_dependencies(
    install_root: &Path,
) -> Result<(), String> {
    let node_modules_root = install_root.join("lib/node_modules");
    let package_roots = match collect_npm_package_roots(&node_modules_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to read {}: {err}",
                node_modules_root.display()
            ));
        }
    };

    for package_root in package_roots {
        let root_node_modules = package_root.join("node_modules");
        for extension_node_modules in collect_nested_node_modules_dirs(&package_root.join("dist"))?
        {
            link_missing_npm_packages(&extension_node_modules, &root_node_modules)?;
        }
    }

    Ok(())
}

pub(crate) fn collect_npm_package_roots(node_modules_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut package_roots = Vec::new();
    for entry in fs::read_dir(node_modules_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if entry.file_name().to_string_lossy().starts_with('@') {
            package_roots.extend(collect_npm_package_roots(&path)?);
            continue;
        }
        package_roots.push(path);
    }
    Ok(package_roots)
}

pub(crate) fn collect_nested_node_modules_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut node_modules_dirs = Vec::new();
    collect_nested_node_modules_dirs_inner(root, &mut node_modules_dirs)?;
    Ok(node_modules_dirs)
}

pub(crate) fn collect_nested_node_modules_dirs_inner(
    root: &Path,
    node_modules_dirs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to read {}: {err}", root.display())),
    };

    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", root.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if entry.file_name() == OsStr::new("node_modules") {
            node_modules_dirs.push(path);
            continue;
        }
        collect_nested_node_modules_dirs_inner(&path, node_modules_dirs)?;
    }

    Ok(())
}

pub(crate) fn link_missing_npm_packages(
    source_root: &Path,
    target_root: &Path,
) -> Result<(), String> {
    let entries = match fs::read_dir(source_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to read {}: {err}", source_root.display())),
    };

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", source_root.display()))?;
        let source = entry.path();
        let name = entry.file_name();
        let target = target_root.join(&name);

        if name.to_string_lossy().starts_with('@') {
            link_missing_npm_packages(&source, &target)?;
            continue;
        }

        if target.exists() || fs::symlink_metadata(&target).is_ok() {
            continue;
        }

        fs::create_dir_all(target_root)
            .map_err(|err| format!("failed to create {}: {err}", target_root.display()))?;
        let relative_source = relative_path_from(
            target
                .parent()
                .ok_or_else(|| format!("failed to resolve parent of {}", target.display()))?,
            &source,
        );
        symlink(&relative_source, &target).map_err(|err| {
            format!(
                "failed to link {} -> {}: {err}",
                target.display(),
                relative_source.display()
            )
        })?;
    }

    Ok(())
}

pub(crate) fn npm_install_sandbox_profile(tmp_root: &Path) -> String {
    format!(
        r#"(version 1)
(allow default)
(deny file-read* (subpath "/Library"))
(deny file-write* (subpath "/Library"))
(deny file-write* (subpath "/System"))
(deny file-write* (subpath "/Applications"))
(deny file-write* (subpath "/etc"))
(deny file-read* (subpath "/Users"))
(deny file-write* (subpath "/Users"))
(allow file-read* (subpath "{}"))
(allow file-write* (subpath "{}"))
"#,
        escape_sandbox_path(tmp_root),
        escape_sandbox_path(tmp_root)
    )
}

pub(crate) fn escape_sandbox_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', r"\\")
        .replace('"', "\\\"")
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn install_vendor_copy_file(
    plan: &InstallPlan,
    _graph: &[FormulaSpec],
    vendor_install: &VendorInstall,
    source: &str,
    destination_dir: &str,
    destination_name: Option<&str>,
    mode: u32,
    create_dirs: &[String],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let download_url = vendor_install.package.download_url.ok_or_else(|| {
        format!(
            "vendor package {} has no download URL",
            vendor_install.package.name
        )
    })?;
    let archive_url = download_url(&vendor_install.version);
    let tmp_dir = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!(
            "failed to create temp dir for {}: {err}",
            vendor_install.package.name
        )
    })?;
    let archive_path = tmp_dir.path().join(vendor_archive_name(&archive_url));
    download_vendor_asset(
        &archive_url,
        &archive_path,
        vendor_install.package.name,
        progress,
    )?;
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("unpacking archive");
    }
    let unpack_root = tmp_dir.path().join("unpacked");
    fs::create_dir_all(&unpack_root)
        .map_err(|err| format!("failed to create {}: {err}", unpack_root.display()))?;
    unpack_vendor_archive(&archive_path, &unpack_root, vendor_install.package.name)?;

    for dir in create_dirs {
        fs::create_dir_all(plan.install_root.join(dir)).map_err(|err| {
            format!(
                "failed to create {}: {err}",
                plan.install_root.join(dir).display()
            )
        })?;
    }

    let source_path = unpack_root.join(source);
    if !source_path.is_file() {
        return Err(format!(
            "vendor package {} expected {} in downloaded archive",
            vendor_install.package.name,
            source_path.display()
        ));
    }

    let destination_root = plan.install_root.join(destination_dir);
    fs::create_dir_all(&destination_root)
        .map_err(|err| format!("failed to create {}: {err}", destination_root.display()))?;
    let filename = destination_name
        .map(OsStr::new)
        .or_else(|| Path::new(source).file_name())
        .ok_or_else(|| format!("invalid vendor source path {source}"))?;
    let destination = destination_root.join(filename);
    fs::copy(&source_path, &destination).map_err(|err| {
        format!(
            "failed to copy {} to {}: {err}",
            source_path.display(),
            destination.display()
        )
    })?;
    let mut permissions = fs::metadata(&destination)
        .map_err(|err| format!("failed to stat {}: {err}", destination.display()))?
        .permissions();
    permissions.set_mode(mode);
    fs::set_permissions(&destination, permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", destination.display()))
}

pub(crate) fn install_vendor_copy_tree(
    plan: &InstallPlan,
    vendor_install: &VendorInstall,
    source: &str,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let download_url = vendor_install.package.download_url.ok_or_else(|| {
        format!(
            "vendor package {} has no download URL",
            vendor_install.package.name
        )
    })?;
    let archive_url = download_url(&vendor_install.version);
    let tmp_dir = TempDir::new_in(&plan.tmp_root).map_err(|err| {
        format!(
            "failed to create temp dir for {}: {err}",
            vendor_install.package.name
        )
    })?;
    let archive_path = tmp_dir.path().join(vendor_archive_name(&archive_url));
    download_vendor_asset(
        &archive_url,
        &archive_path,
        vendor_install.package.name,
        progress,
    )?;
    if let Some(progress) = progress {
        progress.begin_install_phase();
        progress.log("unpacking archive");
    }
    let unpack_root = tmp_dir.path().join("unpacked");
    fs::create_dir_all(&unpack_root)
        .map_err(|err| format!("failed to create {}: {err}", unpack_root.display()))?;
    unpack_vendor_archive(&archive_path, &unpack_root, vendor_install.package.name)?;

    let source_root = unpack_root.join(source);
    if !source_root.is_dir() {
        return Err(format!(
            "vendor package {} expected {} in downloaded archive",
            vendor_install.package.name,
            source_root.display()
        ));
    }

    stage_root_formula(&plan.install_root, &source_root, true)
}

pub(crate) fn package_is_current(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    bottle_tag: &str,
) -> Result<bool, String> {
    if !plan.install_root.is_dir() {
        return Ok(false);
    }

    for install in installs {
        if !receipt_is_current(plan, install, bottle_tag)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn receipt_is_current(
    plan: &InstallPlan,
    install: &InstalledFormula,
    bottle_tag: &str,
) -> Result<bool, String> {
    let Some(receipt) = load_install_receipt(&plan.receipt_path(&install.spec.name))? else {
        return Ok(false);
    };
    Ok(receipt.formula == install.spec.name
        && receipt.version == install.keg_dir_name
        && receipt.bottle_sha256 == install.spec.bottle_sha256
        && receipt.bottle_tag == bottle_tag)
}

pub(crate) fn formula_spec_receipt_is_current(
    plan: &InstallPlan,
    spec: &FormulaSpec,
    bottle_tag: &str,
) -> Result<Option<InstallReceipt>, String> {
    let Some(receipt) = load_install_receipt(&plan.receipt_path(&spec.name))? else {
        return Ok(None);
    };
    if receipt.formula == spec.name
        && receipt.bottle_sha256 == spec.bottle_sha256
        && receipt.bottle_tag == bottle_tag
        && !receipt.owned_paths.is_empty()
    {
        return Ok(Some(receipt));
    }
    Ok(None)
}

pub(crate) fn prepare_clean_install_root(plan: &InstallPlan) -> Result<(), String> {
    if plan.install_root.exists() {
        remove_path(&plan.install_root)?;
    }
    fs::create_dir_all(&plan.install_root)
        .map_err(|err| format!("failed to create {}: {err}", plan.install_root.display()))?;
    Ok(())
}

pub(crate) fn activate_install(plan: &InstallPlan) -> Result<(), String> {
    if plan.install_root == plan.stable_root {
        return Ok(());
    }

    if plan.stable_root.exists() {
        remove_path(&plan.stable_root)?;
    }
    fs::rename(&plan.install_root, &plan.stable_root).map_err(|err| {
        format!(
            "failed to move {} to {}: {err}",
            plan.install_root.display(),
            plan.stable_root.display()
        )
    })?;

    Ok(())
}

pub(crate) fn uninstall_package(package_name: &str) -> Result<(), String> {
    ensure_package_installed(&opt_pkg_root(), package_name)?;
    remove_existing_package_install(&opt_pkg_root(), package_name, &managed_bin_root())
}

pub(crate) fn ensure_package_installed(opt_root: &Path, package_name: &str) -> Result<(), String> {
    let install_root = package_install_root(opt_root, package_name)?;
    match fs::symlink_metadata(&install_root) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("package {package_name} is not installed"))
        }
        Err(err) => Err(format!("failed to stat {}: {err}", install_root.display())),
    }
}

pub(crate) fn prepare_install_target(
    opt_root: &Path,
    package_name: &str,
    intent: InstallIntent,
    bin_dir: &Path,
) -> Result<(), String> {
    let install_root = package_install_root(opt_root, package_name)?;
    match fs::symlink_metadata(&install_root) {
        Ok(_) if intent == InstallIntent::Reinstall => {
            remove_existing_package_install(opt_root, package_name, bin_dir)
        }
        Ok(_) if !install_root_has_valid_receipt(package_name, &install_root)? => {
            remove_existing_package_install(opt_root, package_name, bin_dir)
        }
        Ok(_) if intent == InstallIntent::Update => Ok(()),
        Ok(_) => Err(format!(
            "package {package_name} is already installed; use --force/-f to reinstall"
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to stat {}: {err}", install_root.display())),
    }
}

pub(crate) fn install_root_has_valid_receipt(
    package_name: &str,
    install_root: &Path,
) -> Result<bool, String> {
    let Some(receipt) = load_package_receipt(&install_root.join(ROOT_RECEIPT))? else {
        return Ok(false);
    };
    Ok(receipt.package_name == package_name)
}

pub(crate) fn rollback_failed_install(
    opt_root: &Path,
    package_name: &str,
    bin_dir: &Path,
) -> Result<(), String> {
    let install_root = package_install_root(opt_root, package_name)?;
    match fs::symlink_metadata(&install_root) {
        Ok(_) => remove_existing_package_install(opt_root, package_name, bin_dir),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to stat {}: {err}", install_root.display())),
    }
}

pub(crate) fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|err| format!("failed to remove {}: {err}", path.display()))
    } else {
        fs::remove_dir_all(path)
            .map_err(|err| format!("failed to remove {}: {err}", path.display()))
    }
}

pub(crate) fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|entry| entry == &value) {
        values.push(value);
    }
}

pub(crate) fn current_bottle_tag() -> Result<String, String> {
    match env::consts::OS {
        "macos" => current_macos_bottle_tag(),
        "linux" => current_linux_bottle_tag(),
        other => Err(format!("unsupported operating system {other}")),
    }
}

pub(crate) fn resolve_formula_specs(
    formulas: &[String],
    config: &Config,
    allow_supported_post_install: bool,
) -> Result<Vec<FormulaSpec>, String> {
    let mut visiting = HashSet::new();
    let mut resolved = HashMap::new();
    let mut order = Vec::new();

    for formula in formulas {
        resolve_formula_spec(
            formula,
            config,
            allow_supported_post_install,
            &mut visiting,
            &mut resolved,
            &mut order,
        )?;
    }

    let mut specs = Vec::with_capacity(order.len());
    for name in order {
        let info = resolved
            .remove(&name)
            .ok_or_else(|| format!("missing resolved metadata for {name}"))?;
        let file = select_formula_bottle_file(&name, &info, &config.bottle_tag)?;
        specs.push(FormulaSpec {
            name,
            bottle_sha256: file.sha256.clone(),
            bottle_url: file.url.clone(),
        });
    }

    Ok(specs)
}

pub(crate) fn resolve_vendor_dependency_specs(
    dependencies: &[&str],
    config: &Config,
    allow_supported_post_install: bool,
) -> Result<ResolvedVendorDependencies, String> {
    let (mut formula_names, vendor_names) = partition_dependency_names(dependencies)?;
    let mut vendor_installs = Vec::with_capacity(vendor_names.len());
    for name in vendor_names {
        let package =
            vendor::get(&name).ok_or_else(|| format!("vendor package {name} is not registered"))?;
        let version = (package.version)()?;
        vendor_installs.push(VendorInstall { package, version });
    }
    append_vendor_npm_homebrew_dependencies(&mut formula_names, &vendor_installs);
    let formula_graph =
        resolve_formula_specs(&formula_names, config, allow_supported_post_install)?;

    Ok(ResolvedVendorDependencies {
        formula_graph,
        vendor_installs,
    })
}

pub(crate) fn partition_dependency_names(
    dependencies: &[&str],
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut visiting = HashSet::new();
    let mut resolved_vendors = HashSet::new();
    let mut formula_names = Vec::new();
    let mut vendor_names = Vec::new();
    for dependency in dependencies {
        collect_dependency_names(
            dependency,
            &mut visiting,
            &mut resolved_vendors,
            &mut formula_names,
            &mut vendor_names,
        )?;
    }
    Ok((formula_names, vendor_names))
}

pub(crate) fn collect_dependency_names(
    dependency: &str,
    visiting: &mut HashSet<String>,
    resolved_vendors: &mut HashSet<String>,
    formula_names: &mut Vec<String>,
    vendor_names: &mut Vec<String>,
) -> Result<(), String> {
    let Some(package) = vendor::get(dependency) else {
        push_unique_string(formula_names, dependency.to_string());
        return Ok(());
    };

    if resolved_vendors.contains(dependency) {
        return Ok(());
    }
    if !visiting.insert(dependency.to_string()) {
        return Err(format!("cyclic vendor dependency detected at {dependency}"));
    }

    for child in package.dependencies {
        collect_dependency_names(
            child,
            visiting,
            resolved_vendors,
            formula_names,
            vendor_names,
        )?;
    }

    visiting.remove(dependency);
    resolved_vendors.insert(dependency.to_string());
    vendor_names.push(dependency.to_string());
    Ok(())
}

pub(crate) fn current_linux_bottle_tag() -> Result<String, String> {
    match env::consts::ARCH {
        "aarch64" => Ok("arm64_linux".to_string()),
        "x86_64" => Ok("x86_64_linux".to_string()),
        other => Err(format!("unsupported Linux architecture {other}")),
    }
}

pub(crate) fn current_macos_bottle_tag() -> Result<String, String> {
    let arch_prefix = match env::consts::ARCH {
        "aarch64" => "arm64_",
        "x86_64" => "",
        other => return Err(format!("unsupported macOS architecture {other}")),
    };
    let release = macos_release_name(macos_major_version()?)
        .ok_or_else(|| "unsupported macOS release for Homebrew bottles".to_string())?;
    Ok(format!("{arch_prefix}{release}"))
}

pub(crate) fn macos_major_version() -> Result<u32, String> {
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .map_err(|err| format!("failed to run sw_vers: {err}"))?;
    if !output.status.success() {
        return Err("sw_vers -productVersion failed".to_string());
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("sw_vers returned non-utf8: {err}"))?;
    let major = stdout
        .trim()
        .split('.')
        .next()
        .ok_or_else(|| "sw_vers returned an empty version".to_string())?;
    major
        .parse::<u32>()
        .map_err(|err| format!("failed to parse macOS version {major}: {err}"))
}

pub(crate) fn macos_release_name(major: u32) -> Option<&'static str> {
    match major {
        11 => Some("big_sur"),
        12 => Some("monterey"),
        13 => Some("ventura"),
        14 => Some("sonoma"),
        15 => Some("sequoia"),
        16 | 26 => Some("tahoe"),
        _ => None,
    }
}

pub(crate) fn resolve_formula_spec(
    formula: &str,
    config: &Config,
    allow_supported_post_install: bool,
    visiting: &mut HashSet<String>,
    resolved: &mut HashMap<String, FormulaInfo>,
    order: &mut Vec<String>,
) -> Result<(), String> {
    let formula = canonical_formula_name(formula)?;
    if resolved.contains_key(&formula) {
        return Ok(());
    }
    if !visiting.insert(formula.clone()) {
        return Err(format!("cyclic formula dependency detected at {formula}"));
    }

    let info = fetch_formula_info(&formula)?;
    if info.disabled {
        return Err(format!("formula {formula} is disabled"));
    }
    if formula_skips_unknown_post_install(&formula, &info, allow_supported_post_install) {
        let mut stderr = std::io::stderr();
        warn_skipped_post_install(&formula, &mut stderr);
    }
    ensure_formula_has_bottle(&formula, &info, &config.bottle_tag)?;

    let dependencies = info.dependencies.clone();
    for dependency in dependencies {
        resolve_formula_spec(
            &dependency,
            config,
            allow_supported_post_install,
            visiting,
            resolved,
            order,
        )?;
    }

    visiting.remove(&formula);
    resolved.insert(formula.clone(), info);
    order.push(formula);
    Ok(())
}

pub(crate) fn ensure_formula_has_bottle(
    formula: &str,
    info: &FormulaInfo,
    bottle_tag: &str,
) -> Result<(), String> {
    let _ = select_formula_bottle_file(formula, info, bottle_tag)?;
    Ok(())
}

pub(crate) fn select_formula_bottle_file<'a>(
    formula: &str,
    info: &'a FormulaInfo,
    bottle_tag: &str,
) -> Result<&'a BottleFile, String> {
    let bottle = info
        .bottle
        .stable
        .as_ref()
        .ok_or_else(|| format!("formula {formula} has no stable bottle"))?;
    bottle
        .files
        .get(bottle_tag)
        .or_else(|| bottle.files.get("all"))
        .ok_or_else(|| format!("formula {formula} has no bottle for {bottle_tag} or all"))
}

pub(crate) fn formula_version_string(info: &FormulaInfo) -> String {
    if info.revision == 0 {
        info.versions.stable.clone()
    } else {
        format!("{}_{}", info.versions.stable, info.revision)
    }
}

pub(crate) fn fetch_formula_info(formula: &str) -> Result<FormulaInfo, String> {
    if let Some(info) = fetch_formula_info_by_api_name(formula, formula)? {
        return Ok(info);
    }
    let resolved = resolve_formula_api_alias(formula)?
        .ok_or_else(|| format!("failed to fetch formula metadata for {formula}: http 404"))?;
    fetch_formula_info_by_api_name(formula, &resolved)?
        .ok_or_else(|| format!("failed to fetch formula metadata for {formula}: http 404"))
}

pub(crate) fn formula_metadata_exists(formula: &str) -> Result<bool, String> {
    if fetch_formula_info_by_api_name(formula, formula)?.is_some() {
        return Ok(true);
    }
    Ok(resolve_formula_api_alias(formula)?.is_some())
}

pub(crate) fn fetch_formula_info_by_api_name(
    formula: &str,
    api_name: &str,
) -> Result<Option<FormulaInfo>, String> {
    fetch_optional_json(&format!("{}/{api_name}.json", formula_api_root()), || {
        format!("failed to fetch formula metadata for {formula}")
    })
}

pub(crate) fn resolve_formula_api_alias(formula: &str) -> Result<Option<String>, String> {
    let index = formula_alias_index()?;
    Ok(index.get(formula).cloned())
}

pub(crate) fn fetch_json<T, F>(url: &str, context: F) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
    F: FnOnce() -> String,
{
    let context = context();
    fetch_optional_json(url, || context.clone())?.ok_or_else(|| format!("{context}: http 404"))
}

pub(crate) fn fetch_optional_json<T, F>(url: &str, context: F) -> Result<Option<T>, String>
where
    T: serde::de::DeserializeOwned,
    F: FnOnce() -> String,
{
    let context = context();
    let response = match ureq::get(url).set("User-Agent", USER_AGENT).call() {
        Ok(response) => response,
        Err(UreqError::Status(404, _)) => return Ok(None),
        Err(UreqError::Status(code, _)) => return Err(format!("{context}: http {code}")),
        Err(UreqError::Transport(err)) => return Err(format!("{context}: {err}")),
    };
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| format!("{context}: {err}"))?;
    let value = serde_json::from_slice(&bytes).map_err(|err| format!("{context}: {err}"))?;
    Ok(Some(value))
}

pub(crate) fn ambiguous_install_target_message(package: &str, executable_provider: &str) -> String {
    format!(
        "ambiguous install target '{package}': use `{BREW_PACKAGE_PREFIX}{package}` for the Homebrew \
package or `{executable_provider}` for the package that provides the `{package}` executable"
    )
}

pub(crate) fn formula_skips_unknown_post_install(
    formula: &str,
    info: &FormulaInfo,
    allow_supported_post_install: bool,
) -> bool {
    info.post_install_defined
        && !embedded_post_install_check_skip().contains(formula)
        && !(allow_supported_post_install && post_install_hooks::supports(formula))
}

pub(crate) fn skipped_post_install_message(formula: &str) -> String {
    format!("warning: skipping Homebrew post_install for {formula}; install may be incomplete")
}

pub(crate) fn warn_skipped_post_install<W: Write>(formula: &str, stderr: &mut W) {
    let _ = writeln!(stderr, "{}", skipped_post_install_message(formula));
}

pub(crate) fn download_bottles(
    specs: &[FormulaSpec],
    tmp_root: &Path,
    progress: Option<&InstallProgress>,
) -> Result<HashMap<String, DownloadedBottle>, String> {
    if let Some(progress) = progress {
        progress.begin_download_phase();
    }
    let results: Vec<Result<(String, DownloadedBottle), String>> = specs
        .par_iter()
        .map(|spec| {
            let tmp_dir = TempDir::new_in(tmp_root)
                .map_err(|err| format!("failed to create tmp dir for {}: {err}", spec.name))?;
            let archive_path = tmp_dir.path().join("bottle.tar.gz");
            download_bottle(spec, &archive_path, progress)?;
            Ok((
                spec.name.clone(),
                DownloadedBottle {
                    path: archive_path,
                    _tmp_dir: tmp_dir,
                },
            ))
        })
        .collect();

    let mut downloads = HashMap::with_capacity(specs.len());
    for result in results {
        let (formula, download) = result?;
        downloads.insert(formula, download);
    }
    Ok(downloads)
}

pub(crate) fn download_bottle(
    spec: &FormulaSpec,
    destination: &Path,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.begin_download_for(&spec.name);
    }
    let mut request = ureq::get(&spec.bottle_url).set("User-Agent", USER_AGENT);
    if let Some(repo) = ghcr_repo_from_blob_url(&spec.bottle_url) {
        let token = ghcr_bearer_token(repo)?;
        request = request.set("Authorization", &format!("Bearer {token}"));
    }
    let response = request.call().map_err(|err| match err {
        UreqError::Status(code, _) => {
            format!("failed to download bottle for {}: http {code}", spec.name)
        }
        UreqError::Transport(err) => {
            format!("failed to download bottle for {}: {err}", spec.name)
        }
    })?;
    if let Some(progress) = progress {
        progress.add_download_total_for(
            &spec.name,
            response
                .header("Content-Length")
                .and_then(|value| value.parse::<u64>().ok()),
        );
    }
    let mut reader = response.into_reader();
    let mut file = File::create(destination)
        .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32 * 1024];

    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to read bottle for {}: {err}", spec.name))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|err| format!("failed to write {}: {err}", destination.display()))?;
        if let Some(progress) = progress {
            progress.advance_download_for(&spec.name, count as u64);
        }
        hasher.update(&buffer[..count]);
    }

    let actual = format!("{:x}", hasher.finalize());
    if actual != spec.bottle_sha256 {
        return Err(format!(
            "sha256 mismatch for {}: expected {}, got {}",
            spec.name, spec.bottle_sha256, actual
        ));
    }

    Ok(())
}

pub(crate) fn ghcr_repo_from_blob_url(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://ghcr.io/v2/")?;
    let (repo, _) = rest.split_once("/blobs/")?;
    Some(repo)
}

pub(crate) fn ghcr_bearer_token(repo: &str) -> Result<String, String> {
    let url = format!("https://ghcr.io/token?service=ghcr.io&scope=repository:{repo}:pull");
    let response: GhcrTokenResponse =
        fetch_json(&url, || format!("failed to fetch GHCR token for {repo}"))?;
    Ok(response.token)
}

pub(crate) fn inspect_keg_dirs(
    specs: &[FormulaSpec],
    downloads: &HashMap<String, DownloadedBottle>,
) -> Result<Vec<InstalledFormula>, String> {
    let results: Vec<Result<InstalledFormula, String>> = specs
        .par_iter()
        .map(|spec| {
            let bottle_path = downloads
                .get(&spec.name)
                .ok_or_else(|| format!("missing downloaded bottle for {}", spec.name))?;
            let keg_dir_name = archive_keg_dir_name(&bottle_path.path, &spec.name)?;
            Ok(InstalledFormula {
                spec: spec.clone(),
                keg_dir_name,
                archive_path: bottle_path.path.clone(),
            })
        })
        .collect();

    let mut installs = Vec::with_capacity(specs.len());
    for result in results {
        installs.push(result?);
    }
    Ok(installs)
}

pub(crate) fn archive_keg_dir_name(archive_path: &Path, formula: &str) -> Result<String, String> {
    let file = File::open(archive_path)
        .map_err(|err| format!("failed to open {}: {err}", archive_path.display()))?;
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decoder);
    let mut entries = archive
        .entries()
        .map_err(|err| format!("failed to read {}: {err}", archive_path.display()))?;

    let Some(entry) = entries.next() else {
        return Err(format!("empty bottle archive: {}", archive_path.display()));
    };
    let entry =
        entry.map_err(|err| format!("failed to inspect {}: {err}", archive_path.display()))?;
    let path = entry
        .path()
        .map_err(|err| format!("invalid archive path in {}: {err}", archive_path.display()))?;
    let mut components = path.components();
    let first = components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .ok_or_else(|| format!("invalid top-level path in {}", archive_path.display()))?;
    let second = components
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .ok_or_else(|| format!("missing keg directory in {}", archive_path.display()))?;
    if first != formula {
        return Err(format!(
            "unexpected bottle layout in {}: expected {formula}/..., found {first}/...",
            archive_path.display()
        ));
    }

    Ok(second.to_string())
}

pub(crate) fn build_rewrite_rules(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
) -> Vec<RewriteRule> {
    let mut rules = Vec::with_capacity(installs.len() * 5 + 2);
    let stable_root = plan.stable_root.to_string_lossy().to_string();
    let openssl_cert_destination = plan
        .stable_root
        .join(OPENSSL_CERT_PEM_DESTINATION)
        .to_string_lossy()
        .to_string();
    rules.push(RewriteRule {
        source: HOMEBREW_REPOSITORY_PLACEHOLDER.to_string(),
        destination: stable_root.clone(),
    });
    rules.push(RewriteRule {
        source: HOMEBREW_LIBRARY_PLACEHOLDER.to_string(),
        destination: plan
            .stable_root
            .join("Library")
            .to_string_lossy()
            .to_string(),
    });
    rules.push(RewriteRule {
        source: HOMEBREW_PERL_PLACEHOLDER.to_string(),
        destination: perl_placeholder_target(plan, installs),
    });
    if let Some(java_target) = java_placeholder_target(plan, installs) {
        rules.push(RewriteRule {
            source: HOMEBREW_JAVA_PLACEHOLDER.to_string(),
            destination: java_target,
        });
    }
    // OpenSSL's bundled cert path must land on our managed CA bundle, not /etc.
    rules.push(RewriteRule {
        source: format!("{RELOCATABLE_HOMEBREW_PREFIX}{OPENSSL_CERT_PEM_PATH}"),
        destination: openssl_cert_destination.clone(),
    });
    rules.push(RewriteRule {
        source: format!("{HOMEBREW_PREFIX_PLACEHOLDER}{OPENSSL_CERT_PEM_PATH}"),
        destination: openssl_cert_destination,
    });
    rules.push(RewriteRule {
        source: format!("{RELOCATABLE_HOMEBREW_PREFIX}/etc"),
        destination: "/etc".to_string(),
    });
    rules.push(RewriteRule {
        source: format!("{HOMEBREW_PREFIX_PLACEHOLDER}/etc"),
        destination: "/etc".to_string(),
    });

    for install in installs {
        let target = plan.stable_target_dir(&install.spec.name);
        let target = target.to_string_lossy().to_string();
        let formula_cellar = format!("{RELOCATABLE_HOMEBREW_PREFIX}/Cellar/{}", install.spec.name);
        let cellar = format!(
            "{}/Cellar/{}/{}",
            RELOCATABLE_HOMEBREW_PREFIX, install.spec.name, install.keg_dir_name
        );
        let placeholder_formula_cellar =
            format!("{HOMEBREW_CELLAR_PLACEHOLDER}/{}", install.spec.name);
        let placeholder_cellar = format!(
            "{HOMEBREW_CELLAR_PLACEHOLDER}/{}/{}",
            install.spec.name, install.keg_dir_name
        );
        let escaped_name = install.spec.name.replace('@', "\\@");

        rules.push(RewriteRule {
            source: format!("{cellar}/etc"),
            destination: "/etc".to_string(),
        });
        rules.push(RewriteRule {
            source: format!("{placeholder_cellar}/etc"),
            destination: "/etc".to_string(),
        });
        rules.push(RewriteRule {
            source: formula_cellar,
            destination: target.clone(),
        });
        rules.push(RewriteRule {
            source: cellar,
            destination: target.clone(),
        });
        rules.push(RewriteRule {
            source: placeholder_formula_cellar,
            destination: target.clone(),
        });
        rules.push(RewriteRule {
            source: placeholder_cellar,
            destination: target.clone(),
        });
        rules.push(RewriteRule {
            source: format!("{RELOCATABLE_HOMEBREW_PREFIX}/opt/{}", install.spec.name),
            destination: target.clone(),
        });
        rules.push(RewriteRule {
            source: format!("{HOMEBREW_PREFIX_PLACEHOLDER}/opt/{}", install.spec.name),
            destination: target.clone(),
        });
        if escaped_name != install.spec.name {
            let escaped_formula_cellar =
                format!("{RELOCATABLE_HOMEBREW_PREFIX}/Cellar/{escaped_name}");
            let escaped_placeholder_formula_cellar =
                format!("{HOMEBREW_CELLAR_PLACEHOLDER}/{escaped_name}");
            let escaped_placeholder_cellar = format!(
                "{HOMEBREW_CELLAR_PLACEHOLDER}/{}/{}",
                escaped_name, install.keg_dir_name
            );
            rules.push(RewriteRule {
                source: escaped_formula_cellar,
                destination: target.clone(),
            });
            rules.push(RewriteRule {
                source: escaped_placeholder_formula_cellar,
                destination: target.clone(),
            });
            rules.push(RewriteRule {
                source: format!("{escaped_placeholder_cellar}/etc"),
                destination: "/etc".to_string(),
            });
            rules.push(RewriteRule {
                source: escaped_placeholder_cellar,
                destination: target.clone(),
            });
            rules.push(RewriteRule {
                source: format!("{RELOCATABLE_HOMEBREW_PREFIX}/opt/{escaped_name}"),
                destination: target.clone(),
            });
            rules.push(RewriteRule {
                source: format!("{HOMEBREW_PREFIX_PLACEHOLDER}/opt/{escaped_name}"),
                destination: target,
            });
        }
    }

    rules.push(RewriteRule {
        source: HOMEBREW_PREFIX_PLACEHOLDER.to_string(),
        destination: stable_root,
    });
    rules.sort_by_key(|rule| std::cmp::Reverse(rule.source.len()));
    rules
}

pub(crate) fn perl_placeholder_target(plan: &InstallPlan, installs: &[InstalledFormula]) -> String {
    if installs.iter().any(|install| install.spec.name == "perl") {
        return plan
            .stable_target_dir("perl")
            .join("bin/perl")
            .to_string_lossy()
            .to_string();
    }

    if env::consts::OS == "macos" {
        for candidate in [
            "/usr/bin/perl5.34",
            "/usr/bin/perl5.30",
            "/usr/bin/perl5.18",
            "/usr/bin/perl",
        ] {
            if Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
    }

    "/usr/bin/perl".to_string()
}

pub(crate) fn java_placeholder_target(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
) -> Option<String> {
    let openjdk = installs.iter().find(|install| {
        install.spec.name == "openjdk" || install.spec.name.starts_with("openjdk@")
    })?;
    let java_home = if env::consts::OS == "macos" {
        plan.stable_target_dir(&openjdk.spec.name)
            .join("libexec/openjdk.jdk/Contents/Home")
    } else {
        plan.stable_target_dir(&openjdk.spec.name).join("libexec")
    };
    Some(java_home.to_string_lossy().to_string())
}

pub(crate) fn install_formula(
    config: &Config,
    plan: &InstallPlan,
    install: &InstalledFormula,
    rewrite_rules: &[RewriteRule],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.begin_install_phase_for(&install.spec.name);
    }
    let tmp_root = TempDir::new_in(&plan.tmp_root)
        .map_err(|err| format!("failed to create tmp dir for {}: {err}", install.spec.name))?;
    unpack_bottle(&install.archive_path, tmp_root.path())?;

    let formula_root = tmp_root.path().join(&install.spec.name);
    let keg_root = formula_root.join(&install.keg_dir_name);
    if !keg_root.is_dir() {
        return Err(format!(
            "bottle for {} did not unpack to {}",
            install.spec.name,
            keg_root.display()
        ));
    }

    relocate_tree(
        &keg_root,
        &plan.stable_target_dir(&install.spec.name),
        &install.spec.name,
        rewrite_rules,
        progress,
    )?;
    let owned_paths = stage_formula(plan, install, &keg_root)?;
    write_receipt_with_owned_paths(
        &plan.receipt_path(&install.spec.name),
        install,
        &config.bottle_tag,
        owned_paths,
    )
}

pub(crate) fn stage_formula(
    plan: &InstallPlan,
    install: &InstalledFormula,
    keg_root: &Path,
) -> Result<Vec<String>, String> {
    let keep_root_entries = install.spec.name == plan.root_formula;
    let owned_paths = collect_stageable_owned_paths(keg_root, keep_root_entries)?;
    let root_executables = if install.spec.name == plan.root_formula {
        Some(
            collect_root_executables(keg_root)?
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    stage_root_formula(&plan.install_root, keg_root, keep_root_entries)?;
    if let Some(root_executables) = root_executables {
        write_root_executable_manifest(&plan.root_executables_manifest_path(), &root_executables)?;
    }
    Ok(owned_paths)
}

pub(crate) fn stage_root_formula(
    target_root: &Path,
    keg_root: &Path,
    keep_root_entries: bool,
) -> Result<(), String> {
    let entries = fs::read_dir(keg_root)
        .map_err(|err| format!("failed to read {}: {err}", keg_root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", keg_root.display()))?;
        if !should_stage_root_entry(&entry, keep_root_entries)? {
            remove_path(&entry.path())?;
            continue;
        }
        let source = entry.path();
        let target = target_root.join(entry.file_name());
        merge_path_into(&source, &target)?;
    }
    Ok(())
}

pub(crate) fn should_stage_root_entry(
    entry: &fs::DirEntry,
    keep_root_entries: bool,
) -> Result<bool, String> {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    if name == ".brew" {
        return Ok(false);
    }
    if keep_root_entries || name == ".bottle" {
        return Ok(true);
    }

    let file_type = entry
        .file_type()
        .map_err(|err| format!("failed to stat {}: {err}", entry.path().display()))?;
    Ok(file_type.is_dir())
}

#[cfg(test)]
pub(crate) fn write_receipt(
    path: &Path,
    install: &InstalledFormula,
    bottle_tag: &str,
) -> Result<(), String> {
    let receipt = InstallReceipt {
        formula: install.spec.name.clone(),
        version: install.keg_dir_name.clone(),
        bottle_sha256: install.spec.bottle_sha256.clone(),
        bottle_tag: bottle_tag.to_string(),
        owned_paths: Vec::new(),
    };
    write_install_receipt(path, &receipt)
}

pub(crate) fn write_receipt_with_owned_paths(
    path: &Path,
    install: &InstalledFormula,
    bottle_tag: &str,
    owned_paths: Vec<String>,
) -> Result<(), String> {
    let receipt = InstallReceipt {
        formula: install.spec.name.clone(),
        version: install.keg_dir_name.clone(),
        bottle_sha256: install.spec.bottle_sha256.clone(),
        bottle_tag: bottle_tag.to_string(),
        owned_paths,
    };
    write_install_receipt(path, &receipt)
}

pub(crate) fn write_install_receipt(path: &Path, receipt: &InstallReceipt) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(&receipt)
        .map_err(|err| format!("failed to serialize receipt for {}: {err}", receipt.formula))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(crate) fn load_install_receipt(path: &Path) -> Result<Option<InstallReceipt>, String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub(crate) fn load_root_ownership_manifest(path: &Path) -> Result<Option<StubManifest>, String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub(crate) fn write_root_ownership_manifest(
    plan: &InstallPlan,
    owned_paths: Vec<String>,
) -> Result<(), String> {
    write_stub_manifest(
        &plan.root_ownership_manifest_path(),
        &StubManifest { stubs: owned_paths },
    )
}

pub(crate) fn collect_owned_paths(root: &Path) -> Result<HashSet<String>, String> {
    let mut paths = HashSet::new();
    collect_owned_paths_inner(root, root, &mut paths)?;
    Ok(paths)
}

pub(crate) fn collect_owned_paths_inner(
    root: &Path,
    path: &Path,
    paths: &mut HashSet<String>,
) -> Result<(), String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let child = entry.path();
        let relative = child
            .strip_prefix(root)
            .map_err(|err| format!("failed to relativize {}: {err}", child.display()))?;
        if relative.starts_with(".pkg") {
            continue;
        }
        if entry
            .file_type()
            .map_err(|err| format!("failed to stat {}: {err}", child.display()))?
            .is_dir()
        {
            collect_owned_paths_inner(root, &child, paths)?;
        } else {
            paths.insert(normalize_owned_path(relative)?);
        }
    }
    Ok(())
}

pub(crate) fn sorted_owned_path_difference(
    before: HashSet<String>,
    after: HashSet<String>,
) -> Vec<String> {
    let mut paths = after.difference(&before).cloned().collect::<Vec<_>>();
    paths.sort();
    paths
}

pub(crate) fn collect_stageable_owned_paths(
    keg_root: &Path,
    keep_root_entries: bool,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(keg_root)
        .map_err(|err| format!("failed to read {}: {err}", keg_root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", keg_root.display()))?;
        if !should_stage_root_entry(&entry, keep_root_entries)? {
            continue;
        }
        collect_stageable_owned_paths_inner(keg_root, &entry.path(), &mut paths)?;
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub(crate) fn collect_stageable_owned_paths_inner(
    keg_root: &Path,
    path: &Path,
    paths: &mut Vec<String>,
) -> Result<(), String> {
    let relative = path
        .strip_prefix(keg_root)
        .map_err(|err| format!("failed to relativize {}: {err}", path.display()))?;
    if fs::symlink_metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?
        .is_dir()
    {
        for entry in
            fs::read_dir(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?
        {
            let entry = entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            collect_stageable_owned_paths_inner(keg_root, &entry.path(), paths)?;
        }
    } else {
        paths.push(normalize_owned_path(relative)?);
    }
    Ok(())
}

pub(crate) fn normalize_owned_path(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| format!("non-utf8 owned path {}", path.display()))?
                    .to_string(),
            ),
            _ => return Err(format!("invalid owned path {}", path.display())),
        }
    }
    if parts.is_empty() || parts.iter().any(|part| part == "." || part == "..") {
        return Err(format!("invalid owned path {}", path.display()));
    }
    Ok(parts.join("/"))
}

pub(crate) fn remove_owned_paths(root: &Path, paths: &[String]) -> Result<(), String> {
    let mut paths = paths.to_vec();
    paths.sort_by_key(|path| std::cmp::Reverse(path.matches('/').count()));
    for relative in paths {
        let target = root.join(&relative);
        if fs::symlink_metadata(&target).is_ok() {
            remove_path(&target)?;
        }
    }
    remove_empty_owned_dirs(root)
}

pub(crate) fn remove_empty_owned_dirs(root: &Path) -> Result<(), String> {
    for top in [
        "bin", "sbin", "lib", "include", "share", "etc", "opt", "var",
    ] {
        remove_empty_dirs_under(root, &root.join(top))?;
    }
    Ok(())
}

pub(crate) fn remove_empty_dirs_under(root: &Path, path: &Path) -> Result<bool, String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let mut empty = true;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let child = entry.path();
        if entry
            .file_type()
            .map_err(|err| format!("failed to stat {}: {err}", child.display()))?
            .is_dir()
        {
            if !remove_empty_dirs_under(root, &child)? {
                empty = false;
            }
        } else {
            empty = false;
        }
    }
    if empty && path != root {
        fs::remove_dir(path)
            .map_err(|err| format!("failed to remove {}: {err}", path.display()))?;
    }
    Ok(empty)
}

pub(crate) fn prepare_root_payload_install(plan: &InstallPlan) -> Result<HashSet<String>, String> {
    if incremental_root_is_seeded(plan)
        && let Some(manifest) = load_root_ownership_manifest(&plan.root_ownership_manifest_path())?
    {
        remove_owned_paths(&plan.install_root, &manifest.stubs)?;
    }
    collect_owned_paths(&plan.install_root)
}

pub(crate) fn finish_root_payload_install(
    plan: &InstallPlan,
    before: HashSet<String>,
) -> Result<(), String> {
    let after = collect_owned_paths(&plan.install_root)?;
    write_root_ownership_manifest(plan, sorted_owned_path_difference(before, after))
}

pub(crate) fn merge_path_into(source: &Path, target: &Path) -> Result<(), String> {
    let source_metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to stat {}: {err}", source.display()))?;
    if target.exists() || fs::symlink_metadata(target).is_ok() {
        let target_metadata = fs::symlink_metadata(target)
            .map_err(|err| format!("failed to stat {}: {err}", target.display()))?;
        if source_metadata.is_dir() && target_metadata.is_dir() {
            fs::create_dir_all(target)
                .map_err(|err| format!("failed to create {}: {err}", target.display()))?;
            for entry in fs::read_dir(source)
                .map_err(|err| format!("failed to read {}: {err}", source.display()))?
            {
                let entry =
                    entry.map_err(|err| format!("failed to read {}: {err}", source.display()))?;
                merge_path_into(&entry.path(), &target.join(entry.file_name()))?;
            }
            fs::remove_dir(source)
                .map_err(|err| format!("failed to remove {}: {err}", source.display()))?;
            return Ok(());
        }
        remove_path(target)?;
    } else if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) if source_metadata.is_dir() && target.is_dir() => merge_path_into(source, target),
        Err(err) => Err(format!(
            "failed to move {} to {}: {err}",
            source.display(),
            target.display()
        )),
    }
}

pub(crate) fn unpack_bottle(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|err| format!("failed to open {}: {err}", archive_path.display()))?;
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decoder);
    archive
        .unpack(destination)
        .map_err(|err| format!("failed to unpack {}: {err}", archive_path.display()))
}

pub(crate) fn relocate_tree(
    root: &Path,
    future_root: &Path,
    formula: &str,
    rules: &[RewriteRule],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let allow_failures =
        pkg_allow_contains("relocation-failures") || homebrew_debug_allowance_enabled();
    let mut stderr = std::io::stderr();
    relocate_tree_with_options(
        root,
        future_root,
        formula,
        rules,
        progress,
        allow_failures,
        &mut stderr,
    )
}

pub(crate) fn relocate_tree_with_options<W: Write>(
    root: &Path,
    future_root: &Path,
    formula: &str,
    rules: &[RewriteRule],
    progress: Option<&InstallProgress>,
    allow_failures: bool,
    stderr: &mut W,
) -> Result<(), String> {
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|err| format!("failed to walk {}: {err}", root.display()))?;
        let path = entry.path();

        if entry.file_type().is_symlink() {
            if let Err(err) = relocate_symlink(path, root, future_root, rules) {
                handle_allowed_failure(err, allow_failures, stderr)?;
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        if let Err(err) = relocate_file(path, root, future_root, formula, rules, progress) {
            handle_allowed_failure(err, allow_failures, stderr)?;
        }
    }
    Ok(())
}

pub(crate) fn handle_allowed_failure<W: Write>(
    err: String,
    allow_failure: bool,
    stderr: &mut W,
) -> Result<(), String> {
    if !allow_failure {
        return Err(err);
    }
    let _ = writeln!(stderr, "{err}");
    Ok(())
}

pub(crate) fn pkg_allow_contains(flag: &str) -> bool {
    if !homebrew_debug_allowance_enabled() {
        return false;
    }
    env::var("PKG_ALLOW")
        .ok()
        .is_some_and(|value| pkg_allow_value_contains(&value, flag))
}

pub(crate) fn pkg_allow_value_contains(value: &str, flag: &str) -> bool {
    value
        .split(|ch: char| ch == ':' || ch == ',' || ch.is_ascii_whitespace())
        .any(|item| item == flag)
}

pub(crate) fn relocate_symlink(
    path: &Path,
    root: &Path,
    future_root: &Path,
    rules: &[RewriteRule],
) -> Result<(), String> {
    let target = fs::read_link(path)
        .map_err(|err| format!("failed to read symlink {}: {err}", path.display()))?;
    let rewritten = match target.to_str() {
        Some(target_str) => rewrite_absolute_path(target_str, rules)?.map(PathBuf::from),
        None => None,
    };
    let rewritten = match rewritten {
        Some(rewritten) => Some(rewritten),
        None if target.is_relative() => {
            rewrite_relative_symlink_target(path, root, future_root, &target, rules)?
        }
        None => None,
    };
    let Some(rewritten) = rewritten else {
        return Ok(());
    };

    fs::remove_file(path).map_err(|err| format!("failed to remove {}: {err}", path.display()))?;
    symlink(&rewritten, path).map_err(|err| {
        format!(
            "failed to rewrite symlink {} -> {}: {err}",
            path.display(),
            rewritten.display()
        )
    })
}

pub(crate) fn rewrite_relative_symlink_target(
    path: &Path,
    root: &Path,
    future_root: &Path,
    target: &Path,
    rules: &[RewriteRule],
) -> Result<Option<PathBuf>, String> {
    let relative_path = path
        .strip_prefix(root)
        .map_err(|err| format!("failed to relativize {}: {err}", path.display()))?;
    let source_root = source_keg_root(root)?;
    let source_path = source_root.join(relative_path);
    let source_parent = source_path
        .parent()
        .ok_or_else(|| format!("symlink {} has no parent directory", source_path.display()))?;
    let resolved = normalize_path(&source_parent.join(target));
    if resolved.starts_with(&source_root) {
        return Ok(None);
    }

    let Some(source) = homebrew_relative_symlink_source(&resolved) else {
        return Ok(None);
    };
    let Some(rewritten) = rewrite_absolute_path(&source, rules)? else {
        return Ok(None);
    };

    let future_path = future_root.join(relative_path);
    let future_parent = future_path
        .parent()
        .ok_or_else(|| format!("symlink {} has no parent directory", future_path.display()))?;
    Ok(Some(relative_path_from(
        future_parent,
        Path::new(&rewritten),
    )))
}

pub(crate) fn source_keg_root(root: &Path) -> Result<PathBuf, String> {
    let formula = root
        .parent()
        .and_then(Path::file_name)
        .ok_or_else(|| format!("keg root {} is missing a formula directory", root.display()))?;
    let version = root
        .file_name()
        .ok_or_else(|| format!("keg root {} is missing a version directory", root.display()))?;
    Ok(PathBuf::from(RELOCATABLE_HOMEBREW_PREFIX)
        .join("Cellar")
        .join(formula)
        .join(version))
}

pub(crate) fn homebrew_relative_symlink_source(resolved: &Path) -> Option<String> {
    let resolved = resolved.to_str()?;
    if let Some(opt_path) = resolved.strip_prefix(&format!("{RELOCATABLE_HOMEBREW_PREFIX}/opt/")) {
        return Some(format!("{HOMEBREW_PREFIX_PLACEHOLDER}/opt/{opt_path}"));
    }
    if let Some(cellar_path) =
        resolved.strip_prefix(&format!("{RELOCATABLE_HOMEBREW_PREFIX}/Cellar/"))
    {
        return Some(format!("{HOMEBREW_CELLAR_PLACEHOLDER}/{cellar_path}"));
    }

    None
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    let mut components = Vec::<OsString>::new();
    let mut has_root = false;
    let mut prefix = None;

    for component in path.components() {
        match component {
            Component::Prefix(value) => prefix = Some(value.as_os_str().to_os_string()),
            Component::RootDir => has_root = true,
            Component::CurDir => {}
            Component::ParentDir => match components.last() {
                Some(last) if last != ".." => {
                    components.pop();
                }
                _ if !has_root => components.push(OsString::from("..")),
                _ => {}
            },
            Component::Normal(value) => components.push(value.to_os_string()),
        }
    }

    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }
    if has_root {
        normalized.push(Path::new("/"));
    }
    for component in components {
        normalized.push(component);
    }
    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }

    normalized
}

pub(crate) fn relative_path_from(from: &Path, to: &Path) -> PathBuf {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    if from.is_absolute() != to.is_absolute() {
        return to.to_path_buf();
    }

    let mut shared = 0usize;
    while shared < from_components.len()
        && shared < to_components.len()
        && from_components[shared] == to_components[shared]
    {
        shared += 1;
    }

    if shared == 0 && to.is_absolute() {
        return to.to_path_buf();
    }

    let mut relative = PathBuf::new();
    for component in &from_components[shared..] {
        if matches!(component, Component::Normal(_)) {
            relative.push("..");
        }
    }
    for component in &to_components[shared..] {
        relative.push(component.as_os_str());
    }
    if relative.as_os_str().is_empty() {
        relative.push(".");
    }

    relative
}

pub(crate) fn relocate_file(
    path: &Path,
    root: &Path,
    future_root: &Path,
    formula: &str,
    rules: &[RewriteRule],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if path.extension().and_then(|value| value.to_str()) == Some("a") {
        return Ok(());
    }

    let mut bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;

    if let Ok(text) = std::str::from_utf8(&bytes) {
        if is_documentation_text_path(path, root) {
            return Ok(());
        }
        let rewritten = rewrite_text(text, path, formula, rules)?;
        if rewritten.as_bytes() != bytes.as_slice() {
            ensure_writable(path)?;
            fs::write(path, rewritten.as_bytes())
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        }
        return Ok(());
    }

    let mode = if is_macho(&bytes) {
        BinaryRewriteMode::Macho {
            path,
            root,
            future_root,
        }
    } else {
        BinaryRewriteMode::Slash
    };
    let changed = rewrite_binary(&mut bytes, path, formula, rules, mode)?;
    if changed {
        ensure_writable(path)?;
        fs::write(path, &bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        codesign_if_macho(path, &bytes, progress)?;
    }
    Ok(())
}

pub(crate) fn is_documentation_text_path(path: &Path, root: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut components = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        });
    let first = components.next();
    let second = components.next();
    if first == Some(OsStr::new("share")) && second == Some(OsStr::new("doc")) {
        return true;
    }

    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    let stem = file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem)
        .to_ascii_uppercase();
    let prefixes = [
        "AUTHORS",
        "CHANGELOG",
        "CHANGES",
        "COPYING",
        "HISTORY",
        "LICENSE",
        "NEWS",
        "NOTICE",
        "README",
        "THANKS",
    ];
    prefixes
        .iter()
        .any(|prefix| stem == *prefix || stem.starts_with(&format!("{prefix}-")))
}

pub(crate) fn rewrite_text(
    text: &str,
    path: &Path,
    formula: &str,
    rules: &[RewriteRule],
) -> Result<String, String> {
    let mut rewritten = text.to_string();
    for rule in rules {
        rewritten = rewrite_prefixes_in_text(&rewritten, rule);
    }
    if contains_relocatable_homebrew_reference_text(&rewritten, rules) {
        return Err(unsupported_homebrew_rewrite_error(
            "text", formula, path, text, &rewritten, rules,
        ));
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_prefixes_in_text(text: &str, rule: &RewriteRule) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(&rule.source) {
        let absolute = cursor + offset;
        output.push_str(&text[cursor..absolute]);
        let suffix_index = absolute + rule.source.len();
        let boundary = text
            .as_bytes()
            .get(suffix_index)
            .copied()
            .is_none_or(|byte| byte == b'/' || !is_path_byte(byte));
        if boundary {
            output.push_str(&rule.destination);
            cursor = suffix_index;
        } else {
            output.push_str(&rule.source);
            cursor = suffix_index;
        }
    }
    output.push_str(&text[cursor..]);
    output
}

pub(crate) fn rewrite_binary(
    bytes: &mut [u8],
    path: &Path,
    formula: &str,
    rules: &[RewriteRule],
    mode: BinaryRewriteMode<'_>,
) -> Result<bool, String> {
    let mut changed = false;
    let mut start = 0usize;
    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|offset| start + offset)
            .unwrap_or(bytes.len());
        let segment = &bytes[start..end];
        if contains_relocatable_homebrew_reference_bytes(segment, rules) {
            let rewritten = rewrite_binary_segment_bytes(segment, path, formula, rules, mode)?;
            if rewritten.len() > segment.len() {
                let original = String::from_utf8_lossy(segment);
                let rewritten = String::from_utf8_lossy(&rewritten);
                return Err(format!(
                    "{} cannot be rewritten safely because binary rewrite matched embedded Homebrew path {} and replacement {} is longer",
                    path.display(),
                    original,
                    rewritten
                ));
            }
            bytes[start..start + rewritten.len()].copy_from_slice(&rewritten);
            for byte in &mut bytes[start + rewritten.len()..end] {
                *byte = 0;
            }
            changed = true;
        }

        if end == bytes.len() {
            break;
        }
        start = end + 1;
    }

    if contains_relocatable_homebrew_reference_bytes(bytes, rules) {
        return Err(format!(
            "{} still contains unsupported Homebrew references after NUL-segment relocation",
            path.display()
        ));
    }

    Ok(changed)
}

pub(crate) fn is_path_byte(byte: u8) -> bool {
    SAFE_BINARY_PATH_BYTES.contains(&byte)
}

pub(crate) fn ensure_writable(path: &Path) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    let mut permissions = metadata.permissions();
    let mode = permissions.mode();
    if mode & 0o200 != 0 {
        return Ok(());
    }

    permissions.set_mode(mode | 0o200);
    fs::set_permissions(path, permissions)
        .map_err(|err| format!("failed to make {} writable: {err}", path.display()))
}

pub(crate) fn rewrite_binary_segment_bytes(
    segment: &[u8],
    path: &Path,
    formula: &str,
    rules: &[RewriteRule],
    mode: BinaryRewriteMode<'_>,
) -> Result<Vec<u8>, String> {
    let mut rewritten = segment.to_vec();
    for rule in rules {
        rewritten = rewrite_prefixes_in_bytes(&rewritten, rule, mode);
    }
    if contains_relocatable_homebrew_reference_bytes(&rewritten, rules) {
        if let (Ok(original), Ok(rewritten_text)) = (
            std::str::from_utf8(segment),
            std::str::from_utf8(&rewritten),
        ) {
            return Err(unsupported_homebrew_rewrite_error(
                "binary",
                formula,
                path,
                original,
                rewritten_text,
                rules,
            ));
        }
        return Err(format!(
            "formula {formula}: unsupported Homebrew path remains after binary rewrite in {}",
            path.display()
        ));
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_prefixes_in_bytes(
    segment: &[u8],
    rule: &RewriteRule,
    mode: BinaryRewriteMode<'_>,
) -> Vec<u8> {
    let source = rule.source.as_bytes();
    let destination = binary_rewrite_destination(rule, mode);
    let mut output = Vec::with_capacity(segment.len());
    let mut cursor = 0usize;
    while let Some(offset) = find_subslice(&segment[cursor..], source) {
        let absolute = cursor + offset;
        output.extend_from_slice(&segment[cursor..absolute]);
        let suffix_index = absolute + source.len();
        let boundary = segment
            .get(suffix_index)
            .copied()
            .is_none_or(|byte| byte == b'/' || !is_path_byte(byte));
        if boundary {
            if let BinaryRewriteMode::Macho {
                path,
                root,
                future_root,
            } = mode
            {
                let path_end = segment[suffix_index..]
                    .iter()
                    .position(|byte| !is_path_byte(*byte))
                    .map(|offset| suffix_index + offset)
                    .unwrap_or(segment.len());
                let suffix = &segment[suffix_index..path_end];
                let original_path_len = path_end - absolute;
                if let Some(destination) = macho_binary_rewrite_destination(
                    rule,
                    suffix,
                    original_path_len,
                    path,
                    root,
                    future_root,
                ) {
                    output.extend_from_slice(destination.as_bytes());
                    cursor = path_end;
                    continue;
                }
            }
            output.extend_from_slice(&destination);
            cursor = suffix_index;
        } else {
            output.extend_from_slice(source);
            cursor = suffix_index;
        }
    }
    output.extend_from_slice(&segment[cursor..]);
    output
}

pub(crate) fn binary_rewrite_destination(
    rule: &RewriteRule,
    mode: BinaryRewriteMode<'_>,
) -> Vec<u8> {
    let source = rule.source.as_bytes();
    let destination = rule.destination.as_bytes();
    if matches!(mode, BinaryRewriteMode::Nul) || destination.len() >= source.len() {
        return destination.to_vec();
    }

    let Some(last_slash) = destination.iter().rposition(|byte| *byte == b'/') else {
        return destination.to_vec();
    };
    if last_slash == 0 {
        return destination.to_vec();
    }

    let mut padded = Vec::with_capacity(source.len());
    padded.extend_from_slice(&destination[..=last_slash]);
    padded.extend(std::iter::repeat_n(b'/', source.len() - destination.len()));
    padded.extend_from_slice(&destination[last_slash + 1..]);
    padded
}

pub(crate) fn macho_binary_rewrite_destination(
    rule: &RewriteRule,
    suffix: &[u8],
    max_len: usize,
    path: &Path,
    root: &Path,
    future_root: &Path,
) -> Option<String> {
    let suffix = std::str::from_utf8(suffix).ok()?;
    if !suffix.contains(".dylib") {
        return None;
    }
    let rewritten = format!("{}{}", rule.destination, suffix);
    let rewritten_path = Path::new(&rewritten);
    if !rewritten_path.starts_with(future_root) {
        return Some(rewritten);
    }
    let relative_path = path.strip_prefix(root).ok()?;
    let future_path = future_root.join(relative_path);
    let future_parent = future_path.parent()?;
    let relative = relative_path_from(future_parent, rewritten_path);
    let loader_path = format!("@loader_path/{}", relative.to_string_lossy());
    if loader_path.len() <= max_len {
        return Some(loader_path);
    }
    if rewritten.len() <= max_len {
        return Some(rewritten);
    }
    Some(loader_path)
}

pub(crate) fn unsupported_homebrew_rewrite_error(
    kind: &str,
    formula: &str,
    path: &Path,
    original: &str,
    rewritten: &str,
    rules: &[RewriteRule],
) -> String {
    let from = first_relocatable_homebrew_reference(original, rules)
        .unwrap_or(RELOCATABLE_HOMEBREW_PREFIX);
    let to = first_relocatable_homebrew_reference(rewritten, rules)
        .unwrap_or(RELOCATABLE_HOMEBREW_PREFIX);
    format!(
        "formula {formula}: unsupported Homebrew path remains after {kind} rewrite in {}: \
rewrote {from} -> {to}; original segment: {original}; rewritten segment: {rewritten}",
        path.display()
    )
}

pub(crate) fn run_command_with_logged_output(
    command: &mut Command,
    progress: Option<&InstallProgress>,
    context: &str,
) -> Result<LoggedCommandOutput, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| format!("{context}: {err}"))?;
    let stdout = child
        .stdout
        .take()
        .map(|reader| spawn_output_reader(reader, progress.cloned()));
    let stderr = child
        .stderr
        .take()
        .map(|reader| spawn_output_reader(reader, progress.cloned()));
    let status = child.wait().map_err(|err| format!("{context}: {err}"))?;

    let mut lines = Vec::new();
    if let Some(handle) = stdout {
        lines.extend(join_output_reader(handle, context)?);
    }
    if let Some(handle) = stderr {
        lines.extend(join_output_reader(handle, context)?);
    }

    Ok(LoggedCommandOutput { status, lines })
}

pub(crate) fn spawn_output_reader<R>(
    reader: R,
    progress: Option<InstallProgress>,
) -> thread::JoinHandle<Result<Vec<String>, String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut lines = Vec::new();
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let count = reader
                .read_until(b'\n', &mut buffer)
                .map_err(|err| format!("failed to read subprocess output: {err}"))?;
            if count == 0 {
                break;
            }
            let line = sanitize_progress_message(&String::from_utf8_lossy(&buffer));
            if line.is_empty() {
                continue;
            }
            if let Some(progress) = &progress {
                progress.log(&line);
            }
            lines.push(line);
        }
        Ok(lines)
    })
}

pub(crate) fn join_output_reader(
    handle: thread::JoinHandle<Result<Vec<String>, String>>,
    context: &str,
) -> Result<Vec<String>, String> {
    handle
        .join()
        .map_err(|_| format!("{context}: subprocess output reader panicked"))?
}

pub(crate) fn format_command_output_suffix(lines: &[String]) -> String {
    lines
        .iter()
        .rev()
        .find(|line| !line.is_empty())
        .map(|line| format!(": {line}"))
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub(crate) struct UserIdentity {
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) home: Option<String>,
    pub(crate) name: Option<String>,
}

pub(crate) fn run_isotope_migration(
    plan: &InstallPlan,
    isotope: &IsotopePackageData,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    let Some(script) = isotope.migrate.as_deref() else {
        if let Some(result) = run_generated_isotope_migration(&isotope.name) {
            if is_root() {
                return Err("isotope migration must not run as root".to_string());
            }
            if let Some(progress) = progress {
                progress.log("migrating secrets");
            }
            return result;
        }
        return Ok(());
    };
    let user = current_user_identity()?;
    let temp_parent = if is_root() {
        plan.tmp_root.clone()
    } else {
        env::temp_dir()
    };
    let temp_dir = TempDir::new_in(&temp_parent).map_err(|err| {
        format!(
            "failed to create temp dir for {} migration: {err}",
            isotope.name
        )
    })?;
    let mut temp_permissions = fs::metadata(temp_dir.path())
        .map_err(|err| format!("failed to stat {}: {err}", temp_dir.path().display()))?
        .permissions();
    temp_permissions.set_mode(0o755);
    fs::set_permissions(temp_dir.path(), temp_permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", temp_dir.path().display()))?;
    let script_path = temp_dir.path().join("migrate.sh");
    let rewritten = executable_isotope_migration_script(script, plan, isotope)?;
    fs::write(&script_path, rewritten)
        .map_err(|err| format!("failed to write {}: {err}", script_path.display()))?;
    let mut permissions = fs::metadata(&script_path)
        .map_err(|err| format!("failed to stat {}: {err}", script_path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", script_path.display()))?;

    if let Some(progress) = progress {
        progress.log("migrating secrets");
    }
    let mut command = Command::new(&script_path);
    command.current_dir(&plan.install_root);
    command.env("ISOTOPE_PREFIX", &plan.install_root);
    command.env("ISOTOPE_NAME", &isotope.name);
    if let Some(name) = user.name.as_deref() {
        command.env("USER", name).env("LOGNAME", name);
    }
    if let Some(home) = user.home.as_deref() {
        command.env("HOME", home);
    }
    if is_root() {
        command.uid(user.uid);
        command.gid(user.gid);
    }

    let output = run_command_with_logged_output(
        &mut command,
        progress,
        &format!("failed to run migration for {}", isotope.name),
    )?;
    if output.status.success() {
        return Ok(());
    }
    Err(format_failed_isotope_migration(
        &isotope.name,
        output.status,
        &output.lines,
    ))
}

pub(crate) fn format_failed_isotope_migration(
    name: &str,
    status: ExitStatus,
    lines: &[String],
) -> String {
    match status.code() {
        Some(code) => format!(
            "migration failed for {} with exit code {code}{}",
            name,
            format_command_output_suffix(lines)
        ),
        None => format!(
            "migration terminated by signal for {}{}",
            name,
            format_command_output_suffix(lines)
        ),
    }
}

pub(crate) fn rewrite_isotope_migration_script(
    script: &str,
    plan: &InstallPlan,
    isotope: &IsotopePackageData,
) -> Result<String, String> {
    let target_prefix = plan.install_root.display().to_string();
    let mut rewritten = script.to_string();

    if let Some(replaced_package) = isotope_replaced_package_name(isotope)? {
        let replaced_prefix = package_install_root(&opt_pkg_root(), &replaced_package)?
            .display()
            .to_string();
        rewritten = rewritten
            .replace(&replaced_prefix, &target_prefix)
            .replace(&format!("/opt/{replaced_package}"), &target_prefix);
    }

    for alias in isotope_migration_install_root_aliases(isotope) {
        rewritten = rewritten.replace(&alias, &target_prefix);
    }
    Ok(rewritten)
}

pub(crate) fn isotope_migration_install_root_aliases(isotope: &IsotopePackageData) -> Vec<String> {
    let mut names = Vec::new();
    push_unique_string(
        &mut names,
        isotope_unqualified_name(&isotope.name).to_string(),
    );
    if let Some(repository_leaf) = isotope
        ._repository
        .as_deref()
        .and_then(|repository| repository.rsplit('/').next())
        .filter(|repository_leaf| !repository_leaf.is_empty())
    {
        push_unique_string(&mut names, repository_leaf.to_string());
    }

    let mut aliases = Vec::new();
    for name in names {
        push_unique_string(
            &mut aliases,
            opt_pkg_root()
                .join(ISOTOPE_INSTALL_ROOT_DIR)
                .join(&name)
                .display()
                .to_string(),
        );
        push_unique_string(
            &mut aliases,
            format!("/opt/{ISOTOPE_INSTALL_ROOT_DIR}/{name}"),
        );
        push_unique_string(
            &mut aliases,
            opt_pkg_root()
                .join("isotopes")
                .join(&name)
                .display()
                .to_string(),
        );
        push_unique_string(&mut aliases, format!("/opt/isotopes/{name}"));
    }
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
    aliases
}

pub(crate) fn executable_isotope_migration_script(
    script: &str,
    plan: &InstallPlan,
    isotope: &IsotopePackageData,
) -> Result<String, String> {
    let rewritten = rewrite_isotope_migration_script(script, plan, isotope)?;
    if rewritten.starts_with("#!") {
        let Some((shebang, body)) = rewritten.split_once('\n') else {
            return Ok(format!("{rewritten}\n{}", isotope_migration_root_guard()));
        };
        return Ok(format!(
            "{shebang}\n{}{}",
            isotope_migration_root_guard(),
            body
        ));
    }
    Ok(format!(
        "#!/bin/sh\n{}{}",
        isotope_migration_root_guard(),
        rewritten
    ))
}

pub(crate) fn isotope_migration_root_guard() -> &'static str {
    "if [ \"$(id -u)\" -eq 0 ]; then\n  echo \"isotope migration must not run as root\" >&2\n  exit 77\nfi\n"
}

pub(crate) fn current_user_identity() -> Result<UserIdentity, String> {
    if !is_root() {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        return Ok(UserIdentity {
            uid,
            gid,
            home: env::var("HOME").ok(),
            name: env::var("USER").ok().or_else(|| env::var("LOGNAME").ok()),
        });
    }

    if let (Ok(uid), Ok(gid)) = (env::var("SUDO_UID"), env::var("SUDO_GID"))
        && let (Ok(uid), Ok(gid)) = (uid.parse::<u32>(), gid.parse::<u32>())
    {
        let (home, name) = passwd_entry(uid);
        return Ok(UserIdentity {
            uid,
            gid,
            home,
            name,
        });
    }

    let metadata = fs::metadata("/dev/console")
        .map_err(|err| format!("failed to stat /dev/console for migration user: {err}"))?;
    let uid = metadata.uid();
    let gid = metadata.gid();
    if uid == 0 {
        return Err("could not determine a non-root user for isotope migration".to_string());
    }
    let (home, name) = passwd_entry(uid);
    Ok(UserIdentity {
        uid,
        gid,
        home,
        name,
    })
}

pub(crate) fn passwd_entry(uid: u32) -> (Option<String>, Option<String>) {
    unsafe {
        let pwd = libc::getpwuid(uid);
        if pwd.is_null() {
            return (None, None);
        }
        let entry = *pwd;
        let home = (!entry.pw_dir.is_null()).then(|| {
            std::ffi::CStr::from_ptr(entry.pw_dir)
                .to_string_lossy()
                .into_owned()
        });
        let name = (!entry.pw_name.is_null()).then(|| {
            std::ffi::CStr::from_ptr(entry.pw_name)
                .to_string_lossy()
                .into_owned()
        });
        (home, name)
    }
}

pub(crate) fn codesign_if_macho(
    path: &Path,
    bytes: &[u8],
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if !is_macho(bytes) {
        return Ok(());
    }

    let mut command = Command::new("codesign");
    command.arg("--force").arg("--sign").arg("-").arg(path);
    let output = run_command_with_logged_output(
        &mut command,
        progress,
        &format!("failed to run codesign for {}", path.display()),
    )?;
    if output.status.success() {
        return Ok(());
    }

    Err(match output.status.code() {
        Some(code) => format!(
            "codesign failed for {} with exit code {code}{}",
            path.display(),
            format_command_output_suffix(&output.lines)
        ),
        None => format!(
            "codesign terminated by signal for {}{}",
            path.display(),
            format_command_output_suffix(&output.lines)
        ),
    })
}

pub(crate) fn is_macho(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }

    matches!(
        &bytes[..4],
        [0xfe, 0xed, 0xfa, 0xce]
            | [0xce, 0xfa, 0xed, 0xfe]
            | [0xfe, 0xed, 0xfa, 0xcf]
            | [0xcf, 0xfa, 0xed, 0xfe]
            | [0xca, 0xfe, 0xba, 0xbe]
            | [0xbe, 0xba, 0xfe, 0xca]
            | [0xca, 0xfe, 0xba, 0xbf]
            | [0xbf, 0xba, 0xfe, 0xca]
    )
}

pub(crate) fn rewrite_absolute_path(
    path: &str,
    rules: &[RewriteRule],
) -> Result<Option<String>, String> {
    if !contains_relocatable_homebrew_reference_text(path, rules) {
        return Ok(None);
    }

    for rule in rules {
        if path == rule.source {
            return Ok(Some(rule.destination.clone()));
        }
        if path.starts_with(&rule.source)
            && path.as_bytes().get(rule.source.len()).copied() == Some(b'/')
        {
            let suffix = &path[rule.source.len()..];
            return Ok(Some(format!("{}{}", rule.destination, suffix)));
        }
    }

    Err(format!("unsupported Homebrew path {path}"))
}

pub(crate) fn first_relocatable_homebrew_reference<'a>(
    text: &'a str,
    rules: &[RewriteRule],
) -> Option<&'a str> {
    let mut best: Option<(usize, &str)> = None;
    for marker in rules.iter().map(|rule| rule.source.as_str()).chain([
        HOMEBREW_PREFIX_PLACEHOLDER,
        HOMEBREW_CELLAR_PLACEHOLDER,
        HOMEBREW_REPOSITORY_PLACEHOLDER,
        HOMEBREW_LIBRARY_PLACEHOLDER,
        HOMEBREW_PERL_PLACEHOLDER,
        HOMEBREW_JAVA_PLACEHOLDER,
    ]) {
        if let Some(index) = text.find(marker) {
            match best {
                Some((best_index, _)) if best_index <= index => {}
                _ => best = Some((index, marker)),
            }
        }
    }
    let (index, marker) = best?;
    let tail = &text[index..];
    let end = tail
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | ')' | '('))
        .map(|(offset, _)| offset)
        .unwrap_or(tail.len());
    Some(if end == 0 {
        &text[index..index + marker.len()]
    } else {
        &tail[..end]
    })
}

pub(crate) fn contains_relocatable_homebrew_reference_bytes(
    bytes: &[u8],
    rules: &[RewriteRule],
) -> bool {
    if find_subslice(bytes, RELOCATABLE_HOMEBREW_PREFIX.as_bytes()).is_none()
        && HOMEBREW_NEEDLES
            .into_iter()
            .all(|needle| find_subslice(bytes, needle).is_none())
    {
        return false;
    }

    rules
        .iter()
        .map(|rule| rule.source.as_bytes())
        .chain(HOMEBREW_NEEDLES)
        .any(|needle| find_subslice(bytes, needle).is_some())
}

pub(crate) fn contains_relocatable_homebrew_reference_text(
    text: &str,
    rules: &[RewriteRule],
) -> bool {
    first_relocatable_homebrew_reference(text, rules).is_some()
}

pub(crate) fn build_formula_order(plan: &InstallPlan, graph: &[FormulaSpec]) -> Vec<String> {
    let mut order = vec![plan.root_formula.clone()];
    for spec in graph {
        if spec.name != plan.root_formula {
            order.push(spec.name.clone());
        }
    }
    order
}

pub(crate) fn build_exec_path_entries(plan: &InstallPlan, graph: &[FormulaSpec]) -> Vec<PathBuf> {
    build_path_entries(plan, graph, InstallPlan::stable_target_dir)
}

pub(crate) fn build_install_path_entries(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
) -> Vec<PathBuf> {
    build_path_entries(plan, graph, InstallPlan::actual_target_dir)
}

pub(crate) fn build_path_entries(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    root_for: fn(&InstallPlan, &str) -> PathBuf,
) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    for formula in build_formula_order(plan, graph) {
        let root = root_for(plan, &formula);
        push_unique_path(&mut entries, root.join("bin"));
        let sbin = root.join("sbin");
        if sbin.is_dir() {
            push_unique_path(&mut entries, sbin);
        }
    }
    entries
}

pub(crate) fn push_unique_path(entries: &mut Vec<PathBuf>, path: PathBuf) {
    if !entries.iter().any(|existing| existing == &path) {
        entries.push(path);
    }
}

pub(crate) fn is_executable(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

pub(crate) fn build_exec_path(entries: &[PathBuf]) -> OsString {
    let paths = combined_path_entries(entries);
    env::join_paths(paths).unwrap_or_else(|_| env::var_os("PATH").unwrap_or_default())
}

pub(crate) fn build_install_path(plan: &InstallPlan, graph: &[FormulaSpec]) -> OsString {
    build_exec_path(&build_install_path_entries(plan, graph))
}

pub(crate) fn combined_path_entries(entries: &[PathBuf]) -> Vec<PathBuf> {
    let mut paths = entries.to_vec();
    if let Some(current) = env::var_os("PATH") {
        for entry in env::split_paths(&current) {
            push_unique_path(&mut paths, entry);
        }
    }
    paths
}

pub(crate) fn resolve_command_in_path_entries(
    entries: &[PathBuf],
    executable: &str,
) -> Option<PathBuf> {
    for entry in entries {
        let candidate = entry.join(executable);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn resolve_install_time_command(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    executable: &str,
) -> Option<PathBuf> {
    let entries = combined_path_entries(&build_install_path_entries(plan, graph));
    resolve_command_in_path_entries(&entries, executable)
}

pub(crate) fn download_vendor_asset(
    url: &str,
    destination: &Path,
    name: &str,
    progress: Option<&InstallProgress>,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.begin_download_phase();
    }
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| match err {
            UreqError::Status(code, _) => {
                format!("failed to download vendor asset for {name}: http {code}")
            }
            UreqError::Transport(err) => {
                format!("failed to download vendor asset for {name}: {err}")
            }
        })?;
    if let Some(progress) = progress {
        progress.add_download_total(
            response
                .header("Content-Length")
                .and_then(|value| value.parse::<u64>().ok()),
        );
    }
    let mut reader = response.into_reader();
    let mut file = File::create(destination)
        .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;
    let mut buffer = [0u8; 32 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to read vendor asset for {name}: {err}"))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|err| format!("failed to write {}: {err}", destination.display()))?;
        if let Some(progress) = progress {
            progress.advance_download(count as u64);
        }
    }
    Ok(())
}

pub(crate) fn vendor_archive_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("archive")
        .to_string()
}

pub(crate) fn unpack_vendor_archive(
    archive_path: &Path,
    destination: &Path,
    name: &str,
) -> Result<(), String> {
    let archive_name = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if vendor_archive_is_zip(archive_name) {
        let status = Command::new("ditto")
            .arg("-x")
            .arg("-k")
            .arg(archive_path)
            .arg(destination)
            .status()
            .map_err(|err| format!("failed to unpack vendor archive for {name}: {err}"))?;
        if status.success() {
            return Ok(());
        }

        return Err(match status.code() {
            Some(code) => format!("failed to unpack vendor archive for {name}: exit code {code}"),
            None => format!("failed to unpack vendor archive for {name}: terminated by signal"),
        });
    }
    if vendor_archive_is_tar(archive_name) {
        return unpack_tar_archive(archive_path, destination);
    }

    Err(format!(
        "unsupported vendor archive format for {name}: {}",
        archive_path.display()
    ))
}

pub(crate) fn unpack_cask_payload(
    archive_path: &Path,
    destination: &Path,
    cask_name: &str,
    cask: &EmbeddedCaskMetadata,
) -> Result<(), String> {
    let archive_name = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if vendor_archive_is_zip(archive_name) || vendor_archive_is_tar(archive_name) {
        return unpack_vendor_archive(archive_path, destination, cask_name);
    }

    unpack_direct_cask_binary(archive_path, destination, cask_name, cask)
}

pub(crate) fn unpack_direct_cask_binary(
    archive_path: &Path,
    destination: &Path,
    cask_name: &str,
    cask: &EmbeddedCaskMetadata,
) -> Result<(), String> {
    let binary = match cask.binaries.as_slice() {
        [binary] => binary,
        _ => {
            return Err(format!(
                "unsupported vendor archive format for {cask_name}: {}",
                archive_path.display()
            ));
        }
    };
    let binary_source = Path::new(&binary.source);
    let archive_name = archive_path.file_name().ok_or_else(|| {
        format!(
            "unsupported vendor archive format for {cask_name}: {}",
            archive_path.display()
        )
    })?;
    if binary_source
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty())
        || binary_source.file_name() != Some(archive_name)
    {
        return Err(format!(
            "unsupported vendor archive format for {cask_name}: {}",
            archive_path.display()
        ));
    }

    fs::copy(archive_path, destination.join(binary_source)).map_err(|err| {
        format!(
            "failed to stage direct cask binary {} for {cask_name}: {err}",
            archive_path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn vendor_archive_is_zip(archive_name: &str) -> bool {
    archive_name.ends_with(".zip")
}

pub(crate) fn vendor_archive_is_tar(archive_name: &str) -> bool {
    archive_name.ends_with(".tar.gz") || archive_name.ends_with(".tgz")
}

pub(crate) fn unpack_tar_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let mut file = File::open(archive_path)
        .map_err(|err| format!("failed to open {}: {err}", archive_path.display()))?;
    let mut magic = [0u8; 2];
    let read = file
        .read(&mut magic)
        .map_err(|err| format!("failed to read {}: {err}", archive_path.display()))?;
    drop(file);

    if read == 2 && magic == [0x1f, 0x8b] {
        return unpack_bottle(archive_path, destination);
    }

    unpack_plain_tar(archive_path, destination)
}

pub(crate) fn unpack_plain_tar(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|err| format!("failed to open {}: {err}", archive_path.display()))?;
    let mut archive = Archive::new(BufReader::new(file));
    archive
        .unpack(destination)
        .map_err(|err| format!("failed to unpack {}: {err}", archive_path.display()))
}

pub(crate) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
