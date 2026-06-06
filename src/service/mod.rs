//! Windows Service management module
//! Provides high-level interface for service operations

use crate::error::{PaExecError, Result};
use crate::auth::AuthContext;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod remote;

pub use remote::*;

/// Service state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceState::Stopped => write!(f, "Stopped"),
            ServiceState::StartPending => write!(f, "Start Pending"),
            ServiceState::StopPending => write!(f, "Stop Pending"),
            ServiceState::Running => write!(f, "Running"),
            ServiceState::ContinuePending => write!(f, "Continue Pending"),
            ServiceState::PausePending => write!(f, "Pause Pending"),
            ServiceState::Paused => write!(f, "Paused"),
            ServiceState::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<u32> for ServiceState {
    fn from(state: u32) -> Self {
        match state {
            1 => ServiceState::Stopped,
            2 => ServiceState::StartPending,
            3 => ServiceState::StopPending,
            4 => ServiceState::Running,
            5 => ServiceState::ContinuePending,
            6 => ServiceState::PausePending,
            7 => ServiceState::Paused,
            _ => ServiceState::Unknown,
        }
    }
}

/// Service startup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStartupType {
    Boot,
    System,
    Automatic,
    AutomaticDelayedStart,
    Manual,
    Disabled,
    Unknown,
}

impl fmt::Display for ServiceStartupType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStartupType::Boot => write!(f, "Boot"),
            ServiceStartupType::System => write!(f, "System"),
            ServiceStartupType::Automatic => write!(f, "Automatic"),
            ServiceStartupType::AutomaticDelayedStart => write!(f, "Automatic (Delayed Start)"),
            ServiceStartupType::Manual => write!(f, "Manual"),
            ServiceStartupType::Disabled => write!(f, "Disabled"),
            ServiceStartupType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<u32> for ServiceStartupType {
    fn from(start_type: u32) -> Self {
        match start_type {
            0 => ServiceStartupType::Boot,
            1 => ServiceStartupType::System,
            2 => ServiceStartupType::Automatic,
            3 => ServiceStartupType::Manual,
            4 => ServiceStartupType::Disabled,
            _ => ServiceStartupType::Unknown,
        }
    }
}

/// Service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: ServiceState,
    pub startup_type: ServiceStartupType,
    pub path: String,
    pub account: String,
    pub dependencies: Vec<String>,
}

/// Context for service operations
#[derive(Debug, Clone)]
pub struct ServiceContext {
    pub target_host: String,
    pub auth: Option<AuthContext>,
}

impl ServiceContext {
    /// Create new service context
    pub fn new(host: &str) -> Self {
        Self {
            target_host: host.to_string(),
            auth: None,
        }
    }

    /// Set authentication
    pub fn with_auth(mut self, auth: AuthContext) -> Self {
        self.auth = Some(auth);
        self
    }
}

/// Service action
#[derive(Debug, Clone)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Pause,
    Resume,
    Create { path: String, display_name: String },
    Delete,
}

/// Service operation result
#[derive(Debug, Clone)]
pub struct ServiceResult {
    pub success: bool,
    pub service_name: String,
    pub previous_state: ServiceState,
    pub new_state: ServiceState,
    pub error_message: Option<String>,
}

impl ServiceResult {
    pub fn success(name: &str, prev: ServiceState, new: ServiceState) -> Self {
        Self {
            success: true,
            service_name: name.to_string(),
            previous_state: prev,
            new_state: new,
            error_message: None,
        }
    }

    pub fn failure(name: &str, error: &str) -> Self {
        Self {
            success: false,
            service_name: name.to_string(),
            previous_state: ServiceState::Unknown,
            new_state: ServiceState::Unknown,
            error_message: Some(error.to_string()),
        }
    }
}

/// List all services on target host
pub async fn list_services(ctx: &ServiceContext) -> Result<Vec<ServiceInfo>> {
    remote::enumerate_services_remote(&ctx.target_host, ctx.auth.as_ref()).await
}

