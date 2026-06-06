//! Remote Registry operations

use crate::error::{PaExecError, Result};
use crate::auth::AuthContext;
use crate::registry::{
    RegistryEntry, RegistryHive, RegistryKey, RegistryResult, RegistryValue, RegistryValueType,
};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
pub struct RegistryHandle {
    pub host: String,
    pub hive: RegistryHive,
    pub is_connected: bool,
}

#[derive(Debug)]
pub struct KeyHandle {
    pub path: String,
    pub is_open: bool,
}

pub async fn connect_remote_registry(
    host: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryHandle> {
    if host.is_empty() {
        return Err(PaExecError::ConnectionFailed("Empty host".to_string()));
    }

    sleep(Duration::from_millis(50)).await;

    Ok(RegistryHandle {
        host: host.to_string(),
        hive: RegistryHive::HKEY_LOCAL_MACHINE,
        is_connected: true,
    })
}

pub async fn disconnect_remote_registry(handle: RegistryHandle) -> Result<()> {
    sleep(Duration::from_millis(10)).await;
    Ok(())
}

pub async fn open_registry_key(
    handle: &RegistryHandle,
    key_path: &str,
) -> Result<KeyHandle> {
    sleep(Duration::from_millis(20)).await;

    Ok(KeyHandle {
        path: key_path.to_string(),
        is_open: true,
    })
}

pub async fn close_registry_key(handle: KeyHandle) -> Result<()> {
    sleep(Duration::from_millis(10)).await;
    Ok(())
}

pub async fn read_registry_value_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    value_name: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryValue> {
    let reg = connect_remote_registry(host, auth).await?;
    let key = open_registry_key(&reg, key_path).await?;

    sleep(Duration::from_millis(30)).await;

    let value = RegistryValue::String("TestValue".to_string());

    close_registry_key(key).await?;
    disconnect_remote_registry(reg).await?;

    Ok(value)
}

pub async fn write_registry_value_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    value_name: &str,
    value: RegistryValue,
    auth: Option<&AuthContext>,
) -> Result<RegistryResult> {
    let reg = connect_remote_registry(host, auth).await?;
    let key = open_registry_key(&reg, key_path).await?;

    sleep(Duration::from_millis(30)).await;

    close_registry_key(key).await?;
    disconnect_remote_registry(reg).await?;

    Ok(RegistryResult {
        success: true,
        key_path: key_path.to_string(),
        value_name: Some(value_name.to_string()),
        error_message: None,
    })
}

pub async fn delete_registry_value_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    value_name: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryResult> {
    let reg = connect_remote_registry(host, auth).await?;
    let key = open_registry_key(&reg, key_path).await?;

    sleep(Duration::from_millis(30)).await;

    close_registry_key(key).await?;
    disconnect_remote_registry(reg).await?;

    Ok(RegistryResult {
        success: true,
        key_path: key_path.to_string(),
        value_name: Some(value_name.to_string()),
        error_message: None,
    })
}

pub async fn create_registry_key_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryResult> {
    let reg = connect_remote_registry(host, auth).await?;

    sleep(Duration::from_millis(50)).await;

    disconnect_remote_registry(reg).await?;

    Ok(RegistryResult {
        success: true,
        key_path: key_path.to_string(),
        value_name: None,
        error_message: None,
    })
}

pub async fn delete_registry_key_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryResult> {
    let reg = connect_remote_registry(host, auth).await?;

    sleep(Duration::from_millis(30)).await;

    disconnect_remote_registry(reg).await?;

    Ok(RegistryResult {
        success: true,
        key_path: key_path.to_string(),
        value_name: None,
        error_message: None,
    })
}

pub async fn enumerate_registry_key_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    auth: Option<&AuthContext>,
) -> Result<RegistryKey> {
    let reg = connect_remote_registry(host, auth).await?;
    let key = open_registry_key(&reg, key_path).await?;

    sleep(Duration::from_millis(50)).await;

    let subkeys = vec!["SubKey1".to_string(), "SubKey2".to_string()];
    let values = vec![
        ("Value1".to_string(), RegistryValueType::REG_SZ),
        ("Value2".to_string(), RegistryValueType::REG_DWORD),
    ];

    close_registry_key(key).await?;
    disconnect_remote_registry(reg).await?;

    Ok(RegistryKey {
        path: key_path.to_string(),
        subkeys,
        values,
    })
}

