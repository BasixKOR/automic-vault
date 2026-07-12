use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::isotopes::hardeners::{self, HardenerCommand, HardenerMetadata, executable};

use super::scan::Style;

pub(crate) struct DoctorResult {
    name: &'static str,
    commands: Vec<String>,
    issues: Vec<DoctorIssue>,
}

struct DoctorIssue {
    kind: &'static str,
    command: Option<String>,
    message: String,
    remediation: String,
    stub_path: Option<String>,
    target_path: Option<String>,
    resolved_path: Option<String>,
}

pub(crate) fn run<W: Write>(
    stdout: &mut W,
    selector: Option<&str>,
    json: bool,
    style: Style,
) -> Result<i32, String> {
    let results = diagnose(
        hardeners::metadata(),
        selector,
        &std::env::var_os("PATH").unwrap_or_default(),
    )?;
    let issue_count = results
        .iter()
        .map(|result| result.issues.len())
        .sum::<usize>();
    if json {
        print_json(stdout, &results);
    } else {
        print_human(stdout, &results, issue_count, style);
    }
    Ok(if issue_count == 0 { 0 } else { 1 })
}

fn diagnose(
    hardeners: Vec<HardenerMetadata>,
    selector: Option<&str>,
    path: &OsStr,
) -> Result<Vec<DoctorResult>, String> {
    if let Some(selector) = selector {
        let (hardener, command) =
            select(hardeners, selector).ok_or_else(|| format!("unknown command `{selector}`"))?;
        return Ok(vec![diagnose_one(hardener, command.as_deref(), path)]);
    }

    Ok(hardeners
        .into_iter()
        .filter(|hardener| hardener.detection.applicable)
        .map(|hardener| diagnose_one(hardener, None, path))
        .collect())
}

fn select(
    hardeners: Vec<HardenerMetadata>,
    selector: &str,
) -> Option<(HardenerMetadata, Option<String>)> {
    let canonical = match selector {
        "gh-cli" => "gh",
        "homebrew" => "brew",
        "supabase-cli" => "supabase",
        selector => selector,
    };
    if let Some(index) = hardeners
        .iter()
        .position(|hardener| hardener.name == canonical)
    {
        return Some((hardeners.into_iter().nth(index).unwrap(), None));
    }
    let (index, command) = hardeners.iter().enumerate().find_map(|(index, hardener)| {
        hardener
            .detection
            .commands
            .iter()
            .find(|command| command.name == selector)
            .map(|command| (index, command.name.clone()))
    })?;
    Some((hardeners.into_iter().nth(index).unwrap(), Some(command)))
}

fn diagnose_one(
    hardener: HardenerMetadata,
    command_filter: Option<&str>,
    path: &OsStr,
) -> DoctorResult {
    let commands = hardener
        .detection
        .commands
        .iter()
        .filter(|command| command_filter.is_none_or(|filter| command.name == filter))
        .collect::<Vec<_>>();
    let mut issues = commands
        .iter()
        .filter(|command| !executable(Path::new(&command.target_path)))
        .map(|command| target_issue(hardener.name, command))
        .collect::<Vec<_>>();

    if issues.is_empty() {
        if commands.is_empty() {
            if !hardener.detection.hardened {
                issues.push(hardening_issue(
                    hardener.name,
                    None,
                    None,
                    hardener.detection.target_path,
                ));
            }
        } else {
            for command in &commands {
                if !command.hardened {
                    issues.push(hardening_issue(
                        hardener.name,
                        Some(command.name.clone()),
                        command.stub_path.clone(),
                        Some(command.target_path.clone()),
                    ));
                    continue;
                }
                if let Some(issue) = path_issue(command, path) {
                    issues.push(issue);
                }
            }
        }
    }

    DoctorResult {
        name: hardener.name,
        commands: commands
            .iter()
            .map(|command| command.name.clone())
            .collect(),
        issues,
    }
}

