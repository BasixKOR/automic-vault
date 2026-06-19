use super::*;

pub(crate) const SCANNER_WRAPPER_UI_ENV: &str = "AUTOMIC_VAULT_SCANNER_WRAPPER_UI";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SecretScannerRequest {
    pub(crate) path: Option<PathBuf>,
    pub(crate) skip_paths: Vec<PathBuf>,
    pub(crate) output: OutputMode,
    pub(crate) isotopes_only: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SecretScannerReport {
    pub(crate) scope: SecretScannerScope,
    pub(crate) findings: Vec<SecretScannerFinding>,
    pub(crate) errors: Vec<SecretScannerError>,
    pub(crate) summary: SecretScannerSummary,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SecretScannerScope {
    Full,
    IsotopesOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ShellSecretFlavor {
    Bash,
    Zsh,
}

impl ShellSecretFlavor {
    pub(crate) fn display_name(self) -> &'static str {
        match self {
            ShellSecretFlavor::Bash => "Bash",
            ShellSecretFlavor::Zsh => "Zsh",
        }
    }

    pub(crate) fn source_label(self) -> &'static str {
        match self {
            ShellSecretFlavor::Bash => "file-probe:bash",
            ShellSecretFlavor::Zsh => "file-probe:zsh",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub(crate) struct SecretScannerFinding {
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<usize>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub(crate) struct SecretScannerError {
    pub(crate) source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SecretScannerSummary {
    pub(crate) scanned_files: usize,
    pub(crate) findings: usize,
    pub(crate) errors: usize,
    pub(crate) isotope_detectors: usize,
    pub(crate) file_probes: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SecretScannerEvent<'a> {
    Finding(&'a SecretScannerFinding),
    Error(&'a SecretScannerError),
}

pub(crate) fn run_secret_scan(
    request: &SecretScannerRequest,
) -> Result<SecretScannerReport, String> {
    run_secret_scan_with_events(request, |_| Ok(()))
}

pub(crate) fn run_secret_scan_with_events<F>(
    request: &SecretScannerRequest,
    mut on_event: F,
) -> Result<SecretScannerReport, String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    let mut findings = Vec::new();
    let mut errors = Vec::new();
    let mut seen_findings = HashSet::new();
    let mut seen_errors = HashSet::new();
    let mut isotope_detectors = 0;

    if secret_scan_should_run_isotope_detectors(request) {
        for integration in isotope_integrations::INTEGRATIONS {
            let detector = integration
                .detect_reasons
                .map(|detect_reasons| detect_reasons())
                .or_else(|| {
                    integration.detect.map(|detect| {
                        detect().map(|install_is_insecure| {
                            if install_is_insecure {
                                vec![format!(
                                    "isotope:{} detector found plaintext credential exposure",
                                    integration.name
                                )]
                            } else {
                                Vec::new()
                            }
                        })
                    })
                });

            let Some(result) = detector else {
                continue;
            };
            isotope_detectors += 1;

            match result {
                Ok(reasons) => {
                    for reason in reasons {
                        record_secret_scanner_finding(
                            &mut findings,
                            &mut seen_findings,
                            SecretScannerFinding {
                                source: format!("isotope:{}", integration.name),
                                kind: "detector".to_string(),
                                severity: "high".to_string(),
                                path: None,
                                line: None,
                                message: reason,
                            },
                            &mut on_event,
                        )?;
                    }
                }
                Err(err) => record_secret_scanner_error(
                    &mut errors,
                    &mut seen_errors,
                    SecretScannerError {
                        source: format!("isotope:{}", integration.name),
                        path: None,
                        message: err,
                    },
                    &mut on_event,
                )?,
            }
        }
    }

    let mut scanned_files = 0;
    let mut file_probes = 0;
    if !request.isotopes_only {
        (scanned_files, file_probes) = scan_secret_file_probes(
            request.path.as_deref(),
            &request.skip_paths,
            &mut findings,
            &mut errors,
            &mut seen_findings,
            &mut seen_errors,
            &mut on_event,
        )?;
    }

    findings.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.message.cmp(&right.message))
    });
    findings.dedup();
    errors.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.message.cmp(&right.message))
    });
    errors.dedup();

    Ok(SecretScannerReport {
        scope: if request.isotopes_only {
            SecretScannerScope::IsotopesOnly
        } else {
            SecretScannerScope::Full
        },
        summary: SecretScannerSummary {
            scanned_files,
            findings: findings.len(),
            errors: errors.len(),
            isotope_detectors,
            file_probes,
        },
        findings,
        errors,
    })
}

pub(crate) fn secret_scan_should_run_isotope_detectors(request: &SecretScannerRequest) -> bool {
    request.path.is_none()
}

pub(crate) fn record_secret_scanner_finding<F>(
    findings: &mut Vec<SecretScannerFinding>,
    seen_findings: &mut HashSet<SecretScannerFinding>,
    finding: SecretScannerFinding,
    on_event: &mut F,
) -> Result<(), String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    if seen_findings.insert(finding.clone()) {
        on_event(SecretScannerEvent::Finding(&finding))?;
        findings.push(finding);
    }
    Ok(())
}

pub(crate) fn record_secret_scanner_error<F>(
    errors: &mut Vec<SecretScannerError>,
    seen_errors: &mut HashSet<SecretScannerError>,
    error: SecretScannerError,
    on_event: &mut F,
) -> Result<(), String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    if seen_errors.insert(error.clone()) {
        on_event(SecretScannerEvent::Error(&error))?;
        errors.push(error);
    }
    Ok(())
}