/// Get specific service information
pub async fn get_service(ctx: &ServiceContext, name: &str) -> Result<ServiceInfo> {
    remote::get_service_info_remote(&ctx.target_host, name, ctx.auth.as_ref()).await
}

/// Start a service
pub async fn start_service(ctx: &ServiceContext, name: &str) -> Result<ServiceResult> {
    remote::control_service_remote(
        &ctx.target_host,
        name,
        ServiceAction::Start,
        ctx.auth.as_ref(),
    ).await
}

/// Stop a service
pub async fn stop_service(ctx: &ServiceContext, name: &str) -> Result<ServiceResult> {
    remote::control_service_remote(
        &ctx.target_host,
        name,
        ServiceAction::Stop,
        ctx.auth.as_ref(),
    ).await
}

/// Restart a service
pub async fn restart_service(ctx: &ServiceContext, name: &str) -> Result<ServiceResult> {
    remote::control_service_remote(
        &ctx.target_host,
        name,
        ServiceAction::Restart,
        ctx.auth.as_ref(),
    ).await
}

/// Create a new service
pub async fn create_service(
    ctx: &ServiceContext,
    name: &str,
    display_name: &str,
    path: &str,
    startup_type: ServiceStartupType,
) -> Result<ServiceResult> {
    remote::create_service_remote(
        &ctx.target_host,
        name,
        display_name,
        path,
        startup_type,
        ctx.auth.as_ref(),
    ).await
}

/// Delete a service
pub async fn delete_service(ctx: &ServiceContext, name: &str) -> Result<ServiceResult> {
    remote::control_service_remote(
        &ctx.target_host,
        name,
        ServiceAction::Delete,
        ctx.auth.as_ref(),
    ).await
}

/// Query service status
pub async fn query_service_status(ctx: &ServiceContext, name: &str) -> Result<ServiceState> {
    let info = get_service(ctx, name).await?;
    Ok(info.state)
}

/// Set service startup type
pub async fn set_startup_type(
    ctx: &ServiceContext,
    name: &str,
    startup_type: ServiceStartupType,
) -> Result<()> {
    remote::set_service_startup_type_remote(
        &ctx.target_host,
        name,
        startup_type,
        ctx.auth.as_ref(),
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_context_creation() {
        let ctx = ServiceContext::new("server01");

        assert_eq!(ctx.target_host, "server01");
        assert!(ctx.auth.is_none());
    }

    #[test]
    fn test_service_state_parsing() {
        assert_eq!(ServiceState::from(1), ServiceState::Stopped);
        assert_eq!(ServiceState::from(4), ServiceState::Running);
        assert_eq!(ServiceState::from(999), ServiceState::Unknown);
    }

    #[test]
    fn test_startup_type_variants() {
        assert_eq!(ServiceStartupType::from(2), ServiceStartupType::Automatic);
        assert_eq!(ServiceStartupType::from(3), ServiceStartupType::Manual);
        assert_eq!(ServiceStartupType::from(999), ServiceStartupType::Unknown);
    }

    #[test]
    fn test_service_state_display() {
        assert_eq!(format!("{}", ServiceState::Running), "Running");
        assert_eq!(format!("{}", ServiceState::Stopped), "Stopped");
    }

    #[test]
    fn test_service_result_success() {
        let result = ServiceResult::success("TestService", ServiceState::Stopped, ServiceState::Running);
        assert!(result.success);
        assert_eq!(result.service_name, "TestService");
        assert_eq!(result.previous_state, ServiceState::Stopped);
        assert_eq!(result.new_state, ServiceState::Running);
    }

    #[test]
    fn test_service_result_failure() {
        let result = ServiceResult::failure("TestService", "Access denied");
        assert!(!result.success);
        assert_eq!(result.error_message, Some("Access denied".to_string()));
    }
}
