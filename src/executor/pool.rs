//! Connection pooling for remote execution

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Pooled connection
pub struct PooledConnection<T> {
    pub inner: T,
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
}

impl<T> PooledConnection<T> {
    pub fn new(connection: T) -> Self {
        let now = Instant::now();
        Self {
            inner: connection,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }

    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    pub fn is_idle(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub min_size: usize,
    pub max_age_seconds: u64,
    pub max_idle_seconds: u64,
    pub health_check_interval_seconds: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_size: 2,
            max_age_seconds: 300,
            max_idle_seconds: 60,
            health_check_interval_seconds: 30,
        }
    }
}

/// Generic connection pool
pub struct ConnectionPool<T> {
    _config: PoolConfig,
    _connections: Arc<RwLock<HashMap<String, Arc<Mutex<PooledConnection<T>>>>>>,
}

impl<T: Send + 'static> ConnectionPool<T> {
    pub fn new(_config: PoolConfig) -> Self {
        Self {
            _config,
            _connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn stats(&self) -> PoolStats {
        let _connections = self._connections.read().await;
        PoolStats {
            total_connections: _connections.len(),
            config: self._config.clone(),
        }
    }
}

#[derive(Debug)]
pub struct PoolStats {
    pub total_connections: usize,
    pub config: PoolConfig,
}

/// Failover manager for multiple endpoints
pub struct FailoverManager<T> {
    endpoints: Vec<T>,
    current_index: usize,
    health_status: HashMap<usize, bool>,
}

impl<T: Clone> FailoverManager<T> {
    pub fn new(endpoints: Vec<T>) -> Self {
        let mut health_status = HashMap::new();
        for i in 0..endpoints.len() {
            health_status.insert(i, true);
        }

        Self {
            endpoints,
            current_index: 0,
            health_status,
        }
    }

    pub fn get_next(&mut self) -> Option<T> {
        let start = self.current_index;

        loop {
            if *self.health_status.get(&self.current_index).unwrap_or(&false) {
                let endpoint = self.endpoints.get(self.current_index).cloned();
                self.current_index = (self.current_index + 1) % self.endpoints.len();
                return endpoint;
            }

            self.current_index = (self.current_index + 1) % self.endpoints.len();

            if self.current_index == start {
                return None;
            }
        }
    }

    pub fn mark_failed(&mut self, index: usize) {
        self.health_status.insert(index, false);
    }

    pub fn mark_healthy(&mut self, index: usize) {
        self.health_status.insert(index, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pooled_connection() {
        let conn = PooledConnection::new("test_host".to_string());
        assert_eq!(conn.use_count, 0);
        assert!(!conn.is_expired(Duration::from_secs(60)));
        assert!(!conn.is_idle(Duration::from_secs(1)));
    }

    #[test]
    fn test_failover_manager() {
        let endpoints = vec!["host1".to_string(), "host2".to_string(), "host3".to_string()];
        let mut manager = FailoverManager::new(endpoints);

        // Get next healthy endpoint
        assert_eq!(manager.get_next(), Some("host1".to_string()));
        assert_eq!(manager.get_next(), Some("host2".to_string()));

        // Mark one as failed
        manager.mark_failed(1);
        assert_eq!(manager.get_next(), Some("host3".to_string()));
        assert_eq!(manager.get_next(), Some("host1".to_string()));
    }
}
