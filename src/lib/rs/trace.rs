use super::*;

const TRACE_SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec";
const MAX_TRACE_SCRIPT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceCurlPipe {
    url: String,
    interpreter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceFetchedScript {
    url: String,
    interpreter: String,
    body: String,
    truncated: bool,
}

pub(crate) fn run_trace(invocation: &Invocation, mut args: env::ArgsOs) -> Result<(), String> {
    let request = match parse_trace_request(invocation, &mut args)? {
        Some(request) => request,
        None => return Ok(()),
    };

    let report = run_trace_request(&request)?;
    match request.output {
        OutputMode::Human => print_trace_report(&report),
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string(&report)
                    .map_err(|err| format!("failed to serialize trace report: {err}"))?
            );
        }
        OutputMode::Jsonl => unreachable!("trace parser does not accept jsonl output"),
    }
    Ok(())
}

pub(crate) fn parse_trace_request(
    invocation: &Invocation,
    args: &mut env::ArgsOs,
) -> Result<Option<TraceRequest>, String> {
    parse_trace_request_from_iter(invocation, args)
}

pub(crate) fn parse_trace_request_from_iter<I>(
    invocation: &Invocation,
    args: I,
) -> Result<Option<TraceRequest>, String>
where
    I: Iterator<Item = OsString>,
{
    let mut agent = TraceAgent::Auto;
    let mut command = None;
    let mut output = OutputMode::Human;
    let mut pending_agent = false;

    for arg in args {
        if pending_agent {
            agent = parse_trace_agent(&arg)?;
            pending_agent = false;
            continue;
        }

        if is_help_flag(&arg) {
            print_trace_usage(&invocation.name);
            return Ok(None);
        }

        if is_version_flag(&arg) {
            println!("{} {}", invocation.name, env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }

        if is_json_flag(&arg) {
            output = OutputMode::Json;
            continue;
        }

        match arg.to_str() {
            Some("--agent") => pending_agent = true,
            Some("--jsonl") => return Err("trace does not support --jsonl".to_string()),
            Some(value) if value.starts_with('-') => {
                return Err(format!("unknown argument '{value}'"));
            }
            Some(value) => {
                if command.is_some() {
                    return Err("supports a single shell one-liner".to_string());
                }
                if value.trim().is_empty() {
                    return Err("empty shell one-liner".to_string());
                }
                command = Some(value.to_string());
            }
            None => return Err("shell one-liner must be valid UTF-8".to_string()),
        }
    }

    if pending_agent {
        return Err("missing value for --agent".to_string());
    }

    let Some(command) = command else {
        print_trace_usage(&invocation.name);
        return Err("missing shell one-liner".to_string());
    };

    Ok(Some(TraceRequest {
        command,
        agent,
        output,
    }))
}

fn parse_trace_agent(arg: &OsString) -> Result<TraceAgent, String> {
    match arg.to_str() {
        Some("codex") => Ok(TraceAgent::Codex),
        Some("claude") => Ok(TraceAgent::Claude),
        Some(value) => Err(format!("unknown trace agent '{value}'")),
        None => Err("trace agent must be valid UTF-8".to_string()),
    }
}

fn run_trace_request(request: &TraceRequest) -> Result<TraceReport, String> {
    let resolved = resolve_trace_agent(request.agent)?;
    let fetched_script = fetch_trace_script_for_command(&request.command)?;
    let output = invoke_trace_agent(resolved, &request.command, fetched_script.as_ref())?;
    let parsed = parse_trace_agent_output(&output)?;
    let fetched_script_was_provided = fetched_script.is_some();
    Ok(TraceReport {
        command: request.command.clone(),
        agent: resolved.name().to_string(),
        steps: normalize_trace_steps(parsed.steps, fetched_script_was_provided),
    })
}

fn resolve_trace_agent(agent: TraceAgent) -> Result<TraceAgent, String> {
    match agent {
        TraceAgent::Auto => {
            if executable_on_path("codex").is_some() {
                return Ok(TraceAgent::Codex);
            }
            if executable_on_path("claude").is_some() {
                return Ok(TraceAgent::Claude);
            }
            Err("no supported trace agent found on PATH (expected codex or claude)".to_string())
        }
        TraceAgent::Codex => executable_on_path("codex")
            .map(|_| TraceAgent::Codex)
            .ok_or_else(|| "trace agent 'codex' not found on PATH".to_string()),
        TraceAgent::Claude => executable_on_path("claude")
            .map(|_| TraceAgent::Claude)
            .ok_or_else(|| "trace agent 'claude' not found on PATH".to_string()),
    }
}

impl TraceAgent {
    fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

fn executable_on_path(tool: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for root in env::split_paths(&paths) {
        let candidate = root.join(tool);
        if is_trace_agent_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn fetch_trace_script_for_command(command: &str) -> Result<Option<TraceFetchedScript>, String> {
    let Some(pipe) = parse_simple_curl_shell_pipe(command) else {
        return Ok(None);
    };
    let (body, truncated) = download_trace_script(&pipe.url)?;
    Ok(Some(TraceFetchedScript {
        url: pipe.url,
        interpreter: pipe.interpreter,
        body,
        truncated,
    }))
}

fn parse_simple_curl_shell_pipe(command: &str) -> Option<TraceCurlPipe> {
    let tokens = shell_words_for_trace(command)?;
    let pipe_index = tokens.iter().position(|token| token == "|")?;
    if tokens[pipe_index + 1..].iter().any(|token| token == "|") {
        return None;
    }
    let left = &tokens[..pipe_index];
    let right = &tokens[pipe_index + 1..];
    if left.first().map(String::as_str) != Some("curl") || right.is_empty() {
        return None;
    }

    let interpreter = shell_interpreter_name(&right[0])?;
    let mut urls = Vec::new();
    for token in left.iter().skip(1) {
        if token.starts_with("https://") || token.starts_with("http://") {
            urls.push(token);
        } else if !is_trace_curl_stdout_flag(token) {
            return None;
        }
    }
    if urls.len() != 1 {
        return None;
    }

    Some(TraceCurlPipe {
        url: urls[0].to_string(),
        interpreter: interpreter.to_string(),
    })
}

fn is_trace_curl_stdout_flag(value: &str) -> bool {
    match value {
        "--fail" | "--fail-with-body" | "--silent" | "--show-error" | "--location" => true,
        _ => {
            value.starts_with('-')
                && value
                    .chars()
                    .skip(1)
                    .all(|ch| matches!(ch, 'f' | 's' | 'S' | 'L'))
        }
    }
}

fn shell_interpreter_name(value: &str) -> Option<&'static str> {
    match value {
        "sh" | "/bin/sh" | "/usr/bin/sh" => Some("sh"),
        "bash" | "/bin/bash" | "/usr/bin/bash" => Some("bash"),
        _ => None,
    }
}

fn shell_words_for_trace(command: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            Some('"') => match ch {
                '"' => quote = None,
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                _ => current.push(ch),
            },
            Some(_) => unreachable!("trace tokenizer only tracks shell quote characters"),
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                '|' => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                    words.push("|".to_string());
                }
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }

    if quote.is_some() {
        return None;
    }
    if !current.is_empty() {
        words.push(current);
    }
    Some(words)
}

fn download_trace_script(url: &str) -> Result<(String, bool), String> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| match err {
            UreqError::Status(code, _) => {
                format!("failed to download trace script {url}: HTTP {code}")
            }
            UreqError::Transport(err) => format!("failed to download trace script {url}: {err}"),
        })?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_TRACE_SCRIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("failed to read trace script {url}: {err}"))?;
    let truncated = bytes.len() as u64 > MAX_TRACE_SCRIPT_BYTES;
    if truncated {
        bytes.truncate(MAX_TRACE_SCRIPT_BYTES as usize);
    }
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