fn target_issue(hardener: &str, command: &HardenerCommand) -> DoctorIssue {
    DoctorIssue {
        kind: "target_unavailable",
        command: Some(command.name.clone()),
        message: format!(
            "{} target is missing or not executable: {}",
            command.name, command.target_path
        ),
        remediation: format!(
            "Install {hardener} so {} is executable, then rerun `av doctor {}`.",
            command.target_path, command.name
        ),
        stub_path: command.stub_path.clone(),
        target_path: Some(command.target_path.clone()),
        resolved_path: None,
    }
}

fn hardening_issue(
    hardener: &str,
    command: Option<String>,
    stub_path: Option<String>,
    target_path: Option<String>,
) -> DoctorIssue {
    DoctorIssue {
        kind: "hardening_not_applied",
        message: format!("{hardener} hardening is not applied or is no longer valid"),
        remediation: format!("Run `av harden {hardener}` and follow its instructions."),
        command,
        stub_path,
        target_path,
        resolved_path: None,
    }
}

fn path_issue(command: &HardenerCommand, path: &OsStr) -> Option<DoctorIssue> {
    let stub = command.stub_path.as_deref()?;
    if same_path(Path::new(stub), Path::new(&command.target_path)) {
        return None;
    }
    let resolved = resolve(&command.name, path);
    if resolved
        .as_deref()
        .is_some_and(|resolved| same_path(Path::new(stub), resolved))
    {
        return None;
    }
    let resolved_path = resolved.map(|path| path.display().to_string());
    let message = match &resolved_path {
        Some(resolved) => format!(
            "{} resolves to {resolved} before the hardened stub {stub}",
            command.name
        ),
        None => format!(
            "{} is not available through PATH; expected {stub}",
            command.name
        ),
    };
    Some(DoctorIssue {
        kind: "stub_not_first_on_path",
        command: Some(command.name.clone()),
        message,
        remediation: format!(
            "Put {} before the target directory in PATH.",
            Path::new(stub)
                .parent()
                .unwrap_or_else(|| Path::new(stub))
                .display()
        ),
        stub_path: Some(stub.to_string()),
        target_path: Some(command.target_path.clone()),
        resolved_path,
    })
}

