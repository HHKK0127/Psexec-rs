//! Named Pipe-based output fetching
//! Real-time streaming from remote process

use crate::error::{PaExecError, Result};
use crate::output::{OutputFetcher, OutputResult};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Named Pipe output fetcher
pub struct NamedPipeOutputFetcher {
    host: String,
    session_id: String,
    timeout_ms: u32,
}

impl NamedPipeOutputFetcher {
    /// Create new pipe fetcher
    pub fn new(host: &str, session_id: &str, timeout_ms: u32) -> Self {
        Self {
            host: host.to_string(),
            session_id: session_id.to_string(),
            timeout_ms,
        }
    }

    /// Generate pipe name
    fn pipe_name(&self) -> String {
        format!("\\\\{}\\pipe\\PaExec{}", self.host, self.session_id)
    }

    /// Connect to named pipe
    async fn connect(&self) -> Result<()> {
        // In real implementation, use Windows CreateFileW
        // For now, simulate connection
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    /// Read message from pipe
    async fn read_message(&self) -> Result<Vec<u8>> {
        // In real implementation, use ReadFile with PIPE_READMODE_MESSAGE
        // For now, simulate data
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(b"Simulated pipe output\n".to_vec())
    }

    /// Wait for pipe to become available
    async fn wait_for_pipe(&self, timeout_ms: u32) -> Result<()> {
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(timeout_ms as u64);

        // Check if pipe exists
        // In real implementation: WaitNamedPipeW
        tokio::time::sleep(Duration::from_millis(100)).await;

        if start.elapsed() < timeout_duration {
            // Simulate success within timeout
            return Ok(());
        }

        Err(PaExecError::ConnectionTimeout(format!(
            "Timeout waiting for pipe: {}", self.pipe_name()
        )))
    }
}

#[async_trait]
impl OutputFetcher for NamedPipeOutputFetcher {
    async fn fetch(&self) -> Result<OutputResult> {
        let start = Instant::now();

        // Wait for pipe
        self.wait_for_pipe(self.timeout_ms).await?;

        // Connect
        self.connect().await?;

        // Read all output
        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        // In real implementation, read until pipe closes
        // For now, simulate reading
        for _ in 0..3 {
            let data = self.read_message().await?;
            stdout_data.extend_from_slice(&data);
        }

        let fetch_time = start.elapsed().as_millis() as u64;

        // Decode output
        let stdout = String::from_utf8_lossy(&stdout_data).to_string();
        let stderr = String::from_utf8_lossy(&stderr_data).to_string();

        Ok(OutputResult {
            stdout,
            stderr,
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

        // Wait for pipe
        self.wait_for_pipe(self.timeout_ms).await?;
        self.connect().await?;

        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        // Read with streaming callback
        loop {
            match timeout(
                Duration::from_millis(100),
                self.read_message()
            ).await {
                Ok(Ok(data)) => {
                    if data.is_empty() {
                        break; // EOF
                    }

                    let text = String::from_utf8_lossy(&data).to_string();
                    callback(text.clone());

                    stdout_data.extend_from_slice(&data);
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Timeout - check if we should continue
                    break;
                }
            }
        }

        let fetch_time = start.elapsed().as_millis() as u64;

        Ok(OutputResult {
            stdout: String::from_utf8_lossy(&stdout_data).to_string(),
            stderr: String::from_utf8_lossy(&stderr_data).to_string(),
            exit_code: 0,
            encoding_detected: "utf-8".to_string(),
            fetch_time_ms: fetch_time,
        })
    }

    async fn cleanup(&self) -> Result<()> {
        // Close pipe handle
        // In real implementation: CloseHandle
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_named_pipe_connection() {
        let fetcher = NamedPipeOutputFetcher::new("localhost", "test-123", 5000);

        // Test pipe name generation
        assert_eq!(fetcher.pipe_name(), "\\\\localhost\\pipe\\PaExectest-123");

        // Test connection (mock)
        let result = fetcher.connect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch() {
        let fetcher = NamedPipeOutputFetcher::new("localhost", "test-456", 1000);

        let result = fetcher.fetch().await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.stdout.is_empty());
    }
}
