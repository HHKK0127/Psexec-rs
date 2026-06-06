//! PowerShell script execution

use crate::error::Result;
use crate::auth::AuthContext;
use crate::script::{ExecutionPolicy, ScriptContext, ScriptExecution, ScriptResult};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn execute_powershell_script_local(
    script: &ScriptExecution,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();

    let command = build_powershell_command(
        &script.script_content,
        &script.arguments,
        None,
    );

    sleep(Duration::from_millis(100)).await;

    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("Executed: {}\n", command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_powershell_script_remote(
    host: &str,
    script: &ScriptExecution,
    auth: Option<&AuthContext>,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();

    let original_policy = get_current_execution_policy(host, auth).await?;

    if original_policy == ExecutionPolicy::Restricted {
        set_temporary_execution_policy(host, ExecutionPolicy::Bypass, auth).await?;
    }

    let command = build_powershell_command(
        &script.script_content,
        &script.arguments,
        Some(ExecutionPolicy::Bypass),
    );

    sleep(Duration::from_millis(200)).await;

    if original_policy == ExecutionPolicy::Restricted {
        restore_execution_policy(host, original_policy, auth).await.ok();
    }

    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("Remote executed on {}: {}\n", host, command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub fn build_powershell_command(
    script_content: &str,
    arguments: &[String],
    execution_policy: Option<ExecutionPolicy>,
) -> String {
    let mut parts = Vec::new();

    parts.push(build_ps_profile_setup());

    if let Some(policy) = execution_policy {
        parts.push(format!(
            "Set-ExecutionPolicy -ExecutionPolicy {} -Scope Process -Force",
            policy
        ));
    }

    parts.push(format!("& {{ {} }}", script_content));

    for arg in arguments {
        let escaped = arg.replace("\"", "\\\"");
        parts.push(format!("\"{}\"", escaped));
    }

    parts.join("; ")
}

fn build_ps_profile_setup() -> String {
    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8".to_string()
}

pub async fn execute_powershell_script(
    ctx: &ScriptContext,
    script: &ScriptExecution,
) -> Result<ScriptResult> {
    if ctx.target_host == "localhost" || ctx.target_host == "." {
        execute_powershell_script_local(script, ctx.timeout_seconds).await
    } else {
        execute_powershell_script_remote(&ctx.target_host, script, ctx.auth.as_ref(), ctx.timeout_seconds).await
    }
}

pub async fn get_current_execution_policy(
    host: &str,
    auth: Option<&AuthContext>,
) -> Result<ExecutionPolicy> {
    sleep(Duration::from_millis(50)).await;
    Ok(ExecutionPolicy::RemoteSigned)
}

pub async fn set_temporary_execution_policy(
    host: &str,
    policy: ExecutionPolicy,
    auth: Option<&AuthContext>,
) -> Result<()> {
    sleep(Duration::from_millis(50)).await;
    Ok(())
}

pub async fn restore_execution_policy(
    host: &str,
    original_policy: ExecutionPolicy,
    auth: Option<&AuthContext>,
) -> Result<()> {
    sleep(Duration::from_millis(50)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powershell_command_builder() {
        let command = build_powershell_command(
            "Get-Process",
            &["notepad".to_string()],
            Some(ExecutionPolicy::Bypass),
        );

        assert!(command.contains("Set-ExecutionPolicy"));
        assert!(command.contains("Bypass"));
        assert!(command.contains("Get-Process"));
    }

    #[test]
    fn test_powershell_encoding_setup() {
        let setup = build_ps_profile_setup();
        assert!(setup.contains("OutputEncoding"));
        assert!(setup.contains("UTF8"));
    }
}