fn resolve(command: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|directory| directory.join(command))
        .find(|candidate| executable(candidate))
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn print_json(stdout: &mut dyn Write, results: &[DoctorResult]) {
    let report = serde_json::json!({
        "results": results.iter().map(|result| serde_json::json!({
            "name": result.name,
            "commands": result.commands,
            "issues": result.issues.iter().map(|issue| serde_json::json!({
                "kind": issue.kind,
                "command": issue.command,
                "message": issue.message,
                "remediation": issue.remediation,
                "stub_path": issue.stub_path,
                "target_path": issue.target_path,
                "resolved_path": issue.resolved_path,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    let _ = writeln!(stdout, "{report}");
}

fn print_human(stdout: &mut dyn Write, results: &[DoctorResult], issue_count: usize, style: Style) {
    let _ = writeln!(stdout, "╭─ doctor");
    let _ = writeln!(stdout, "│");
    if results.is_empty() {
        let _ = writeln!(
            stdout,
            "╰─ {}",
            style.paint("32", "No applicable hardeners found")
        );
        return;
    }
    for result in results {
        if result.issues.is_empty() {
            let _ = writeln!(
                stdout,
                "├─ {} {}",
                result.name,
                style.paint("32", "healthy ✔︎")
            );
        } else {
            let _ = writeln!(stdout, "├─ {}", result.name);
            for issue in &result.issues {
                let _ = writeln!(stdout, "│  ├─ {}", style.paint("33", &issue.message));
                let _ = writeln!(stdout, "│  ╰─ {}", issue.remediation);
            }
        }
    }
    let summary = if issue_count == 0 {
        style.paint("32", "No problems found")
    } else if issue_count == 1 {
        style.paint("33", "1 issue requires attention")
    } else {
        style.paint("33", format!("{issue_count} issues require attention"))
    };
    let _ = writeln!(stdout, "│");
    let _ = writeln!(stdout, "╰─ {summary}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isotopes::hardeners::{HardenerDetection, HardenerMetadata};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn skips_inapplicable_hardeners_but_explicit_selection_reports_target() {
        let hardeners = vec![hardener(
            "node",
            false,
            command("npm", false, "/missing/stub", "/missing/npm"),
        )];
        assert!(
            diagnose(hardeners, None, OsStr::new(""))
                .unwrap()
                .is_empty()
        );

        let results = diagnose(
            vec![hardener(
                "node",
                false,
                command("npm", false, "/missing/stub", "/missing/npm"),
            )],
            Some("npm"),
            OsStr::new(""),
        )
        .unwrap();
        assert_eq!(results[0].issues[0].kind, "target_unavailable");
    }

    #[test]
    fn accepts_hardener_aliases_and_limits_executable_selection() {
        let dir = temp_dir("aliases");
        let jf = executable_file(&dir.join("jf"));
        let jfrog = executable_file(&dir.join("jfrog"));
        let hardeners = vec![HardenerMetadata {
            name: "jfrog-cli",
            documentation: "",
            detection: HardenerDetection::commands(
                true,
                vec![
                    command("jf", true, jf.to_str().unwrap(), jf.to_str().unwrap()),
                    command(
                        "jfrog",
                        true,
                        jfrog.to_str().unwrap(),
                        jfrog.to_str().unwrap(),
                    ),
                ],
            ),
            secret_gate: None,
        }];
        let results = diagnose(hardeners, Some("jf"), dir.as_os_str()).unwrap();
        assert_eq!(results[0].commands, ["jf"]);
        assert!(results[0].issues.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_hardening_and_path_precedence() {
        let dir = temp_dir("path");
        let stub_dir = dir.join("stub");
        let target_dir = dir.join("target");
        fs::create_dir_all(&stub_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        let stub = executable_file(&stub_dir.join("aws"));
        let target = executable_file(&target_dir.join("aws"));

        let unhardened = diagnose(
            vec![hardener(
                "aws",
                false,
                command(
                    "aws",
                    false,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            stub_dir.as_os_str(),
        )
        .unwrap();
        assert_eq!(unhardened[0].issues[0].kind, "hardening_not_applied");

        let shadowed_path = std::env::join_paths([&target_dir, &stub_dir]).unwrap();
        let shadowed = diagnose(
            vec![hardener(
                "aws",
                true,
                command(
                    "aws",
                    true,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            &shadowed_path,
        )
        .unwrap();
        assert_eq!(shadowed[0].issues[0].kind, "stub_not_first_on_path");
        assert_eq!(
            shadowed[0].issues[0].resolved_path.as_deref(),
            target.to_str()
        );

        let healthy_path = std::env::join_paths([&stub_dir, &target_dir]).unwrap();
        let healthy = diagnose(
            vec![hardener(
                "aws",
                true,
                command(
                    "aws",
                    true,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            &healthy_path,
        )
        .unwrap();
        assert!(healthy[0].issues.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    fn hardener(name: &'static str, hardened: bool, command: HardenerCommand) -> HardenerMetadata {
        HardenerMetadata {
            name,
            documentation: "",
            detection: HardenerDetection::commands(hardened, vec![command]),
            secret_gate: None,
        }
    }

    fn command(name: &str, hardened: bool, stub: &str, target: &str) -> HardenerCommand {
        HardenerCommand {
            name: name.to_string(),
            hardened,
            stub_path: Some(stub.to_string()),
            target_path: target.to_string(),
        }
    }

    fn executable_file(path: &Path) -> PathBuf {
        fs::write(path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        path.to_path_buf()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("av-doctor-{label}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