pub(crate) fn print_secret_scanner_report_streaming(
    request: &SecretScannerRequest,
) -> Result<(), String> {
    let mut printer = SecretScannerStreamPrinter::new(request);
    printer.begin()?;
    let report = run_secret_scan_with_events(request, |event| printer.print_event(event))?;
    printer.finish(&report)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SecretScannerStreamFormat {
    Plain,
    Rich,
    Wrapped,
}

pub(crate) struct SecretScannerStreamPrinter {
    pub(crate) format: SecretScannerStreamFormat,
    pub(crate) color: bool,
    pub(crate) scope: SecretScannerScope,
    pub(crate) finding_count: usize,
    pub(crate) printed_findings_header: bool,
    pub(crate) printed_warnings_header: bool,
}

impl SecretScannerStreamPrinter {
    pub(crate) fn new(request: &SecretScannerRequest) -> Self {
        let stdout_is_rich = scan_stdout_is_rich();
        let format = if scanner_wrapper_ui_enabled() && stdout_is_rich {
            SecretScannerStreamFormat::Wrapped
        } else if stdout_is_rich {
            SecretScannerStreamFormat::Rich
        } else {
            SecretScannerStreamFormat::Plain
        };
        Self {
            format,
            color: !matches!(format, SecretScannerStreamFormat::Plain)
                && scan_stdout_supports_ansi(),
            scope: if request.isotopes_only {
                SecretScannerScope::IsotopesOnly
            } else {
                SecretScannerScope::Full
            },
            finding_count: 0,
            printed_findings_header: false,
            printed_warnings_header: false,
        }
    }

    pub(crate) fn begin(&mut self) -> Result<(), String> {
        match self.format {
            SecretScannerStreamFormat::Plain => {
                println!("Automic Vault scan");
                println!("Scope: {}", secret_scanner_scope_label(self.scope));
            }
            SecretScannerStreamFormat::Rich => {
                let status = scan_paint(">", ScanStyle::Heading, self.color);
                let scope = format!(
                    "{}: {}",
                    scan_paint("Scope", ScanStyle::Dim, self.color),
                    secret_scanner_scope_label(self.scope)
                );
                print_scan_box(
                    "Automic Vault Scan",
                    &[
                        format!("{status} Scanning plaintext credential exposure"),
                        scope,
                    ],
                    self.color,
                );
            }
            SecretScannerStreamFormat::Wrapped => {
                let rail = scan_rail(ScanStyle::Dim, self.color);
                println!("{rail}");
                let rail = scan_rail(ScanStyle::Heading, self.color);
                println!(
                    "{rail} {} Scanning plaintext credential exposure",
                    scan_paint(">", ScanStyle::Heading, self.color)
                );
                let rail = scan_rail(ScanStyle::Dim, self.color);
                println!(
                    "{rail}   {}     {}",
                    scan_paint("Scope", ScanStyle::Dim, self.color),
                    secret_scanner_scope_label(self.scope)
                );
            }
        }
        flush_secret_scanner_stdout()
    }

    pub(crate) fn print_event(&mut self, event: SecretScannerEvent<'_>) -> Result<(), String> {
        match event {
            SecretScannerEvent::Finding(finding) => self.print_finding(finding),
            SecretScannerEvent::Error(error) => self.print_error(error),
        }
    }

    pub(crate) fn print_finding(&mut self, finding: &SecretScannerFinding) -> Result<(), String> {
        self.finding_count += 1;
        match self.format {
            SecretScannerStreamFormat::Plain => {
                if !self.printed_findings_header {
                    println!();
                    println!("Findings:");
                    self.printed_findings_header = true;
                }
                println!(
                    "{}. {} {} - {}",
                    self.finding_count, finding.severity, finding.source, finding.message
                );
                if let Some(location) = secret_scanner_finding_location(finding) {
                    println!("   {location}");
                }
            }
            SecretScannerStreamFormat::Rich => {
                if !self.printed_findings_header {
                    println!();
                    println!("{}", scan_paint("Findings", ScanStyle::Heading, self.color));
                    self.printed_findings_header = true;
                }
                let severity =
                    scan_paint(&finding.severity, scan_severity_style(finding), self.color);
                println!(
                    "  {}. {} {}",
                    self.finding_count,
                    severity,
                    scan_paint(&finding.source, ScanStyle::Dim, self.color)
                );
                if let Some(location) = secret_scanner_finding_location(finding) {
                    println!(
                        "     {}",
                        scan_paint(&location, ScanStyle::Path, self.color)
                    );
                }
                println!("     {}", finding.message);
            }
            SecretScannerStreamFormat::Wrapped => {
                if !self.printed_findings_header {
                    let rail = scan_rail(ScanStyle::Dim, self.color);
                    println!("{rail}");
                    let rail = scan_rail(ScanStyle::Heading, self.color);
                    println!(
                        "{rail} {}",
                        scan_paint("Findings", ScanStyle::Heading, self.color)
                    );
                    self.printed_findings_header = true;
                }
                let severity =
                    scan_paint(&finding.severity, scan_severity_style(finding), self.color);
                let rail = scan_rail(scan_severity_style(finding), self.color);
                println!(
                    "{rail}   {}. {} {}",
                    self.finding_count,
                    severity,
                    scan_paint(&finding.source, ScanStyle::Dim, self.color)
                );
                if let Some(location) = secret_scanner_finding_location(finding) {
                    let rail = scan_rail(ScanStyle::Path, self.color);
                    println!(
                        "{rail}      {}",
                        scan_paint(&location, ScanStyle::Path, self.color)
                    );
                }
                let rail = scan_rail(ScanStyle::Dim, self.color);
                println!("{rail}      {}", finding.message);
            }
        }
        flush_secret_scanner_stdout()
    }

    pub(crate) fn print_error(&mut self, error: &SecretScannerError) -> Result<(), String> {
        match self.format {
            SecretScannerStreamFormat::Plain => {
                if !self.printed_warnings_header {
                    eprintln!();
                    eprintln!("Warnings");
                    self.printed_warnings_header = true;
                }
                print_secret_scanner_warning_line(error, false);
                flush_secret_scanner_stderr()
            }
            SecretScannerStreamFormat::Rich => {
                if !self.printed_warnings_header {
                    eprintln!();
                    eprintln!("{}", scan_paint("Warnings", ScanStyle::Warning, self.color));
                    self.printed_warnings_header = true;
                }
                print_secret_scanner_warning_line(error, self.color);
                flush_secret_scanner_stderr()
            }
            SecretScannerStreamFormat::Wrapped => {
                if !self.printed_warnings_header {
                    let rail = scan_rail(ScanStyle::Dim, self.color);
                    println!("{rail}");
                    let rail = scan_rail(ScanStyle::Warning, self.color);
                    println!(
                        "{rail} {}",
                        scan_paint("Warnings", ScanStyle::Warning, self.color)
                    );
                    self.printed_warnings_header = true;
                }
                print_wrapped_secret_scanner_warning_line(error, self.color);
                flush_secret_scanner_stdout()
            }
        }
    }

    pub(crate) fn finish(&mut self, report: &SecretScannerReport) -> Result<(), String> {
        match self.format {
            SecretScannerStreamFormat::Plain => {
                if report.findings.is_empty() {
                    println!("No plaintext secret exposure detected.");
                }
                println!(
                    "Summary: {}, {}, {}, {}.",
                    pluralize(report.summary.findings, "finding", "findings"),
                    pluralize(report.summary.errors, "warning", "warnings"),
                    pluralize(
                        report.summary.isotope_detectors,
                        "isotope detector",
                        "isotope detectors"
                    ),
                    secret_scanner_file_probe_summary(report)
                );
            }
            SecretScannerStreamFormat::Rich => {
                println!();
                if report.findings.is_empty() {
                    println!(
                        "{} No plaintext secret exposure detected",
                        scan_paint("✓", ScanStyle::Success, self.color)
                    );
                }
                println!("{}", scan_paint("Summary", ScanStyle::Heading, self.color));
                println!(
                    "  {} · {} · {} · {}",
                    pluralize(report.summary.findings, "finding", "findings"),
                    pluralize(report.summary.errors, "warning", "warnings"),
                    pluralize(
                        report.summary.isotope_detectors,
                        "isotope detector",
                        "isotope detectors"
                    ),
                    secret_scanner_file_probe_summary(report)
                );
            }
            SecretScannerStreamFormat::Wrapped => {
                if report.findings.is_empty() {
                    let rail = scan_rail(ScanStyle::Dim, self.color);
                    println!("{rail}");
                    let rail = scan_rail(ScanStyle::Success, self.color);
                    println!(
                        "{rail} {} No plaintext secret exposure detected",
                        scan_paint("✓", ScanStyle::Success, self.color)
                    );
                }
                let rail = scan_rail(ScanStyle::Dim, self.color);
                println!("{rail}");
                println!(
                    "{rail}   {}   {}",
                    scan_paint("Checked", ScanStyle::Dim, self.color),
                    pluralize(
                        report.summary.isotope_detectors,
                        "isotope detector",
                        "isotope detectors"
                    )
                );
                println!(
                    "{rail}   {}     {}",
                    scan_paint("Files", ScanStyle::Dim, self.color),
                    secret_scanner_file_probe_summary(report)
                );
                println!(
                    "{rail}   {}  {}",
                    scan_paint("Warnings", ScanStyle::Dim, self.color),
                    pluralize(report.summary.errors, "warning", "warnings")
                );
            }
        }
        flush_secret_scanner_stdout()
    }
}

pub(crate) fn flush_secret_scanner_stdout() -> Result<(), String> {
    std::io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush scan output: {err}"))
}

pub(crate) fn flush_secret_scanner_stderr() -> Result<(), String> {
    std::io::stderr()
        .flush()
        .map_err(|err| format!("failed to flush scan warnings: {err}"))
}

pub(crate) fn scanner_wrapper_ui_enabled() -> bool {
    env::var(SCANNER_WRAPPER_UI_ENV).is_ok_and(|value| !value.is_empty() && value != "0")
}

pub(crate) fn print_secret_scanner_warning_line(error: &SecretScannerError, color: bool) {
    let source = scan_paint(&error.source, ScanStyle::Dim, color);
    match &error.path {
        Some(path) => eprintln!(
            "  {} {source} {} - {}",
            scan_paint("⚠", ScanStyle::Warning, color),
            scan_paint(path, ScanStyle::Path, color),
            error.message
        ),
        None => eprintln!(
            "  {} {source} - {}",
            scan_paint("⚠", ScanStyle::Warning, color),
            error.message
        ),
    }
}

pub(crate) fn print_wrapped_secret_scanner_warning_line(error: &SecretScannerError, color: bool) {
    let rail = scan_rail(ScanStyle::Warning, color);
    let source = scan_paint(&error.source, ScanStyle::Dim, color);
    match &error.path {
        Some(path) => println!(
            "{rail}   {} {source} {} - {}",
            scan_paint("!", ScanStyle::Warning, color),
            scan_paint(path, ScanStyle::Path, color),
            error.message
        ),
        None => println!(
            "{rail}   {} {source} - {}",
            scan_paint("!", ScanStyle::Warning, color),
            error.message
        ),
    }
}

pub(crate) fn print_scan_box(title: &str, lines: &[String], color: bool) {
    let width = scan_box_width(lines);
    println!(
        "{}",
        scan_paint(
            &format!(
                "╭─ {title} {}╮",
                "─".repeat(width.saturating_sub(title.len()))
            ),
            ScanStyle::Accent,
            color
        )
    );
    for line in lines {
        println!("│  {}", pad_scan_line(line, width));
    }
    println!(
        "{}",
        scan_paint(
            &format!("╰{}╯", "─".repeat(width + 3)),
            ScanStyle::Accent,
            color
        )
    );
}

pub(crate) fn scan_box_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| strip_ansi_width(line))
        .max()
        .unwrap_or(42)
        .clamp(42, 76)
}

