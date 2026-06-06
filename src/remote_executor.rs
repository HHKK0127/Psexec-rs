use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::thread;
use encoding_rs::Encoding;

/// Execute a remote command and stream output to UI
///
/// This function spawns a background thread that:
/// 1. Connects to remote computers via PowerShell
/// 2. Executes the command
/// 3. Sends output lines back via mpsc channel
/// 4. Returns immediately with a receiver for UI updates
pub fn execute_remote_command(
    computers: &str,
    command: &str,
    use_custom_auth: bool,
    username: &str,
    password: &str,
) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();

    let computers = computers.to_string();
    let command = command.to_string();
    let username = username.to_string();
    let password = password.to_string();

    thread::spawn(move || {
        // Parse computer list
        let computer_list: Vec<&str> = computers
            .split(',')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();

        if computer_list.is_empty() {
            let _ = tx.send("Error: No computers specified".to_string());
            return;
        }

        // For each computer, execute the command
        for (i, computer) in computer_list.iter().enumerate() {
            if i > 0 {
                let _ = tx.send("".to_string());
            }

            let _ = tx.send(format!("=== Connecting to {} ===", computer));

            // Build PowerShell command
            let ps_command = build_powershell_command(
                computer,
                &command,
                use_custom_auth,
                &username,
                &password,
            );

            // Execute and capture output
            match execute_powershell(&ps_command) {
                Ok(output) => {
                    for line in output {
                        let _ = tx.send(line);
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("Error: {}", e));
                }
            }

            let _ = tx.send(format!("=== Completed {} ===", computer));
        }

        let _ = tx.send("".to_string());
        let _ = tx.send("All commands completed.".to_string());
    });

    rx
}

/// Build PowerShell command for remote execution
///
/// Includes:
/// - UTF-8 output encoding to prevent character corruption
/// - Error handling with detailed error messages
fn build_powershell_command(
    computer: &str,
    command: &str,
    use_custom_auth: bool,
    username: &str,
    password: &str,
) -> String {
    let clean_computer = computer.trim_start_matches("\\");

    // Set UTF-8 encoding for PowerShell output
    let encoding_setup = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $PSDefaultParameterValues['*:Encoding']='utf8';";

    if use_custom_auth && !username.is_empty() {
        // With custom credentials (using Invoke-Command with -Credential)
        format!(
            "{} $cred = New-Object System.Management.Automation.PSCredential('{}', (ConvertTo-SecureString '{}' -AsPlainText -Force)); \
            Invoke-Command -ComputerName {} -ScriptBlock {{ {} }} -Credential $cred 2>&1",
            encoding_setup, username, password, clean_computer, command
        )
    } else {
        // With current user
        format!(
            "{} Invoke-Command -ComputerName {} -ScriptBlock {{ {} }} 2>&1",
            encoding_setup, clean_computer, command
        )
    }
}

/// Execute PowerShell command and return output lines
///
/// Handles character encoding issues by:
/// 1. Requesting UTF-8 from PowerShell
/// 2. Detecting and converting multiple encodings
/// 3. Using lossy conversion as fallback
fn execute_powershell(script: &str) -> Result<Vec<String>, String> {
    let mut output = Vec::new();

    // Start PowerShell process with UTF-8 output encoding
    let mut child = Command::new("powershell.exe")
        .args(&["-NoProfile", "-OutputFormat", "Text"])
        .arg("-Command")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    // Read stdout with encoding detection
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    // Convert line with automatic encoding detection
                    let converted = convert_encoding(&l);
                    output.push(converted);
                }
                Err(e) => {
                    output.push(format!("[Read Error: {}]", e));
                }
            }
        }
    }

    // Read stderr with encoding detection
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let converted = convert_encoding(&l);
                    output.push(format!("[ERROR] {}", converted));
                }
                Err(e) => {
                    output.push(format!("[Read Error: {}]", e));
                }
            }
        }
    }

    // Wait for process to complete
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for process: {}", e))?;

    if !status.success() {
        output.push(format!("[Exit Code: {}]", status.code().unwrap_or(-1)));
    } else {
        output.push("[Success]".to_string());
    }

    Ok(output)
}

/// Convert string with automatic encoding detection
///
/// Handles various character encodings that PowerShell might output:
/// - UTF-8 (preferred, no conversion needed)
/// - Shift_JIS (Japanese Windows)
/// - GB18030 (Chinese Windows)
/// - EUC-KR (Korean Windows)
/// - Windows-1252 (Western Windows)
///
/// Algorithm:
/// 1. Check if string is already valid UTF-8
/// 2. Try to detect encoding using chardet
/// 3. Try common encodings in order
/// 4. Fall back to lossy UTF-8 conversion
fn convert_encoding(input: &str) -> String {
    // If ASCII or already looks like valid UTF-8, return as-is
    if input.is_ascii() {
        return input.to_string();
    }

    // For strings that might have encoding issues, check encoding
    let input_bytes = input.as_bytes();

    // Try to detect encoding using chardet
    // chardet::detect returns (encoding_name, confidence, language) as a tuple
    let detection = chardet::detect(input_bytes);
    if detection.1 > 0.7 {
        // High confidence detection
        if let Some(enc) = Encoding::for_label(detection.0.as_bytes()) {
            let (converted, _, _) = enc.decode(input_bytes);
            return converted.trim().to_string();
        }
    }

    // Try common encodings in order of likelihood
    let encodings_to_try = [
        "shift_jis",
        "shift-jis",
        "gb18030",
        "euc-kr",
        "windows-1252",
        "utf-8",
    ];

    for enc_name in &encodings_to_try {
        if let Some(enc) = Encoding::for_label(enc_name.as_bytes()) {
            let (converted, _, had_errors) = enc.decode(input_bytes);
            // Accept conversion if it didn't have too many errors or is known good for the encoding
            if !had_errors || enc == encoding_rs::SHIFT_JIS || enc == encoding_rs::GBK {
                let result = converted.trim().to_string();
                if !result.is_empty() && !result.contains('\u{FFFD}') {
                    // Result is valid and doesn't have replacement characters
                    return result;
                }
            }
        }
    }

    // Final fallback: use lossy UTF-8 conversion with character replacement
    String::from_utf8_lossy(input_bytes).trim().to_string()
}

/// Alternative: Simple local command execution (for testing)
pub fn execute_local_command(command: &str) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    let command = command.to_string();

    thread::spawn(move || {
        let _ = tx.send(format!("Executing: {}", command));
        let _ = tx.send("".to_string());

        match execute_powershell(&command) {
            Ok(output) => {
                for line in output {
                    let _ = tx.send(line);
                }
            }
            Err(e) => {
                let _ = tx.send(format!("Error: {}", e));
            }
        }
    });

    rx
}
