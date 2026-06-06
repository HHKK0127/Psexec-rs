//! Output fetching module for remote command execution results
//! Supports Named Pipe and SMB-based output retrieval

use crate::error::{PaExecError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod pipe;
pub mod smb;

pub use pipe::NamedPipeOutputFetcher;
pub use smb::SMBOutputFetcher;

/// Method for fetching output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMethod {
    /// Named Pipe streaming
    NamedPipe,
    /// SMB file-based
    SMBFile,
    /// Real-time streaming
    Streaming,
}

/// Context for output fetching
#[derive(Debug, Clone)]
pub struct OutputFetchContext {
    pub method: OutputMethod,
    pub target_host: String,
    pub session_id: String,
    pub timeout_seconds: u32,
    pub encoding: String,
}

impl OutputFetchContext {
    /// Create new output fetch context
    pub fn new(method: OutputMethod, host: &str, session_id: &str) -> Self {
        Self {
            method,
            target_host: host.to_string(),
            session_id: session_id.to_string(),
            timeout_seconds: 30,
            encoding: "utf-8".to_string(),
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set character encoding
    pub fn with_encoding(mut self, enc: &str) -> Self {
        self.encoding = enc.to_string();
        self
    }
}

/// Result of output fetch operation
#[derive(Debug, Clone)]
pub struct OutputResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub encoding_detected: String,
    pub fetch_time_ms: u64,
}

impl OutputResult {
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
            encoding_detected: "utf-8".to_string(),
            fetch_time_ms: 0,
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output
    pub fn combined_output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Trait for output fetchers
#[async_trait]
pub trait OutputFetcher: Send + Sync {
    /// Fetch complete output
    async fn fetch(&self) -> Result<OutputResult>;

    /// Fetch with streaming callback
    async fn fetch_streaming(&self, callback: Box<dyn Fn(String) + Send>) -> Result<OutputResult>;

    /// Cleanup resources
    async fn cleanup(&self) -> Result<()>;
}

/// Get appropriate output fetcher for context
pub async fn get_output_fetcher(ctx: &OutputFetchContext) -> Result<Box<dyn OutputFetcher>> {
    match ctx.method {
        OutputMethod::NamedPipe | OutputMethod::Streaming => {
            Ok(Box::new(NamedPipeOutputFetcher::new(
                &ctx.target_host,
                &ctx.session_id,
                ctx.timeout_seconds * 1000,
            )))
        }
        OutputMethod::SMBFile => {
            Ok(Box::new(SMBOutputFetcher::new(
                &ctx.target_host,
                &ctx.session_id,
                ctx.timeout_seconds * 1000,
            )))
        }
    }
}

/// Convenience function to fetch output
pub async fn fetch_output(ctx: &OutputFetchContext) -> Result<OutputResult> {
    let fetcher = get_output_fetcher(ctx).await?;
    let result = fetcher.fetch().await;
    fetcher.cleanup().await.ok(); // Best effort cleanup
    result
}

/// Detect encoding from byte data
pub fn detect_encoding(data: &[u8]) -> String {
    // Simple BOM detection
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        "utf-8".to_string()
    } else if data.starts_with(&[0xFF, 0xFE]) {
        "utf-16le".to_string()
    } else if data.starts_with(&[0xFE, 0xFF]) {
        "utf-16be".to_string()
    } else {
        // Default to UTF-8
        "utf-8".to_string()
    }
}

/// Decode bytes with specified encoding
pub fn decode_output(data: &[u8], encoding: &str) -> Result<String> {
    match encoding.to_lowercase().as_str() {
        "utf-8" => {
            String::from_utf8(data.to_vec())
                .map_err(|e| PaExecError::OutputDecodeFailed(format!("UTF-8 decode: {}", e)))
        }
        "utf-16" | "utf-16le" => {
            if data.len() % 2 != 0 {
                return Err(PaExecError::OutputDecodeFailed("Odd byte count for UTF-16".to_string()));
            }
            let u16_data: Vec<u16> = data.chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16(&u16_data)
                .map_err(|e| PaExecError::OutputDecodeFailed(format!("UTF-16 decode: {}", e)))
        }
        "utf-16be" => {
            if data.len() % 2 != 0 {
                return Err(PaExecError::OutputDecodeFailed("Odd byte count for UTF-16BE".to_string()));
            }
            let u16_data: Vec<u16> = data.chunks_exact(2)
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16(&u16_data)
                .map_err(|e| PaExecError::OutputDecodeFailed(format!("UTF-16BE decode: {}", e)))
        }
        "shift_jis" | "sjis" => {
            // Would require encoding_rs crate
            // For now, try UTF-8 fallback
            String::from_utf8(data.to_vec())
                .map_err(|e| PaExecError::OutputDecodeFailed(format!("SJIS decode: {}", e)))
        }
        _ => {
            // Unknown encoding, try UTF-8
            String::from_utf8(data.to_vec())
                .map_err(|e| PaExecError::OutputDecodeFailed(format!("Unknown encoding: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_fetch_context() {
        let ctx = OutputFetchContext::new(
            OutputMethod::NamedPipe,
            "server01",
            "session-123"
        ).with_timeout(60)
         .with_encoding("shift_jis");

        assert_eq!(ctx.target_host, "server01");
        assert_eq!(ctx.session_id, "session-123");
        assert_eq!(ctx.timeout_seconds, 60);
        assert_eq!(ctx.encoding, "shift_jis");
    }

    #[test]
    fn test_encoding_detection() {
        // UTF-8 BOM
        let data = vec![0xEF, 0xBB, 0xBF, 0x48, 0x65, 0x6C, 0x6C, 0x6F];
        assert_eq!(detect_encoding(&data), "utf-8");

        // UTF-16 LE BOM
        let data = vec![0xFF, 0xFE, 0x48, 0x00];
        assert_eq!(detect_encoding(&data), "utf-16le");

        // No BOM (default UTF-8)
        let data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F];
        assert_eq!(detect_encoding(&data), "utf-8");
    }

    #[test]
    fn test_decode_output_utf8() {
        let data = b"Hello, World!".to_vec();
        let result = decode_output(&data, "utf-8").unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_decode_output_utf16le() {
        let data: Vec<u8> = vec![0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F, 0x00];
        let result = decode_output(&data, "utf-16le").unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_output_result() {
        let result = OutputResult::new(
            "stdout content".to_string(),
            "stderr content".to_string(),
            0
        );

        assert!(result.is_success());
        assert_eq!(result.stdout, "stdout content");
        assert_eq!(result.stderr, "stderr content");
        assert_eq!(result.combined_output(), "stdout contentstderr content");
    }
}