pub(crate) fn pad_scan_line(line: &str, width: usize) -> String {
    let visible = strip_ansi_width(line);
    format!("{line}{} │", " ".repeat(width.saturating_sub(visible)))
}

pub(crate) fn strip_ansi_width(line: &str) -> usize {
    let mut width = 0;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }
    width
}

pub(crate) fn secret_scanner_finding_location(finding: &SecretScannerFinding) -> Option<String> {
    match (&finding.path, finding.line) {
        (Some(path), Some(line)) => Some(format!("{path}:{line}")),
        (Some(path), None) => Some(path.clone()),
        (None, _) => None,
    }
}

pub(crate) fn secret_scanner_scope_label(scope: SecretScannerScope) -> &'static str {
    match scope {
        SecretScannerScope::Full => "isotope detectors and file probes",
        SecretScannerScope::IsotopesOnly => "isotope detectors only",
    }
}

pub(crate) fn secret_scanner_file_probe_summary(report: &SecretScannerReport) -> String {
    match report.scope {
        SecretScannerScope::Full => pluralize(
            report.summary.scanned_files,
            "file scanned",
            "files scanned",
        ),
        SecretScannerScope::IsotopesOnly => "file probes skipped".to_string(),
    }
}

pub(crate) fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ScanStyle {
    Accent,
    Dim,
    Error,
    Heading,
    Path,
    Success,
    Warning,
}

pub(crate) fn scan_severity_style(finding: &SecretScannerFinding) -> ScanStyle {
    match finding.severity.as_str() {
        "critical" | "high" => ScanStyle::Error,
        _ => ScanStyle::Warning,
    }
}

