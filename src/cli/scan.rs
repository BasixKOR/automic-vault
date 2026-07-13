use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use crate::{Finding, isotopes};

const TEXT_WIDTH: usize = 72;

#[derive(Clone, Copy)]
pub(crate) struct Style {
    pub(crate) color: bool,
}

impl Style {
    pub(crate) fn plain() -> Self {
        Self { color: false }
    }

    pub(crate) fn paint(self, code: &str, text: impl AsRef<str>) -> String {
        let text = text.as_ref();
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

pub(crate) fn run<W: Write>(stdout: &mut W, style: Style, show_all: bool) -> i32 {
    let findings = scan_home(home());
    print(stdout, &findings, style, show_all);
    0
}

pub(crate) fn run_json<W: Write>(stdout: &mut W) -> i32 {
    let findings = scan_home(home());
    let report = serde_json::json!({
        "findings": findings.iter().map(json_finding).collect::<Vec<_>>(),
    });
    let _ = writeln!(stdout, "{report}");
    0
}

pub(crate) fn run_detectors_json<W: Write>(stdout: &mut W) -> i32 {
    let report = serde_json::json!({
        "detectors": isotopes::detector_metadata().into_iter().map(|detector| {
            serde_json::json!({
                "name": detector.name,
                "homepage": detector.homepage,
                "docs_url": detector.docs_url,
                "documentation": detector.documentation,
            })
        }).collect::<Vec<_>>(),
    });
    let _ = writeln!(stdout, "{report}");
    0
}

pub(crate) fn run_hardeners_json<W: Write>(stdout: &mut W) -> i32 {
    let report = serde_json::json!({
        "hardeners": isotopes::hardener_metadata().into_iter().map(|hardener| {
            let secret_gate = hardener.secret_gate.map(|gate| serde_json::json!({
                "id": gate.id,
                "key_patterns": gate.key_patterns,
                "routes": gate.routes.into_iter().map(|route| serde_json::json!({
                    "operation": route.operation,
                    "script_path": route.script_path,
                    "target_path": route.target_path,
                    "caller_identifiers": route.caller_identifiers,
                    "key_patterns": route.key_patterns,
                    "replace_existing_env": route.replace_existing_env,
                    "allow_missing_keys": route.allow_missing_keys,
                })).collect::<Vec<_>>(),
            }));
            serde_json::json!({
                "name": hardener.name,
                "documentation": hardener.documentation,
                "hardened": hardener.detection.hardened,
                "applicable": hardener.detection.applicable,
                "stub_path": hardener.detection.stub_path,
                "target_path": hardener.detection.target_path,
                "commands": hardener.detection.commands.into_iter().map(|command| serde_json::json!({
                    "name": command.name,
                    "hardened": command.hardened,
                    "stub_path": command.stub_path,
                    "target_path": command.target_path,
                    "required_paths": command.required_paths,
                })).collect::<Vec<_>>(),
                "secret_gate": secret_gate,
            })
        }).collect::<Vec<_>>(),
    });
    let _ = writeln!(stdout, "{report}");
    0
}

fn home() -> OsString {
    std::env::var_os("HOME").unwrap_or_default()
}

fn scan_home(home: impl AsRef<Path>) -> Vec<Finding> {
    isotopes::findings(home.as_ref())
}

fn print<W: Write>(stdout: &mut W, findings: &[Finding], style: Style, show_all: bool) {
    let visible = findings
        .iter()
        .filter(|finding| show_all || !is_hidden(finding))
        .collect::<Vec<_>>();
    let hidden = findings
        .iter()
        .filter(|finding| !show_all && is_hidden(finding))
        .collect::<Vec<_>>();

    let _ = writeln!(stdout, "╭─ {}", style.paint("36", "system exposure audit"));
    let _ = writeln!(stdout, "│");
    if findings.is_empty() {
        let _ = writeln!(stdout, "◇ {}", style.paint("32", "No problems found"));
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "╰─ {}", style.paint("2", "vault sealed"));
        return;
    }

    if visible.is_empty() {
        let _ = writeln!(
            stdout,
            "◇ {}",
            style.paint("32", "No high-severity problems found")
        );
        let _ = writeln!(stdout, "│");
    }

    let finding_summary = if visible.len() == 1 {
        "1 finding requires attention".to_string()
    } else {
        format!("{} findings require attention", visible.len())
    };
    if !visible.is_empty() {
        let _ = writeln!(stdout, "◆ {}", style.paint("33", finding_summary));
        let _ = writeln!(stdout, "│");
    }
    for (index, finding) in visible.iter().enumerate() {
        let branch = if index + 1 == visible.len() {
            "└"
        } else {
            "├"
        };
        let _ = writeln!(
            stdout,
            "{branch}─ {} {}",
            style.paint("1", format!("{}.", index + 1)),
            style.paint("1;35", finding.source)
        );
        let _ = writeln!(
            stdout,
            "│  {} {}",
            style.paint("2", "severity"),
            style.paint(
                severity_color(finding.severity),
                finding.severity.to_ascii_uppercase(),
            )
        );
        let _ = writeln!(stdout, "│  {}", style.paint("2", "homepage"));
        let _ = writeln!(stdout, "│  {}", style.paint("36", finding.homepage));
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "problem"));
        write_wrapped(stdout, "│  ", &finding.explanation, style, None);
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "solution"));
        write_wrapped(stdout, "│  ", &finding.solution, style, None);
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "more details & caveats"));
        let _ = writeln!(stdout, "│  {}", style.paint("36", finding.docs_url));
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "affected files"));
        if finding.affected.is_empty() {
            let _ = writeln!(stdout, "│  • not reported by this detector");
        } else {
            for affected in &finding.affected {
                write_wrapped_with_continuation(
                    stdout,
                    "│  • ",
                    "│    ",
                    &format!("{}:{}", affected.path, affected.line),
                    style,
                    Some("36"),
                );
            }
        }
        let _ = writeln!(stdout, "│");
    }
    if !hidden.is_empty() {
        let _ = writeln!(stdout, "◇ {}", style.paint("33", hidden_summary(&hidden)));
        let _ = writeln!(stdout, "│");
    }
    let _ = writeln!(stdout, "╰─ {}", style.paint("2", "scan complete"));
}

