//! Memory Pool System - Phase 4.6
//! Object pooling for Vec<u8> and String allocation optimization

use std::sync::Mutex;

/// Memory pool for reusing allocations
pub struct MemoryPool<T> {
    pool: Mutex<Vec<T>>,
    max_size: usize,
}

impl<T> MemoryPool<T> {
    /// Create new memory pool with maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get object from pool or create new
    pub fn acquire(&self) -> T
    where
        T: Default,
    {
        let mut pool = self.pool.lock().unwrap();
        pool.pop().unwrap_or_default()
    }

    /// Return object to pool for reuse
    pub fn release(&self, item: T) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_size {
            pool.push(item);
        }
    }

    /// Get current pool size
    pub fn pool_size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }

    /// Clear all pooled objects
    pub fn clear(&self) {
        self.pool.lock().unwrap().clear();
    }
}

/// Specialized pool for Vec<u8>
pub struct ByteBufferPool {
    pool: Mutex<Vec<Vec<u8>>>,
    max_pool_size: usize,
    max_buffer_size: usize,
}

impl ByteBufferPool {
    /// Create new byte buffer pool
    pub fn new(max_pool_size: usize, max_buffer_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::new()),
            max_pool_size,
            max_buffer_size,
        }
    }

    /// Acquire buffer from pool
    pub fn acquire(&self) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap();
        pool.pop().unwrap_or_else(|| Vec::with_capacity(self.max_buffer_size))
    }

    /// Release buffer back to pool
    pub fn release(&self, mut buffer: Vec<u8>) {
        // Reuse buffer if it's not too large
        if buffer.capacity() <= self.max_buffer_size {
            buffer.clear();
            let mut pool = self.pool.lock().unwrap();
            if pool.len() < self.max_pool_size {
                pool.push(buffer);
            }
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let pool = self.pool.lock().unwrap();
        PoolStats {
            pool_count: pool.len(),
            total_capacity: pool.iter().map(|b| b.capacity()).sum(),
        }
    }
}

/// Specialized pool for String
pub struct StringPool {
    pool: Mutex<Vec<String>>,
    max_pool_size: usize,
}

impl StringPool {
    /// Create new string pool
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::new()),
            max_pool_size,
        }
    }

    /// Acquire string from pool
    pub fn acquire(&self) -> String {
        let mut pool = self.pool.lock().unwrap();
        pool.pop().unwrap_or_default()
    }

    /// Release string back to pool
    pub fn release(&self, mut string: String) {
        string.clear();
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push(string);
        }
    }

    /// Get pool size
    pub fn pool_size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub pool_count: usize,
    pub total_capacity: usize,
}

impl PoolStats {
    /// Average buffer size in pool
    pub fn average_capacity(&self) -> usize {
        if self.pool_count == 0 {
            0
        } else {
            self.total_capacity / self.pool_count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_acquire_release() {
        let pool: MemoryPool<Vec<u8>> = MemoryPool::new(10);

        let buffer = pool.acquire();
        assert_eq!(pool.pool_size(), 0);

        pool.release(buffer);
        assert_eq!(pool.pool_size(), 1);
    }

    #[test]
    fn test_memory_pool_max_size() {
        let pool: MemoryPool<Vec<u8>> = MemoryPool::new(2);

        let b1 = pool.acquire();
        let b2 = pool.acquire();
        let b3 = pool.acquire();

        pool.release(b1);
        pool.release(b2);
        pool.release(b3); // Should be dropped, pool is full

        assert_eq!(pool.pool_size(), 2);
    }

    #[test]
    fn test_byte_buffer_pool() {
        let pool = ByteBufferPool::new(5, 1024);

        let mut buffer = pool.acquire();
        buffer.extend_from_slice(b"Hello");

        pool.release(buffer);

        let buffer2 = pool.acquire();
        assert!(buffer2.is_empty()); // Cleared before reuse
    }

    #[test]
    fn test_byte_buffer_pool_stats() {
        let pool = ByteBufferPool::new(3, 256);

        let b1 = pool.acquire();
        let b2 = pool.acquire();

        pool.release(b1);
        pool.release(b2);

        let stats = pool.stats();
        assert_eq!(stats.pool_count, 2);
    }

    #[test]
    fn test_string_pool() {
        let pool = StringPool::new(5);

        let mut s = pool.acquire();
        s.push_str("Hello");

        pool.release(s);

        let s2 = pool.acquire();
        assert!(s2.is_empty());
        assert_eq!(pool.pool_size(), 0);
    }

    #[test]
    fn test_string_pool_reuse() {
        let pool = StringPool::new(3);

        let s1 = pool.acquire();
        pool.release(s1);
        assert_eq!(pool.pool_size(), 1);

        let s2 = pool.acquire();
        assert_eq!(pool.pool_size(), 0);
    }
}