pub(crate) fn scan_paint(text: &str, style: ScanStyle, color: bool) -> String {
    if !color {
        return text.to_string();
    }

    let code = match style {
        ScanStyle::Accent => "38;2;224;90;71",
        ScanStyle::Dim => "2",
        ScanStyle::Error => "31;1",
        ScanStyle::Heading => "1",
        ScanStyle::Path => "36",
        ScanStyle::Success => "32;1",
        ScanStyle::Warning => "33;1",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

pub(crate) fn scan_rail(style: ScanStyle, color: bool) -> String {
    scan_paint("│", style, color)
}

pub(crate) fn scan_stdout_is_rich() -> bool {
    env::var("CLICOLOR_FORCE").is_ok_and(|value| value != "0")
        || (std::io::stdout().is_terminal() && env::var("TERM").map_or(true, |term| term != "dumb"))
}

pub(crate) fn scan_stdout_supports_ansi() -> bool {
    output_supports_ansi(std::io::stdout().is_terminal())
}

pub(crate) fn output_supports_ansi(is_terminal: bool) -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    if env::var("CLICOLOR_FORCE").is_ok_and(|value| value != "0") {
        return true;
    }

    is_terminal && env::var("TERM").map_or(true, |term| term != "dumb")
}

pub(crate) struct SecretScanSkips {
    pub(crate) paths: HashSet<PathBuf>,
    pub(crate) cwd: Option<PathBuf>,
}

impl SecretScanSkips {
    pub(crate) fn new(root: Option<&Path>, skip_paths: &[PathBuf]) -> Self {
        let cwd = env::current_dir().ok().map(|path| normalize_path(&path));
        let raw_base = secret_scan_raw_skip_base(root);
        let mut paths = HashSet::new();

        for skip_path in skip_paths {
            if skip_path.is_absolute() {
                paths.insert(normalize_path(skip_path));
                continue;
            }

            let raw_skip_path = normalize_path(&raw_base.join(skip_path));
            paths.insert(raw_skip_path.clone());
            if !raw_skip_path.is_absolute()
                && let Some(cwd) = &cwd
            {
                paths.insert(normalize_path(&cwd.join(&raw_skip_path)));
            }
        }

        Self { paths, cwd }
    }

    pub(crate) fn should_skip(&self, path: &Path) -> bool {
        if self.paths.is_empty() {
            return false;
        }

        let normalized = normalize_path(path);
        if self.paths.contains(&normalized) {
            return true;
        }

        if normalized.is_absolute() {
            return false;
        }

        self.cwd
            .as_ref()
            .is_some_and(|cwd| self.paths.contains(&normalize_path(&cwd.join(normalized))))
    }
}

pub(crate) fn secret_scan_raw_skip_base(root: Option<&Path>) -> PathBuf {
    match root {
        Some(root) if root.is_dir() => root.to_path_buf(),
        Some(root) => root.parent().map(Path::to_path_buf).unwrap_or_default(),
        None => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

pub(crate) fn scan_secret_file_probes<F>(
    root: Option<&Path>,
    skip_paths: &[PathBuf],
    findings: &mut Vec<SecretScannerFinding>,
    errors: &mut Vec<SecretScannerError>,
    seen_findings: &mut HashSet<SecretScannerFinding>,
    seen_errors: &mut HashSet<SecretScannerError>,
    on_event: &mut F,
) -> Result<(usize, usize), String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    match root {
        Some(path) => scan_secret_file_probes_under_root(
            path,
            skip_paths,
            findings,
            errors,
            seen_findings,
            seen_errors,
            on_event,
        ),
        None => {
            let skips = SecretScanSkips::new(None, skip_paths);
            let mut scanned_files = 0;
            let mut file_probes = 0;
            for path in default_secret_scan_paths() {
                if skips.should_skip(&path) {
                    continue;
                }
                scan_secret_probe_path(
                    &path,
                    findings,
                    errors,
                    seen_findings,
                    seen_errors,
                    on_event,
                    &mut scanned_files,
                    &mut file_probes,
                )?;
            }
            Ok((scanned_files, file_probes))
        }
    }
}

pub(crate) fn scan_secret_file_probes_under_root<F>(
    root: &Path,
    skip_paths: &[PathBuf],
    findings: &mut Vec<SecretScannerFinding>,
    errors: &mut Vec<SecretScannerError>,
    seen_findings: &mut HashSet<SecretScannerFinding>,
    seen_errors: &mut HashSet<SecretScannerError>,
    on_event: &mut F,
) -> Result<(usize, usize), String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    if !root.exists() {
        return Err(format!("scan path does not exist: {}", root.display()));
    }
    let skips = SecretScanSkips::new(Some(root), skip_paths);
    if root.is_file() {
        if skips.should_skip(root) {
            return Ok((0, 0));
        }
        let mut scanned_files = 0;
        let mut file_probes = 0;
        scan_secret_probe_path(
            root,
            findings,
            errors,
            seen_findings,
            seen_errors,
            on_event,
            &mut scanned_files,
            &mut file_probes,
        )?;
        return Ok((scanned_files, file_probes));
    }
    if !root.is_dir() {
        return Err(format!(
            "scan path is not a file or directory: {}",
            root.display()
        ));
    }
    if skips.should_skip(root) {
        return Ok((0, 0));
    }
    fs::read_dir(root)
        .map_err(|err| format!("failed to read scan path {}: {err}", root.display()))?;

    let mut scanned_files = 0;
    let mut file_probes = 0;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !secret_scan_should_skip_entry(entry) && !skips.should_skip(entry.path())
        })
    {
        match entry {
            Ok(entry) if entry.file_type().is_file() => {
                scan_secret_probe_path(
                    entry.path(),
                    findings,
                    errors,
                    seen_findings,
                    seen_errors,
                    on_event,
                    &mut scanned_files,
                    &mut file_probes,
                )?;
            }
            Ok(_) => {}
            Err(err) => record_secret_scanner_error(
                errors,
                seen_errors,
                SecretScannerError {
                    source: "file-probe".to_string(),
                    path: err.path().map(|path| path.display().to_string()),
                    message: format!("failed to walk entry: {err}"),
                },
                on_event,
            )?,
        }
    }

    Ok((scanned_files, file_probes))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn scan_secret_probe_path<F>(
    path: &Path,
    findings: &mut Vec<SecretScannerFinding>,
    errors: &mut Vec<SecretScannerError>,
    seen_findings: &mut HashSet<SecretScannerFinding>,
    seen_errors: &mut HashSet<SecretScannerError>,
    on_event: &mut F,
    scanned_files: &mut usize,
    file_probes: &mut usize,
) -> Result<(), String>
where
    F: for<'a> FnMut(SecretScannerEvent<'a>) -> Result<(), String>,
{
    *file_probes += 1;
    match scan_secret_file(path) {
        Ok(file_findings) => {
            if path.is_file() {
                *scanned_files += 1;
            }
            for finding in file_findings {
                record_secret_scanner_finding(findings, seen_findings, finding, on_event)?;
            }
        }
        Err(err) => record_secret_scanner_error(
            errors,
            seen_errors,
            SecretScannerError {
                source: "file-probe".to_string(),
                path: Some(path.display().to_string()),
                message: err,
            },
            on_event,
        )?,
    }
    Ok(())
}

pub(crate) fn secret_scan_should_skip_entry(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    if matches!(
        entry.file_name().to_str(),
        Some(
            ".git"
                | ".hg"
                | ".svn"
                | ".codex-worktrees"
                | ".build"
                | ".next"
                | "target"
                | "dist"
                | "node_modules"
                | "Vendor"
                | "vendor"
                | ".cache"
                | "cache"
                | "artifacts"
                | "DerivedData"
        )
    ) {
        return true;
    }

    let path = entry.path().to_string_lossy();
    path.contains("/isotopes/") || path.contains("/radioisotopes/")
}

pub(crate) fn default_secret_scan_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        for relative in DEFAULT_SECRET_SCAN_CWD_FILES {
            paths.push(cwd.join(relative));
        }
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for relative in DEFAULT_SECRET_SCAN_HOME_FILES {
            paths.push(home.join(relative));
        }
    }

    paths.extend(shell_secret_candidate_paths(ShellSecretFlavor::Bash));
    paths.extend(shell_secret_candidate_paths(ShellSecretFlavor::Zsh));

    paths.sort();
    paths.dedup();
    paths
}

pub(crate) const DEFAULT_SECRET_SCAN_CWD_FILES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".npmrc",
    ".pypirc",
    ".netrc",
];

pub(crate) const DEFAULT_SECRET_SCAN_HOME_FILES: &[&str] = &[
    ".env",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".git-credentials",
    ".aws/credentials",
    ".kube/config",
    ".config/gh/hosts.yml",
];

pub(crate) const BASH_SECRET_SCAN_HOME_FILES: &[&str] =
    &[".bashrc", ".bash_profile", ".bash_login", ".profile"];

pub(crate) const ZSH_SECRET_SCAN_HOME_FILES: &[&str] =
    &[".zshenv", ".zprofile", ".zshrc", ".zlogin", ".zlogout"];

pub(crate) const SECRET_SCAN_MAX_FILE_BYTES: u64 = 1024 * 1024;
pub(crate) const AUTOMIC_VAULT_DOTENV_ENCRYPTED_PREFIX: &str = "encrypted:";

pub(crate) fn bash_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secret_insecurity_reasons(ShellSecretFlavor::Bash)
}

pub(crate) fn zsh_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secret_insecurity_reasons(ShellSecretFlavor::Zsh)
}

