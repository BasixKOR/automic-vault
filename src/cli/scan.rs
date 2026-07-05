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

    fn paint(self, code: &str, text: impl AsRef<str>) -> String {
        let text = text.as_ref();
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

pub(crate) fn run<W: Write>(stdout: &mut W, style: Style) -> i32 {
    let findings = scan_home(home());
    print(stdout, &findings, style);
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

fn home() -> OsString {
    std::env::var_os("HOME").unwrap_or_default()
}

fn scan_home(home: impl AsRef<Path>) -> Vec<Finding> {
    isotopes::findings(home.as_ref())
}

fn print<W: Write>(stdout: &mut W, findings: &[Finding], style: Style) {
    let _ = writeln!(stdout, "╭─ {}", style.paint("36", "system exposure audit"));
    let _ = writeln!(stdout, "│");
    if findings.is_empty() {
        let _ = writeln!(stdout, "◇ {}", style.paint("32", "No problems found"));
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "╰─ {}", style.paint("2", "vault sealed"));
        return;
    }

    let finding_summary = if findings.len() == 1 {
        "1 finding requires attention".to_string()
    } else {
        format!("{} findings require attention", findings.len())
    };
    let _ = writeln!(stdout, "◆ {}", style.paint("33", finding_summary));
    let _ = writeln!(stdout, "│");
    for (index, finding) in findings.iter().enumerate() {
        let branch = if index + 1 == findings.len() {
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
            style.paint("31;1", finding.severity.to_ascii_uppercase())
        );
        let _ = writeln!(
            stdout,
            "│  {} {}",
            style.paint("2", "homepage"),
            finding.homepage
        );
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "problem"));
        write_wrapped(stdout, "│  ", &finding.explanation, style, None);
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "solution"));
        write_wrapped(stdout, "│  ", &finding.solution, style, None);
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
        let _ = writeln!(stdout, "│  {}", style.paint("1", "read more"));
        write_wrapped(stdout, "│  ", finding.docs_url, style, Some("36"));
        let _ = writeln!(stdout, "│");
    }
    let _ = writeln!(stdout, "╰─ {}", style.paint("2", "scan complete"));
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

        print(&mut stdout, &[fake_finding()], Style::plain());

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "╭─ system exposure audit\n│\n◆ 1 finding requires attention\n│\n└─ 1. example\n│  severity HIGH\n│  homepage https://example.test/\n│\n│  problem\n│  Example detector found a risky setting\n│\n│  solution\n│  Run `examplectl fix` or edit the affected file.\n│\n│  affected files\n│  • /tmp/example.conf:7\n│\n│  read more\n│  https://example.test/docs/example.md\n│\n╰─ scan complete\n"
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

        print(&mut stdout, &[], Style { color: true });

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .starts_with("╭─ \x1b[36msystem exposure audit\x1b[0m\n")
        );
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

        assert!(output.contains(r#""name":"git""#));
        assert!(output.contains(r#""docs_url":"https://github.com/automic-vault/automic-vault/main/docs/securing-git.md""#));
        assert!(output.contains(r##""documentation":"# git Detector"##));
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