fn is_trace_agent_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn invoke_trace_agent(
    agent: TraceAgent,
    command: &str,
    fetched_script: Option<&TraceFetchedScript>,
) -> Result<String, String> {
    let schema = trace_output_schema();
    let prompt = trace_prompt(command, fetched_script);
    match agent {
        TraceAgent::Codex => invoke_codex_trace(&prompt, &schema),
        TraceAgent::Claude => invoke_claude_trace(&prompt, &schema),
        TraceAgent::Auto => unreachable!("trace agent must be resolved before invocation"),
    }
}

fn invoke_codex_trace(prompt: &str, schema: &str) -> Result<String, String> {
    let runtime = trace_runtime_dir()?;
    let schema_path = runtime.path().join("schema.json");
    fs::write(&schema_path, schema)
        .map_err(|err| format!("failed to write trace schema file: {err}"))?;

    let mut command = sandboxed_trace_command(runtime.path(), "codex", TraceAgent::Codex)?;
    let mut child = command
        .arg("exec")
        .arg("--ephemeral")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--output-schema")
        .arg(&schema_path)
        .arg("--color")
        .arg("never")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to start codex trace agent: {err}"))?;
    write_child_stdin(&mut child, prompt)?;
    collect_trace_agent_output("codex", child)
}

fn invoke_claude_trace(prompt: &str, schema: &str) -> Result<String, String> {
    let runtime = trace_runtime_dir()?;
    let mut command = sandboxed_trace_command(runtime.path(), "claude", TraceAgent::Claude)?;
    let mut child = command
        .arg("-p")
        .arg("--no-session-persistence")
        .arg("--permission-mode")
        .arg("plan")
        .arg("--json-schema")
        .arg(schema)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to start claude trace agent: {err}"))?;
    write_child_stdin(&mut child, prompt)?;
    collect_trace_agent_output("claude", child)
}

fn trace_runtime_dir() -> Result<tempfile::TempDir, String> {
    tempfile::Builder::new()
        .prefix("automic-vault.trace.")
        .tempdir_in("/tmp")
        .map_err(|err| format!("failed to create trace runtime directory: {err}"))
}

fn sandboxed_trace_command(
    runtime_root: &Path,
    program: &str,
    agent: TraceAgent,
) -> Result<Command, String> {
    if !is_trace_agent_executable_file(Path::new(TRACE_SANDBOX_EXEC_PATH)) {
        return Err("sandbox-exec is required for trace agent isolation".to_string());
    }

    let profile_path = runtime_root.join("trace-agent.sb");
    fs::write(&profile_path, trace_sandbox_profile(runtime_root, agent))
        .map_err(|err| format!("failed to write trace sandbox profile: {err}"))?;

    let mut command = Command::new(TRACE_SANDBOX_EXEC_PATH);
    command.arg("-f").arg(profile_path).arg(program);
    Ok(command)
}

fn trace_sandbox_profile(runtime_root: &Path, agent: TraceAgent) -> String {
    let mut profile = format!(
        "(version 1)\n(allow default)\n(deny file-write*)\n(allow file-write* (literal \"/dev/null\"))\n(allow file-write* (subpath \"{}\"))",
        escape_trace_sandbox_path(runtime_root)
    );

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        let state_dir = match agent {
            TraceAgent::Auto => None,
            TraceAgent::Codex => Some(home.join(".codex")),
            TraceAgent::Claude => Some(home.join(".claude")),
        };
        if let Some(state_dir) = state_dir {
            profile.push_str(&format!(
                "\n(allow file-write* (subpath \"{}\"))",
                escape_trace_sandbox_path(&state_dir)
            ));
        }
    }

    profile
}

fn escape_trace_sandbox_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn write_child_stdin(child: &mut process::Child, prompt: &str) -> Result<(), String> {
    let Some(mut stdin) = child.stdin.take() else {
        return Err("failed to open trace agent stdin".to_string());
    };
    stdin
        .write_all(prompt.as_bytes())
        .and_then(|_| stdin.flush())
        .map_err(|err| format!("failed to write trace prompt: {err}"))
}

fn collect_trace_agent_output(agent: &str, child: process::Child) -> Result<String, String> {
    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for {agent} trace agent: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        if message.is_empty() {
            return Err(format!(
                "{agent} trace agent exited without a successful status"
            ));
        }
        return Err(format!("{agent} trace agent failed: {message}"));
    }
    String::from_utf8(output.stdout)
        .map_err(|err| format!("{agent} trace agent returned non-UTF-8 output: {err}"))
}

