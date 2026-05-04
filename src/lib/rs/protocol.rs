use super::*;

use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};

static SERVER_RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_shutdown_signal(_signal: i32) {
    SERVER_RUNNING.store(false, Ordering::SeqCst);
}

pub(crate) fn run_server() -> Result<(), String> {
    SERVER_RUNNING.store(true, Ordering::SeqCst);
    let paths = ProtocolPaths::resolve()?;
    fs::create_dir_all(&paths.socket_dir)
        .map_err(|err| format!("failed to create {}: {err}", paths.socket_dir.display()))?;
    fs::create_dir_all(&paths.log_dir)
        .map_err(|err| format!("failed to create {}: {err}", paths.log_dir.display()))?;

    if paths.socket_path.exists() {
        match UnixStream::connect(&paths.socket_path) {
            Ok(_) => {
                return Err(
                    "nucleus protocol daemon already running; client mode is not yet implemented"
                        .to_string(),
                );
            }
            Err(_) => {
                fs::remove_file(&paths.socket_path).map_err(|err| {
                    format!(
                        "failed to remove stale socket {}: {err}",
                        paths.socket_path.display()
                    )
                })?;
            }
        }
    }

    let logger = Logger::new(&paths.log_path)?;
    install_signal_handlers();

    let listener = UnixListener::bind(&paths.socket_path)
        .map_err(|err| format!("failed to bind {}: {err}", paths.socket_path.display()))?;
    let cleanup = SocketCleanup::new(paths.socket_path.clone());
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("failed to configure {}: {err}", paths.socket_path.display()))?;
    log_message(&logger, "protocol server started");

    while SERVER_RUNNING.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(err) = stream.set_nonblocking(false) {
                    log_message(
                        &logger,
                        &format!("failed to configure client stream: {err}"),
                    );
                    continue;
                }
                let logger = logger.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_client(stream, &logger) {
                        log_message(&logger, &format!("client error: {err}"));
                    }
                });
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                log_message(&logger, &format!("listener accept failed: {err}"));
                return Err(format!("protocol accept failed: {err}"));
            }
        }
    }

    log_message(&logger, "protocol server shutting down");
    drop(cleanup);
    Ok(())
}

fn handle_client(stream: UnixStream, logger: &Logger) -> Result<(), String> {
    let reader_stream = stream
        .try_clone()
        .map_err(|err| format!("failed to clone client stream: {err}"))?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read request: {err}"))?;
        if bytes == 0 {
            return Ok(());
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }

        let Some(response) = dispatch_line(trimmed, logger) else {
            continue;
        };
        writer
            .write_all(response.as_bytes())
            .map_err(|err| format!("failed to write response: {err}"))?;
        writer
            .write_all(b"\n")
            .map_err(|err| format!("failed to write response delimiter: {err}"))?;
        writer
            .flush()
            .map_err(|err| format!("failed to flush response: {err}"))?;
    }
}

fn dispatch_line(line: &str, logger: &Logger) -> Option<String> {
    let request: core::ProtocolRequest = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(err) => {
            log_message(logger, &format!("ignored invalid JSON request: {err}"));
            return None;
        }
    };

    Some(
        match dispatch_request(request) {
            Ok(response) => serde_json::to_string(&response),
            Err(response) => serde_json::to_string(&response),
        }
        .unwrap_or_else(|err| {
            serde_json::json!({
                "id": 0_u64,
                "error": {
                    "code": 500,
                    "message": format!("failed to serialize response: {err}")
                }
            })
            .to_string()
        }),
    )
}

fn dispatch_request(
    request: core::ProtocolRequest,
) -> Result<serde_json::Value, core::ProtocolErrorResponse> {
    let method = core::ProtocolMethod::parse(&request.method)
        .ok_or_else(|| core::error_response(request.id, 404, "unknown method"))?;

    match method {
        core::ProtocolMethod::PackagesListInstalled => {
            respond(request.id, request.params, |params: EmptyParams| {
                let _ = params;
                ops::list_installed_packages()
            })
        }
        core::ProtocolMethod::PackagesListAvailable => {
            respond(request.id, request.params, |params: PageParams| {
                ops::list_available_packages(params.offset, params.limit)
            })
        }
        core::ProtocolMethod::PackagesListPulse => {
            respond(request.id, request.params, |params: PageParams| {
                ops::list_pulse_packages(params.offset, params.limit)
            })
        }
        core::ProtocolMethod::PackagesInfo => {
            respond(request.id, request.params, |params: PackageInfoParams| {
                ops::package_info(&params.package)
            })
        }
        core::ProtocolMethod::PackagesSearch => {
            respond(request.id, request.params, |params: SearchParams| {
                ops::search_packages(&params.query, params.offset, params.limit)
            })
        }
        core::ProtocolMethod::PackagesListOutdated => {
            respond(request.id, request.params, |params: EmptyParams| {
                let _ = params;
                ops::list_outdated_packages()
            })
        }
        core::ProtocolMethod::PackagesIsotopeMigrationPlan => {
            respond(request.id, request.params, |params: IsotopeParams| {
                ops::isotope_migration_plan(&params.isotope)
            })
        }
        core::ProtocolMethod::PackagesMigrateIsotope => {
            respond(request.id, request.params, |params: IsotopeParams| {
                ops::migrate_isotope(&params.isotope)
            })
        }
        core::ProtocolMethod::SystemInfo => {
            respond(request.id, request.params, |params: EmptyParams| {
                let _ = params;
                Ok(ops::system_info())
            })
        }
    }
}

