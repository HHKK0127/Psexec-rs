//! CLI command response types for GUI integration

use std::fmt;

/// Service information response
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: ServiceState,
    pub path: String,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Running,
    Stopped,
    Paused,
    Other,
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceState::Running => write!(f, "Running"),
            ServiceState::Stopped => write!(f, "Stopped"),
            ServiceState::Paused => write!(f, "Paused"),
            ServiceState::Other => write!(f, "Other"),
        }
    }
}

/// Service list response
#[derive(Clone, Debug)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceInfo>,
    pub count: usize,
}

/// Registry entry information
#[derive(Clone, Debug)]
pub struct RegistryEntryInfo {
    pub name: String,
    pub value_type: String,
    pub data: String,
}

/// Registry list response
#[derive(Clone, Debug)]
pub struct RegistryListResponse {
    pub path: String,
    pub entries: Vec<RegistryEntryInfo>,
    pub count: usize,
}

/// Registry operation result
#[derive(Clone, Debug)]
pub struct RegistryOpResult {
    pub success: bool,
    pub message: String,
}

/// Script execution result
#[derive(Clone, Debug)]
pub struct ScriptExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
}

/// Generic operation result
#[derive(Clone, Debug)]
pub struct OperationResult {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

impl OperationResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            details: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Union type for CLI command responses (for async channel transmission)
#[derive(Clone, Debug)]
pub enum CliResponse {
    ServiceList(Result<ServiceListResponse, String>),
    ServiceOp(Result<OperationResult, String>),
    RegistryList(Result<RegistryListResponse, String>),
    RegistryOp(Result<RegistryOpResult, String>),
    ScriptExec(Result<ScriptExecResult, String>),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_info_creation() {
        let service = ServiceInfo {
            name: "test".to_string(),
            display_name: "Test Service".to_string(),
            state: ServiceState::Running,
            path: "C:\\test.exe".to_string(),
        };
        assert_eq!(service.name, "test");
        assert_eq!(service.state, ServiceState::Running);
    }

    #[test]
    fn test_service_state_display() {
        assert_eq!(ServiceState::Running.to_string(), "Running");
        assert_eq!(ServiceState::Stopped.to_string(), "Stopped");
    }

    #[test]
    fn test_operation_result() {
        let result = OperationResult::success("Operation completed");
        assert!(result.success);
        assert_eq!(result.message, "Operation completed");

        let error = OperationResult::error("Operation failed");
        assert!(!error.success);
    }

    #[test]
    fn test_registry_entry() {
        let entry = RegistryEntryInfo {
            name: "TestKey".to_string(),
            value_type: "REG_SZ".to_string(),
            data: "TestValue".to_string(),
        };
        assert_eq!(entry.name, "TestKey");
        assert_eq!(entry.value_type, "REG_SZ");
    }
}