fn trace_prompt(command: &str, fetched_script: Option<&TraceFetchedScript>) -> String {
    let fetched_script_section = fetched_script
        .map(format_fetched_script_section)
        .unwrap_or_default();
    format!(
        "\
You are tracing a shell one-liner for Automic Vault.

Do not execute the one-liner. Interpret it statically.

Return JSON only, matching the provided schema.

Descriptions must be diagnostic, not historical. Use concise present-tense
action summaries such as \"Downloads...\", \"Installs...\", \"Adds...\", or
\"May modify...\". Do not use conditional wording such as \"Would download...\"
and do not use past tense such as \"Downloaded...\", \"Executed...\",
\"Created...\", or \"Wrote...\".

Only report consequential steps that write files or change files. Include file
creation, content writes, appends, overwrites, deletions, moves, chmod/chown,
install/service writes, and generated executable changes. Group consecutive
events that are part of the same file-changing action into one step. For
example, creating a file, setting permissions, and filling it with data should
be one step, not three.

Summarize installer behavior at the level a user needs to understand before
running it. Do not report incidental temporary directory creation, staged
filenames, mount-point filenames, or cleanup of temporary staging artifacts as
separate steps. If an installer downloads a DMG and mounts it before copying an
app into /Applications, keep that as two steps: \"Downloads and mounts the
DMG.\" and \"Installs the app into /Applications.\".

For large multi-platform installers, group mutually exclusive platform branches
into user-level categories instead of listing every OS-specific command. Prefer
around 5-8 steps that cover missing tooling/system dependencies, repository
checkout or update, language environments and dependencies, command launcher
setup, configuration files, interactive setup, and background services.

Do include network fetches or network calls when they are part of, or explain,
a file-changing step, such as downloading an install script before it writes
files. Do not report unrelated reads, stdout-only output, or speculation with
low confidence.

If the one-liner downloads code from a URL and pipes it directly into an
interpreter such as sh, bash, zsh, python, ruby, node, or perl, report that as
one network-backed installer execution step. When a fetched script body is
provided below, continue tracing into that script body and report the concrete
file-changing steps that the script would perform. Use the script body instead
of guessing from the URL alone. When a fetched script body is provided, do not
report the outer curl-to-interpreter pipe as its own step unless it writes a
file; the fetched URL is context for analyzing the script body.

Use concise human descriptions. Prefer concrete paths when the one-liner
reveals them; otherwise use a clear path phrase such as \"installer-selected
destination\". Use a null path in that case.

Shell one-liner:
{command}
{fetched_script_section}"
    )
}

fn format_fetched_script_section(script: &TraceFetchedScript) -> String {
    let truncation = if script.truncated {
        format!(
            "\nThe fetched script was truncated to the first {} bytes for analysis.",
            MAX_TRACE_SCRIPT_BYTES
        )
    } else {
        String::new()
    };
    format!(
        "\n\nThe CLI already downloaded the script that the one-liner would pipe into `{}`. Do not download it again. Analyze this fetched script body as the input that `{}` would receive.\n\nFetched URL: {}\n{}\
----- BEGIN FETCHED SCRIPT -----\n{}\n----- END FETCHED SCRIPT -----",
        script.interpreter, script.interpreter, script.url, truncation, script.body
    )
}

fn trace_output_schema() -> String {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["steps"],
        "properties": {
            "steps": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["description", "operation", "path", "network"],
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "A concise present-tense user-facing step. Related file creation, permissions, and content writes must be grouped; incidental temporary staging and cleanup should be omitted."
                        },
                        "operation": {
                            "type": "string",
                            "description": "Short operation label such as create, modify, delete, move, chmod, install, or unknown."
                        },
                        "path": {
                            "type": ["string", "null"],
                            "description": "Changed path when known, otherwise null."
                        },
                        "network": {
                            "type": ["string", "null"],
                            "description": "Network fetch or call involved in this file-changing step, when relevant."
                        }
                    }
                }
            }
        }
    })
    .to_string()
}

fn parse_trace_agent_output(output: &str) -> Result<TraceAgentOutput, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("trace agent returned empty output".to_string());
    }
    parse_trace_agent_output_json(trimmed)
        .or_else(|_| parse_trace_agent_json_envelope(trimmed))
        .or_else(|_| parse_trace_agent_embedded_json(trimmed))
        .map_err(|err| format!("failed to parse trace agent output: {err}"))
}

fn parse_trace_agent_output_json(value: &str) -> Result<TraceAgentOutput, serde_json::Error> {
    serde_json::from_str::<TraceAgentOutput>(value)
}

fn parse_trace_agent_json_envelope(value: &str) -> Result<TraceAgentOutput, serde_json::Error> {
    let envelope = serde_json::from_str::<serde_json::Value>(value)?;
    if let Some(result) = envelope.get("result").and_then(|result| result.as_str()) {
        return serde_json::from_str::<TraceAgentOutput>(result);
    }
    if let Some(message) = envelope.get("message").and_then(|message| message.as_str()) {
        return serde_json::from_str::<TraceAgentOutput>(message);
    }
    serde_json::from_value::<TraceAgentOutput>(envelope)
}

fn parse_trace_agent_embedded_json(value: &str) -> Result<TraceAgentOutput, serde_json::Error> {
    let starts = value
        .char_indices()
        .filter_map(|(index, ch)| (ch == '{').then_some(index))
        .collect::<Vec<_>>();
    let ends = value
        .char_indices()
        .filter_map(|(index, ch)| (ch == '}').then_some(index + ch.len_utf8()))
        .collect::<Vec<_>>();
    for start in starts {
        for end in ends.iter().rev().copied() {
            if end <= start {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<TraceAgentOutput>(&value[start..end]) {
                return Ok(parsed);
            }
        }
    }
    serde_json::from_str::<TraceAgentOutput>(value)
}

fn normalize_trace_steps(
    steps: Vec<TraceStep>,
    fetched_script_was_provided: bool,
) -> Vec<TraceStep> {
    let steps = steps
        .into_iter()
        .filter_map(|mut step| {
            if is_incidental_trace_step(&step)
                || (fetched_script_was_provided && is_outer_fetched_script_step(&step))
            {
                return None;
            }
            step.description = normalize_trace_description(&step.description);
            step.operation = step.operation.trim().to_string();
            step.path = step
                .path
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty());
            step.network = step
                .network
                .map(|network| network.trim().to_string())
                .filter(|network| !network.is_empty());
            (!step.description.is_empty() && !step.operation.is_empty()).then_some(step)
        })
        .flat_map(split_combined_dmg_install_step)
        .collect::<Vec<_>>();
    coalesce_complex_installer_steps(steps)
}