fn respond<Params, ResultBody, F>(
    id: u64,
    params: serde_json::Value,
    handler: F,
) -> Result<serde_json::Value, core::ProtocolErrorResponse>
where
    Params: for<'de> Deserialize<'de>,
    ResultBody: Serialize,
    F: FnOnce(Params) -> Result<ResultBody, String>,
{
    let params = serde_json::from_value(params)
        .map_err(|err| core::error_response(id, 500, format!("invalid params: {err}")))?;
    let result = handler(params).map_err(|err| core::error_response(id, 500, err))?;
    serde_json::to_value(core::success_response(id, result))
        .map_err(|err| core::error_response(id, 500, format!("failed to encode result: {err}")))
}

fn install_signal_handlers() {
    unsafe {
        let handler = handle_shutdown_signal as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGHUP, handler);
    }
}

fn log_message(logger: &Logger, message: &str) {
    let mut file = match logger.file.lock() {
        Ok(file) => file,
        Err(_) => return,
    };
    let _ = writeln!(file, "{message}");
}

#[derive(Debug)]
struct ProtocolPaths {
    socket_dir: PathBuf,
    socket_path: PathBuf,
    log_dir: PathBuf,
    log_path: PathBuf,
}

impl ProtocolPaths {
    fn resolve() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_string())?;
        let socket_dir = home
            .join("Library")
            .join("Application Support")
            .join("Automic Vault");
        let log_dir = home.join("Library").join("Logs");
        Ok(Self {
            socket_path: socket_dir.join("nucleus.sock"),
            socket_dir,
            log_path: log_dir.join("nucleus.log"),
            log_dir,
        })
    }
}

#[derive(Clone)]
struct Logger {
    file: Arc<Mutex<File>>,
}

impl Logger {
    fn new(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }
}

struct SocketCleanup {
    socket_path: PathBuf,
}

impl SocketCleanup {
    fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

#[derive(Debug, Deserialize)]
struct EmptyParams {}

#[derive(Debug, Deserialize)]
struct SearchParams {
    query: String,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct PageParams {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct PackageInfoParams {
    package: String,
}

#[derive(Debug, Deserialize)]
struct IsotopeParams {
    isotope: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_method_rejects_unknown_names() {
        assert_eq!(core::ProtocolMethod::parse("packages.install"), None);
    }

    #[test]
    fn protocol_method_parses_isotope_migration_methods() {
        assert_eq!(
            core::ProtocolMethod::parse("packages.isotopeMigrationPlan"),
            Some(core::ProtocolMethod::PackagesIsotopeMigrationPlan)
        );
        assert_eq!(
            core::ProtocolMethod::parse("packages.migrateIsotope"),
            Some(core::ProtocolMethod::PackagesMigrateIsotope)
        );
    }

    #[test]
    fn dispatch_request_rejects_unknown_methods() {
        let error = dispatch_request(core::ProtocolRequest {
            id: 7,
            method: "packages.install".to_string(),
            params: serde_json::json!({}),
        })
        .unwrap_err();

        assert_eq!(error.id, 7);
        assert_eq!(error.error.code, 404);
    }

    #[test]
    fn dispatch_request_returns_system_info() {
        let response = dispatch_request(core::ProtocolRequest {
            id: 3,
            method: "system.info".to_string(),
            params: serde_json::json!({}),
        })
        .unwrap();

        assert_eq!(
            response,
            serde_json::json!({
                "id": 3,
                "result": {
                    "version": env!("CARGO_PKG_VERSION"),
                    "protocolVersion": core::PROTOCOL_VERSION,
                    "buildId": env!("NUKE_BUILD_ID")
                }
            })
        );
    }

    #[test]
    fn dispatch_request_rejects_invalid_package_info_params() {
        let error = dispatch_request(core::ProtocolRequest {
            id: 11,
            method: "packages.info".to_string(),
            params: serde_json::json!({}),
        })
        .unwrap_err();

        assert_eq!(error.id, 11);
        assert_eq!(error.error.code, 500);
    }
}
