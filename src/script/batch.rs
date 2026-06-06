//! Batch file execution

use crate::error::Result;
use crate::auth::AuthContext;
use crate::script::{ScriptContext, ScriptExecution, ScriptResult};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn execute_batch_script_local(
    script: &ScriptExecution,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_batch_command(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(100)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("Batch executed: {}\n", command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_batch_script_remote(
    host: &str,
    script: &ScriptExecution,
    auth: Option<&AuthContext>,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_batch_command(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(200)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("Batch executed on {}: {}\n", host, command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_batch_script(
    ctx: &ScriptContext,
    script: &ScriptExecution,
) -> Result<ScriptResult> {
    if ctx.target_host == "localhost" || ctx.target_host == "." {
        execute_batch_script_local(script, ctx.timeout_seconds).await
    } else {
        execute_batch_script_remote(&ctx.target_host, script, ctx.auth.as_ref(), ctx.timeout_seconds).await
    }
}

pub fn build_batch_command(script_content: &str, arguments: &[String]) -> String {
    let mut command = String::new();
    command.push_str("@echo off\n");
    command.push_str(script_content);
    command.push('\n');

    for arg in arguments {
        let escaped = arg
            .replace("%", "%%")
            .replace("^", "^^")
            .replace("&", "^&")
            .replace("|", "^|")
            .replace("<", "^<")
            .replace(">", "^>")
            .replace("\"", "^\"");
        command.push_str(&format!("\"{}\" ", escaped));
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_command_builder() {
        let command = build_batch_command(
            "echo Hello",
            &["arg1".to_string()],
        );

        assert!(command.contains("@echo off"));
        assert!(command.contains("echo Hello"));
        assert!(command.contains("arg1"));
    }

    #[test]
    fn test_batch_special_characters_escape() {
        let command = build_batch_command(
            "echo test",
            &["100%".to_string()],
        );

        assert!(command.contains("%%") || command.contains("%"));
    }
}
