use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const USAGE: &str = "\
Usage: av proxy [--replace-existing-env] +KEY [+KEY...] [--] COMMAND [args...]

Runs COMMAND with random secret references and an explicitly authorized HTTP/S proxy.
Secret values are released to the proxy only when an authorized request needs them.";

const MANAGED_ENVIRONMENT: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
    "SSL_CERT_FILE",
    "NODE_EXTRA_CA_CERTS",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
    "GIT_SSL_CAINFO",
    "AWS_CA_BUNDLE",
];

#[derive(Debug, PartialEq, Eq)]
struct Options {
    replace_existing_env: bool,
    keys: Vec<String>,
    target: OsString,
    args: Vec<OsString>,
}

#[derive(Debug, PartialEq, Eq)]
struct StartRequest {
    keys: Vec<String>,
    target: String,
    args: Vec<String>,
    cwd: String,
    replace_existing_env: bool,
    env_conflicts: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct SessionEnvironment {
    proxy_url: String,
    ca_certificate_path: PathBuf,
    references: BTreeMap<String, String>,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match dispatch(args, stdout) {
        Ok(Some(options)) => exec(options, stderr),
        Ok(None) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "av proxy: {err}");
            1
        }
    }
}

fn dispatch(args: Vec<OsString>, stdout: &mut dyn Write) -> Result<Option<Options>, String> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        writeln!(stdout, "{USAGE}").ok();
        return Ok(None);
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--version" || arg == "-V")
    {
        writeln!(stdout, "av proxy {}", env!("CARGO_PKG_VERSION")).ok();
        return Ok(None);
    }
    match parse(args) {
        Ok(options) => Ok(Some(options)),
        Err(err) => {
            if err.starts_with("missing ") {
                writeln!(stdout, "{USAGE}").ok();
            }
            Err(err)
        }
    }
}

fn parse(args: Vec<OsString>) -> Result<Options, String> {
    let mut replace_existing_env = false;
    let mut keys = Vec::new();
    let mut seen_keys = BTreeSet::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        if arg == "--replace-existing-env" {
            if replace_existing_env {
                return Err("duplicate option: --replace-existing-env".into());
            }
            replace_existing_env = true;
            continue;
        }
        let value = arg
            .to_str()
            .ok_or_else(|| "proxy arguments must be valid UTF-8".to_string())?;
        if let Some(key) = value.strip_prefix('+') {
            super::inject::validate_key_name(key)?;
            if !seen_keys.insert(key.to_string()) {
                return Err(format!("duplicate key requested: {key}"));
            }
            keys.push(key.to_string());
            continue;
        }
        if arg == "--" {
            let target = iter
                .next()
                .ok_or_else(|| "missing target command".to_string())?;
            if keys.is_empty() {
                return Err("missing secret reference".into());
            }
            keys.sort();
            return Ok(Options {
                replace_existing_env,
                keys,
                target,
                args: iter.collect(),
            });
        }
        if value.starts_with('-') {
            return Err(format!("unknown option: {value}"));
        }
        if keys.is_empty() {
            return Err("at least one +KEY must be provided before the target".into());
        }
        keys.sort();
        return Ok(Options {
            replace_existing_env,
            keys,
            target: arg,
            args: iter.collect(),
        });
    }

    if keys.is_empty() {
        Err("missing secret reference and target command".into())
    } else {
        Err("missing target command".into())
    }
}

fn exec(options: Options, stderr: &mut dyn Write) -> i32 {
    if unsafe { libc::geteuid() } == 0 {
        let _ = writeln!(stderr, "av proxy: must not be run as root");
        return 1;
    }
    let target = match super::inject::resolve_target(&options.target) {
        Ok(target) => target,
        Err(err) => {
            let _ = writeln!(stderr, "av proxy: {err}");
            return 1;
        }
    };
    let request = match start_request(&options, &target) {
        Ok(request) => request,
        Err(err) => {
            let _ = writeln!(stderr, "av proxy: {err}");
            return 1;
        }
    };
    if !request.env_conflicts.is_empty() && !options.replace_existing_env {
        let _ = writeln!(
            stderr,
            "av proxy: existing proxy or CA environment would make interception ambiguous: {} (replace with: --replace-existing-env)",
            request.env_conflicts.join(", ")
        );
        return 1;
    }
    let session = match start_session(&request) {
        Ok(session) => session,
        Err(err) => {
            let _ = writeln!(stderr, "av proxy: {err}");
            return 1;
        }
    };
    let environment = match build_environment(&options, session) {
        Ok(environment) => environment,
        Err(err) => {
            let _ = writeln!(stderr, "av proxy: {err}");
            return 1;
        }
    };

    let err = Command::new(&target)
        .args(&options.args)
        .env_clear()
        .envs(environment)
        .exec();
    let _ = writeln!(
        stderr,
        "av proxy: failed to execute {}: {err}",
        target.display()
    );
    1
}

