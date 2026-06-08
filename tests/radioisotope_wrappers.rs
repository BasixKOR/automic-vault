use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ORIGINAL_MARKER: &str = "radioisotope-e2e-original";
const USER_ARG: &str = "--radioisotope-e2e";
const MIN_AV_INJECT_SHELL_WRAPPER_COUNT: usize = 90;

#[derive(Debug)]
struct WrapperTemplate {
    isotope: String,
    source: PathBuf,
    line: usize,
    raw_script: String,
}

#[test]
fn radioisotope_av_inject_shell_wrappers_run_with_missing_optional_credentials() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let root = radioisotope_root();
    let templates = collect_av_inject_shell_wrappers(&root);
    assert!(
        templates.len() >= MIN_AV_INJECT_SHELL_WRAPPER_COUNT,
        "expected at least {MIN_AV_INJECT_SHELL_WRAPPER_COUNT} radioisotope av inject shell wrappers under {}, found {}",
        root.display(),
        templates.len()
    );

    let temp = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();

    for (index, template) in templates.iter().enumerate() {
        if let Err(err) = run_wrapper_template(template, index, temp.path()) {
            failures.push(format!(
                "{}:{} ({})\n{}",
                template.source.display(),
                template.line,
                template.isotope,
                err
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} radioisotope wrapper e2e failure(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn collect_av_inject_shell_wrappers(root: &Path) -> Vec<WrapperTemplate> {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read radioisotope root {}: {err}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();

    let mut wrappers = Vec::new();
    for isotope_dir in entries {
        let source = isotope_dir.join("post-install.rs");
        if !source.exists() {
            continue;
        }
        let contents = fs::read_to_string(&source).unwrap();
        for (line, raw_script) in raw_string_literals(&contents) {
            let Some(first_line) = raw_script.lines().next() else {
                continue;
            };
            if first_line.starts_with("#!/usr/local/bin/av inject --allow-missing-keys ")
                && first_line.ends_with(" /bin/sh")
            {
                wrappers.push(WrapperTemplate {
                    isotope: isotope_dir
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    source: source.clone(),
                    line,
                    raw_script,
                });
            }
        }
    }
    wrappers
}

fn radioisotope_root() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_RADIOISOTOPES_REPO")
        .or_else(|| option_env!("AUTOMIC_VAULT_RADIOISOTOPES_REPO").map(std::ffi::OsString::from))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("data/radioisotopes"))
}

fn raw_string_literals(contents: &str) -> Vec<(usize, String)> {
    let mut literals = Vec::new();
    let mut search_from = 0;

    while let Some(relative_start) = contents[search_from..].find("r#\"") {
        let start = search_from + relative_start;
        let body_start = start + 3;
        let Some(relative_end) = contents[body_start..].find("\"#") else {
            panic!("unterminated raw string literal near byte {start}");
        };
        let body_end = body_start + relative_end;
        let line = contents[..start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        literals.push((line, contents[body_start..body_end].to_string()));
        search_from = body_end + 2;
    }

    literals
}

fn run_wrapper_template(
    template: &WrapperTemplate,
    index: usize,
    root: &Path,
) -> Result<(), String> {
    let case_root = root.join(format!(
        "{index:03}-{}",
        sanitize_key_fragment(&template.isotope)
    ));
    fs::create_dir_all(&case_root).map_err(|err| err.to_string())?;
    let home = case_root.join("home");
    let tmp = case_root.join("tmp");
    fs::create_dir_all(&home).map_err(|err| err.to_string())?;
    fs::create_dir_all(&tmp).map_err(|err| err.to_string())?;

    let original = case_root.join("original");
    let wrapper = case_root.join("wrapper");
    write_executable(&original, original_script()).map_err(|err| err.to_string())?;
    let script = materialize_wrapper_script(
        template,
        &original,
        Path::new(env!("CARGO_BIN_EXE_av")),
        &format!(
            "AV_E2E_{}_{}",
            sanitize_key_fragment(&template.isotope),
            index
        ),
    )?;
    write_executable(&wrapper, script.as_bytes()).map_err(|err| err.to_string())?;

    let output = Command::new(&wrapper)
        .arg(USER_ARG)
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("TMPDIR", &tmp)
        .output()
        .map_err(|err| format!("failed to execute wrapper: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "wrapper exited with {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }
    if !stdout.contains(ORIGINAL_MARKER) {
        return Err(format!(
            "wrapper did not reach original executable\nstdout:\n{}\nstderr:\n{}",
            stdout, stderr
        ));
    }
    if !stdout.lines().any(|line| line == format!("arg:{USER_ARG}")) {
        return Err(format!(
            "wrapper did not preserve user argument\nstdout:\n{}\nstderr:\n{}",
            stdout, stderr
        ));
    }

    Ok(())
}

fn materialize_wrapper_script(
    template: &WrapperTemplate,
    original: &Path,
    av: &Path,
    key: &str,
) -> Result<String, String> {
    let original_argument = shell_single_argument(original);
    let mut script = template.raw_script.clone();
    if script.contains("original='{}'") {
        script = script.replacen(
            "original='{}'",
            &format!("original='{original_argument}'"),
            1,
        );
    } else if script.contains("exec '{}'") {
        script = script.replacen("exec '{}'", &format!("exec '{original_argument}'"), 1);
    } else {
        return Err(
            "wrapper template does not contain a supported original placeholder".to_string(),
        );
    }

    script = script.replace("{{", "{").replace("}}", "}");
    let first_line_end = script
        .find('\n')
        .ok_or_else(|| "wrapper template is missing a body".to_string())?;
    let mut materialized = format!(
        "#!{} inject --allow-missing-keys +{} /bin/sh",
        av.display(),
        key
    );
    materialized.push_str(&script[first_line_end..]);
    Ok(materialized)
}

fn shell_single_argument(path: &Path) -> String {
    path.to_string_lossy().replace('\'', r#"'\''"#)
}

fn sanitize_key_fragment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn write_executable(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

fn original_script() -> &'static [u8] {
    br#"#!/bin/sh
set -eu
printf "radioisotope-e2e-original\n"
for arg do
  printf "arg:%s\n" "$arg"
done
"#
}
