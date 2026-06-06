use crate::auth::AuthContext;
use std::process::{Command, Stdio};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSchedulerMode {
    Demand,
    Create,
    Change,
}

pub fn execute_via_task_scheduler(
    target: &str,
    command: &str,
    auth: &AuthContext,
    mode: TaskSchedulerMode,
) -> Result<(i32, String, String), String> {
    let task_name = format!("PAExec_{}", Uuid::new_v4().to_string()[..8].to_uppercase());

    match mode {
        TaskSchedulerMode::Demand => execute_demand_mode(target, &task_name, command, auth),
        TaskSchedulerMode::Create => execute_create_mode(target, &task_name, command, auth),
        TaskSchedulerMode::Change => {
            // Change mode: modify existing task (requires task to exist)
            Err("Change mode requires existing task - not yet implemented".to_string())
        }
    }
}

fn execute_demand_mode(
    target: &str,
    task_name: &str,
    command: &str,
    auth: &AuthContext,
) -> Result<(i32, String, String), String> {
    // Create temporary task and run immediately
    let ps_cmd = build_demand_mode_command(target, task_name, command, auth);
    execute_powershell(&ps_cmd)
}

fn execute_create_mode(
    target: &str,
    task_name: &str,
    command: &str,
    auth: &AuthContext,
) -> Result<(i32, String, String), String> {
    // Create task with time trigger, runs once then auto-deletes (OPSEC optimized)
    let ps_cmd = build_create_mode_command(target, task_name, command, auth);
    execute_powershell(&ps_cmd)
}

fn build_demand_mode_command(
    target: &str,
    task_name: &str,
    command: &str,
    auth: &AuthContext,
) -> String {
    let mut ps_cmd = String::new();

    // Set encoding
    ps_cmd.push_str("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; ");

    // Build credential parameter if specified
    let cred_param = if let Some(user) = &auth.method.username() {
        let domain = auth.method.domain().unwrap_or_default();
        if !domain.is_empty() {
            format!("@{{\nCredential = New-Object System.Management.Automation.PSCredential('{}\\\\{}', (ConvertTo-SecureString 'password' -AsPlainText -Force))\n}}", domain, user)
        } else {
            format!("@{{\nCredential = New-Object System.Management.Automation.PSCredential('{}', (ConvertTo-SecureString 'password' -AsPlainText -Force))\n}}", user)
        }
    } else {
        String::new()
    };

    // Register task
    ps_cmd.push_str(&format!(
        "$TaskAction = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument '/c {}';\n",
        command.replace("'", "''")
    ));

    ps_cmd.push_str(&format!(
        "$TaskTrigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddSeconds(1);\n"
    ));

    ps_cmd.push_str(&format!(
        "$TaskSettings = New-ScheduledTaskSettingsSet -RunOnlyIfNetworkAvailable -DeleteExpiredTaskAfter (New-TimeSpan -Minutes 1);\n"
    ));

    ps_cmd.push_str(&format!(
        "$Task = New-ScheduledTask -Action $TaskAction -Trigger $TaskTrigger -Settings $TaskSettings;\n"
    ));

    // Register on remote computer
    if cred_param.is_empty() {
        ps_cmd.push_str(&format!(
            "Register-ScheduledTask -TaskName '{}' -InputObject $Task -CimSession (New-CimSession -ComputerName '{}');\n",
            task_name, target
        ));
    } else {
        ps_cmd.push_str(&format!(
            "Register-ScheduledTask -TaskName '{}' -InputObject $Task -CimSession (New-CimSession -ComputerName '{}' {});\n",
            task_name, target, cred_param
        ));
    }

    // Start task
    ps_cmd.push_str(&format!(
        "Start-ScheduledTask -TaskName '{}' -CimSession (New-CimSession -ComputerName '{}');\n",
        task_name, target
    ));

    // Get result
    ps_cmd.push_str(&format!(
        "$task = Get-ScheduledTaskInfo -TaskName '{}' -CimSession (New-CimSession -ComputerName '{}');\n",
        task_name, target
    ));

    ps_cmd.push_str(&format!(
        "Write-Output \"Exit Code: $($task.LastTaskResult)\";\n"
    ));

    // Cleanup
    ps_cmd.push_str(&format!(
        "Unregister-ScheduledTask -TaskName '{}' -CimSession (New-CimSession -ComputerName '{}') -Confirm:$false;\n",
        task_name, target
    ));

    ps_cmd
}

fn build_create_mode_command(
    target: &str,
    task_name: &str,
    command: &str,
    auth: &AuthContext,
) -> String {
    // Similar to demand mode but with create-specific settings
    // For now, use same implementation
    build_demand_mode_command(target, task_name, command, auth)
}

fn execute_powershell(command: &str) -> Result<(i32, String, String), String> {
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((exit_code, stdout, stderr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthMethod;

    #[test]
    fn test_demand_mode_command() {
        let auth = AuthContext::new(AuthMethod::current_user(), "localhost");
        let cmd = build_demand_mode_command("localhost", "TestTask", "whoami", &auth);

        assert!(cmd.contains("New-ScheduledTaskAction"));
        assert!(cmd.contains("whoami"));
        assert!(cmd.contains("Register-ScheduledTask"));
    }

    #[test]
    fn test_task_scheduler_mode_variants() {
        assert_eq!(TaskSchedulerMode::Demand, TaskSchedulerMode::Demand);
        assert_ne!(TaskSchedulerMode::Demand, TaskSchedulerMode::Create);
    }
}