fn start_request(options: &Options, target: &PathBuf) -> Result<StartRequest, String> {
    let current_env = std::env::vars_os().collect::<BTreeMap<_, _>>();
    let env_conflicts = MANAGED_ENVIRONMENT
        .iter()
        .filter(|name| current_env.contains_key(std::ffi::OsStr::new(name)))
        .map(|name| (*name).to_string())
        .collect();
    Ok(StartRequest {
        keys: options.keys.clone(),
        target: target.display().to_string(),
        args: options
            .args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect(),
        cwd: std::env::current_dir()
            .map_err(|err| format!("failed to read current directory: {err}"))?
            .display()
            .to_string(),
        replace_existing_env: options.replace_existing_env,
        env_conflicts,
    })
}

fn build_environment(
    options: &Options,
    session: SessionEnvironment,
) -> Result<BTreeMap<OsString, OsString>, String> {
    if session.proxy_url.is_empty() {
        return Err("approval returned an empty proxy URL".into());
    }
    if !session.ca_certificate_path.is_absolute() {
        return Err("approval returned a non-absolute CA certificate path".into());
    }
    if session.references.len() != options.keys.len()
        || options
            .keys
            .iter()
            .any(|key| session.references.get(key).is_none_or(String::is_empty))
    {
        return Err("approval returned an incomplete set of secret references".into());
    }

    let mut environment = std::env::vars_os().collect::<BTreeMap<_, _>>();
    for name in MANAGED_ENVIRONMENT {
        environment.remove(std::ffi::OsStr::new(name));
    }
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        environment.insert(name.into(), session.proxy_url.clone().into());
    }
    environment.insert("NO_PROXY".into(), OsString::new());
    environment.insert("no_proxy".into(), OsString::new());
    for name in [
        "SSL_CERT_FILE",
        "NODE_EXTRA_CA_CERTS",
        "REQUESTS_CA_BUNDLE",
        "CURL_CA_BUNDLE",
        "GIT_SSL_CAINFO",
        "AWS_CA_BUNDLE",
    ] {
        environment.insert(
            name.into(),
            session.ca_certificate_path.clone().into_os_string(),
        );
    }
    for (key, reference) in session.references {
        environment.insert(key.into(), reference.into());
    }
    Ok(environment)
}

#[cfg(target_os = "macos")]
fn start_session(_request: &StartRequest) -> Result<SessionEnvironment, String> {
    Err("Secret Proxy sessions are not supported by this Automic Vault app build".into())
}

#[cfg(not(target_os = "macos"))]
fn start_session(_request: &StartRequest) -> Result<SessionEnvironment, String> {
    Err("Secret Proxy sessions are only available on macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn options() -> Options {
        Options {
            replace_existing_env: false,
            keys: vec!["API_TOKEN".into()],
            target: "/usr/bin/true".into(),
            args: Vec::new(),
        }
    }

    #[test]
    fn parses_and_sorts_secret_references() {
        assert_eq!(
            parse(os(&["+Z_TOKEN", "+A_TOKEN", "--", "node", "server.js"])).unwrap(),
            Options {
                replace_existing_env: false,
                keys: vec!["A_TOKEN".into(), "Z_TOKEN".into()],
                target: "node".into(),
                args: os(&["server.js"]),
            }
        );
    }

    #[test]
    fn rejects_missing_duplicate_and_invalid_references() {
        assert!(parse(os(&["--", "node"])).is_err());
        assert!(parse(os(&["+TOKEN", "+TOKEN", "node"])).is_err());
        assert!(parse(os(&["+BAD-NAME", "node"])).is_err());
        assert!(
            parse(os(&[
                "--replace-existing-env",
                "--replace-existing-env",
                "+TOKEN",
                "node"
            ]))
            .is_err()
        );
    }

    #[test]
    fn builds_reference_only_environment() {
        let environment = build_environment(
            &options(),
            SessionEnvironment {
                proxy_url: "http://av:credential@127.0.0.1:1234".into(),
                ca_certificate_path: "/tmp/session-ca.pem".into(),
                references: BTreeMap::from([("API_TOKEN".into(), "avref_random".into())]),
            },
        )
        .unwrap();
        assert_eq!(
            environment.get(std::ffi::OsStr::new("API_TOKEN")),
            Some(&OsString::from("avref_random"))
        );
        assert_eq!(
            environment.get(std::ffi::OsStr::new("HTTPS_PROXY")),
            Some(&OsString::from("http://av:credential@127.0.0.1:1234"))
        );
        assert_eq!(
            environment.get(std::ffi::OsStr::new("NO_PROXY")),
            Some(&OsString::new())
        );
    }

    #[test]
    fn rejects_incomplete_session_material() {
        assert!(
            build_environment(
                &options(),
                SessionEnvironment {
                    proxy_url: "http://127.0.0.1:1234".into(),
                    ca_certificate_path: "/tmp/session-ca.pem".into(),
                    references: BTreeMap::new(),
                }
            )
            .is_err()
        );
    }
}