fn normalize_trace_description(description: &str) -> String {
    let description = description.trim();
    for (prefix, replacement) in [
        ("Would download ", "Downloads "),
        ("Would execute ", "Executes "),
        ("Would create ", "Creates "),
        ("Would write ", "Writes "),
        ("Would append ", "Appends "),
        ("Would install ", "Installs "),
        ("Would modify ", "Modifies "),
        ("Would change ", "Changes "),
        ("Would delete ", "Deletes "),
        ("Would remove ", "Removes "),
        ("Would move ", "Moves "),
        ("Would set ", "Sets "),
        ("Downloaded ", "Downloads "),
        ("Downloads ", "Downloads "),
        ("Executed ", "Executes "),
        ("Executes ", "Executes "),
        ("Created ", "Creates "),
        ("Creates ", "Creates "),
        ("Wrote ", "Writes "),
        ("Writes ", "Writes "),
        ("Appended ", "Appends "),
        ("Appends ", "Appends "),
        ("Installed ", "Installs "),
        ("Installs ", "Installs "),
        ("Modified ", "Modifies "),
        ("Modifies ", "Modifies "),
        ("Changed ", "Changes "),
        ("Changes ", "Changes "),
        ("Deleted ", "Deletes "),
        ("Deletes ", "Deletes "),
        ("Removed ", "Removes "),
        ("Removes ", "Removes "),
        ("Moved ", "Moves "),
        ("Moves ", "Moves "),
        ("Set ", "Sets "),
        ("Sets ", "Sets "),
    ] {
        if let Some(rest) = description.strip_prefix(prefix) {
            return clean_trace_action_tense(&format!("{replacement}{rest}"));
        }
    }
    clean_trace_action_tense(description)
}

fn is_incidental_trace_step(step: &TraceStep) -> bool {
    let operation = step.operation.trim().to_ascii_lowercase();
    let description = step.description.trim().to_ascii_lowercase();
    let path = step
        .path
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let mentions_temp = description.contains("temporary")
        || description.contains("temp ")
        || description.contains("tempdir")
        || description.contains("tmp")
        || description.contains("staging")
        || description.contains("staged")
        || path.starts_with("/tmp/")
        || path.contains("/tmp/");

    if matches!(operation.as_str(), "delete" | "remove" | "cleanup")
        && (mentions_temp || description.contains("cleanup"))
    {
        return true;
    }

    matches!(operation.as_str(), "create" | "mkdir")
        && mentions_temp
        && step
            .network
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
}

fn is_outer_fetched_script_step(step: &TraceStep) -> bool {
    if step
        .path
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        let description = step.description.trim().to_ascii_lowercase();
        return description.contains("installer script")
            && (description.contains("runs")
                || description.contains("run ")
                || description.contains("executes")
                || description.contains("execute ")
                || description.contains("with sh")
                || description.contains("with bash"));
    }
    false
}

fn split_combined_dmg_install_step(step: TraceStep) -> Vec<TraceStep> {
    let description = step.description.to_ascii_lowercase();
    if !(description.contains("dmg")
        && description.contains("mount")
        && description.contains("install")
        && description.contains("/applications"))
    {
        return vec![step];
    }

    let dmg_description = if description.contains("automic vault") {
        "Downloads and mounts the Automic Vault DMG."
    } else {
        "Downloads and mounts the DMG."
    };

    vec![
        TraceStep {
            description: dmg_description.to_string(),
            operation: "download".to_string(),
            path: None,
            network: step.network.clone(),
        },
        TraceStep {
            description: "Installs the app into /Applications.".to_string(),
            operation: "install".to_string(),
            path: Some("/Applications".to_string()),
            network: None,
        },
    ]
}

#[derive(Debug, Default)]
struct ComplexInstallerBuckets {
    tooling: bool,
    repository: bool,
    python: bool,
    node_browser: bool,
    launcher: bool,
    config: bool,
    setup: bool,
    gateway: bool,
}

impl ComplexInstallerBuckets {
    fn count(&self) -> usize {
        [
            self.tooling,
            self.repository,
            self.python,
            self.node_browser,
            self.launcher,
            self.config,
            self.setup,
            self.gateway,
        ]
        .into_iter()
        .filter(|matched| *matched)
        .count()
    }
}

