//! File transfer module for remote file operations
//! Supports SMB/Admin$ share uploads and downloads

use crate::error::{PaExecError, Result};
use crate::auth::AuthContext;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod chunks;
pub mod smb;

pub use chunks::ChunkManager;
pub use smb::*;

/// Direction of file transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Local to remote (upload)
    Upload,
    /// Remote to local (download)
    Download,
}

/// Method of file transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferMethod {
    /// SMB Admin$ share (C$\Windows\PAExec)
    SMBAdminShare,
    /// Generic UNC path
    UNCPath,
}

/// Context for file transfer operations
#[derive(Debug, Clone)]
pub struct FileTransferContext {
    pub direction: TransferDirection,
    pub source_path: String,
    pub dest_path: String,
    pub method: TransferMethod,
    pub chunk_size: usize,
    pub follow_symlinks: bool,
    pub auth: Option<AuthContext>,
}

impl FileTransferContext {
    /// Create a new file transfer context
    pub fn new(direction: TransferDirection, src: &str, dst: &str) -> Self {
        Self {
            direction,
            source_path: src.to_string(),
            dest_path: dst.to_string(),
            method: TransferMethod::SMBAdminShare,
            chunk_size: 10 * 1024 * 1024, // 10MB default
            follow_symlinks: false,
            auth: None,
        }
    }

    /// Set custom chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Set transfer method
    pub fn with_method(mut self, method: TransferMethod) -> Self {
        self.method = method;
        self
    }

    /// Set authentication context
    pub fn with_auth(mut self, auth: AuthContext) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Enable symlink following
    pub fn with_follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }
}

/// Result of a file transfer operation
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub bytes_transferred: u64,
    pub files_count: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

impl TransferResult {
    pub fn success(bytes: u64, files: usize) -> Self {
        Self {
            bytes_transferred: bytes,
            files_count: files,
            success: true,
            error_message: None,
        }
    }

    pub fn failure(error: &str) -> Self {
        Self {
            bytes_transferred: 0,
            files_count: 0,
            success: false,
            error_message: Some(error.to_string()),
        }
    }
}

/// Metadata for remote files
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: String,
}

/// Execute file transfer based on context
pub async fn transfer(ctx: &FileTransferContext) -> Result<TransferResult> {
    match ctx.method {
        TransferMethod::SMBAdminShare | TransferMethod::UNCPath => {
            smb::transfer_smb(ctx).await
        }
    }
}

/// Transfer a single file
pub async fn transfer_file(ctx: &FileTransferContext, file_path: &str) -> Result<u64> {
    let mut ctx = ctx.clone();
    ctx.source_path = file_path.to_string();

    match ctx.method {
        TransferMethod::SMBAdminShare | TransferMethod::UNCPath => {
            smb::transfer_file_smb(&ctx).await
        }
    }
}

/// Transfer entire directory
pub async fn transfer_directory(ctx: &FileTransferContext, dir_path: &str) -> Result<TransferResult> {
    let mut ctx = ctx.clone();
    ctx.source_path = dir_path.to_string();

    match ctx.method {
        TransferMethod::SMBAdminShare | TransferMethod::UNCPath => {
            smb::transfer_directory_smb(&ctx).await
        }
    }
}

/// List remote directory contents
pub async fn list_remote_directory(
    host: &str,
    path: &str,
    auth: Option<&AuthContext>,
) -> Result<Vec<FileMetadata>> {
    smb::list_directory_smb(host, path, auth).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_context_creation() {
        let ctx = FileTransferContext::new(
            TransferDirection::Upload,
            "/local/file.txt",
            "C$\\Windows\\PAExec\\file.txt"
        );

        assert_eq!(ctx.direction, TransferDirection::Upload);
        assert_eq!(ctx.source_path, "/local/file.txt");
        assert_eq!(ctx.dest_path, "C$\\Windows\\PAExec\\file.txt");
        assert_eq!(ctx.chunk_size, 10 * 1024 * 1024);
        assert_eq!(ctx.method, TransferMethod::SMBAdminShare);
    }

    #[test]
    fn test_chunk_size_customization() {
        let ctx = FileTransferContext::new(
            TransferDirection::Download,
            "remote",
            "local"
        ).with_chunk_size(5 * 1024 * 1024); // 5MB

        assert_eq!(ctx.chunk_size, 5 * 1024 * 1024);
    }

    #[test]
    fn test_transfer_result_success() {
        let result = TransferResult::success(1024, 1);
        assert!(result.success);
        assert_eq!(result.bytes_transferred, 1024);
        assert_eq!(result.files_count, 1);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_transfer_result_failure() {
        let result = TransferResult::failure("Connection refused");
        assert!(!result.success);
        assert_eq!(result.error_message, Some("Connection refused".to_string()));
    }
}
