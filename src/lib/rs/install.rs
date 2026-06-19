use super::*;

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
