//! Logging and caching module for execution tracking

use crate::error::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Log entry for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    #[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
    pub timestamp: DateTime<Local>,
    pub computer: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub success: bool,
}

fn serialize_datetime<S>(dt: &DateTime<Local>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = dt.to_rfc3339();
    serializer.serialize_str(&s)
}

fn deserialize_datetime<'de, D>(deserializer: D) -> std::result::Result<DateTime<Local>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(Error::custom)
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log file path
    pub log_file: Option<PathBuf>,
    /// Maximum log file size (bytes)
    pub max_file_size: u64,
    /// Number of log files to keep
    pub max_files: usize,
    /// Log level
    pub log_level: LogLevel,
    /// Include stdout/stderr in log
    pub capture_output: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_file: None,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            log_level: LogLevel::Info,
            capture_output: true,
        }
    }
}

/// File logger
pub struct FileLogger {
    config: LoggerConfig,
    current_size: Arc<RwLock<u64>>,
}

impl FileLogger {
    pub fn new(config: LoggerConfig) -> Result<Self> {
        // Create log directory if needed
        if let Some(ref path) = config.log_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(Self {
            config,
            current_size: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn log(&self, entry: &ExecutionLogEntry) -> Result<()> {
        if self.config.log_file.is_none() {
            return Ok(());
        }

        // Check if we need to rotate
        self.check_rotation().await?;

        // Format log entry
        let log_line = format!(
            "[{}] {} | {} {} | Exit: {} | Duration: {}ms\n",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            entry.computer,
            entry.command,
            entry.arguments.join(" "),
            entry.exit_code,
            entry.duration_ms
        );

        // Write to file
        let path = self.config.log_file.as_ref().unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        file.write_all(log_line.as_bytes())?;

        if self.config.capture_output {
            if !entry.stdout.is_empty() {
                file.write_all(b"STDOUT:\n")?;
                file.write_all(entry.stdout.as_bytes())?;
                file.write_all(b"\n")?;
            }
            if !entry.stderr.is_empty() {
                file.write_all(b"STDERR:\n")?;
                file.write_all(entry.stderr.as_bytes())?;
                file.write_all(b"\n")?;
            }
            file.write_all(b"---\n")?;
        }

        // Update size
        let mut size = self.current_size.write().await;
        *size += log_line.len() as u64;

        Ok(())
    }

    async fn check_rotation(&self) -> Result<()> {
        let size = *self.current_size.read().await;

        if size >= self.config.max_file_size {
            self.rotate_files().await?;
        }

        Ok(())
    }

    async fn rotate_files(&self) -> Result<()> {
        let base_path = self.config.log_file.as_ref().unwrap();
        let base_str = base_path.to_string_lossy();

        // Remove oldest file
        let oldest = format!("{}.{}", base_str, self.config.max_files);
        let _ = fs::remove_file(&oldest);

        // Shift existing files
        for i in (1..self.config.max_files).rev() {
            let from = format!("{}.{}", base_str, i);
            let to = format!("{}.{}", base_str, i + 1);
            let _ = fs::rename(&from, &to);
        }

        // Rename current to .1
        let backup = format!("{}.1", base_str);
        let _ = fs::rename(base_path, &backup);

        // Reset size
        *self.current_size.write().await = 0;

        Ok(())
    }
}

/// Global cache for execution results
pub struct ExecutionCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    ttl_seconds: u64,
}

#[derive(Debug, Clone)]
struct CachedResult {
    entry: ExecutionLogEntry,
    cached_at: DateTime<Local>,
}

impl ExecutionCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    pub async fn get(&self, key: &str) -> Option<ExecutionLogEntry> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(key) {
            let age = Local::now().signed_duration_since(cached.cached_at);
            if age.num_seconds() < self.ttl_seconds as i64 {
                return Some(cached.entry.clone());
            }
        }

        None
    }

    pub async fn set(&self, key: &str, entry: ExecutionLogEntry) {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), CachedResult {
            entry,
            cached_at: Local::now(),
        });
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Local::now();

        cache.retain(|_, v| {
            let age = now.signed_duration_since(v.cached_at);
            age.num_seconds() < self.ttl_seconds as i64
        });
    }
}

/// Combined logger with caching
pub struct ExecutionLogger {
    file_logger: Option<FileLogger>,
    cache: ExecutionCache,
}

impl ExecutionLogger {
    pub fn new(file_config: Option<LoggerConfig>, cache_ttl_seconds: u64) -> Result<Self> {
        let file_logger = match file_config {
            Some(config) => Some(FileLogger::new(config)?),
            None => None,
        };

        Ok(Self {
            file_logger,
            cache: ExecutionCache::new(cache_ttl_seconds),
        })
    }

    pub async fn log_execution(&self, entry: ExecutionLogEntry) -> Result<()> {
        // Log to file
        if let Some(ref logger) = self.file_logger {
            logger.log(&entry).await?;
        }

        // Cache result
        let cache_key = format!("{}:{}", entry.computer, entry.command);
        self.cache.set(&cache_key, entry).await;

        Ok(())
    }

    pub async fn get_cached(&self, computer: &str, command: &str) -> Option<ExecutionLogEntry> {
        let key = format!("{}:{}", computer, command);
        self.cache.get(&key).await
    }

    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_cache() {
        let cache = ExecutionCache::new(60);

        let entry = ExecutionLogEntry {
            timestamp: Local::now(),
            computer: "server1".to_string(),
            command: "cmd".to_string(),
            arguments: vec![],
            exit_code: 0,
            stdout: "".to_string(),
            stderr: "".to_string(),
            duration_ms: 100,
            success: true,
        };

        cache.set("key1", entry.clone()).await;

        let retrieved = cache.get("key1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().computer, "server1");

        // Non-existent key
        assert!(cache.get("nonexistent").await.is_none());
    }
}
