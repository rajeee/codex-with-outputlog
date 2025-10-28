use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use tokio::fs;

use crate::error::CodexErr;
use crate::error::SandboxErr;
use crate::exec::ExecToolCallOutput;

const LOG_SUBDIR: &str = "log";

fn sanitized_component(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "command".to_string()
    } else {
        sanitized
    }
}

async fn ensure_log_dir(codex_home: &Path) -> std::io::Result<PathBuf> {
    let dir = codex_home.join(LOG_SUBDIR);
    fs::create_dir_all(&dir).await?;
    Ok(dir)
}

async fn write_log_file(dir: &Path, name: &str, body: String) -> std::io::Result<PathBuf> {
    let path = dir.join(name);
    fs::write(&path, body).await?;
    Ok(path)
}

pub(crate) async fn log_command_output(
    codex_home: &Path,
    command: &[String],
    cwd: &Path,
    output: &ExecToolCallOutput,
) -> std::io::Result<PathBuf> {
    let dir = ensure_log_dir(codex_home).await?;
    let now = Utc::now();
    let command_name = command
        .first()
        .map(|part| sanitized_component(part))
        .unwrap_or_else(|| "command".to_string());
    let filename = format!("{}_{}.log", command_name, now.format("%Y%m%dT%H%M%S%.3fZ"));

    let mut body = String::new();
    let timestamp = now.to_rfc3339();
    let joined_command = if command.is_empty() {
        String::from("<empty command>")
    } else {
        command.join(" ")
    };

    let _ = writeln!(body, "Timestamp: {timestamp}");
    let _ = writeln!(body, "Command: {joined_command}");
    let _ = writeln!(body, "Working Directory: {}", cwd.display());
    let _ = writeln!(body, "Exit Code: {}", output.exit_code);
    let _ = writeln!(body, "Duration (ms): {}", output.duration.as_millis());
    let _ = writeln!(body, "Timed Out: {}", output.timed_out);
    if let Some(lines) = output.stdout.truncated_after_lines {
        let _ = writeln!(body, "Stdout truncated after {lines} lines");
    }
    if let Some(lines) = output.stderr.truncated_after_lines {
        let _ = writeln!(body, "Stderr truncated after {lines} lines");
    }
    if let Some(lines) = output.aggregated_output.truncated_after_lines {
        let _ = writeln!(body, "Aggregated output truncated after {lines} lines");
    }

    let _ = writeln!(body, "\n=== STDOUT ===\n{}", output.stdout.text);
    let _ = writeln!(body, "\n=== STDERR ===\n{}", output.stderr.text);
    let _ = writeln!(
        body,
        "\n=== AGGREGATED OUTPUT ===\n{}",
        output.aggregated_output.text
    );

    write_log_file(&dir, &filename, body).await
}

pub(crate) async fn log_network_exchange(
    codex_home: &Path,
    operation: &str,
    request: &str,
    response: &str,
) -> std::io::Result<PathBuf> {
    let dir = ensure_log_dir(codex_home).await?;
    let now = Utc::now();
    let operation_name = sanitized_component(operation);
    let filename = format!(
        "{}_{}.log",
        operation_name,
        now.format("%Y%m%dT%H%M%S%.3fZ")
    );

    let mut body = String::new();
    let timestamp = now.to_rfc3339();
    let _ = writeln!(body, "Timestamp: {timestamp}");
    let _ = writeln!(body, "Operation: {operation}");
    let _ = writeln!(body, "\n=== REQUEST ===\n{request}");
    let _ = writeln!(body, "\n=== RESPONSE ===\n{response}");

    write_log_file(&dir, &filename, body).await
}

pub(crate) fn exec_output_from_error(err: &CodexErr) -> Option<&ExecToolCallOutput> {
    match err {
        CodexErr::Sandbox(SandboxErr::Denied { output })
        | CodexErr::Sandbox(SandboxErr::Timeout { output }) => Some(output),
        _ => None,
    }
}
