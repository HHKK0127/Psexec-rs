//! SMB file-based output fetching
//! Polls for output file on admin$ share

use crate::error::{PaExecError, Result};
use crate::output::{OutputFetcher, OutputResult};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// SMB-based output fetcher
pub struct SMBOutputFetcher {
    host: String,
    session_id: String,
    timeout_ms: u32,
}

impl SMBOutputFetcher {
    /// Create new SMB fetcher
    pub fn new(host: &str, session_id: &str, timeout_ms: u32) -> Self {
        Self {
            host: host.to_string(),
            session_id: session_id.to_string(),
            timeout_ms,
        }
    }

    /// Generate output file path
    fn output_file_path(&self) -> String {
        format!("\\\\{}\\admin$\\PAExec\\{}_output.txt", self.host, self.session_id)
    }

    /// Check if output file exists
    async fn output_file_exists(&self) -> Result<bool> {
        // In real implementation, check via SMB
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(true) // Simulate file exists
    }

    /// Read output file
    async fn read_output_file(&self) -> Result<String> {
        // In real implementation, read via SMB
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok("Simulated SMB output content".to_string())
    }

    /// Delete output file
    async fn delete_output_file(&self) -> Result<()> {
        // In real implementation, delete via SMB
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(())
    }
}

#[async_trait]
impl OutputFetcher for SMBOutputFetcher {
    async fn fetch(&self) -> Result<OutputResult> {
        let start = Instant::now();

        // Poll for output file
        let output = poll_output_file(
            &self.host,
            &self.output_file_path(),
            self.timeout_ms,
            100,
        ).await?;

        let fetch_time = start.elapsed().as_millis() as u64;

        // Cleanup
        self.delete_output_file().await.ok();

        Ok(OutputResult {
            stdout: output,
            stderr: String::new(),
            exit_code: 0,
            encoding_detected: "utf-8".to_string(),
            fetch_time_ms: fetch_time,
        })
    }

    async fn fetch_streaming(
        &self,
        callback: Box<dyn Fn(String) + Send>
    ) -> Result<OutputResult> {
        let start = Instant::now();
        let mut last_size: u64 = 0;
        let mut full_output = String::new();

        let poll_interval = Duration::from_millis(100);
        let timeout_duration = Duration::from_millis(self.timeout_ms as u64);
        let deadline = Instant::now() + timeout_duration;

        while Instant::now() < deadline {
            match read_output_incrementally(
                &self.host,
                &self.output_file_path(),
                &mut last_size,
            ).await {
                Ok(new_data) => {
                    if !new_data.is_empty() {
                        callback(new_data.clone());
                        full_output.push_str(&new_data);
                    }

                    // Check if process completed (file stopped growing)
                    sleep(Duration::from_millis(500)).await;

                    let current_size = get_file_size(&self.host, &self.output_file_path()).await?;
                    if current_size == last_size {
                        // No change, assume complete
                        break;
                    }
                }
                Err(_e) => {
                    // File might not exist yet
                    sleep(poll_interval).await;
                }
            }
        }

        let fetch_time = start.elapsed().as_millis() as u64;

        // Cleanup
        self.delete_output_file().await.ok();

        Ok(OutputResult {
            stdout: full_output,
            stderr: String::new(),
            exit_code: 0,
            encoding_detected: "utf-8".to_string(),
            fetch_time_ms: fetch_time,
        })
    }

    async fn cleanup(&self) -> Result<()> {
        self.delete_output_file().await
    }
}

/// Poll for output file with timeout
async fn poll_output_file(
    host: &str,
    file_path: &str,
    timeout_ms: u32,
    poll_interval_ms: u32,
) -> Result<String> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
    let interval = Duration::from_millis(poll_interval_ms as u64);

    // Check if file exists
    // In real implementation: SMB stat
    sleep(interval).await;

    if Instant::now() < deadline {
        // Simulate file found within timeout
        return Ok("Polled output content".to_string());
    }

    Err(PaExecError::ConnectionTimeout(format!(
        "Timeout waiting for output file: {}", file_path
    )))
}

/// Read only new data since last read
async fn read_output_incrementally(
    host: &str,
    file_path: &str,
    last_size: &mut u64,
) -> Result<String> {
    // In real implementation: SMB read with offset
    let current_size = get_file_size(host, file_path).await?;

    if current_size <= *last_size {
        return Ok(String::new()); // No new data
    }

    let new_size = current_size - *last_size;

    // Read only new portion
    // In real implementation: ReadFile with offset
    tokio::time::sleep(Duration::from_millis(5)).await;

    *last_size = current_size;

    Ok(format!("New data ({} bytes)", new_size))
}

/// Get file size via SMB
async fn get_file_size(host: &str, file_path: &str) -> Result<u64> {
    // In real implementation: SMB stat
    tokio::time::sleep(Duration::from_millis(5)).await;
    Ok(1024) // Simulated size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_output_file_polling() {
        let result = poll_output_file(
            "server01",
            "\\\\server01\\admin$\\PAExec\\test.txt",
            5000,
            100,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_smb_fetcher() {
        let fetcher = SMBOutputFetcher::new("server01", "session-abc", 5000);

        // Test file path generation
        assert!(fetcher.output_file_path().contains("session-abc"));

        // Test fetch (mock)
        let result = fetcher.fetch().await;
        assert!(result.is_ok());
    }
}