pub(crate) fn shell_secret_insecurity_reasons(
    shell: ShellSecretFlavor,
) -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in shell_secret_candidate_paths(shell) {
        for finding in scan_secret_file(&path)? {
            let location = secret_scanner_finding_location(&finding)
                .unwrap_or_else(|| path.display().to_string());
            reasons.push(format!(
                "{} startup file contains plaintext-looking credential assignment: {} ({})",
                shell.display_name(),
                location,
                finding.message
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

pub(crate) fn shell_secret_candidate_paths(shell: ShellSecretFlavor) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match shell {
        ShellSecretFlavor::Bash => {
            if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
                for relative in BASH_SECRET_SCAN_HOME_FILES {
                    paths.push(home.join(relative));
                }
            }
            if let Some(path) = env::var_os("BASH_ENV").filter(|value| !value.is_empty()) {
                paths.push(PathBuf::from(path));
            }
        }
        ShellSecretFlavor::Zsh => {
            if let Some(base) = env::var_os("ZDOTDIR")
                .filter(|value| !value.is_empty())
                .or_else(|| env::var_os("HOME"))
                .map(PathBuf::from)
            {
                for relative in ZSH_SECRET_SCAN_HOME_FILES {
                    paths.push(base.join(relative));
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

pub(crate) fn scan_secret_file(path: &Path) -> Result<Vec<SecretScannerFinding>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to stat {}: {err}", path.display())),
    };
    if !metadata.is_file() || metadata.len() > SECRET_SCAN_MAX_FILE_BYTES {
        return Ok(Vec::new());
    }

    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if bytes.contains(&0) {
        return Ok(Vec::new());
    }
    let Ok(contents) = String::from_utf8(bytes) else {
        return Ok(Vec::new());
    };

    let mut findings = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if let Some(finding) = scan_secret_line(path, index + 1, line) {
            findings.push(finding);
        }
    }
    Ok(findings)
}

pub(crate) fn scan_secret_line(
    path: &Path,
    line_number: usize,
    line: &str,
) -> Option<SecretScannerFinding> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with(';')
        || trimmed.starts_with("//")
        || secret_line_looks_like_source_string_fixture(path, trimmed)
    {
        return None;
    }

    if trimmed.contains("BEGIN ") && trimmed.contains("PRIVATE KEY") {
        if secret_private_key_line_is_fixture(path, trimmed) {
            return None;
        }
        return Some(secret_file_finding(
            path,
            line_number,
            "private-key",
            "critical",
            "Private key material appears in a readable file",
        ));
    }

    let Some(assignment) = parse_secret_assignment(trimmed) else {
        if secret_line_contains_standalone_token_literal(path, trimmed) {
            return Some(secret_file_finding(
                path,
                line_number,
                "token-literal",
                "high",
                "Known token-shaped value appears in a readable file",
            ));
        }
        return None;
    };
    if secret_assignment_looks_like_source_code(&assignment) {
        return None;
    }

    let value = normalized_secret_value(assignment.value);
    if secret_path_looks_like_env_file(path) && secret_value_looks_like_encrypted_dotenv(value) {
        return None;
    }
    let key_is_sensitive = secret_key_name_is_sensitive(assignment.key);
    let value_has_known_shape = secret_value_has_known_secret_shape(value);
    let value_has_strong_shape = secret_value_has_high_entropy_shape(value);
    let credential_context = secret_path_looks_like_credential_file(path);
    let source_context = secret_path_looks_like_source_file(path);
    let value_is_real = value_has_known_shape
        || (source_context
            && key_is_sensitive
            && secret_assignment_value_is_literal(assignment.value)
            && value_has_strong_shape)
        || (!source_context
            && key_is_sensitive
            && credential_context
            && (secret_value_is_real(value) || secret_sensitive_env_value_is_real(value)))
        || (!source_context && key_is_sensitive && value_has_strong_shape);
    if !value_is_real || secret_value_is_test_fixture(path, value) {
        return None;
    }
    if secret_path_looks_like_test_fixture(path) && key_is_sensitive {
        return None;
    }

    if key_is_sensitive {
        let key = shell_assignment_key_name(assignment.key).trim();
        return Some(secret_file_finding(
            path,
            line_number,
            "secret-assignment",
            "high",
            &format!("Plaintext-looking credential assigned to {key}"),
        ));
    }

    if value_has_known_shape {
        return Some(secret_file_finding(
            path,
            line_number,
            "token-literal",
            "high",
            "Known token-shaped value appears in a readable file",
        ));
    }

    None
}

pub(crate) fn secret_file_finding(
    path: &Path,
    line: usize,
    kind: &str,
    severity: &str,
    message: &str,
) -> SecretScannerFinding {
    SecretScannerFinding {
        source: secret_file_probe_source(path).to_string(),
        kind: kind.to_string(),
        severity: severity.to_string(),
        path: Some(path.display().to_string()),
        line: Some(line),
        message: message.to_string(),
    }
}

pub(crate) fn secret_file_probe_source(path: &Path) -> &'static str {
    secret_shell_startup_file_flavor(path).map_or("file-probe", ShellSecretFlavor::source_label)
}

pub(crate) struct SecretAssignment<'a> {
    pub(crate) key: &'a str,
    pub(crate) value: &'a str,
    pub(crate) separator: SecretAssignmentSeparator,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecretAssignmentSeparator {
    Equals,
    Colon,
}

pub(crate) fn parse_secret_assignment(line: &str) -> Option<SecretAssignment<'_>> {
    let line = line.strip_prefix("- ").unwrap_or(line);
    let equals = find_secret_assignment_equals(line);
    let colon = find_secret_assignment_colon(line);
    match (equals, colon) {
        (Some(eq), Some(colon)) if eq < colon => Some(SecretAssignment {
            key: &line[..eq],
            value: &line[eq + 1..],
            separator: SecretAssignmentSeparator::Equals,
        }),
        (Some(_), Some(colon)) => Some(SecretAssignment {
            key: &line[..colon],
            value: &line[colon + 1..],
            separator: SecretAssignmentSeparator::Colon,
        }),
        (Some(eq), None) => Some(SecretAssignment {
            key: &line[..eq],
            value: &line[eq + 1..],
            separator: SecretAssignmentSeparator::Equals,
        }),
        (None, Some(colon)) => Some(SecretAssignment {
            key: &line[..colon],
            value: &line[colon + 1..],
            separator: SecretAssignmentSeparator::Colon,
        }),
        (None, None) => None,
    }
}

pub(crate) fn find_secret_assignment_equals(line: &str) -> Option<usize> {
    for (index, ch) in line.char_indices() {
        if ch != '=' {
            continue;
        }
        let previous = line[..index].chars().next_back();
        let next = line[index + ch.len_utf8()..].chars().next();
        if previous.is_some_and(|ch| matches!(ch, '!' | '<' | '>' | '='))
            || next.is_some_and(|ch| matches!(ch, '=' | '>'))
        {
            continue;
        }
        return Some(index);
    }
    None
}

pub(crate) fn find_secret_assignment_colon(line: &str) -> Option<usize> {
    for (index, ch) in line.char_indices() {
        if ch != ':' {
            continue;
        }
        let previous = line[..index].chars().next_back();
        let next = line[index + ch.len_utf8()..].chars().next();
        if previous.is_some_and(|ch| ch == ':') || next.is_some_and(|ch| ch == ':' || ch == '/') {
            continue;
        }
        return Some(index);
    }
    None
}

pub(crate) fn secret_assignment_looks_like_source_code(assignment: &SecretAssignment<'_>) -> bool {
    let key = assignment.key.trim();
    let value = assignment.value.trim();
    if key.starts_with("case ") {
        return true;
    }

    if assignment.separator == SecretAssignmentSeparator::Colon
        && (key.starts_with("let ")
            || key.starts_with("var ")
            || key.starts_with("const ")
            || key.starts_with("pub "))
    {
        return true;
    }

    if key.contains('(')
        || secret_key_looks_like_source_code(key)
        || secret_key_looks_like_source_reference(key)
        || secret_key_name_is_noncredential_metadata(key)
        || secret_key_looks_like_freeform_text(key)
    {
        return true;
    }

    if assignment.separator == SecretAssignmentSeparator::Colon
        && (key.contains('(')
            || secret_key_looks_like_freeform_text(key)
            || secret_value_looks_like_freeform_text(value)
            || secret_value_looks_like_type_annotation(value))
    {
        return true;
    }

    if secret_quoted_value_looks_like_source_expression(value) {
        return true;
    }

    secret_unquoted_value_looks_like_source_reference(value)
}

pub(crate) fn secret_key_looks_like_source_code(key: &str) -> bool {
    let trimmed = key.trim_start();
    trimmed.starts_with("type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("protocol ")
        || trimmed.starts_with("union ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("export function ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("if(")
        || trimmed.starts_with('.')
        || trimmed.starts_with("WHERE ")
        || trimmed.starts_with("where ")
}

pub(crate) fn secret_key_looks_like_source_reference(key: &str) -> bool {
    let key = key.trim();
    if key.starts_with('"') || key.starts_with('\'') {
        return false;
    }
    key.contains("->")
        || key.contains("::")
        || key.contains(',')
        || (key.contains('[') && key.contains(']'))
        || (key.contains('.') && key.chars().all(source_key_reference_char))
}

pub(crate) fn source_key_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

pub(crate) fn secret_key_looks_like_freeform_text(key: &str) -> bool {
    let key = key.trim();
    if key.starts_with("export ")
        || key.starts_with("readonly ")
        || key.starts_with("declare ")
        || key.starts_with("typeset ")
        || key.starts_with("local ")
        || key.starts_with("let ")
        || key.starts_with("var ")
        || key.starts_with("const ")
    {
        return false;
    }
    let key = key.trim_matches('"').trim_matches('\'').trim_matches('`');
    if key.starts_with('/') || key.ends_with('/') {
        return true;
    }
    key.contains(',') || key.split_whitespace().count() > 1
}

pub(crate) fn secret_value_looks_like_freeform_text(value: &str) -> bool {
    if secret_raw_value_is_quoted(value) {
        return false;
    }
    let value = value
        .split_once('#')
        .map_or(value, |(before_comment, _)| before_comment)
        .trim();
    value.split_whitespace().count() >= 4 && value.chars().any(char::is_alphabetic)
}

pub(crate) fn secret_value_looks_like_type_annotation(value: &str) -> bool {
    let mut words = value.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    let has_more_words =
        words.any(|word| !word.chars().all(|ch| matches!(ch, '{' | '}' | ',' | ';')));
    let first = first.trim_matches(|ch: char| {
        matches!(
            ch,
            '?' | '!' | ')' | '(' | '[' | ']' | '<' | '>' | ',' | ';' | '{' | '}'
        )
    });
    let first = first
        .trim_start_matches('&')
        .trim_start_matches('\'')
        .trim_end_matches('\'');
    if first.is_empty() || first.starts_with('"') || first.starts_with('\'') {
        return false;
    }
    if matches!(first, "Bearer" | "Basic") {
        return false;
    }
    if first.chars().next().is_some_and(char::is_uppercase)
        && value.contains('=')
        && value.contains("nil")
    {
        return true;
    }
    matches!(
        first,
        "String"
            | "Bool"
            | "Boolean"
            | "Int"
            | "Integer"
            | "Double"
            | "Float"
            | "Date"
            | "Data"
            | "URL"
            | "UUID"
            | "static"
            | "str"
            | "string"
            | "bytes"
            | "bool"
            | "boolean"
            | "number"
            | "object"
            | "array"
    ) || (first.chars().next().is_some_and(char::is_uppercase)
        && (!has_more_words || first.contains('<')))
}

pub(crate) fn secret_unquoted_value_looks_like_source_reference(value: &str) -> bool {
    if secret_raw_value_is_quoted(value) {
        return false;
    }
    let value = value.trim().trim_end_matches([',', ';']);
    if value.is_empty() {
        return false;
    }
    if secret_unquoted_value_looks_like_placeholder_or_pattern(value)
        || secret_unquoted_value_looks_like_source_expression(value)
    {
        return true;
    }
    if value.starts_with('.') || value.contains('(') || value.contains("->") || value.contains("::")
    {
        return true;
    }
    if secret_value_has_known_token_shape(value) || secret_value_looks_like_jwt(value) {
        return false;
    }
    if value.contains('.') && value.chars().all(source_reference_char) {
        return true;
    }
    source_identifier(value).is_some_and(|identifier| {
        !identifier.chars().any(|ch| ch.is_ascii_digit())
            && identifier.chars().any(char::is_uppercase)
            && secret_key_name_is_sensitive(identifier)
    })
}

pub(crate) fn secret_unquoted_value_looks_like_placeholder_or_pattern(value: &str) -> bool {
    value == "?"
        || value.starts_with("//")
        || value.starts_with("{{")
        || value.starts_with("<%")
        || value.starts_with('{')
        || (value.starts_with('{') && value.ends_with('}'))
        || (value.starts_with('/') && value.ends_with('/'))
}

pub(crate) fn secret_unquoted_value_looks_like_source_expression(value: &str) -> bool {
    value.starts_with("f\"")
        || value.starts_with("f'")
        || value.starts_with('&')
        || value.starts_with('!')
        || value.starts_with("if ")
        || value.starts_with("self.")
        || value.starts_with("match ")
        || value.starts_with("process.env.")
        || value.starts_with("typeof ")
        || value.starts_with("ReturnType<")
        || value.contains(" as ")
        || value.contains(" + ")
        || value.contains(" - ")
        || value.contains(" ?? ")
        || value.contains("\\(")
        || value.contains(" * ")
        || value.contains(" ? ")
        || value.contains(" : ")
        || value.contains(" && ")
        || value.contains(" || ")
        || value.contains(" === ")
        || value.contains(" !== ")
        || value.contains(" == ")
        || value.contains(" != ")
        || value.contains(" <= ")
        || value.contains(" >= ")
        || (value.contains('[') && value.contains(']'))
        || value.ends_with('?')
        || value.ends_with('{')
}

pub(crate) fn secret_quoted_value_looks_like_source_expression(value: &str) -> bool {
    secret_raw_value_is_quoted(value)
        && (value.contains("\\(")
            || value.contains(".into()")
            || value.contains(".to_owned()")
            || value.contains(".to_string()")
            || value.contains(".spanned("))
}

pub(crate) fn secret_raw_value_is_quoted(value: &str) -> bool {
    let value = value.trim();
    value.starts_with('"') || value.starts_with('\'')
}

pub(crate) fn secret_assignment_value_is_literal(value: &str) -> bool {
    secret_raw_value_is_quoted(value)
}

pub(crate) fn source_identifier(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(source_reference_char) {
        return None;
    }
    Some(value.rsplit('.').next().unwrap_or(value))
}

pub(crate) fn source_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

pub(crate) fn normalized_secret_value(value: &str) -> &str {
    let value = value
        .trim()
        .trim_end_matches([',', ';', '}', ']', ')', ':'])
        .trim();
    let value = if secret_raw_value_is_quoted(value) {
        value
    } else {
        value
            .split_once('#')
            .map_or(value, |(before_comment, _)| before_comment)
            .trim()
    };
    value.trim_matches('"').trim_matches('\'').trim()
}

pub(crate) fn secret_key_name_is_sensitive(key: &str) -> bool {
    if secret_key_name_is_noncredential_metadata(key) {
        return false;
    }

    let key = normalized_secret_key_name(key);
    key == "token"
        || key == "password"
        || key == "passwd"
        || key == "authorization"
        || key.contains("token")
        || key.contains("api_key")
        || key.contains("apikey")
        || key.contains("access_key")
        || key.contains("secret")
        || key.contains("auth_token")
        || key.contains("private_key")
        || key.contains("refresh_token")
        || key.contains("id_token")
        || key.contains("client_secret")
}

pub(crate) fn secret_key_name_is_noncredential_metadata(key: &str) -> bool {
    let key = normalized_secret_key_name(key);
    let compact = key.replace('_', "");
    let mentions_secretish_word = compact.contains("token")
        || compact.contains("secret")
        || compact.contains("password")
        || compact.contains("key");

    mentions_secretish_word
        && (compact.ends_with("type")
            || compact.ends_with("types")
            || compact.ends_with("name")
            || compact.ends_with("names")
            || compact.ends_with("prefix")
            || compact.ends_with("suffix")
            || compact.ends_with("service")
            || compact.ends_with("hash")
            || compact.ends_with("label")
            || compact.ends_with("labels")
            || compact.ends_with("pattern")
            || compact.ends_with("patterns")
            || compact.ends_with("class")
            || compact.ends_with("size")
            || compact.ends_with("margin")
            || compact.ends_with("padding")
            || compact.ends_with("width")
            || compact.ends_with("threshold")
            || compact.ends_with("version")
            || compact.ends_with("color")
            || compact.ends_with("dir")
            || compact.ends_with("path")
            || compact.ends_with("file")
            || compact.ends_with("url")
            || compact.ends_with("uri"))
}

pub(crate) fn normalized_secret_key_name(key: &str) -> String {
    shell_assignment_key_name(key)
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
        .chars()
        .map(|ch| match ch {
            '-' | '.' | ' ' => '_',
            _ => ch,
        })
        .collect()
}

pub(crate) fn shell_assignment_key_name(key: &str) -> &str {
    let key = key.trim();
    let Some((command, mut rest)) = shell_word(key) else {
        return key;
    };
    if !matches!(
        command,
        "export" | "readonly" | "declare" | "typeset" | "local"
    ) {
        return key;
    }

    while let Some((word, after_word)) = shell_word(rest) {
        if !word.starts_with('-') {
            return if after_word.trim().is_empty() && shell_assignment_word_looks_like_name(word) {
                word
            } else {
                key
            };
        }
        rest = after_word;
    }

    key
}

pub(crate) fn shell_word(value: &str) -> Option<(&str, &str)> {
    let value = value.trim_start();
    if value.is_empty() {
        return None;
    }
    let end = value
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(value.len());
    Some((&value[..end], &value[end..]))
}

pub(crate) fn shell_assignment_word_looks_like_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_alphanumeric())
}

pub(crate) fn secret_value_is_real(value: &str) -> bool {
    if secret_value_is_obviously_not_real(value) {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    if secret_value_has_known_secret_shape(value) {
        return true;
    }
    if lower == "secret_secret" {
        return true;
    }

    !secret_value_looks_like_package_or_label(value)
}

pub(crate) fn secret_sensitive_env_value_is_real(value: &str) -> bool {
    if value.len() < 12 || secret_value_is_obviously_not_real(value) {
        return false;
    }
    let has_alpha = value.chars().any(|ch| ch.is_ascii_alphabetic());
    let has_digit = value.chars().any(|ch| ch.is_ascii_digit());
    (value.len() >= 16 && has_alpha) || (value.len() >= 12 && has_alpha && has_digit)
}

pub(crate) fn secret_value_looks_like_encrypted_dotenv(value: &str) -> bool {
    let Some(payload) = value.strip_prefix(AUTOMIC_VAULT_DOTENV_ENCRYPTED_PREFIX) else {
        return false;
    };
    !payload.is_empty()
        && payload
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '='))
}

