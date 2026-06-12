//! Script execution module

use crate::error::{PaExecError, Result};
use crate::auth::AuthContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub mod powershell;
pub mod vbscript;
pub mod batch;
pub mod executor;

pub use powershell::*;
pub use vbscript::*;
pub use batch::*;
pub use executor::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptType {
    PowerShell,
    VBScript,
    Batch,
    JavaScript,
}

impl fmt::Display for ScriptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptType::PowerShell => write!(f, "PowerShell"),
            ScriptType::VBScript => write!(f, "VBScript"),
            ScriptType::Batch => write!(f, "Batch"),
            ScriptType::JavaScript => write!(f, "JavaScript"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionPolicy {
    Restricted,
    AllSigned,
    RemoteSigned,
    Unrestricted,
    Bypass,
}

impl fmt::Display for ExecutionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionPolicy::Restricted => write!(f, "Restricted"),
            ExecutionPolicy::AllSigned => write!(f, "AllSigned"),
            ExecutionPolicy::RemoteSigned => write!(f, "RemoteSigned"),
            ExecutionPolicy::Unrestricted => write!(f, "Unrestricted"),
            ExecutionPolicy::Bypass => write!(f, "Bypass"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptContext {
    pub script_type: ScriptType,
    pub target_host: String,
    pub execution_policy: Option<ExecutionPolicy>,
    pub timeout_seconds: Option<u32>,
    pub auth: Option<AuthContext>,
}

impl ScriptContext {
    pub fn new(script_type: ScriptType, host: &str) -> Self {
        Self {
            script_type,
            target_host: host.to_string(),
            execution_policy: None,
            timeout_seconds: None,
            auth: None,
        }
    }

    pub fn with_execution_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.execution_policy = Some(policy);
        self
    }

    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    pub fn with_auth(mut self, auth: AuthContext) -> Self {
        self.auth = Some(auth);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ScriptExecution {
    pub script_content: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<String>,
    pub environment_vars: Option<HashMap<String, String>>,
}

impl ScriptExecution {
    pub fn new(script_content: &str) -> Self {
        Self {
            script_content: script_content.to_string(),
            arguments: Vec::new(),
            working_directory: None,
            environment_vars: None,
        }
    }

    pub fn with_arguments(mut self, args: Vec<String>) -> Self {
        self.arguments = args;
        self
    }

    pub fn with_working_directory(mut self, dir: &str) -> Self {
        self.working_directory = Some(dir.to_string());
        self
    }

    pub fn with_env_var(mut self, key: &str, value: &str) -> Self {
        if self.environment_vars.is_none() {
            self.environment_vars = Some(HashMap::new());
        }
        self.environment_vars.as_mut().unwrap().insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct ScriptResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub policy_enforced: bool,
}

impl ScriptResult {
    pub fn success(stdout: String, stderr: String) -> Self {
        Self {
            exit_code: 0,
            stdout,
            stderr,
            execution_time_ms: 0,
            policy_enforced: true,
        }
    }

    pub fn failure(exit_code: i32, stderr: String) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr,
            execution_time_ms: 0,
            policy_enforced: true,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

pub async fn execute_script(
    ctx: &ScriptContext,
    script: &ScriptExecution,
) -> Result<ScriptResult> {
    match ctx.script_type {
        ScriptType::PowerShell => {
            execute_powershell_script(ctx, script).await
        }
        ScriptType::VBScript => {
            execute_vbscript(ctx, script).await
        }
        ScriptType::Batch => {
            execute_batch_script(ctx, script).await
        }
        ScriptType::JavaScript => {
            execute_javascript(ctx, script).await
        }
    }
}

pub async fn get_execution_policy(ctx: &ScriptContext) -> Result<ExecutionPolicy> {
    if ctx.script_type != ScriptType::PowerShell {
        return Err(PaExecError::ExecutionFailed(
            "Execution policy only applies to PowerShell".to_string()
        ));
    }
    powershell::get_current_execution_policy(&ctx.target_host, ctx.auth.as_ref()).await
}

pub async fn set_execution_policy(ctx: &ScriptContext, policy: ExecutionPolicy) -> Result<()> {
    if ctx.script_type != ScriptType::PowerShell {
        return Err(PaExecError::ExecutionFailed(
            "Execution policy only applies to PowerShell".to_string()
        ));
    }
    powershell::set_temporary_execution_policy(&ctx.target_host, policy, ctx.auth.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_context_creation() {
        let ctx = ScriptContext::new(ScriptType::PowerShell, "server01")
            .with_execution_policy(ExecutionPolicy::Bypass)
            .with_timeout(60);

        assert_eq!(ctx.script_type, ScriptType::PowerShell);
        assert_eq!(ctx.target_host, "server01");
    }

    #[test]
    fn test_script_execution_building() {
        let script = ScriptExecution::new("Write-Host 'Hello'")
            .with_arguments(vec!["arg1".to_string()])
            .with_working_directory("C:\\Temp");

        assert_eq!(script.script_content, "Write-Host 'Hello'");
        assert_eq!(script.arguments.len(), 1);
    }

    #[test]
    fn test_script_type_variants() {
        assert_eq!(format!("{}", ScriptType::PowerShell), "PowerShell");
        assert_eq!(format!("{}", ScriptType::Batch), "Batch");
    }

    #[test]
    fn test_execution_policy_variants() {
        assert_eq!(format!("{}", ExecutionPolicy::Bypass), "Bypass");
    }

    #[test]
    fn test_script_result() {
        let result = ScriptResult::success("output".to_string(), "".to_string());
        assert!(result.is_success());
        assert_eq!(result.exit_code, 0);

        let result = ScriptResult::failure(1, "error".to_string());
        assert!(!result.is_success());
    }
}
