//! CLI command handlers for PAExec-rs

use crate::cli::{ServiceCommands, RegistryCommands};
use crate::service::ServiceContext;
use crate::error::Result;
use crate::cli_response::{
    ServiceInfo as CliServiceInfo, ServiceState as CliServiceState, ServiceListResponse,
    RegistryEntryInfo, RegistryListResponse, ScriptExecResult, OperationResult,
};

/// Handle exec command
pub async fn handle_exec(
    host: Option<String>,
    command: String,
    method: Option<String>,
    working_dir: Option<String>,
    copy: bool,
) -> Result<()> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    println!("[*] Executing command on: {}", target_host);
    println!("[*] Command: {}", command);

    if let Some(method) = method {
        println!("[*] Execution method: {}", method);
    }

    if let Some(dir) = working_dir {
        println!("[*] Working directory: {}", dir);
    }

    if copy {
        println!("[*] File copy enabled");
    }

    println!("[*] Command execution not yet implemented");
    Ok(())
}

/// Handle service commands
pub async fn handle_service_command(
    host: Option<String>,
    cmd: ServiceCommands,
) -> Result<()> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = ServiceContext::new(&target_host);

    match cmd {
        ServiceCommands::List { host: _, filter, running_only } => {
            println!("[*] Listing services on: {}", target_host);

            match crate::service::list_services(&ctx).await {
                Ok(services) => {
                    let mut filtered = services;

                    // Apply filter
                    if let Some(f) = &filter {
                        filtered.retain(|s| {
                            s.name.to_lowercase().contains(&f.to_lowercase())
                                || s.display_name.to_lowercase().contains(&f.to_lowercase())
                        });
                    }

                    // Filter running only
                    if running_only {
                        filtered.retain(|s| {
                            format!("{:?}", s.state).contains("Running")
                        });
                    }

                    if filtered.is_empty() {
                        println!("[!] No services found");
                    } else {
                        println!("[+] Found {} service(s):", filtered.len());
                        println!();
                        for svc in filtered {
                            println!("  Service: {}", svc.name);
                            println!("    Display: {}", svc.display_name);
                            println!("    State: {:?}", svc.state);
                            println!("    Path: {}", svc.path);
                            println!();
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error listing services: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Get { name, host: _ } => {
            println!("[*] Getting service info: {}", name);

            match crate::service::get_service(&ctx, &name).await {
                Ok(service) => {
                    println!("[+] Service details:");
                    println!("  Name: {}", service.name);
                    println!("  Display: {}", service.display_name);
                    println!("  State: {:?}", service.state);
                    println!("  Startup Type: {:?}", service.startup_type);
                    println!("  Path: {}", service.path);
                    println!("  Account: {}", service.account);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error getting service: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Start { name, host: _ } => {
            println!("[*] Starting service: {}", name);

            match crate::service::start_service(&ctx, &name).await {
                Ok(result) => {
                    if result.success {
                        println!("[+] Service started successfully");
                    } else {
                        eprintln!("[!] Failed: {}", result.error_message.unwrap_or_default());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error starting service: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Stop { name, host: _ } => {
            println!("[*] Stopping service: {}", name);

            match crate::service::stop_service(&ctx, &name).await {
                Ok(result) => {
                    if result.success {
                        println!("[+] Service stopped successfully");
                    } else {
                        eprintln!("[!] Failed: {}", result.error_message.unwrap_or_default());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error stopping service: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Restart { name, host: _ } => {
            println!("[*] Restarting service: {}", name);

            match crate::service::restart_service(&ctx, &name).await {
                Ok(result) => {
                    if result.success {
                        println!("[+] Service restarted successfully");
                    } else {
                        eprintln!("[!] Failed: {}", result.error_message.unwrap_or_default());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error restarting service: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Create { name, display_name, path, host: _, startup_type } => {
            println!("[*] Creating service: {}", name);

            let display = display_name.unwrap_or_else(|| name.clone());
            let svc_startup = crate::service::ServiceStartupType::Automatic;

            match crate::service::create_service(&ctx, &name, &display, &path, svc_startup).await {
                Ok(result) => {
                    if result.success {
                        println!("[+] Service created successfully");
                    } else {
                        eprintln!("[!] Failed: {}", result.error_message.unwrap_or_default());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error creating service: {}", e);
                    Err(e)
                }
            }
        }

        ServiceCommands::Delete { name, host: _, force } => {
            println!("[*] Deleting service: {}", name);

            match crate::service::delete_service(&ctx, &name).await {
                Ok(result) => {
                    if result.success {
                        println!("[+] Service deleted successfully");
                    } else {
                        eprintln!("[!] Failed: {}", result.error_message.unwrap_or_default());
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error deleting service: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// Handle registry commands
pub async fn handle_registry_command(
    host: Option<String>,
    cmd: RegistryCommands,
) -> Result<()> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    println!("[*] Registry operation on: {}", target_host);

    match cmd {
        RegistryCommands::Read { key, value, host: _ } => {
            println!("[*] Reading registry: {}\\{}", key, value);

            let ctx = crate::registry::RegistryContext::new(&target_host, crate::registry::RegistryHive::HKEY_LOCAL_MACHINE);

            match crate::registry::read_registry_value(&ctx, &key, &value).await {
                Ok(val) => {
                    println!("[+] Value: {:?}", val);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error reading registry: {}", e);
                    Err(e)
                }
            }
        }

        RegistryCommands::Write { key, value, data, r#type, host: _ } => {
            println!("[*] Writing registry: {}\\{}", key, value);
            println!("[*] Data: {} (type: {})", data, r#type);

            let ctx = crate::registry::RegistryContext::new(&target_host, crate::registry::RegistryHive::HKEY_LOCAL_MACHINE);

            let registry_value = match r#type.to_uppercase().as_str() {
                "REG_SZ" | "REG_EXPAND_SZ" => crate::registry::RegistryValue::String(data.clone()),
                "REG_DWORD" => {
                    match data.parse::<u32>() {
                        Ok(num) => crate::registry::RegistryValue::Dword(num),
                        Err(_) => {
                            eprintln!("[!] Invalid DWORD value: {}", data);
                            return Err(crate::error::PaExecError::ExecutionFailed(
                                "Invalid DWORD value".to_string(),
                            ).into());
                        }
                    }
                }
                "REG_QWORD" => {
                    match data.parse::<u64>() {
                        Ok(num) => crate::registry::RegistryValue::Qword(num),
                        Err(_) => {
                            eprintln!("[!] Invalid QWORD value: {}", data);
                            return Err(crate::error::PaExecError::ExecutionFailed(
                                "Invalid QWORD value".to_string(),
                            ).into());
                        }
                    }
                }
                _ => crate::registry::RegistryValue::String(data.clone()),
            };

            match crate::registry::write_registry_value(&ctx, &key, &value, registry_value).await {
                Ok(_) => {
                    println!("[+] Value written successfully");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error writing registry: {}", e);
                    Err(e)
                }
            }
        }

        RegistryCommands::Delete { key, value, host: _ } => {
            println!("[*] Deleting registry: {}\\{}", key, value);

            let ctx = crate::registry::RegistryContext::new(&target_host, crate::registry::RegistryHive::HKEY_LOCAL_MACHINE);

            match crate::registry::delete_registry_value(&ctx, &key, &value).await {
                Ok(_) => {
                    println!("[+] Value deleted successfully");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error deleting registry: {}", e);
                    Err(e)
                }
            }
        }

        RegistryCommands::List { key, host: _, recursive } => {
            println!("[*] Listing registry: {}", key);
            if recursive {
                println!("[*] Recursive listing enabled");
            }

            let ctx = crate::registry::RegistryContext::new(&target_host, crate::registry::RegistryHive::HKEY_LOCAL_MACHINE);

            match crate::registry::enumerate_registry_key(&ctx, &key).await {
                Ok(reg_key) => {
                    if reg_key.values.is_empty() {
                        println!("[!] No entries found");
                    } else {
                        println!("[+] Found {} value(s):", reg_key.values.len());
                        for (name, value_type) in &reg_key.values {
                            println!("  {}: {:?}", name, value_type);
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[!] Error listing registry: {}", e);
                    Err(e)
                }
            }
        }
    }
}

/// Handle script command
pub async fn handle_script_command(
    host: Option<String>,
    script_type: String,
    file: String,
    policy: Option<String>,
    args: Option<String>,
) -> Result<()> {
    use std::fs;

    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    println!("[*] Executing script on: {}", target_host);
    println!("[*] Type: {}", script_type);
    println!("[*] File: {}", file);

    // Read script file
    let script_content = match fs::read_to_string(&file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("[!] Error reading script file: {}", e);
            return Err(crate::error::PaExecError::ExecutionFailed(
                format!("Failed to read script file: {}", e),
            ).into());
        }
    };

    // Parse script type
    let parsed_type = match script_type.to_lowercase().as_str() {
        "ps" | "powershell" => crate::script::ScriptType::PowerShell,
        "vbs" | "vbscript" => crate::script::ScriptType::VBScript,
        "batch" | "bat" => crate::script::ScriptType::Batch,
        "js" | "javascript" => crate::script::ScriptType::JavaScript,
        _ => {
            eprintln!("[!] Unknown script type: {}", script_type);
            return Err(crate::error::PaExecError::ExecutionFailed(
                format!("Unknown script type: {}", script_type),
            ).into());
        }
    };

    let ctx = crate::script::ScriptContext::new(parsed_type, &target_host);
    let mut script = crate::script::ScriptExecution::new(&script_content);

    // Add arguments
    if let Some(arg_string) = args {
        script = script.with_arguments(
            arg_string
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
        );
    }

    println!("[*] Executing script...");

    match crate::script::execute_script(&ctx, &script).await {
        Ok(result) => {
            println!("[+] Script execution completed");
            println!("    Exit code: {}", result.exit_code);
            println!("    Execution time: {}ms", result.execution_time_ms);

            if !result.stdout.is_empty() {
                println!("\n[STDOUT]:");
                println!("{}", result.stdout);
            }

            if !result.stderr.is_empty() {
                println!("\n[STDERR]:");
                println!("{}", result.stderr);
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("[!] Error executing script: {}", e);
            Err(e)
        }
    }
}

/// Handle file transfer command
pub async fn handle_transfer_command(
    host: Option<String>,
    direction: String,
    source: String,
    destination: String,
) -> Result<()> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    println!("[*] File transfer on: {}", target_host);
    println!("[*] Direction: {}", direction);
    println!("[*] Source: {}", source);
    println!("[*] Destination: {}", destination);
    println!("[*] File transfer not yet implemented");
    Ok(())
}

/// Handle interactive shell command
pub async fn handle_shell_command(
    host: Option<String>,
    timeout: Option<u32>,
) -> Result<()> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    println!("[*] Interactive shell on: {}", target_host);

    if let Some(t) = timeout {
        println!("[*] Timeout: {} seconds", t);
    }

    println!("[*] Interactive shell not yet implemented");
    Ok(())
}

// ============================================================================
// GUI-oriented response functions (return structured types)
// ============================================================================

/// Get service list for GUI integration
pub async fn get_service_list(
    host: Option<String>,
    filter: Option<String>,
    running_only: bool,
) -> Result<ServiceListResponse> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = ServiceContext::new(&target_host);

    match crate::service::list_services(&ctx).await {
        Ok(services) => {
            let mut filtered = services
                .into_iter()
                .map(|s| CliServiceInfo {
                    name: s.name.clone(),
                    display_name: s.display_name.clone(),
                    state: map_service_state(&s.state),
                    path: s.path.clone(),
                })
                .collect::<Vec<_>>();

            // Apply filter
            if let Some(f) = &filter {
                filtered.retain(|s| {
                    s.name.to_lowercase().contains(&f.to_lowercase())
                        || s.display_name.to_lowercase().contains(&f.to_lowercase())
                });
            }

            // Filter running only
            if running_only {
                filtered.retain(|s| s.state == CliServiceState::Running);
            }

            let count = filtered.len();
            Ok(ServiceListResponse {
                services: filtered,
                count,
            })
        }
        Err(e) => Err(e),
    }
}

/// Convert Phase 3 ServiceState to CLI response ServiceState
fn map_service_state(state: &crate::service::ServiceState) -> CliServiceState {
    match state {
        crate::service::ServiceState::Running => CliServiceState::Running,
        crate::service::ServiceState::Stopped => CliServiceState::Stopped,
        crate::service::ServiceState::Paused => CliServiceState::Paused,
        _ => CliServiceState::Other,
    }
}

/// Start a service (for GUI)
pub async fn start_service_op(
    host: Option<String>,
    name: String,
) -> Result<OperationResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = ServiceContext::new(&target_host);

    match crate::service::start_service(&ctx, &name).await {
        Ok(result) => {
            let op_result = if result.success {
                OperationResult::success(format!("Service '{}' started successfully", name))
            } else {
                OperationResult::error(
                    result
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string()),
                )
            };
            Ok(op_result)
        }
        Err(e) => Err(e),
    }
}

/// Stop a service (for GUI)
pub async fn stop_service_op(
    host: Option<String>,
    name: String,
) -> Result<OperationResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = ServiceContext::new(&target_host);

    match crate::service::stop_service(&ctx, &name).await {
        Ok(result) => {
            let op_result = if result.success {
                OperationResult::success(format!("Service '{}' stopped successfully", name))
            } else {
                OperationResult::error(
                    result
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string()),
                )
            };
            Ok(op_result)
        }
        Err(e) => Err(e),
    }
}

/// Restart a service (for GUI)
pub async fn restart_service_op(
    host: Option<String>,
    name: String,
) -> Result<OperationResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = ServiceContext::new(&target_host);

    match crate::service::restart_service(&ctx, &name).await {
        Ok(result) => {
            let op_result = if result.success {
                OperationResult::success(format!("Service '{}' restarted successfully", name))
            } else {
                OperationResult::error(
                    result
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string()),
                )
            };
            Ok(op_result)
        }
        Err(e) => Err(e),
    }
}

/// Get registry entries (for GUI)
pub async fn get_registry_entries(
    host: Option<String>,
    path: String,
) -> Result<RegistryListResponse> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = crate::registry::RegistryContext::new(
        &target_host,
        crate::registry::RegistryHive::HKEY_LOCAL_MACHINE,
    );

    match crate::registry::enumerate_registry_key(&ctx, &path).await {
        Ok(reg_key) => {
            let entries = reg_key
                .values
                .into_iter()
                .map(|(name, value_type)| RegistryEntryInfo {
                    name,
                    value_type: format!("{:?}", value_type),
                    data: String::new(), // Will be loaded separately if needed
                })
                .collect::<Vec<_>>();

            let count = entries.len();
            Ok(RegistryListResponse {
                path,
                entries,
                count,
            })
        }
        Err(e) => Err(e),
    }
}

/// Write registry value (for GUI)
pub async fn write_registry_op(
    host: Option<String>,
    key: String,
    value: String,
    data: String,
    r#type: String,
) -> Result<OperationResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = crate::registry::RegistryContext::new(
        &target_host,
        crate::registry::RegistryHive::HKEY_LOCAL_MACHINE,
    );

    let registry_value = match r#type.to_uppercase().as_str() {
        "REG_SZ" | "REG_EXPAND_SZ" => crate::registry::RegistryValue::String(data.clone()),
        "REG_DWORD" => match data.parse::<u32>() {
            Ok(num) => crate::registry::RegistryValue::Dword(num),
            Err(_) => {
                return Ok(OperationResult::error(format!(
                    "Invalid DWORD value: {}",
                    data
                )))
            }
        },
        "REG_QWORD" => match data.parse::<u64>() {
            Ok(num) => crate::registry::RegistryValue::Qword(num),
            Err(_) => {
                return Ok(OperationResult::error(format!(
                    "Invalid QWORD value: {}",
                    data
                )))
            }
        },
        _ => crate::registry::RegistryValue::String(data.clone()),
    };

    match crate::registry::write_registry_value(&ctx, &key, &value, registry_value).await {
        Ok(_) => {
            Ok(OperationResult::success(format!(
                "Registry value '{}\\{}' written successfully",
                key, value
            )))
        }
        Err(e) => Err(e),
    }
}

/// Delete registry value (for GUI)
pub async fn delete_registry_op(
    host: Option<String>,
    key: String,
    value: String,
) -> Result<OperationResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());
    let ctx = crate::registry::RegistryContext::new(
        &target_host,
        crate::registry::RegistryHive::HKEY_LOCAL_MACHINE,
    );

    match crate::registry::delete_registry_value(&ctx, &key, &value).await {
        Ok(_) => {
            Ok(OperationResult::success(format!(
                "Registry value '{}\\{}' deleted successfully",
                key, value
            )))
        }
        Err(e) => Err(e),
    }
}

/// Execute script (for GUI)
pub async fn execute_script_op(
    host: Option<String>,
    script_type: String,
    content: String,
    arguments: Option<String>,
) -> Result<ScriptExecResult> {
    let target_host = host.unwrap_or_else(|| "localhost".to_string());

    // Parse script type
    let parsed_type = match script_type.to_lowercase().as_str() {
        "ps" | "powershell" => crate::script::ScriptType::PowerShell,
        "vbs" | "vbscript" => crate::script::ScriptType::VBScript,
        "batch" | "bat" => crate::script::ScriptType::Batch,
        "js" | "javascript" => crate::script::ScriptType::JavaScript,
        _ => {
            return Err(crate::error::PaExecError::ExecutionFailed(
                format!("Unknown script type: {}", script_type),
            )
            .into());
        }
    };

    let ctx = crate::script::ScriptContext::new(parsed_type, &target_host);
    let mut script = crate::script::ScriptExecution::new(&content);

    // Add arguments if provided
    if let Some(arg_string) = arguments {
        script = script.with_arguments(
            arg_string
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
        );
    }

    match crate::script::execute_script(&ctx, &script).await {
        Ok(result) => Ok(ScriptExecResult {
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            execution_time_ms: result.execution_time_ms,
        }),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_handler() {
        let result = handle_exec(
            Some("localhost".to_string()),
            "whoami".to_string(),
            None,
            None,
            false,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_list_handler() {
        let cmd = ServiceCommands::List {
            host: None,
            filter: None,
            running_only: false,
        };
        let result = handle_service_command(Some("localhost".to_string()), cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_read_handler() {
        let cmd = RegistryCommands::Read {
            key: "HKEY_LOCAL_MACHINE\\Software".to_string(),
            value: "Test".to_string(),
            host: None,
        };
        let result = handle_registry_command(Some("localhost".to_string()), cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_script_handler() {
        // Note: This test uses a non-existent file, so it's expected to fail
        // In real usage, provide a valid script file path
        let result = handle_script_command(
            Some("localhost".to_string()),
            "ps".to_string(),
            "nonexistent_test_script.ps1".to_string(),
            None,
            None,
        )
        .await;
        // Script execution should fail because the file doesn't exist
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transfer_handler() {
        let result = handle_transfer_command(
            Some("localhost".to_string()),
            "upload".to_string(),
            "C:\\local\\file.txt".to_string(),
            "C:\\remote\\file.txt".to_string(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shell_handler() {
        let result = handle_shell_command(Some("localhost".to_string()), Some(30)).await;
        assert!(result.is_ok());
    }
}