pub(crate) fn secret_value_has_known_secret_shape(value: &str) -> bool {
    secret_value_has_known_token_shape(value)
        || secret_value_looks_like_posthog_project_key(value)
        || secret_value_looks_like_jwt(value)
}

pub(crate) fn secret_value_has_high_entropy_shape(value: &str) -> bool {
    if value.len() < 20
        || secret_value_is_obviously_not_real(value)
        || secret_value_looks_like_package_or_label(value)
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }

    let mut has_lower = false;
    let mut has_upper = false;
    let mut has_digit = false;
    let mut has_symbol = false;
    for ch in value.chars() {
        if ch.is_ascii_lowercase() {
            has_lower = true;
        } else if ch.is_ascii_uppercase() {
            has_upper = true;
        } else if ch.is_ascii_digit() {
            has_digit = true;
        } else if matches!(ch, '_' | '-' | '.' | '+' | '/' | '=') {
            has_symbol = true;
        } else {
            return false;
        }
    }

    let category_count = usize::from(has_lower)
        + usize::from(has_upper)
        + usize::from(has_digit)
        + usize::from(has_symbol);
    let has_alpha = has_lower || has_upper;
    has_alpha && has_digit && ((value.len() >= 24 && category_count >= 3) || value.len() >= 32)
}