fn coalesce_complex_installer_steps(steps: Vec<TraceStep>) -> Vec<TraceStep> {
    let is_complex_hermes_installer = steps.iter().any(is_complex_hermes_installer_step);
    if steps.len() < 8 || !is_complex_hermes_installer {
        return steps;
    }

    let mut buckets = ComplexInstallerBuckets::default();
    for step in &steps {
        match complex_installer_bucket(&step.description) {
            Some(ComplexInstallerBucket::Tooling) => buckets.tooling = true,
            Some(ComplexInstallerBucket::Repository) => buckets.repository = true,
            Some(ComplexInstallerBucket::Python) => buckets.python = true,
            Some(ComplexInstallerBucket::NodeBrowser) => buckets.node_browser = true,
            Some(ComplexInstallerBucket::Launcher) => buckets.launcher = true,
            Some(ComplexInstallerBucket::Config) => buckets.config = true,
            Some(ComplexInstallerBucket::Setup) => buckets.setup = true,
            Some(ComplexInstallerBucket::Gateway) => buckets.gateway = true,
            None => {}
        }
    }

    if is_complex_hermes_installer && buckets.repository && buckets.python && buckets.config {
        buckets.launcher = true;
        buckets.setup = true;
    }

    if buckets.count() < 5 {
        return steps;
    }

    let mut summarized = Vec::new();
    if buckets.tooling {
        summarized.push(summary_trace_step(
            "May install missing tooling and system dependencies such as uv, Python, Git, Node.js, ripgrep, ffmpeg, and build tools.",
            "install",
            None,
        ));
    }
    if buckets.repository {
        summarized.push(summary_trace_step(
            "Clones or updates the Hermes Agent repository in the selected install directory.",
            "install",
            Some("selected install directory"),
        ));
    }
    if buckets.python {
        summarized.push(summary_trace_step(
            "Creates or recreates the Python environment and installs Hermes Agent dependencies.",
            "install",
            Some("Hermes Agent checkout"),
        ));
    }
    if buckets.node_browser {
        summarized.push(summary_trace_step(
            "Installs Node, browser, and TUI dependencies when supported.",
            "install",
            Some("Hermes Agent checkout"),
        ));
    }
    if buckets.launcher {
        summarized.push(summary_trace_step(
            "Writes the hermes launcher and may update shell startup files for PATH.",
            "install",
            Some("command link directory"),
        ));
    }
    if buckets.config {
        summarized.push(summary_trace_step(
            "Creates Hermes home directories, config files, environment files, persona, and bundled skills.",
            "create",
            Some("Hermes home"),
        ));
    }
    if buckets.setup {
        summarized.push(summary_trace_step(
            "May run the interactive setup wizard and configure browser or messaging settings.",
            "modify",
            Some("Hermes home"),
        ));
    }
    if buckets.gateway {
        summarized.push(summary_trace_step(
            "May install or start the Hermes gateway service or background process.",
            "install",
            Some("Hermes gateway"),
        ));
    }

    summarized
}

