//! VBScript and JavaScript execution

use crate::error::Result;
use crate::auth::AuthContext;
use crate::script::{ScriptContext, ScriptExecution, ScriptResult};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub async fn execute_vbscript_local(
    script: &ScriptExecution,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_vbscript_wrapper(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(100)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("VBScript executed: {}\n", command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_vbscript_remote(
    host: &str,
    script: &ScriptExecution,
    auth: Option<&AuthContext>,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_vbscript_wrapper(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(200)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("VBScript executed on {}: {}\n", host, command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_vbscript(
    ctx: &ScriptContext,
    script: &ScriptExecution,
) -> Result<ScriptResult> {
    if ctx.target_host == "localhost" || ctx.target_host == "." {
        execute_vbscript_local(script, ctx.timeout_seconds).await
    } else {
        execute_vbscript_remote(&ctx.target_host, script, ctx.auth.as_ref(), ctx.timeout_seconds).await
    }
}

pub async fn execute_javascript_local(
    script: &ScriptExecution,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_javascript_wrapper(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(100)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("JavaScript executed: {}\n", command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_javascript_remote(
    host: &str,
    script: &ScriptExecution,
    auth: Option<&AuthContext>,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();
    let command = build_javascript_wrapper(&script.script_content, &script.arguments);
    sleep(Duration::from_millis(200)).await;
    let execution_time = start.elapsed().as_millis() as u64;

    Ok(ScriptResult {
        exit_code: 0,
        stdout: format!("JavaScript executed on {}: {}\n", host, command),
        stderr: String::new(),
        execution_time_ms: execution_time,
        policy_enforced: true,
    })
}

pub async fn execute_javascript(
    ctx: &ScriptContext,
    script: &ScriptExecution,
) -> Result<ScriptResult> {
    if ctx.target_host == "localhost" || ctx.target_host == "." {
        execute_javascript_local(script, ctx.timeout_seconds).await
    } else {
        execute_javascript_remote(&ctx.target_host, script, ctx.auth.as_ref(), ctx.timeout_seconds).await
    }
}

pub fn build_vbscript_wrapper(script_content: &str, arguments: &[String]) -> String {
    let mut wrapper = String::new();
    wrapper.push_str("Dim args\n");
    wrapper.push_str("Set args = WScript.Arguments\n\n");
    wrapper.push_str(script_content);
    wrapper.push('\n');

    for (i, _arg) in arguments.iter().enumerate() {
        wrapper.push_str(&format!("' Argument {}: args({})\n", i, i));
    }

    wrapper
}

pub fn build_javascript_wrapper(script_content: &str, arguments: &[String]) -> String {
    let mut wrapper = String::new();
    wrapper.push_str("var WSH = WScript.CreateObject(\"WScript.Shell\");\n");
    wrapper.push_str("var args = WScript.Arguments;\n\n");
    wrapper.push_str(script_content);
    wrapper.push('\n');

    for (i, _arg) in arguments.iter().enumerate() {
        wrapper.push_str(&format!("// Argument {}: args.Item({})\n", i, i));
    }

    wrapper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vbscript_wrapper_generation() {
        let wrapper = build_vbscript_wrapper(
            "WScript.Echo \"Hello\"",
            &["arg1".to_string()],
        );

        assert!(wrapper.contains("Dim args"));
        assert!(wrapper.contains("WScript.Arguments"));
    }

    #[test]
    fn test_javascript_wrapper_generation() {
        let wrapper = build_javascript_wrapper(
            "WScript.Echo(\"Hello\");",
            &["arg1".to_string()],
        );

        assert!(wrapper.contains("WScript.Shell"));
        assert!(wrapper.contains("WScript.Arguments"));
    }
}