pub(crate) fn secret_value_is_obviously_not_real(value: &str) -> bool {
    if value.len() < 6 || value.contains("${") {
        return true;
    }

    let lower = value.to_ascii_lowercase();
    if lower.starts_with("options:") {
        return true;
    }
    let comparable =
        lower.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && !matches!(ch, '_' | '-'));
    if matches!(
        comparable,
        "secret"
            | "password"
            | "token"
            | "example"
            | "changeme"
            | "change_me"
            | "replace_me"
            | "redacted"
            | "access_token"
            | "refresh_token"
            | "id_token"
            | "client_secret"
            | "api_key"
            | "none"
            | "null"
            | "true"
            | "false"
            | "string"
            | "bytes"
            | "write"
            | "read"
            | "hashed"
            | "nullptr"
            | "nil"
    ) {
        return true;
    }

    lower.contains("example")
        || lower.contains("placeholder")
        || lower.contains("your_")
        || lower.contains("your-")
        || lower.contains("...")
        || lower.contains("***")
        || lower.contains("fake")
        || value.contains('…')
        || lower.contains("\\n")
        || lower.contains("base64url")
        || value.starts_with('$')
        || (lower.starts_with("env(") && lower.ends_with(')'))
        || lower.contains(".into()")
        || lower.contains(".to_string()")
        || lower.contains(".spanned(")
        || lower.contains("getenv(")
        || (value.contains('<') && value.contains('>'))
        || lower.chars().all(|ch| ch.is_ascii_digit())
        || lower.chars().all(|ch| ch == 'x' || ch == '*')
        || (value.starts_with('{') && value.ends_with('}'))
        || value.starts_with("{{")
        || value.starts_with('<')
        || (value.starts_with('%') && value.ends_with('%'))
        || secret_value_looks_like_file_path(value)
        || secret_value_looks_like_public_url(value)
        || secret_value_looks_like_version_requirement(value)
}