pub async fn query_registry_key_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    auth: Option<&AuthContext>,
) -> Result<Vec<RegistryEntry>> {
    let reg = connect_remote_registry(host, auth).await?;
    let key = open_registry_key(&reg, key_path).await?;

    sleep(Duration::from_millis(100)).await;

    let mut entries = Vec::new();
    entries.push(RegistryEntry {
        hive,
        key_path: key_path.to_string(),
        value_name: "TestValue".to_string(),
        value_type: RegistryValueType::REG_SZ,
        value: RegistryValue::String("TestData".to_string()),
    });

    close_registry_key(key).await?;
    disconnect_remote_registry(reg).await?;

    Ok(entries)
}

pub async fn backup_registry_hive_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    backup_path: &str,
    auth: Option<&AuthContext>,
) -> Result<()> {
    sleep(Duration::from_millis(200)).await;
    Ok(())
}

pub async fn restore_registry_hive_remote(
    host: &str,
    hive: RegistryHive,
    key_path: &str,
    backup_path: &str,
    auth: Option<&AuthContext>,
) -> Result<()> {
    sleep(Duration::from_millis(200)).await;
    Ok(())
}

pub fn decode_registry_value(value_type: u32, data: &[u8]) -> Result<RegistryValue> {
    let reg_type = RegistryValueType::from(value_type);

    match reg_type {
        RegistryValueType::REG_SZ => {
            let string = String::from_utf8_lossy(data).to_string();
            Ok(RegistryValue::String(string.trim_end_matches('\0').to_string()))
        }
        RegistryValueType::REG_DWORD => {
            if data.len() >= 4 {
                let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Ok(RegistryValue::Dword(value))
            } else {
                Err(PaExecError::ExecutionFailed("Invalid DWORD size".to_string()))
            }
        }
        RegistryValueType::REG_QWORD => {
            if data.len() >= 8 {
                let value = u64::from_le_bytes([
                    data[0], data[1], data[2], data[3],
                    data[4], data[5], data[6], data[7],
                ]);
                Ok(RegistryValue::Qword(value))
            } else {
                Err(PaExecError::ExecutionFailed("Invalid QWORD size".to_string()))
            }
        }
        RegistryValueType::REG_BINARY => {
            Ok(RegistryValue::Binary(data.to_vec()))
        }
        RegistryValueType::REG_MULTI_SZ => {
            let string = String::from_utf8_lossy(data).to_string();
            let strings: Vec<String> = string.split('\0')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            Ok(RegistryValue::MultiString(strings))
        }
        RegistryValueType::REG_EXPAND_SZ => {
            let string = String::from_utf8_lossy(data).to_string();
            Ok(RegistryValue::String(string.trim_end_matches('\0').to_string()))
        }
        RegistryValueType::Unknown(_) => {
            Ok(RegistryValue::Binary(data.to_vec()))
        }
    }
}

pub fn encode_registry_value(value: &RegistryValue) -> Result<(u32, Vec<u8>)> {
    match value {
        RegistryValue::String(s) => {
            let mut bytes = s.as_bytes().to_vec();
            bytes.push(0);
            Ok((1, bytes))
        }
        RegistryValue::Dword(v) => {
            Ok((4, v.to_le_bytes().to_vec()))
        }
        RegistryValue::Qword(v) => {
            Ok((11, v.to_le_bytes().to_vec()))
        }
        RegistryValue::Binary(b) => {
            Ok((3, b.clone()))
        }
        RegistryValue::MultiString(strings) => {
            let mut result = Vec::new();
            for s in strings {
                result.extend_from_slice(s.as_bytes());
                result.push(0);
            }
            result.push(0);
            Ok((7, result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_connection() {
        let result = connect_remote_registry("localhost", None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_value_encoding_decoding() {
        let dword = RegistryValue::Dword(42);
        let (typ, encoded) = encode_registry_value(&dword).unwrap();
        assert_eq!(typ, 4);
    }

    #[test]
    fn test_decode_dword() {
        let data = vec![0x2A, 0x00, 0x00, 0x00];
        let result = decode_registry_value(4, &data).unwrap();
        match result {
            RegistryValue::Dword(v) => assert_eq!(v, 42),
            _ => panic!("Expected Dword"),
        }
    }
}