fn is_hidden(finding: &Finding) -> bool {
    matches!(finding.severity, "medium" | "low")
}

fn severity_color(severity: &str) -> &str {
    match severity {
        "medium" => "33;1",
        "low" => "33",
        _ => "31;1",
    }
}

fn hidden_summary(findings: &[&Finding]) -> String {
    let medium = findings
        .iter()
        .filter(|finding| finding.severity == "medium")
        .count();
    let low = findings
        .iter()
        .filter(|finding| finding.severity == "low")
        .count();
    let counts = [(medium, "medium"), (low, "low")]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, severity)| format!("{count} {severity}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let finding = if findings.len() == 1 {
        "finding"
    } else {
        "findings"
    };
    format!("{counts} {finding} hidden, rerun with `--show-all` to view")
}

fn json_finding(finding: &Finding) -> serde_json::Value {
    serde_json::json!({
        "source": finding.source,
        "severity": finding.severity,
        "homepage": finding.homepage,
        "explanation": finding.explanation,
        "solution": finding.solution,
        "affected": finding.affected.iter().map(|affected| {
            serde_json::json!({
                "path": affected.path,
                "line": affected.line,
            })
        }).collect::<Vec<_>>(),
        "docs_url": finding.docs_url,
    })
}

fn write_wrapped<W: Write>(
    stdout: &mut W,
    prefix: &str,
    text: &str,
    style: Style,
    color: Option<&str>,
) {
    write_wrapped_with_continuation(stdout, prefix, prefix, text, style, color);
}

fn write_wrapped_with_continuation<W: Write>(
    stdout: &mut W,
    first_prefix: &str,
    continuation_prefix: &str,
    text: &str,
    style: Style,
    color: Option<&str>,
) {
    for (line_number, line) in wrap_text(text, TEXT_WIDTH).into_iter().enumerate() {
        let rendered = match color {
            Some(code) => style.paint(code, &line),
            None => line,
        };
        let prefix = if line_number == 0 {
            first_prefix
        } else {
            continuation_prefix
        };
        let _ = writeln!(stdout, "{prefix}{rendered}");
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        wrap_paragraph(paragraph, width, &mut lines);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn wrap_paragraph(paragraph: &str, width: usize, lines: &mut Vec<String>) {
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        if line.is_empty() {
            push_word(word, width, &mut line, lines);
        } else if line.len() + 1 + word.len() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut line));
            push_word(word, width, &mut line, lines);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
}

