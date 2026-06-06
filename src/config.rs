//! Configuration management for PAExec-rs GUI

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Timeout in seconds for operations
    pub timeout_seconds: u32,

    /// Enable result caching
    pub enable_caching: bool,

    /// Service host history
    pub service_host_history: Vec<String>,

    /// Last used service host
    pub last_service_host: String,

    /// Registry host history
    pub registry_host_history: Vec<String>,

    /// Last used registry host
    pub last_registry_host: String,

    /// Script type preference
    pub preferred_script_type: String,

    /// Window size (width, height)
    #[serde(default)]
    pub window_size: (u32, u32),
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            enable_caching: false,
            service_host_history: vec!["localhost".to_string()],
            last_service_host: "localhost".to_string(),
            registry_host_history: vec!["localhost".to_string()],
            last_registry_host: "localhost".to_string(),
            preferred_script_type: "powershell".to_string(),
            window_size: (1200, 800),
        }
    }
}

impl AppConfig {
    /// Get the configuration file path
    pub fn config_path() -> PathBuf {
        // Use home directory or current directory
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        home.join(".psexec-rs").join("config.json")
    }

    /// Load configuration from file
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Failed to parse config: {}", e);
                        return Self::default();
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read config: {}", e);
                    return Self::default();
                }
            }
        }

        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        fs::write(&path, json)?;
        Ok(())
    }

    /// Add host to service history
    pub fn add_service_host(&mut self, host: String) {
        if !host.is_empty() && !self.service_host_history.contains(&host) {
            self.service_host_history.insert(0, host.clone());
            if self.service_host_history.len() > 20 {
                self.service_host_history.pop();
            }
        }
        self.last_service_host = host;
    }

    /// Add host to registry history
    pub fn add_registry_host(&mut self, host: String) {
        if !host.is_empty() && !self.registry_host_history.contains(&host) {
            self.registry_host_history.insert(0, host.clone());
            if self.registry_host_history.len() > 20 {
                self.registry_host_history.pop();
            }
        }
        self.last_registry_host = host;
    }

    /// Set preferred script type
    pub fn set_preferred_script_type(&mut self, script_type: String) {
        self.preferred_script_type = script_type;
    }

    /// Update timeout setting
    pub fn set_timeout(&mut self, seconds: u32) {
        self.timeout_seconds = seconds;
    }

    /// Toggle caching setting
    pub fn set_caching(&mut self, enabled: bool) {
        self.enable_caching = enabled;
    }
}

/// Cache entry for API results
#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: std::time::SystemTime,
    pub ttl_seconds: u64,
}

impl<T> CacheEntry<T> {
    /// Check if cache entry is still valid
    pub fn is_valid(&self) -> bool {
        match self.timestamp.elapsed() {
            Ok(elapsed) => elapsed.as_secs() < self.ttl_seconds,
            Err(_) => false,
        }
    }
}

/// Simple cache manager for API results
#[derive(Clone, Debug)]
pub struct ResultCache {
    /// Cache timeout in seconds (default: 60)
    pub ttl_seconds: u64,
}

impl Default for ResultCache {
    fn default() -> Self {
        Self {
            ttl_seconds: 60,
        }
    }
}

impl ResultCache {
    /// Create a new cache with default TTL
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cache entry
    pub fn create_entry<T: Clone>(&self, data: T) -> CacheEntry<T> {
        CacheEntry {
            data,
            timestamp: std::time::SystemTime::now(),
            ttl_seconds: self.ttl_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.timeout_seconds, 30);
        assert!(!config.enable_caching);
        assert_eq!(config.last_service_host, "localhost");
    }

    #[test]
    fn test_add_service_host() {
        let mut config = AppConfig::default();
        config.add_service_host("remote.example.com".to_string());
        assert_eq!(config.service_host_history[0], "remote.example.com");
        assert_eq!(config.last_service_host, "remote.example.com");
    }

    #[test]
    fn test_cache_validity() {
        let cache = ResultCache::default();
        let entry = cache.create_entry("test data");
        assert!(entry.is_valid());
    }

    #[test]
    fn test_set_timeout() {
        let mut config = AppConfig::default();
        config.set_timeout(60);
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_duplicate_host_not_added() {
        let mut config = AppConfig::default();
        let initial_len = config.service_host_history.len();
        config.add_service_host("localhost".to_string());
        assert_eq!(config.service_host_history.len(), initial_len);
    }
}
