use crate::auth::AuthContext;
use std::process::{Command, Stdio};

pub fn execute_via_wmi(
    target: &str,
    command: &str,
    auth: &AuthContext,
    working_dir: Option<&str>,
) -> Result<(i32, String, String), String> {
    let auth_user = auth
        .method
        .username()
        .unwrap_or_else(|| std::env::var("USERNAME").unwrap_or_default());
    let auth_domain = auth.method.domain().unwrap_or_default();

    // Construct PowerShell command to invoke WMI Create method
    let ps_cmd = build_wmi_powershell_command(
        target,
        command,
        working_dir,
        &auth_user,
        &auth_domain,
    );

    execute_powershell(&ps_cmd)
}

fn build_wmi_powershell_command(
    target: &str,
    command: &str,
    working_dir: Option<&str>,
    username: &str,
    domain: &str,
) -> String {
    let mut ps_cmd = String::new();

    // Set encoding to UTF-8 for proper output
    ps_cmd.push_str("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; ");

    // Build credential if needed
    if !username.is_empty() {
        ps_cmd.push_str("$cred = New-Object System.Management.Automation.PSCredential(");
        if !domain.is_empty() {
            ps_cmd.push_str(&format!("'{}\\{}',", domain, username));
        } else {
            ps_cmd.push_str(&format!("'{}',", username));
        }
        ps_cmd.push_str("(ConvertTo-SecureString 'password' -AsPlainText -Force)); ");
    }

    // Build WMI query
    ps_cmd.push_str(&format!(
        "$processStartupConfig = New-Object System.Management.ManagementClass -ArgumentList ('\\\\{}\\root\\cimv2','Win32_ProcessStartup',$null); ",
        target
    ));

    ps_cmd.push_str(&format!(
        "$processStartupConfig.Properties['CurrentDirectory'].Value = '{}'; ",
        working_dir.unwrap_or("C:\\Windows\\System32")
    ));

    ps_cmd.push_str(&format!(
        "$managementScope = New-Object System.Management.ManagementScope -ArgumentList '\\\\{}\\root\\cimv2'; ",
        target
    ));

    if !username.is_empty() {
        ps_cmd.push_str("$managementScope.Options.Authentication = [System.Management.AuthenticationLevel]::PacketPrivacy; ");
        ps_cmd.push_str("$managementScope.Options.Impersonation = [System.Management.ImpersonationLevel]::Impersonate; ");
    }

    ps_cmd.push_str("$managementScope.Connect(); ");

    ps_cmd.push_str(&format!(
        "$managementPath = New-Object System.Management.ManagementPath -ArgumentList 'Win32_Process'; "
    ));

    ps_cmd.push_str(&format!(
        "$managementClass = New-Object System.Management.ManagementClass -ArgumentList $managementScope, $managementPath, $null; "
    ));

    ps_cmd.push_str(&format!(
        "$managementClass.InvokeMethod('Create', @('{}'), $null); ",
        command.replace("'", "''")
    ));

    ps_cmd
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
    fn test_wmi_command_builder() {
        let auth = AuthContext::new(AuthMethod::current_user(), "localhost");
        let cmd = build_wmi_powershell_command("localhost", "cmd.exe /c whoami", None, "", "");

        assert!(cmd.contains("Win32_Process"));
        assert!(cmd.contains("Create"));
        assert!(cmd.contains("cmd.exe /c whoami"));
    }

    #[test]
    fn test_wmi_with_domain_credentials() {
        let auth = AuthContext::new(
            AuthMethod::with_credentials("admin", "pass123", Some("DOMAIN")),
            "server.local",
        );

        let cmd = build_wmi_powershell_command(
            "server.local",
            "ipconfig",
            Some("C:\\"),
            "admin",
            "DOMAIN",
        );

        assert!(cmd.contains("DOMAIN\\admin"));
        assert!(cmd.contains("ipconfig"));
    }
}
