//! Chunk management for large file transfers
//! Handles splitting, hashing, and progress tracking

use crate::error::{PaExecError, Result};
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Information about a single chunk
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub index: usize,
    pub offset: u64,
    pub size: usize,
    pub hash: String,
    pub transferred: bool,
}

/// Manager for file chunking operations
#[derive(Debug)]
pub struct ChunkManager {
    pub total_size: u64,
    pub chunk_size: usize,
    pub chunk_count: usize,
    chunks: Vec<ChunkInfo>,
    transferred_count: usize,
}

impl ChunkManager {
    /// Create new chunk manager
    pub fn new(file_size: u64, chunk_size: usize) -> Self {
        let chunk_count = ((file_size + chunk_size as u64 - 1) / chunk_size as u64) as usize;

        let mut chunks = Vec::with_capacity(chunk_count);
        for i in 0..chunk_count {
            let offset = i as u64 * chunk_size as u64;
            let size = std::cmp::min(chunk_size, (file_size - offset) as usize);

            chunks.push(ChunkInfo {
                index: i,
                offset,
                size,
                hash: String::new(), // Will be computed later
                transferred: false,
            });
        }

        Self {
            total_size: file_size,
            chunk_size,
            chunk_count,
            chunks,
            transferred_count: 0,
        }
    }

    /// Create from existing chunk info
    pub fn from_chunks(file_size: u64, chunk_size: usize, chunks: Vec<ChunkInfo>) -> Self {
        let transferred_count = chunks.iter().filter(|c| c.transferred).count();

        Self {
            total_size: file_size,
            chunk_size,
            chunk_count: chunks.len(),
            chunks,
            transferred_count,
        }
    }

    /// Get next pending chunk
    pub fn next_chunk(&mut self) -> Option<&ChunkInfo> {
        self.chunks.iter().find(|c| !c.transferred)
    }

    /// Mark chunk as transferred
    pub fn mark_transferred(&mut self, index: usize) -> Result<()> {
        if let Some(chunk) = self.chunks.get_mut(index) {
            if !chunk.transferred {
                chunk.transferred = true;
                self.transferred_count += 1;
            }
            Ok(())
        } else {
            Err(PaExecError::FileTransferFailed(format!(
                "Invalid chunk index: {}", index
            )))
        }
    }

    /// Check if all chunks are transferred
    pub fn all_transferred(&self) -> bool {
        self.transferred_count >= self.chunk_count
    }

    /// Get transfer progress percentage
    pub fn progress_percent(&self) -> f32 {
        if self.chunk_count == 0 {
            return 100.0;
        }
        (self.transferred_count as f32 / self.chunk_count as f32) * 100.0
    }

    /// Compute SHA256 hash of chunk data
    pub fn compute_chunk_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[ChunkInfo] {
        &self.chunks
    }

    /// Get transferred bytes count
    pub fn transferred_bytes(&self) -> u64 {
        self.chunks
            .iter()
            .filter(|c| c.transferred)
            .map(|c| c.size as u64)
            .sum()
    }
}

/// Split file into chunks
pub async fn split_file(path: &str, chunk_size: usize) -> Result<Vec<ChunkInfo>> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| PaExecError::FileTransferFailed(format!("Cannot get metadata: {}", e)))?;

    let file_size = metadata.len();
    let chunk_count = ((file_size + chunk_size as u64 - 1) / chunk_size as u64) as usize;

    let mut chunks = Vec::with_capacity(chunk_count);

    // Open file for reading
    let mut file = File::open(path)
        .map_err(|e| PaExecError::FileTransferFailed(format!("Cannot open file: {}", e)))?;

    for i in 0..chunk_count {
        let offset = i as u64 * chunk_size as u64;
        let size = std::cmp::min(chunk_size, (file_size - offset) as usize);

        // Read chunk data
        let mut buffer = vec![0u8; size];
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| PaExecError::FileTransferFailed(format!("Seek failed: {}", e)))?;
        file.read_exact(&mut buffer)
            .map_err(|e| PaExecError::FileTransferFailed(format!("Read failed: {}", e)))?;

        // Compute hash
        let hash = ChunkManager::compute_chunk_hash(&buffer);

        chunks.push(ChunkInfo {
            index: i,
            offset,
            size,
            hash,
            transferred: false,
        });
    }

    Ok(chunks)
}

/// Merge chunks into single file
pub async fn merge_chunks(chunk_paths: Vec<&str>, output_path: &str) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut output = tokio::fs::File::create(output_path).await
        .map_err(|e| PaExecError::FileTransferFailed(format!("Cannot create output: {}", e)))?;

    for (i, chunk_path) in chunk_paths.iter().enumerate() {
        let data = tokio::fs::read(chunk_path).await
            .map_err(|e| PaExecError::FileTransferFailed(format!(
                "Cannot read chunk {}: {}", i, e
            )))?;

        output.write_all(&data).await
            .map_err(|e| PaExecError::FileTransferFailed(format!(
                "Cannot write chunk {}: {}", i, e
            )))?;
    }

    output.flush().await
        .map_err(|e| PaExecError::FileTransferFailed(format!("Flush failed: {}", e)))?;

    Ok(())
}

/// Calculate SHA256 hash of entire file
pub fn calculate_file_hash(path: &str) -> Result<String> {
    let mut file = File::open(path)
        .map_err(|e| PaExecError::FileTransferFailed(format!("Cannot open file: {}", e)))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)
            .map_err(|e| PaExecError::FileTransferFailed(format!("Read failed: {}", e)))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify transfer integrity by comparing hashes
pub async fn verify_transfer(local_path: &str, remote_hash: &str) -> Result<bool> {
    let local_hash = calculate_file_hash(local_path)?;
    Ok(local_hash == remote_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_calculation() {
        // 25MB file with 10MB chunks = 3 chunks
        let manager = ChunkManager::new(25 * 1024 * 1024, 10 * 1024 * 1024);
        assert_eq!(manager.chunk_count, 3);
        assert_eq!(manager.chunks[0].size, 10 * 1024 * 1024);
        assert_eq!(manager.chunks[1].size, 10 * 1024 * 1024);
        assert_eq!(manager.chunks[2].size, 5 * 1024 * 1024);
    }

    #[test]
    fn test_chunk_hash_computation() {
        let data = b"test data for hashing";
        let hash = ChunkManager::compute_chunk_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 is 64 hex chars

        // Verify consistency
        let hash2 = ChunkManager::compute_chunk_hash(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_progress_percentage() {
        let mut manager = ChunkManager::new(100, 10); // 10 chunks

        assert_eq!(manager.progress_percent(), 0.0);

        manager.mark_transferred(0).unwrap();
        let progress = manager.progress_percent();
        assert!((progress - 10.0).abs() < 0.01);

        manager.mark_transferred(1).unwrap();
        manager.mark_transferred(2).unwrap();
        let progress = manager.progress_percent();
        assert!((progress - 30.0).abs() < 0.01);

        // Mark all remaining
        for i in 3..10 {
            manager.mark_transferred(i).unwrap();
        }
        assert_eq!(manager.progress_percent(), 100.0);
        assert!(manager.all_transferred());
    }

    #[test]
    fn test_next_chunk() {
        let mut manager = ChunkManager::new(100, 50); // 2 chunks

        let chunk = manager.next_chunk().unwrap();
        assert_eq!(chunk.index, 0);

        manager.mark_transferred(0).unwrap();

        let chunk = manager.next_chunk().unwrap();
        assert_eq!(chunk.index, 1);

        manager.mark_transferred(1).unwrap();
        assert!(manager.next_chunk().is_none());
    }
}