fn summary_trace_step(description: &str, operation: &str, path: Option<&str>) -> TraceStep {
    TraceStep {
        description: description.to_string(),
        operation: operation.to_string(),
        path: path.map(str::to_string),
        network: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComplexInstallerBucket {
    Tooling,
    Repository,
    Python,
    NodeBrowser,
    Launcher,
    Config,
    Setup,
    Gateway,
}

fn is_complex_hermes_installer_step(step: &TraceStep) -> bool {
    let description = step.description.to_ascii_lowercase();
    description.contains("hermes")
        || description.contains("gateway")
        || description.contains("playwright")
}

fn complex_installer_bucket(description: &str) -> Option<ComplexInstallerBucket> {
    let description = description.to_ascii_lowercase();
    if description.contains("gateway") {
        return Some(ComplexInstallerBucket::Gateway);
    }
    if description.contains("setup wizard")
        || description.contains("whatsapp session")
        || description.contains("interactive setup")
        || description.contains("messaging")
    {
        return Some(ComplexInstallerBucket::Setup);
    }
    if description.contains("hermes home")
        || description.contains("configuration")
        || description.contains("config file")
        || description.contains("environment file")
        || description.contains("persona file")
        || description.contains("bundled skills")
        || description.contains("skills into")
        || description.contains("browser executable path")
        || description.contains("chrome or chromium executable path")
    {
        return Some(ComplexInstallerBucket::Config);
    }
    if description.contains("launcher")
        || description.contains("command link")
        || description.contains("shell startup")
        || description.contains("startup files")
        || description.contains(" for path")
    {
        return Some(ComplexInstallerBucket::Launcher);
    }
    if description.contains("node.js dependencies")
        || description.contains("node dependencies")
        || description.contains("browser tooling")
        || description.contains("browser system dependencies")
        || description.contains("playwright")
        || description.contains("tui")
    {
        return Some(ComplexInstallerBucket::NodeBrowser);
    }
    if description.contains("virtual environment")
        || description.contains("python environment")
        || description.contains("python dependencies")
        || description.contains("python package")
        || description.contains("hermes agent dependencies")
        || description.contains("hermes agent python dependencies")
        || description.contains("selected python environment")
        || description.contains("psutil")
    {
        return Some(ComplexInstallerBucket::Python);
    }
    if description.contains("repository")
        || description.contains("git checkout")
        || description.contains("git stash")
        || description.contains("local changes")
    {
        return Some(ComplexInstallerBucket::Repository);
    }
    if description.contains("install uv")
        || description.contains("installs uv")
        || description.contains("install python")
        || description.contains("installs python")
        || description.contains("install git")
        || description.contains("installs git")
        || description.contains("install node.js")
        || description.contains("installs node.js")
        || description.contains("missing platform packages")
        || description.contains("ripgrep")
        || description.contains("ffmpeg")
        || description.contains("build dependencies")
        || description.contains("build tools")
        || description.contains("platform package manager")
        || description.contains("termux pkg")
        || description.contains("homebrew")
    {
        return Some(ComplexInstallerBucket::Tooling);
    }
    None
}

fn clean_trace_action_tense(description: &str) -> String {
    let description = description
        .replace(" and executed ", " and executes ")
        .replace(" and execute ", " and executes ")
        .replace(" and write ", " and writes ")
        .replace(" and writes ", " and writes ")
        .replace(" and wrote ", " and writes ")
        .replace(" and mount ", " and mounts ")
        .replace(" and mounted ", " and mounts ")
        .replace(" and copy ", " and copies ")
        .replace(" and copied ", " and copies ")
        .replace(" and remove ", " and removes ")
        .replace(" and removed ", " and removes ")
        .replace(" for installation.", ".")
        .replace(
            "Installs the mounted app into /Applications.",
            "Installs the app into /Applications.",
        )
        .replace(
            "Installs the contained app into /Applications.",
            "Installs the app into /Applications.",
        )
        .replace(
            "Installs the verified app into /Applications.",
            "Installs the app into /Applications.",
        )
        .replace(" using the app name found in the DMG.", ".")
        .replace(" using the app bundle name found in the DMG.", ".")
        .replace(" using the app bundle's existing basename.", ".")
        .replace("; executed ", "; executes ")
        .replace("; would execute ", "; executes ")
        .replace(" then executed ", " then executes ")
        .replace(" then would execute ", " then executes ");
    remove_incidental_file_choices(&description)
}

fn remove_incidental_file_choices(description: &str) -> String {
    let mut description = description.to_string();
    for marker in [
        " into an installer-selected temporary directory as ",
        " into a temporary directory as ",
        " into the temporary directory as ",
        " into the temporary working directory as ",
    ] {
        while let Some(start) = description.find(marker) {
            let end = description[start..]
                .rfind('.')
                .map(|offset| start + offset)
                .unwrap_or(description.len());
            description.replace_range(start..end, "");
        }
    }
    description
}

fn print_trace_report(report: &TraceReport) {
    if report.steps.is_empty() {
        println!("No file-changing steps identified.");
        return;
    }
    for (index, step) in report.steps.iter().enumerate() {
        println!("{}. {}", index + 1, step.description);
    }
}

pub(crate) fn is_trace_subcommand(value: &str) -> bool {
    value == "trace"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_output_schema_requires_all_step_properties() {
        let schema: serde_json::Value = serde_json::from_str(&trace_output_schema()).unwrap();
        let required = schema["properties"]["steps"]["items"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            required,
            vec!["description", "operation", "path", "network"]
        );
        assert_eq!(
            schema["properties"]["steps"]["items"]["properties"]["path"]["type"],
            serde_json::json!(["string", "null"])
        );
        assert_eq!(
            schema["properties"]["steps"]["items"]["properties"]["network"]["type"],
            serde_json::json!(["string", "null"])
        );
    }

    #[test]
    fn trace_prompt_reports_remote_script_execution_as_step() {
        let prompt = trace_prompt("curl https://example.test/install.sh | sh", None);

        assert!(prompt.contains("Descriptions must be diagnostic"));
        assert!(prompt.contains("present-tense"));
        assert!(prompt.contains("action summaries"));
        assert!(prompt.contains("Downloads"));
        assert!(prompt.contains("Do not use conditional wording"));
        assert!(prompt.contains("Do not report incidental temporary directory creation"));
        assert!(prompt.contains("keep that as two steps"));
        assert!(prompt.contains("downloads code from a URL"));
        assert!(prompt.contains("pipes it directly into an"));
        assert!(prompt.contains("one network-backed installer execution step"));
        assert!(prompt.contains("Use a null path"));
    }

    #[test]
    fn trace_prompt_includes_fetched_script_body() {
        let script = TraceFetchedScript {
            url: "https://example.test/install.sh".to_string(),
            interpreter: "sh".to_string(),
            body: "install -m 0755 av /usr/local/bin/av\n".to_string(),
            truncated: false,
        };
        let prompt = trace_prompt("curl https://example.test/install.sh | sh", Some(&script));

        assert!(prompt.contains("The CLI already downloaded the script"));
        assert!(prompt.contains("Fetched URL: https://example.test/install.sh"));
        assert!(prompt.contains("----- BEGIN FETCHED SCRIPT -----"));
        assert!(prompt.contains("install -m 0755 av /usr/local/bin/av"));
        assert!(prompt.contains("----- END FETCHED SCRIPT -----"));
    }

    #[test]
    fn trace_prompt_mentions_truncated_fetched_script() {
        let script = TraceFetchedScript {
            url: "https://example.test/install.sh".to_string(),
            interpreter: "bash".to_string(),
            body: "echo hi\n".to_string(),
            truncated: true,
        };
        let prompt = trace_prompt("curl https://example.test/install.sh | bash", Some(&script));

        assert!(prompt.contains("truncated to the first"));
        assert!(prompt.contains(&MAX_TRACE_SCRIPT_BYTES.to_string()));
    }

    #[test]
    fn parse_simple_curl_shell_pipe_accepts_url_piped_to_shell() {
        assert_eq!(
            parse_simple_curl_shell_pipe("curl -fsSL 'https://example.test/install.sh' | sh"),
            Some(TraceCurlPipe {
                url: "https://example.test/install.sh".to_string(),
                interpreter: "sh".to_string(),
            })
        );
        assert_eq!(
            parse_simple_curl_shell_pipe("curl https://example.test/install.sh|/bin/bash -s --"),
            Some(TraceCurlPipe {
                url: "https://example.test/install.sh".to_string(),
                interpreter: "bash".to_string(),
            })
        );
        assert_eq!(
            parse_simple_curl_shell_pipe(
                "curl --fail --silent --show-error --location https://example.test/install.sh | bash"
            ),
            Some(TraceCurlPipe {
                url: "https://example.test/install.sh".to_string(),
                interpreter: "bash".to_string(),
            })
        );
    }

    #[test]
    fn parse_simple_curl_shell_pipe_rejects_non_simple_commands() {
        assert_eq!(
            parse_simple_curl_shell_pipe("wget https://example.test/install.sh -O- | sh"),
            None
        );
        assert_eq!(
            parse_simple_curl_shell_pipe(
                "curl https://example.test/a.sh https://example.test/b.sh | sh"
            ),
            None
        );
        assert_eq!(
            parse_simple_curl_shell_pipe("curl https://example.test/install.sh | tee x | sh"),
            None
        );
        assert_eq!(
            parse_simple_curl_shell_pipe("curl https://example.test/install.sh > install.sh"),
            None
        );
        assert_eq!(
            parse_simple_curl_shell_pipe("curl -o install.sh https://example.test/install.sh | sh"),
            None
        );
    }

    #[test]
    fn shell_words_for_trace_handles_quotes_and_pipe_boundaries() {
        assert_eq!(
            shell_words_for_trace("curl 'https://example.test/a b.sh'|\"bash\""),
            Some(vec![
                "curl".to_string(),
                "https://example.test/a b.sh".to_string(),
                "|".to_string(),
                "bash".to_string(),
            ])
        );
        assert_eq!(shell_words_for_trace("curl 'unterminated"), None);
    }

    #[test]
    fn trace_sandbox_profile_denies_writes_except_runtime_and_agent_state() {
        let profile = trace_sandbox_profile(Path::new("/tmp/trace-runtime"), TraceAgent::Codex);

        assert!(profile.contains("(deny file-write*)"));
        assert!(profile.contains(r#"(allow file-write* (literal "/dev/null"))"#));
        assert!(profile.contains(r#"(allow file-write* (subpath "/tmp/trace-runtime"))"#));
        assert!(profile.contains(".codex"));
        assert!(!profile.contains(".claude"));
    }

    #[test]
    fn normalize_trace_description_avoids_completed_action_wording() {
        assert_eq!(
            normalize_trace_description("Downloaded install script and executed it with sh."),
            "Downloads install script and executes it with sh."
        );
        assert_eq!(
            normalize_trace_description("Downloads a script and writes /usr/local/bin/av."),
            "Downloads a script and writes /usr/local/bin/av."
        );
        assert_eq!(
            normalize_trace_description("Writes /usr/local/bin/av."),
            "Writes /usr/local/bin/av."
        );
        assert_eq!(
            normalize_trace_description("Would download the DMG and mount it."),
            "Downloads the DMG and mounts it."
        );
        assert_eq!(
            normalize_trace_description(
                "Installs the signed app bundle into /Applications using the app name found in the DMG."
            ),
            "Installs the signed app bundle into /Applications."
        );
        assert_eq!(
            normalize_trace_description("Installs the mounted app into /Applications."),
            "Installs the app into /Applications."
        );
        assert_eq!(
            normalize_trace_description("Installs the verified app into /Applications."),
            "Installs the app into /Applications."
        );
        assert_eq!(
            normalize_trace_description("May modify installer-selected files."),
            "May modify installer-selected files."
        );
    }

    #[test]
    fn normalize_trace_steps_omits_incidental_temp_staging_and_cleanup() {
        let steps = normalize_trace_steps(
            vec![
                TraceStep {
                    description: "Would create an installer-selected temporary directory."
                        .to_string(),
                    operation: "create".to_string(),
                    path: None,
                    network: None,
                },
                TraceStep {
                    description: "Would download the DMG into a temporary directory as av.dmg."
                        .to_string(),
                    operation: "create".to_string(),
                    path: Some("/tmp/av.dmg".to_string()),
                    network: Some("https://example.test/av.dmg".to_string()),
                },
                TraceStep {
                    description: "Would remove the temporary directory during cleanup.".to_string(),
                    operation: "delete".to_string(),
                    path: None,
                    network: None,
                },
            ],
            false,
        );

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].description, "Downloads the DMG.");
    }

    #[test]
    fn normalize_trace_steps_omits_outer_pipe_step_when_script_was_fetched() {
        let steps = normalize_trace_steps(
            vec![
                TraceStep {
                    description: "Downloads the installer script and runs it with sh.".to_string(),
                    operation: "install".to_string(),
                    path: None,
                    network: Some("https://example.test/install.sh".to_string()),
                },
                TraceStep {
                    description: "Downloads the DMG and mounts it.".to_string(),
                    operation: "install".to_string(),
                    path: None,
                    network: Some("https://example.test/app.dmg".to_string()),
                },
            ],
            true,
        );

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].description, "Downloads the DMG and mounts it.");
    }

    #[test]
    fn normalize_trace_steps_splits_combined_dmg_install_summary() {
        let steps = normalize_trace_steps(
            vec![TraceStep {
                description:
                    "Downloads the Automic Vault DMG, mounts it, and installs the contained app into /Applications."
                        .to_string(),
                operation: "install".to_string(),
                path: Some("/Applications".to_string()),
                network: Some("https://example.test/AutomicVault.dmg".to_string()),
            }],
            true,
        );

        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[0].description,
            "Downloads and mounts the Automic Vault DMG."
        );
        assert_eq!(steps[0].operation, "download");
        assert_eq!(
            steps[0].network,
            Some("https://example.test/AutomicVault.dmg".to_string())
        );
        assert_eq!(steps[1].description, "Installs the app into /Applications.");
        assert_eq!(steps[1].operation, "install");
        assert_eq!(steps[1].path, Some("/Applications".to_string()));
        assert_eq!(steps[1].network, None);
    }

    #[test]
    fn normalize_trace_steps_coalesces_complex_hermes_installer() {
        let steps = normalize_trace_steps(
            vec![
                trace_step("May install uv into the user-local executable directory."),
                trace_step("May install Python 3.11 through uv's managed Python installation."),
                trace_step("May install Git through Termux pkg."),
                trace_step("May install Node.js under Hermes home and add node, npm, and npx shims to the user-local bin directory."),
                trace_step("May install ripgrep, ffmpeg, and build dependencies through the platform package manager."),
                trace_step("Clones or updates the Hermes Agent repository at the installer-selected destination."),
                trace_step("May stash and restore local changes in an existing Hermes Agent git checkout before updating it."),
                trace_step("Creates or recreates the Python virtual environment inside the Hermes Agent checkout."),
                trace_step("Installs Hermes Agent Python dependencies into the selected Python environment."),
                trace_step("May patch and prebuild psutil for Android Python installs."),
                trace_step("Installs Node.js dependencies for browser tooling in the Hermes Agent checkout."),
                trace_step("May install Playwright Chromium and browser system dependencies."),
                trace_step("Installs TUI Node.js dependencies when the TUI package is present."),
                trace_step("Adds the hermes launcher script to the command link directory and marks it executable."),
                trace_step("May append the user-local bin directory to shell startup files."),
                trace_step("Creates the Hermes home directory structure for config, sessions, logs, caches, memories, hooks, pairing, cron, and skills."),
                trace_step("Creates the Hermes environment file from the repository template or as an empty file."),
                trace_step("May append the detected Chrome or Chromium executable path to the Hermes environment file."),
                trace_step("Creates the Hermes config file from the repository template."),
                trace_step("Creates the Hermes persona file."),
                trace_step("Syncs or copies bundled skills into Hermes home."),
                trace_step("May modify Hermes configuration through the interactive setup wizard."),
                trace_step("May create WhatsApp session files during interactive pairing."),
                trace_step("May install and start the Hermes gateway as a background service."),
                trace_step("May start the Hermes gateway in the background and writes its log file."),
            ],
            true,
        );

        assert_eq!(
            steps
                .iter()
                .map(|step| step.description.as_str())
                .collect::<Vec<_>>(),
            vec![
                "May install missing tooling and system dependencies such as uv, Python, Git, Node.js, ripgrep, ffmpeg, and build tools.",
                "Clones or updates the Hermes Agent repository in the selected install directory.",
                "Creates or recreates the Python environment and installs Hermes Agent dependencies.",
                "Installs Node, browser, and TUI dependencies when supported.",
                "Writes the hermes launcher and may update shell startup files for PATH.",
                "Creates Hermes home directories, config files, environment files, persona, and bundled skills.",
                "May run the interactive setup wizard and configure browser or messaging settings.",
                "May install or start the Hermes gateway service or background process.",
            ]
        );
    }

    #[test]
    fn normalize_trace_steps_coalesces_partially_grouped_hermes_installer() {
        let steps = normalize_trace_steps(
            vec![
                trace_step("Installs uv when missing into the user-level uv install location."),
                trace_step("Installs Python 3.11 through uv when no suitable Python is available."),
                trace_step("Installs missing platform packages for Git, Node.js, ripgrep, ffmpeg, Termux build tools, Playwright browser libraries, or Debian/Ubuntu Python build dependencies."),
                trace_step("Installs a Hermes-managed Node.js runtime and command symlinks when Node.js is missing on desktop platforms."),
                trace_step("Clones or updates the Hermes Agent repository, including stashing and restoring local changes during updates."),
                trace_step("Creates or recreates the Python virtual environment and installs Hermes Agent Python dependencies into it."),
                trace_step("Installs Node.js project dependencies, Playwright Chromium when no system browser is found, and TUI dependencies when package manifests exist."),
                trace_step("Installs the hermes launcher shim and updates shell startup files so the command path is available."),
                trace_step("Creates Hermes data/config directories and seeds .env, config.yaml, SOUL.md, browser environment settings, and bundled skills."),
                trace_step("Runs the interactive setup wizard, which may write API keys and settings into Hermes configuration files."),
                trace_step("Installs and starts the messaging gateway as a systemd service, or starts it in background mode with logs when messaging tokens are configured."),
            ],
            true,
        );

        assert_eq!(steps.len(), 8);
        assert_eq!(
            steps[0].description,
            "May install missing tooling and system dependencies such as uv, Python, Git, Node.js, ripgrep, ffmpeg, and build tools."
        );
        assert_eq!(
            steps[7].description,
            "May install or start the Hermes gateway service or background process."
        );
    }

    #[test]
    fn normalize_trace_steps_preserves_hermes_launcher_and_setup_categories() {
        let steps = normalize_trace_steps(
            vec![
                trace_step("Installs uv when missing into the user-level uv install location."),
                trace_step("Installs missing platform packages for Git, Node.js, ripgrep, ffmpeg, Termux build tools, Playwright browser libraries, or Debian/Ubuntu Python build dependencies."),
                trace_step("Clones or updates the Hermes Agent repository, including stashing and restoring local changes during updates."),
                trace_step("Creates or recreates the Python virtual environment and installs Hermes Agent Python dependencies into it."),
                trace_step("Installs Node.js project dependencies, Playwright Chromium when no system browser is found, and TUI dependencies when package manifests exist."),
                trace_step("Creates Hermes data/config directories and seeds .env, config.yaml, SOUL.md, browser environment settings, and bundled skills."),
                trace_step("Installs and starts the messaging gateway as a systemd service, or starts it in background mode with logs when messaging tokens are configured."),
                trace_step("May install ripgrep, ffmpeg, and build dependencies through the platform package manager."),
                trace_step("May install Python 3.11 through uv's managed Python installation."),
            ],
            true,
        );

        assert_eq!(
            steps
                .iter()
                .map(|step| step.description.as_str())
                .collect::<Vec<_>>(),
            vec![
                "May install missing tooling and system dependencies such as uv, Python, Git, Node.js, ripgrep, ffmpeg, and build tools.",
                "Clones or updates the Hermes Agent repository in the selected install directory.",
                "Creates or recreates the Python environment and installs Hermes Agent dependencies.",
                "Installs Node, browser, and TUI dependencies when supported.",
                "Writes the hermes launcher and may update shell startup files for PATH.",
                "Creates Hermes home directories, config files, environment files, persona, and bundled skills.",
                "May run the interactive setup wizard and configure browser or messaging settings.",
                "May install or start the Hermes gateway service or background process.",
            ]
        );
    }

    #[test]
    fn normalize_trace_steps_coalesces_eight_step_hermes_trace() {
        let steps = normalize_trace_steps(
            vec![
                trace_step("Installs missing installer/runtime tooling, including uv, Python 3.11, Git, and optional package-manager dependencies such as ripgrep, ffmpeg, and build tools."),
                trace_step("Installs Node.js for browser tools and creates node, npm, and npx launcher symlinks."),
                trace_step("Checks out or updates the Hermes Agent repository, stashing and optionally restoring local changes during updates."),
                trace_step("Creates or recreates the Python virtual environment and installs Hermes Agent Python dependencies in editable mode."),
                trace_step("Installs JavaScript dependencies for browser and TUI tooling, and installs Playwright Chromium or related browser system libraries when no system Chrome/Chromium is found."),
                trace_step("Creates the hermes command launcher, makes it executable, and may add its directory to shell PATH configuration files."),
                trace_step("Creates Hermes data/config directories, writes .env/config/persona files from templates or defaults, and syncs bundled skills."),
                trace_step("May write messaging gateway service files or start a background gateway with logs when messaging tokens are configured and setup is accepted."),
            ],
            true,
        );

        assert_eq!(steps.len(), 8);
        assert_eq!(
            steps[6].description,
            "May run the interactive setup wizard and configure browser or messaging settings."
        );
        assert_eq!(
            steps[7].description,
            "May install or start the Hermes gateway service or background process."
        );
    }

    fn trace_step(description: &str) -> TraceStep {
        TraceStep {
            description: description.to_string(),
            operation: "install".to_string(),
            path: None,
            network: None,
        }
    }
}
