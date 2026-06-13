//! Integration tests for PAExec-rs
//! Tests cross-module functionality and end-to-end workflows

#[cfg(test)]
mod integration_tests {
    use psexec_rs::{
        cli::parse_command_line_args,
        config::AppConfig,
        executor::{ExecutionContext, ExecutionMethod},
        auth::{AuthContext, AuthMethod},
    };

    // ============================================================
    // CLI → Executor Pipeline Tests
    // ============================================================

    #[test]
    fn test_cli_to_executor_basic_execution() {
        // Parse CLI arguments for basic remote execution
        let args = vec![
            "\\\\localhost".to_string(),
            "notepad.exe".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify parsed arguments
        assert_eq!(cli.computer_list, vec!["localhost"]);
        assert_eq!(cli.app, "notepad.exe");
        assert!(cli.app_args.is_empty());
        assert!(!cli.use_system_account);
    }

    #[test]
    fn test_cli_to_executor_with_auth() {
        // Parse CLI with authentication
        let args = vec![
            "\\\\remote-host".to_string(),
            "-u".to_string(),
            "admin".to_string(),
            "-p".to_string(),
            "password".to_string(),
            "notepad".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify authentication settings
        assert_eq!(cli.computer_list, vec!["remote-host"]);
        assert_eq!(cli.user, Some("admin".to_string()));
        assert_eq!(cli.password, Some("password".to_string()));
        assert_eq!(cli.app, "notepad");
    }

    #[test]
    fn test_cli_to_executor_with_system_account() {
        // Parse CLI with system account flag
        let args = vec![
            "\\\\server".to_string(),
            "-s".to_string(),
            "powershell".to_string(),
            "-c".to_string(),
            "Get-Process".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify system account setting
        assert!(cli.use_system_account);
        assert_eq!(cli.app, "powershell");
    }

    // ============================================================
    // Config → Execution Pipeline Tests
    // ============================================================

    #[test]
    fn test_execution_context_creation() {
        // Create execution context from CLI settings
        let auth = AuthContext::new(AuthMethod::current_user(), "target-host");
        let exec_context = ExecutionContext::new(ExecutionMethod::SMBService, auth, "cmd.exe")
            .with_timeout(60)
            .with_working_directory("C:\\temp");

        // Verify context initialization
        assert_eq!(exec_context.method, ExecutionMethod::SMBService);
        assert_eq!(exec_context.timeout_seconds, Some(60));
        assert_eq!(exec_context.working_directory, Some("C:\\temp".to_string()));
        assert_eq!(exec_context.command, "cmd.exe");
    }

    #[test]
    fn test_config_with_execution() {
        // Load config and verify execution setup
        let config = AppConfig::default();

        // Verify config is ready for execution
        assert!(config.timeout_seconds > 0);
        assert!(!config.service_host_history.is_empty());
    }

    // ============================================================
    // Error Handling & Resilience Tests
    // ============================================================

    #[test]
    fn test_invalid_computer_name_handling() {
        // Test handling of invalid computer names
        let args = vec![
            "invalid".to_string(),  // Missing UNC prefix
            "cmd".to_string(),
        ];

        let result = parse_command_line_args(&args);

        // Should fail validation (no computer list)
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_computers_parsing() {
        // Test parsing multiple computer names
        let args = vec![
            "\\\\server1,server2,server3".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify all computers parsed
        assert_eq!(cli.computer_list.len(), 3);
        assert_eq!(cli.computer_list[0], "server1");
        assert_eq!(cli.computer_list[1], "server2");
        assert_eq!(cli.computer_list[2], "server3");
    }

    #[test]
    fn test_file_copy_with_execution() {
        // Test parsing file copy + execution combo
        let args = vec![
            "\\\\target-host".to_string(),
            "-c".to_string(),
            "-f".to_string(),
            "C:\\local\\app.exe".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify file copy settings
        assert!(cli.copy_files);
        assert!(cli.force_copy);
        assert_eq!(cli.app, "C:\\local\\app.exe");
    }

    // ============================================================
    // GUI ↔ CLI Integration Tests (Conceptual)
    // ============================================================

    #[test]
    fn test_cli_args_to_execution_context() {
        // Simulate CLI args being converted to execution context
        let args = vec![
            "\\\\server".to_string(),
            "-i".to_string(),
            "1".to_string(),
            "-w".to_string(),
            "C:\\work".to_string(),
            "app.exe".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify conversion to execution context params
        assert!(cli.interactive);
        assert_eq!(cli.session_id, Some(1));
        assert_eq!(cli.working_dir, Some("C:\\work".to_string()));
        assert_eq!(cli.app, "app.exe");
    }

    #[test]
    fn test_timeout_and_retry_integration() {
        // Test timeout and retry settings together
        let args = vec![
            "\\\\slow-host".to_string(),
            "-n".to_string(),
            "30".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("Failed to parse CLI args");

        // Verify timeout setting
        assert_eq!(cli.timeout_seconds, Some(30));

        // Create execution context with timeout
        let auth = AuthContext::new(AuthMethod::current_user(), "slow-host");
        let exec_context = ExecutionContext::new(ExecutionMethod::SMBService, auth, "cmd.exe")
            .with_timeout(30)
            .with_working_directory(cli.working_dir.as_deref().unwrap_or("C:\\"));

        assert_eq!(exec_context.timeout_seconds, Some(30));
    }

    // ============================================================
    // End-to-End Workflow Tests
    // ============================================================

    #[test]
    fn test_basic_remote_execution_workflow() {
        // Simulate basic remote execution workflow:
        // 1. Parse CLI args
        // 2. Setup execution context
        // 3. Verify parameters are valid

        let args = vec![
            "\\\\target".to_string(),
            "-u".to_string(),
            "user".to_string(),
            "-p".to_string(),
            "pass".to_string(),
            "-c".to_string(),
            "notepad".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("CLI parse failed");

        // Verify workflow prerequisites
        assert!(!cli.computer_list.is_empty());
        assert!(cli.user.is_some());
        assert!(cli.password.is_some());
        assert!(cli.copy_files);
        assert!(!cli.app.is_empty());

        // Create auth context with credentials
        let auth_method = AuthMethod::with_credentials(
            cli.user.as_ref().unwrap(),
            cli.password.as_ref().unwrap(),
            None,
        );
        let auth = AuthContext::new(auth_method, &cli.computer_list[0]);

        // Verify auth is ready
        assert_eq!(auth.target_host, cli.computer_list[0]);
        assert!(auth.method.username().is_some());
    }

    #[test]
    fn test_batch_execution_integration() {
        // Test batch execution setup
        let args = vec![
            "\\\\host1,host2".to_string(),
            "powershell".to_string(),
            "-c".to_string(),
            "Write-Host".to_string(),
            "test".to_string(),
        ];

        let cli = parse_command_line_args(&args).expect("CLI parse failed");

        // Verify batch prerequisites
        assert_eq!(cli.computer_list.len(), 2);
        assert_eq!(cli.app, "powershell");

        // Create execution context for batch
        let auth = AuthContext::new(AuthMethod::current_user(), "host1");
        let exec_context = ExecutionContext::new(ExecutionMethod::SMBService, auth, "powershell.exe");

        // Verify ready for batch execution
        assert_eq!(exec_context.method, ExecutionMethod::SMBService);
        assert_eq!(exec_context.command, "powershell.exe");
    }
}
