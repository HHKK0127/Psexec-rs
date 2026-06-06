//! Windows Registry operations module
//! Provides high-level interface for registry manipulation

use crate::error::{PaExecError, Result};
use crate::auth::AuthContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod remote;

pub use remote::*;

/// Registry hive enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryHive {
    HKEY_LOCAL_MACHINE,
    HKEY_CURRENT_USER,
    HKEY_CLASSES_ROOT,
    HKEY_CURRENT_CONFIG,
    HKEY_USERS,
    HKEY_PERFORMANCE_DATA,
}

impl RegistryHive {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegistryHive::HKEY_LOCAL_MACHINE => "HKEY_LOCAL_MACHINE",
            RegistryHive::HKEY_CURRENT_USER => "HKEY_CURRENT_USER",
            RegistryHive::HKEY_CLASSES_ROOT => "HKEY_CLASSES_ROOT",
            RegistryHive::HKEY_CURRENT_CONFIG => "HKEY_CURRENT_CONFIG",
            RegistryHive::HKEY_USERS => "HKEY_USERS",
            RegistryHive::HKEY_PERFORMANCE_DATA => "HKEY_PERFORMANCE_DATA",
        }
    }
}

/// Registry value type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryValueType {
    REG_SZ,
    REG_DWORD,
    REG_QWORD,
    REG_BINARY,
    REG_MULTI_SZ,
    REG_EXPAND_SZ,
    Unknown(u32),
}

impl From<u32> for RegistryValueType {
    fn from(value: u32) -> Self {
        match value {
            1 => RegistryValueType::REG_SZ,
            4 => RegistryValueType::REG_DWORD,
            11 => RegistryValueType::REG_QWORD,
            3 => RegistryValueType::REG_BINARY,
            7 => RegistryValueType::REG_MULTI_SZ,
            2 => RegistryValueType::REG_EXPAND_SZ,
            _ => RegistryValueType::Unknown(value),
        }
    }
}

impl From<RegistryValueType> for u32 {
    fn from(value: RegistryValueType) -> Self {
        match value {
            RegistryValueType::REG_SZ => 1,
            RegistryValueType::REG_DWORD => 4,
            RegistryValueType::REG_QWORD => 11,
            RegistryValueType::REG_BINARY => 3,
            RegistryValueType::REG_MULTI_SZ => 7,
            RegistryValueType::REG_EXPAND_SZ => 2,
            RegistryValueType::Unknown(v) => v,
        }
    }
}

/// Registry value
#[derive(Debug, Clone)]
pub enum RegistryValue {
    String(String),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    MultiString(Vec<String>),
}

/// Registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub hive: RegistryHive,
    pub key_path: String,
    pub value_name: String,
    pub value_type: RegistryValueType,
    pub value: RegistryValue,
}

/// Registry context
#[derive(Debug, Clone)]
pub struct RegistryContext {
    pub target_host: String,
    pub hive: RegistryHive,
    pub auth: Option<AuthContext>,
}

impl RegistryContext {
    pub fn new(host: &str, hive: RegistryHive) -> Self {
        Self {
            target_host: host.to_string(),
            hive,
            auth: None,
        }
    }

    pub fn with_auth(mut self, auth: AuthContext) -> Self {
        self.auth = Some(auth);
        self
    }
}

/// Registry key information
#[derive(Debug, Clone)]
pub struct RegistryKey {
    pub path: String,
    pub subkeys: Vec<String>,
    pub values: Vec<(String, RegistryValueType)>,
}

/// Registry operation result
#[derive(Debug, Clone)]
pub struct RegistryResult {
    pub success: bool,
    pub key_path: String,
    pub value_name: Option<String>,
    pub error_message: Option<String>,
}

/// Read registry value
pub async fn read_registry_value(
    ctx: &RegistryContext,
    key_path: &str,
    value_name: &str,
) -> Result<RegistryValue> {
    remote::read_registry_value_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        value_name,
        ctx.auth.as_ref(),
    ).await
}

/// Write registry value
pub async fn write_registry_value(
    ctx: &RegistryContext,
    key_path: &str,
    value_name: &str,
    value: RegistryValue,
) -> Result<RegistryResult> {
    remote::write_registry_value_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        value_name,
        value,
        ctx.auth.as_ref(),
    ).await
}

/// Delete registry value
pub async fn delete_registry_value(
    ctx: &RegistryContext,
    key_path: &str,
    value_name: &str,
) -> Result<RegistryResult> {
    remote::delete_registry_value_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        value_name,
        ctx.auth.as_ref(),
    ).await
}

/// Create registry key
pub async fn create_registry_key(
    ctx: &RegistryContext,
    key_path: &str,
) -> Result<RegistryResult> {
    remote::create_registry_key_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        ctx.auth.as_ref(),
    ).await
}

/// Delete registry key
pub async fn delete_registry_key(
    ctx: &RegistryContext,
    key_path: &str,
) -> Result<RegistryResult> {
    remote::delete_registry_key_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        ctx.auth.as_ref(),
    ).await
}

/// Enumerate registry key
pub async fn enumerate_registry_key(
    ctx: &RegistryContext,
    key_path: &str,
) -> Result<RegistryKey> {
    remote::enumerate_registry_key_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        ctx.auth.as_ref(),
    ).await
}

/// Query all values in a key
pub async fn query_registry_key(
    ctx: &RegistryContext,
    key_path: &str,
) -> Result<Vec<RegistryEntry>> {
    remote::query_registry_key_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        ctx.auth.as_ref(),
    ).await
}

/// Backup registry hive
pub async fn backup_registry_hive(
    ctx: &RegistryContext,
    key_path: &str,
    backup_path: &str,
) -> Result<()> {
    remote::backup_registry_hive_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        backup_path,
        ctx.auth.as_ref(),
    ).await
}

/// Restore registry hive
pub async fn restore_registry_hive(
    ctx: &RegistryContext,
    key_path: &str,
    backup_path: &str,
) -> Result<()> {
    remote::restore_registry_hive_remote(
        &ctx.target_host,
        ctx.hive,
        key_path,
        backup_path,
        ctx.auth.as_ref(),
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_context_creation() {
        let ctx = RegistryContext::new("server01", RegistryHive::HKEY_LOCAL_MACHINE);

        assert_eq!(ctx.target_host, "server01");
        assert!(matches!(ctx.hive, RegistryHive::HKEY_LOCAL_MACHINE));
    }

    #[test]
    fn test_registry_value_types() {
        assert_eq!(RegistryValueType::from(1), RegistryValueType::REG_SZ);
        assert_eq!(RegistryValueType::from(4), RegistryValueType::REG_DWORD);
        assert_eq!(RegistryValueType::from(11), RegistryValueType::REG_QWORD);
    }

    #[test]
    fn test_registry_value_encoding() {
        let val = RegistryValue::Dword(42);
        match val {
            RegistryValue::Dword(v) => assert_eq!(v, 42),
            _ => panic!("Expected Dword"),
        }
    }

    #[test]
    fn test_hive_variants() {
        assert_eq!(RegistryHive::HKEY_LOCAL_MACHINE.as_str(), "HKEY_LOCAL_MACHINE");
        assert_eq!(RegistryHive::HKEY_CURRENT_USER.as_str(), "HKEY_CURRENT_USER");
    }
}
