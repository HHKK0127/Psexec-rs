//! Real script execution using tokio::process::Command

use crate::error::{PaExecError, Result};
use crate::script::{ScriptExecution, ScriptResult, ScriptType};
use std::process::Stdio;
use std::time::Instant;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Execute script locally using real process
pub async fn execute_script_local(
    script_type: ScriptType,
    execution: &ScriptExecution,
    timeout_seconds: Option<u32>,
) -> Result<ScriptResult> {
    let start = Instant::now();

    // Create temporary script file
    let (script_path, interpreter) = create_script_file(script_type, &execution.script_content).await?;

    // Build command
    let mut cmd = Command::new(&interpreter);

    // Add script path
    cmd.arg(&script_path);

    // Add arguments
    for arg in &execution.arguments {
        cmd.arg(arg);
    }

    // Set working directory
    if let Some(ref wd) = execution.working_directory {
        cmd.current_dir(wd);
    }

    // Set environment variables
    if let Some(ref env_vars) = execution.environment_vars {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    // Configure stdio
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    // Spawn process
    let mut child = cmd.spawn()
        .map_err(|e| PaExecError::ExecutionFailed(format!("Failed to spawn process: {}", e)))?;

    // Wait for completion with timeout
    let timeout_duration = timeout_seconds
        .map(|s| Duration::from_secs(s as u64))
        .unwrap_or(Duration::from_secs(300));

    let result = timeout(timeout_duration, async {
        let stdout = child.stdout.take()
            .ok_or_else(|| PaExecError::ExecutionFailed("Failed to capture stdout".to_string()))?;

        let stderr = child.stderr.take()
            .ok_or_else(|| PaExecError::ExecutionFailed("Failed to capture stderr".to_string()))?;

        let mut stdout_reader = tokio::io::BufReader::new(stdout);
        let mut stderr_reader = tokio::io::BufReader::new(stderr);

        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        // Read stdout and stderr concurrently
        let stdout_future = async {
            let mut buf = Vec::new();
            let _ = stdout_reader.read_to_end(&mut buf).await;
            buf
        };

        let stderr_future = async {
            let mut buf = Vec::new();
            let _ = stderr_reader.read_to_end(&mut buf).await;
            buf
        };

        let (stdout_buf, stderr_buf) = tokio::join!(stdout_future, stderr_future);
        stdout_data = stdout_buf;
        stderr_data = stderr_buf;

        // Wait for process to exit
        let status = child.wait().await
            .map_err(|e| PaExecError::ExecutionFailed(format!("Process wait failed: {}", e)))?;

        Ok::<(i32, Vec<u8>, Vec<u8>), PaExecError>((
            status.code().unwrap_or(-1),
            stdout_data,
            stderr_data,
        ))
    }).await;

    // Cleanup temp file
    let _ = fs::remove_file(&script_path).await;

    match result {
        Ok(Ok((exit_code, stdout_data, stderr_data))) => {
            let stdout = String::from_utf8_lossy(&stdout_data).to_string();
            let stderr = String::from_utf8_lossy(&stderr_data).to_string();

            Ok(ScriptResult {
                exit_code,
                stdout,
                stderr,
                execution_time_ms: start.elapsed().as_millis() as u64,
                policy_enforced: true,
            })
        }
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Timeout - kill process
            let _ = child.kill().await;
            Err(PaExecError::ExecutionFailed("Script execution timed out".to_string()))
        }
    }
}

/// Create temporary script file
async fn create_script_file(
    script_type: ScriptType,
    content: &str,
) -> Result<(std::path::PathBuf, String)> {
    let temp_dir = std::env::temp_dir();
    let (filename, interpreter) = match script_type {
        ScriptType::PowerShell => {
            ("script.ps1", "powershell.exe")
        }
        ScriptType::VBScript => {
            ("script.vbs", "cscript.exe")
        }
        ScriptType::Batch => {
            ("script.bat", "cmd.exe")
        }
        ScriptType::JavaScript => {
            ("script.js", "cscript.exe")
        }
    };

    let script_path = temp_dir.join(format!("psexec_{}_{}",
        std::process::id(),
        filename
    ));

    // Write script content
    fs::write(&script_path, content).await
        .map_err(|e| PaExecError::ExecutionFailed(format!("Failed to write script: {}", e)))?;

    // For PowerShell, add execution policy bypass
    let interpreter = match script_type {
        ScriptType::PowerShell => format!("{} -ExecutionPolicy Bypass -File", interpreter),
        _ => interpreter.to_string(),
    };

    Ok((script_path, interpreter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptExecution;

    #[tokio::test]
    async fn test_batch_script_creation() {
        let execution = ScriptExecution::new("echo Hello World");
        let (path, interpreter) = create_script_file(ScriptType::Batch, &execution.script_content)
            .await
            .unwrap();

        assert!(path.exists());
        assert_eq!(interpreter, "cmd.exe");
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "echo Hello World");

        let _ = fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn test_powershell_script_creation() {
        let execution = ScriptExecution::new("Write-Output 'test'");
        let (_, interpreter) = create_script_file(ScriptType::PowerShell, &execution.script_content)
            .await
            .unwrap();

        assert!(interpreter.contains("powershell.exe"));
        assert!(interpreter.contains("ExecutionPolicy"));
    }
}
