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
