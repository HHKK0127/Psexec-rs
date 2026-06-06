use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum PaExecError {
    // Authentication errors (-1 to -3)
    AuthenticationFailed(String),
    CredentialsInvalid(String),
    AuthMethodNotSupported(String),

    // Connection errors (-4 to -6)
    ConnectionFailed(String),
    ConnectionTimeout(String),
    HostUnreachable(String),

    // Execution errors (-7 to -9)
    ExecutionFailed(String),
    ExecutionTimeout(String),
    CommandNotFound(String),

    // Output/Transfer errors (-10 to -12)
    OutputFetchFailed(String),
    FileTransferFailed(String),
    OutputDecodeFailed(String),

    // Generic errors
    IOError(String),
    Unknown(String),
}

impl PaExecError {
    pub fn error_code(&self) -> i32 {
        match self {
            PaExecError::AuthenticationFailed(_) => -1,
            PaExecError::CredentialsInvalid(_) => -2,
            PaExecError::AuthMethodNotSupported(_) => -3,
            PaExecError::ConnectionFailed(_) => -4,
            PaExecError::ConnectionTimeout(_) => -5,
            PaExecError::HostUnreachable(_) => -6,
            PaExecError::ExecutionFailed(_) => -7,
            PaExecError::ExecutionTimeout(_) => -8,
            PaExecError::CommandNotFound(_) => -9,
            PaExecError::OutputFetchFailed(_) => -10,
            PaExecError::FileTransferFailed(_) => -11,
            PaExecError::OutputDecodeFailed(_) => -12,
            PaExecError::IOError(_) => -1,
            PaExecError::Unknown(_) => -1,
        }
    }

    pub fn with_retry_context(self, attempt: u32, max_attempts: u32) -> String {
        format!(
            "{} (attempt {}/{})",
            self.to_string(),
            attempt,
            max_attempts
        )
    }
}

impl fmt::Display for PaExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaExecError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            PaExecError::CredentialsInvalid(msg) => write!(f, "Invalid credentials: {}", msg),
            PaExecError::AuthMethodNotSupported(msg) => {
                write!(f, "Authentication method not supported: {}", msg)
            }
            PaExecError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            PaExecError::ConnectionTimeout(msg) => write!(f, "Connection timeout: {}", msg),
            PaExecError::HostUnreachable(msg) => write!(f, "Host unreachable: {}", msg),
            PaExecError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            PaExecError::ExecutionTimeout(msg) => write!(f, "Execution timeout: {}", msg),
            PaExecError::CommandNotFound(msg) => write!(f, "Command not found: {}", msg),
            PaExecError::OutputFetchFailed(msg) => write!(f, "Failed to fetch output: {}", msg),
            PaExecError::FileTransferFailed(msg) => write!(f, "File transfer failed: {}", msg),
            PaExecError::OutputDecodeFailed(msg) => write!(f, "Failed to decode output: {}", msg),
            PaExecError::IOError(msg) => write!(f, "IO error: {}", msg),
            PaExecError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for PaExecError {}

impl From<io::Error> for PaExecError {
    fn from(err: io::Error) -> Self {
        PaExecError::IOError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PaExecError>;

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    pub fn aggressive() -> Self {
        RetryPolicy {
            max_attempts: 20,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
            backoff_multiplier: 1.5,
        }
    }

    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay = (self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32)) as u64;
        delay.min(self.max_delay_ms)
    }

    /// Execute async closure with retry logic
    pub async fn execute<F, Fut, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = PaExecError::Unknown("No attempts made".to_string());

        for attempt in 0..self.max_attempts {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = e;
                    if attempt < self.max_attempts - 1 {
                        let delay = self.calculate_delay(attempt);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(PaExecError::AuthenticationFailed("test".into()).error_code(), -1);
        assert_eq!(
            PaExecError::CredentialsInvalid("test".into()).error_code(),
            -2
        );
        assert_eq!(
            PaExecError::ConnectionFailed("test".into()).error_code(),
            -4
        );
        assert_eq!(
            PaExecError::ExecutionFailed("test".into()).error_code(),
            -7
        );
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 5);

        let delay_1 = policy.calculate_delay(1);
        let delay_2 = policy.calculate_delay(2);
        assert!(delay_2 > delay_1);

        // Test max delay cap
        let delay_max = policy.calculate_delay(10);
        assert_eq!(delay_max, policy.max_delay_ms);
    }

    #[test]
    fn test_retry_context() {
        let err = PaExecError::ExecutionFailed("test".into());
        let with_context = err.with_retry_context(2, 5);
        assert!(with_context.contains("attempt 2/5"));
    }
}