pub(crate) fn secret_value_is_test_fixture(path: &Path, value: &str) -> bool {
    if secret_path_looks_like_reference_fixture(path) {
        return true;
    }

    if !secret_path_looks_like_test_fixture(path) {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    if secret_value_has_known_token_shape(value) || secret_value_looks_like_jwt(value) {
        return true;
    }
    if lower.contains("token") || lower.contains("secret") {
        return true;
    }

    matches!(
        lower.as_str(),
        "password123"
            | "handoff-token"
            | "test-token"
            | "test-password"
            | "polar_test_token"
            | "polar_webhook_secret"
    )
}

pub(crate) fn secret_path_looks_like_test_fixture(path: &Path) -> bool {
    let path = path.to_string_lossy().to_ascii_lowercase();
    path.contains("/test/")
        || path.contains("/tests/")
        || path.contains("\\test\\")
        || path.contains("\\tests\\")
        || path.contains(".test.")
        || path.contains(".spec.")
        || path.contains("_test.")
        || path.contains("_tests.")
}

pub(crate) fn secret_path_looks_like_reference_fixture(path: &Path) -> bool {
    let path = path.to_string_lossy().to_ascii_lowercase();
    path.contains("/testdata/")
        || path.contains("/fixtures/")
        || path.contains("/fixture/")
        || path.contains("/examples/")
        || path.contains("/example/")
        || path.contains("/samples/")
        || path.contains("/sample/")
        || path.contains("/cavs_samples/")
        || path.contains("/wycheproof/")
        || path.contains("/doc/")
        || path.contains("/docs/")
        || path.contains("/share/man/")
        || path.contains("/share/info/")
        || path.contains("/man/man")
        || path.contains("/resources/bundled/")
        || path.ends_with(".sample")
        || path.ends_with(".strings")
}

pub(crate) fn secret_path_looks_like_credential_file(path: &Path) -> bool {
    if secret_path_looks_like_env_file(path) {
        return true;
    }
    if secret_shell_startup_file_flavor(path).is_some() {
        return true;
    }

    let normalized_path = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if normalized_path.ends_with("/.aws/credentials")
        || normalized_path.ends_with("/.kube/config")
        || normalized_path.ends_with("/.config/gh/hosts.yml")
    {
        return true;
    }

    matches!(
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(".npmrc" | ".pypirc" | ".netrc" | ".git-credentials")
    )
}

pub(crate) fn secret_path_looks_like_env_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();
    file_name == ".env"
        || file_name == ".envrc"
        || file_name.starts_with(".env.")
        || file_name.ends_with(".env")
        || file_name.contains(".env.")
}

pub(crate) fn secret_shell_startup_file_flavor(path: &Path) -> Option<ShellSecretFlavor> {
    if let Some(bash_env) = env::var_os("BASH_ENV")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        && secret_paths_match(path, &bash_env)
    {
        return Some(ShellSecretFlavor::Bash);
    }

    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    match file_name.as_str() {
        ".bashrc" | ".bash_profile" | ".bash_login" | ".profile" => Some(ShellSecretFlavor::Bash),
        ".zshenv" | ".zprofile" | ".zshrc" | ".zlogin" | ".zlogout" => Some(ShellSecretFlavor::Zsh),
        _ => None,
    }
}

pub(crate) fn secret_paths_match(left: &Path, right: &Path) -> bool {
    normalize_path(left) == normalize_path(right)
}

pub(crate) fn secret_private_key_line_is_fixture(path: &Path, line: &str) -> bool {
    secret_path_looks_like_reference_fixture(path)
        || (secret_path_looks_like_source_file(path) && !line.starts_with("-----BEGIN "))
}

pub(crate) fn secret_path_looks_like_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "c" | "cc"
                | "cpp"
                | "cxx"
                | "h"
                | "hh"
                | "hpp"
                | "hxx"
                | "go"
                | "rs"
                | "swift"
                | "js"
                | "jsx"
                | "sh"
                | "bash"
                | "zsh"
                | "ts"
                | "tsx"
                | "py"
                | "rb"
                | "pm"
                | "erl"
                | "hrl"
        )
    )
}

pub(crate) fn secret_line_looks_like_source_string_fixture(path: &Path, line: &str) -> bool {
    if !secret_path_looks_like_source_file(path) {
        return false;
    }
    let line = line.trim_start();
    (line.starts_with('"')
        || (line.starts_with('r') && line.contains("#\""))
        || line.starts_with("r\"")
        || line.starts_with("br#\""))
        && (line.contains('=') || line.contains(':'))
}

pub(crate) fn secret_line_contains_standalone_token_literal(path: &Path, line: &str) -> bool {
    if secret_path_looks_like_test_fixture(path) || secret_path_looks_like_reference_fixture(path) {
        return false;
    }
    if secret_path_looks_like_source_file(path) {
        return secret_line_contains_quoted_secret_literal(line);
    }
    line.split(|ch: char| !token_shape_char(ch))
        .any(secret_value_has_known_secret_shape)
}

pub(crate) fn secret_line_contains_quoted_secret_literal(line: &str) -> bool {
    let mut chars = line.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if !matches!(ch, '"' | '\'') {
            continue;
        }

        let quote = ch;
        let mut escaped = false;
        let start = chars.peek().map_or(line.len(), |(index, _)| *index);
        for (index, next) in chars.by_ref() {
            if escaped {
                escaped = false;
                continue;
            }
            if next == '\\' {
                escaped = true;
                continue;
            }
            if next == quote {
                let value = &line[start..index];
                if secret_value_has_known_secret_shape(value) {
                    return true;
                }
                break;
            }
        }
    }
    false
}

pub(crate) fn secret_value_looks_like_file_path(value: &str) -> bool {
    value.starts_with("~/")
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with('/')
        || value.ends_with(".pem")
        || value.ends_with(".key")
}

pub(crate) fn secret_value_looks_like_public_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !value.contains('@')
        && !lower.contains("token=")
        && !lower.contains("access_token=")
        && !lower.contains("api_key=")
        && !lower.contains("apikey=")
        && !lower.contains("secret=")
}

pub(crate) fn secret_value_looks_like_version_requirement(value: &str) -> bool {
    value.contains('.')
        && value.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '.' | '^' | '~' | '*' | '|' | '&' | '<' | '>' | '=' | '!' | ' ' | '-'
                )
        })
}

pub(crate) fn secret_value_looks_like_package_or_label(value: &str) -> bool {
    if value.len() > 48 {
        return false;
    }
    let mut has_alpha = false;
    let mut has_upper = false;
    let mut has_digit = false;
    let mut has_space = false;
    for ch in value.chars() {
        if ch.is_ascii_alphabetic() {
            has_alpha = true;
            has_upper |= ch.is_ascii_uppercase();
            continue;
        }
        if ch.is_ascii_digit() {
            has_digit = true;
            continue;
        }
        if ch.is_ascii_whitespace() {
            has_space = true;
            continue;
        }
        if matches!(ch, '_' | '-' | '.' | '/' | ':') {
            continue;
        }
        return false;
    }
    has_alpha && (!has_upper || !has_digit) && (!has_space || has_upper)
}

pub(crate) fn secret_value_looks_like_jwt(value: &str) -> bool {
    if value.len() < 80 {
        return false;
    }
    let mut parts = value.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    let Some(third) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && [first, second, third]
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(base64_url_char))
}

pub(crate) fn base64_url_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '=')
}

pub(crate) fn secret_value_has_known_token_shape(value: &str) -> bool {
    let value = value.trim();
    if !value.chars().all(token_shape_char) {
        return false;
    }
    (value.starts_with("ghp_") && value.len() > 20)
        || (value.starts_with("gho_") && value.len() > 20)
        || (value.starts_with("ghs_") && value.len() > 20)
        || (value.starts_with("github_pat_") && value.len() > 30)
        || (value.starts_with("glpat-") && value.len() > 20)
        || (value.starts_with("xoxb-") && value.len() > 20)
        || (value.starts_with("xoxp-") && value.len() > 20)
        || (value.starts_with("sk_live_") && value.len() > 20)
        || (value.starts_with("npm_") && value.len() > 12)
        || (value.starts_with("sk-") && value.len() > 20)
        || (value.starts_with("xai-") && value.len() > 20)
        || (value.starts_with("AKIA") && value.len() >= 16)
}

pub(crate) fn secret_value_looks_like_posthog_project_key(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("phc_") && value.len() > 20 && value.chars().all(token_shape_char)
}

pub(crate) fn token_shape_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}