fn push_word(word: &str, width: usize, line: &mut String, lines: &mut Vec<String>) {
    if word.len() <= width {
        line.push_str(word);
        return;
    }

    let mut chunk = String::new();
    let mut len = 0;
    for ch in word.chars() {
        chunk.push(ch);
        len += 1;
        if len == width {
            lines.push(std::mem::take(&mut chunk));
            len = 0;
        }
    }
    line.push_str(&chunk);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_displays_findings() {
        let mut stdout = Vec::new();

        print(&mut stdout, &[fake_finding()], Style::plain(), false);

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "╭─ system exposure audit\n│\n◆ 1 finding requires attention\n│\n└─ 1. example\n│  severity HIGH\n│  homepage\n│  https://example.test/\n│\n│  problem\n│  Example detector found a risky setting\n│\n│  solution\n│  Run `examplectl fix` or edit the affected file.\n│\n│  more details & caveats\n│  https://example.test/docs/example.md\n│\n│  affected files\n│  • /tmp/example.conf:7\n│\n╰─ scan complete\n"
        );
    }

    #[test]
    fn print_displays_unattributed_findings_without_fake_file_location() {
        let mut stdout = Vec::new();

        print(
            &mut stdout,
            &[Finding {
                source: "example",
                homepage: "https://example.test/",
                severity: "high",
                explanation: "Example detector found a risky setting".to_string(),
                solution: "Run `examplectl fix`.".to_string(),
                affected: Vec::new(),
                docs_url: "https://example.test/docs/example.md",
            }],
            Style::plain(),
            false,
        );

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("│  • not reported by this detector\n")
        );
    }

    #[test]
    fn styled_output_uses_ansi() {
        let mut stdout = Vec::new();

        print(&mut stdout, &[], Style { color: true }, false);

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .starts_with("╭─ \x1b[36msystem exposure audit\x1b[0m\n")
        );
    }

    #[test]
    fn print_hides_medium_and_low_findings_by_default() {
        let mut stdout = Vec::new();
        let mut medium = fake_finding();
        medium.severity = "medium";
        let mut another_medium = fake_finding();
        another_medium.severity = "medium";
        let mut low = fake_finding();
        low.severity = "low";

        print(
            &mut stdout,
            &[medium, another_medium, low],
            Style::plain(),
            false,
        );

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "╭─ system exposure audit\n│\n◇ No high-severity problems found\n│\n◇ 2 medium and 1 low findings hidden, rerun with `--show-all` to view\n│\n╰─ scan complete\n"
        );
    }

    #[test]
    fn show_all_displays_medium_and_low_in_amber() {
        let mut stdout = Vec::new();
        let mut medium = fake_finding();
        medium.severity = "medium";
        let mut low = fake_finding();
        low.severity = "low";

        print(&mut stdout, &[medium, low], Style { color: true }, true);
        let output = String::from_utf8(stdout).unwrap();

        assert!(output.contains("severity\x1b[0m \x1b[33;1mMEDIUM\x1b[0m"));
        assert!(output.contains("severity\x1b[0m \x1b[33mLOW\x1b[0m"));
        assert!(!output.contains("findings hidden"));
    }

    #[test]
    fn json_output_reports_findings() {
        let mut stdout = Vec::new();

        let report = serde_json::json!({
            "findings": [json_finding(&fake_finding())],
        });
        let _ = writeln!(stdout, "{report}");

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            r#"{"findings":[{"affected":[{"line":7,"path":"/tmp/example.conf"}],"docs_url":"https://example.test/docs/example.md","explanation":"Example detector found a risky setting","homepage":"https://example.test/","severity":"high","solution":"Run `examplectl fix` or edit the affected file.","source":"example"}]}"#.to_string() + "\n"
        );
    }

    #[test]
    fn detectors_json_reports_metadata() {
        let mut stdout = Vec::new();

        assert_eq!(run_detectors_json(&mut stdout), 0);
        let output = String::from_utf8(stdout).unwrap();

        assert!(output.contains(r#""name":"git-credential-fill""#));
        assert!(output.contains(r#""name":"git-credential-oauth""#));
        assert!(output.contains(r#""name":"git-credentials-file""#));
        assert!(output.contains(r##""documentation":"# git-credential-fill Detector"##));
    }

    #[test]
    fn hardeners_json_reports_metadata() {
        let mut stdout = Vec::new();

        assert_eq!(run_hardeners_json(&mut stdout), 0);
        let output = String::from_utf8(stdout).unwrap();

        assert!(output.contains(r#""name":"aws""#));
        assert!(output.contains(r#""name":"gh""#));
        assert!(output.contains(r#""name":"sudo""#));
        assert!(output.contains(r#""hardened":"#));
        assert!(output.contains(r#""stub_path":"#));
        assert!(output.contains(r#""target_path":"#));
        assert!(output.contains(r###""documentation":"# GitHub CLI"###));

        let report: serde_json::Value = serde_json::from_str(&output).unwrap();
        let hardeners = report["hardeners"].as_array().unwrap();
        let gate = |name: &str| {
            &hardeners
                .iter()
                .find(|hardener| hardener["name"] == name)
                .unwrap()["secret_gate"]
        };
        assert_eq!(gate("gh")["id"], "gh");
        assert_eq!(gate("supabase")["id"], "supabase");
        assert_eq!(gate("aws")["routes"][0]["operation"], "inject");
        assert!(gate("brew").is_null());
        assert!(gate("sudo").is_null());
        assert_eq!(gate("jfrog-cli")["routes"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn wraps_long_lines_inside_the_rail() {
        let lines = wrap_text(
            "Run `examplectl harden very-long-target-name` or edit the affected configuration file.",
            48,
        );

        assert_eq!(
            lines,
            vec![
                "Run `examplectl harden very-long-target-name` or",
                "edit the affected configuration file.",
            ]
        );
    }

    fn fake_finding() -> Finding {
        Finding {
            source: "example",
            homepage: "https://example.test/",
            severity: "high",
            explanation: "Example detector found a risky setting".to_string(),
            solution: "Run `examplectl fix` or edit the affected file.".to_string(),
            affected: vec![crate::AffectedFile {
                path: "/tmp/example.conf".to_string(),
                line: 7,
            }],
            docs_url: "https://example.test/docs/example.md",
        }
    }
}
