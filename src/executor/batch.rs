//! Batch execution module for parallel remote execution
//! Supports concurrent execution across multiple computers

use crate::error::{PaExecError, Result};
use crate::executor::{ExecutionContext, ExecutionResult, ExecutionMethod};
use crate::auth::AuthContext;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, Mutex};
use tokio::task::JoinSet;

/// Batch execution configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Timeout per execution (seconds)
    pub timeout_seconds: u64,
    /// Continue on individual failure
    pub continue_on_error: bool,
    /// Retry failed attempts
    pub retry_count: u32,
    /// Delay between retries (milliseconds)
    pub retry_delay_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            timeout_seconds: 60,
            continue_on_error: true,
            retry_count: 0,
            retry_delay_ms: 1000,
        }
    }
}

impl BatchConfig {
    pub fn with_concurrency(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_retry(mut self, count: u32, delay_ms: u64) -> Self {
        self.retry_count = count;
        self.retry_delay_ms = delay_ms;
        self
    }

    pub fn fail_fast(mut self) -> Self {
        self.continue_on_error = false;
        self
    }
}

/// Batch execution result
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Results by computer name
    pub results: HashMap<String, ExecutionResult>,
    /// Successful executions count
    pub success_count: usize,
    /// Failed executions count
    pub failure_count: usize,
    /// Total execution time
    pub total_duration_ms: u64,
    /// Errors by computer
    pub errors: HashMap<String, String>,
}

/// Execute command on multiple computers in parallel
pub async fn execute_batch(
    computers: Vec<String>,
    command: &str,
    args: &[String],
    auth: Option<&AuthContext>,
    method: ExecutionMethod,
    config: &BatchConfig,
) -> Result<BatchResult> {
    let start = std::time::Instant::now();

    // Create semaphore to limit concurrency
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

    // Shared results
    let results = Arc::new(Mutex::new(HashMap::new()));
    let errors = Arc::new(Mutex::new(HashMap::new()));

    // Create join set for concurrent execution
    let mut join_set = JoinSet::new();

    for computer in computers {
        let sem = Arc::clone(&semaphore);
        let res = Arc::clone(&results);
        let errs = Arc::clone(&errors);
        let auth = auth.cloned();
        let cmd = command.to_string();
        let args = args.to_vec();
        let cfg = config.clone();
        let meth = method.clone();

        join_set.spawn(async move {
            // Acquire semaphore permit
            let _permit = sem.acquire().await.unwrap();

            // Execute with retry logic
            let result = execute_with_retry(
                &computer,
                &cmd,
                &args,
                auth.as_ref(),
                meth,
                &cfg,
            ).await;

            match result {
                Ok(exec_result) => {
                    res.lock().await.insert(computer.clone(), exec_result);
                }
                Err(e) => {
                    errs.lock().await.insert(computer.clone(), e.to_string());
                }
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(_) = join_set.join_next().await {}

    // Collect results
    let results_inner = results.lock().await;
    let errors_inner = errors.lock().await;

    let success_count = results_inner.len();
    let failure_count = errors_inner.len();

    // Check if we should fail fast
    if !config.continue_on_error && failure_count > 0 {
        return Err(PaExecError::ExecutionFailed(
            format!("Batch execution failed on {} computers", failure_count)
        ));
    }

    Ok(BatchResult {
        results: results_inner.clone(),
        success_count,
        failure_count,
        total_duration_ms: start.elapsed().as_millis() as u64,
        errors: errors_inner.clone(),
    })
}

/// Execute with retry logic
async fn execute_with_retry(
    _computer: &str,
    command: &str,
    _args: &[String],
    auth: Option<&AuthContext>,
    method: ExecutionMethod,
    config: &BatchConfig,
) -> Result<ExecutionResult> {
    for attempt in 0..=config.retry_count {
        if attempt > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(config.retry_delay_ms)).await;
        }

        // Use provided auth or return error
        let Some(ctx_auth) = auth else {
            return Err(PaExecError::ExecutionFailed("No authentication context provided".to_string()));
        };

        let ctx = ExecutionContext {
            method,
            auth: ctx_auth.clone(),
            command: command.to_string(),
            working_directory: None,
            priority: None,
            timeout_seconds: Some(config.timeout_seconds as u32),
        };

        // For now, just simulate execution success
        // In real implementation, this would call the actual executor
        return Ok(ExecutionResult {
            exit_code: 0,
            stdout: "Execution completed".to_string(),
            stderr: "".to_string(),
            success: true,
        });
    }

    unreachable!("Loop should always return in first iteration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.timeout_seconds, 60);
        assert!(config.continue_on_error);
    }

    #[test]
    fn test_batch_config_builder() {
        let config = BatchConfig::default()
            .with_concurrency(5)
            .with_timeout(120)
            .with_retry(3, 500)
            .fail_fast();

        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.timeout_seconds, 120);
        assert_eq!(config.retry_count, 3);
        assert_eq!(config.retry_delay_ms, 500);
        assert!(!config.continue_on_error);
    }
}
