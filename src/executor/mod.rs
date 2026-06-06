use crate::auth::AuthContext;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMethod {
    SMBService,
    WMI,
    TaskScheduler,
    DCOM,
}

impl fmt::Display for ExecutionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionMethod::SMBService => write!(f, "SMB Service"),
            ExecutionMethod::WMI => write!(f, "WMI"),
            ExecutionMethod::TaskScheduler => write!(f, "Task Scheduler"),
            ExecutionMethod::DCOM => write!(f, "DCOM"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub method: ExecutionMethod,
    pub auth: AuthContext,
    pub command: String,
    pub working_directory: Option<String>,
    pub priority: Option<u32>,
    pub timeout_seconds: Option<u32>,
}

impl ExecutionContext {
    pub fn new(method: ExecutionMethod, auth: AuthContext, command: &str) -> Self {
        ExecutionContext {
            method,
            auth,
            command: command.to_string(),
            working_directory: None,
            priority: None,
            timeout_seconds: None,
        }
    }

    pub fn with_working_directory(mut self, path: &str) -> Self {
        self.working_directory = Some(path.to_string());
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl ExecutionResult {
    pub fn success(exit_code: i32, stdout: String, stderr: String) -> Self {
        ExecutionResult {
            exit_code,
            stdout,
            stderr,
            success: exit_code == 0,
        }
    }

    pub fn failed(exit_code: i32, stderr: String) -> Self {
        ExecutionResult {
            exit_code,
            stdout: String::new(),
            stderr,
            success: false,
        }
    }
}

pub mod wmi;
pub mod task_scheduler;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthMethod;

    #[test]
    fn test_execution_context() {
        let auth = AuthContext::new(AuthMethod::current_user(), "server.example.com");
        let ctx = ExecutionContext::new(ExecutionMethod::WMI, auth, "Get-Process")
            .with_timeout(30)
            .with_working_directory("C:\\Windows");

        assert_eq!(ctx.method, ExecutionMethod::WMI);
        assert_eq!(ctx.timeout_seconds, Some(30));
        assert_eq!(ctx.working_directory, Some("C:\\Windows".to_string()));
    }

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult::success(0, "output".to_string(), String::new());
        assert!(result.success);
        assert_eq!(result.exit_code, 0);

        let failed = ExecutionResult::failed(1, "error".to_string());
        assert!(!failed.success);
    }
}
