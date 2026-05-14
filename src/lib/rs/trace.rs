use super::*;

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
    let output = invoke_trace_agent(resolved, &request.command)?;
    let parsed = parse_trace_agent_output(&output)?;
    Ok(TraceReport {
        command: request.command.clone(),
        agent: resolved.name().to_string(),
        steps: normalize_trace_steps(parsed.steps),
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

fn is_trace_agent_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn invoke_trace_agent(agent: TraceAgent, command: &str) -> Result<String, String> {
    let schema = trace_output_schema();
    let prompt = trace_prompt(command);
    match agent {
        TraceAgent::Codex => invoke_codex_trace(&prompt, &schema),
        TraceAgent::Claude => invoke_claude_trace(&prompt, &schema),
        TraceAgent::Auto => unreachable!("trace agent must be resolved before invocation"),
    }
}

fn invoke_codex_trace(prompt: &str, schema: &str) -> Result<String, String> {
    let mut schema_file = tempfile::NamedTempFile::new()
        .map_err(|err| format!("failed to create trace schema file: {err}"))?;
    schema_file
        .write_all(schema.as_bytes())
        .map_err(|err| format!("failed to write trace schema file: {err}"))?;
    schema_file
        .flush()
        .map_err(|err| format!("failed to flush trace schema file: {err}"))?;

    let mut child = Command::new("codex")
        .arg("exec")
        .arg("--ephemeral")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--output-schema")
        .arg(schema_file.path())
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
    let mut child = Command::new("claude")
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

fn trace_prompt(command: &str) -> String {
    format!(
        "\
You are tracing a shell one-liner for Automic Vault.

Do not execute the one-liner. Interpret it statically.

Return JSON only, matching the provided schema.

Only report consequential steps that write files or change files. Include file
creation, content writes, appends, overwrites, deletions, moves, chmod/chown,
install/service writes, and generated executable changes. Group consecutive
events that are part of the same file-changing action into one step. For
example, creating a file, setting permissions, and filling it with data should
be one step, not three.

Do include network fetches or network calls when they are part of, or explain,
a file-changing step, such as downloading an install script before it writes
files. Do not report unrelated reads, stdout-only output, or speculation with
low confidence.

If the one-liner downloads code from a URL and pipes it directly into an
interpreter such as sh, bash, zsh, python, ruby, node, or perl, report that as
one network-backed installer execution step even when the exact changed paths
are not visible from the one-liner alone. Use a null path in that case.

Use concise human descriptions. Prefer concrete paths when the one-liner
reveals them; otherwise use a clear path phrase such as \"installer-selected
destination\".

Shell one-liner:
{command}"
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
                            "description": "A concise user-facing step. Related file creation, permissions, and content writes must be grouped."
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

fn normalize_trace_steps(steps: Vec<TraceStep>) -> Vec<TraceStep> {
    steps
        .into_iter()
        .filter_map(|mut step| {
            step.description = step.description.trim().to_string();
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
        .collect()
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
        let prompt = trace_prompt("curl https://example.test/install.sh | sh");

        assert!(prompt.contains("downloads code from a URL"));
        assert!(prompt.contains("pipes it directly into an"));
        assert!(prompt.contains("one network-backed installer execution step"));
        assert!(prompt.contains("Use a null path"));
    }
}
