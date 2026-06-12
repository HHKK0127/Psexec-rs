//! Command-line interface for PAExec-rs
//! Supports both modern clap-based CLI and legacy PsExec-compatible syntax

use clap::{Parser, Subcommand};

use std::collections::HashMap;

/// Parsed command line arguments (PsExec-compatible)
#[derive(Debug, Clone)]
pub struct CliArgs {
    // Target specification
    pub computer_list: Vec<String>,
    pub app: String,
    pub app_args: Vec<String>,

    // Authentication
    pub user: Option<String>,
    pub password: Option<String>,
    pub use_ntlm: bool,
    pub use_kerberos: bool,

    // Process execution options
    pub use_system_account: bool,           // -s
    pub interactive: bool,                  // -i
    pub session_id: Option<i32>,            // -i <session>
    pub run_limited: bool,                  // -l
    pub no_profile: bool,                   // -noprofile

    // File copy options
    pub copy_files: bool,                   // -c
    pub force_copy: bool,                   // -f
    pub version_check_copy: bool,           // -v
    pub copy_to: Option<String>,            // -csrc <path>

    // Working environment
    pub working_dir: Option<String>,        // -w
    pub environment: HashMap<String, String>, // -e

    // Process control
    pub dont_wait: bool,                    // -d
    pub timeout_seconds: Option<u32>,       // -n <seconds>
    pub terminate_after: Option<u32>,       // -t <seconds>
    pub kill_if_hung: bool,                 // -h

    // Process priority
    pub priority: ProcessPriority,           // -low, -belownormal, etc.

    // Processor affinity
    pub processors: Option<Vec<u16>>,       // -a <processors>

    // Console/display options
    pub console: bool,                      // -x
    pub elevate: bool,                      // -elevated

    // EULA acceptance
    pub accept_eula: bool,                  // -accepteula

    // Remote PAExec options
    pub remote_paexec_path: Option<String>, // -rpath
    pub service_name: Option<String>,       // -sname
    pub service_display_name: Option<String>, // -sdname

    // Input/output redirection
    pub redirect_stdin: Option<String>,     // -stdin
    pub redirect_stdout: Option<String>,    // -stdout
    pub redirect_stderr: Option<String>,    // -stderr

    // Background mode
    pub background: bool,                   // -background

    // Help
    pub show_help: bool,                    // -?, /?, -h, --help
    pub show_usage: bool,                   // Legacy compatibility
}

/// Process priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPriority {
    Realtime,       // -realtime
    High,           // -high
    AboveNormal,    // -abovenormal
    Normal,         // default
    BelowNormal,    // -belownormal
    Idle,           // -low, -idle
}

impl Default for ProcessPriority {
    fn default() -> Self {
        ProcessPriority::Normal
    }
}

/// Modern clap-based CLI
#[derive(Parser, Debug)]
#[command(name = "psexec-rs")]
#[command(about = "Windows remote execution and management tool")]
#[command(version)]
pub struct ModernCli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(global = true, short, long)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a command remotely or locally
    Exec {
        /// Target hostname or 'localhost'
        #[arg(short, long)]
        host: Option<String>,

        /// Command to execute
        #[arg(value_name = "COMMAND")]
        command: String,

        /// Execution method (smb, wmi, task, dcom)
        #[arg(short, long)]
        method: Option<String>,

        /// Working directory on remote host
        #[arg(short, long)]
        working_dir: Option<String>,

        /// Copy executable before execution
        #[arg(short, long)]
        copy: bool,
    },

    /// Service management
    #[command(subcommand)]
    Service(ServiceCommands),

    /// Registry operations
    #[command(subcommand)]
    Registry(RegistryCommands),

    /// Script execution
    Script {
        /// Script type (ps, vbs, batch, js)
        #[arg(short, long)]
        r#type: String,

        /// Path to script file or inline script
        #[arg(short, long)]
        file: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Execution policy (PowerShell only)
        #[arg(short, long)]
        policy: Option<String>,

        /// Script arguments
        #[arg(short, long)]
        args: Option<String>,
    },

    /// File transfer
    Transfer {
        /// upload or download
        #[arg(short, long)]
        direction: String,

        /// Source path
        #[arg(short, long)]
        source: String,

        /// Destination path
        #[arg(short, long)]
        destination: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Interactive shell
    Shell {
        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Command timeout in seconds
        #[arg(short, long)]
        timeout: Option<u32>,
    },

    /// Service mode (internal)
    #[command(hide = true)]
    ServiceMode,
}

#[derive(Subcommand, Debug)]
pub enum ServiceCommands {
    /// List services
    List {
        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Filter by name
        #[arg(short, long)]
        filter: Option<String>,

        /// Running only
        #[arg(long)]
        running_only: bool,
    },

    /// Get service details
    Get {
        /// Service name
        #[arg(value_name = "NAME")]
        name: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Start service
    Start {
        /// Service name
        #[arg(value_name = "NAME")]
        name: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Stop service
    Stop {
        /// Service name
        #[arg(value_name = "NAME")]
        name: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Restart service
    Restart {
        /// Service name
        #[arg(value_name = "NAME")]
        name: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Create service
    Create {
        /// Service name
        #[arg(short, long)]
        name: String,

        /// Display name
        #[arg(short, long)]
        display_name: Option<String>,

        /// Path to executable
        #[arg(short, long)]
        path: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Startup type
        #[arg(short, long)]
        startup_type: Option<String>,
    },

    /// Delete service
    Delete {
        /// Service name
        #[arg(value_name = "NAME")]
        name: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Force deletion
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// Read registry value
    Read {
        /// Registry key path (e.g., HKEY_LOCAL_MACHINE\Software\...)
        #[arg(short, long)]
        key: String,

        /// Value name
        #[arg(short, long)]
        value: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Write registry value
    Write {
        /// Registry key path
        #[arg(short, long)]
        key: String,

        /// Value name
        #[arg(short, long)]
        value: String,

        /// Data to write
        #[arg(short, long)]
        data: String,

        /// Data type (REG_SZ, REG_DWORD, etc.)
        #[arg(short, long, default_value = "REG_SZ")]
        r#type: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// Delete registry value
    Delete {
        /// Registry key path
        #[arg(short, long)]
        key: String,

        /// Value name
        #[arg(short, long)]
        value: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,
    },

    /// List registry key contents
    List {
        /// Registry key path
        #[arg(short, long)]
        key: String,

        /// Target host
        #[arg(short, long)]
        host: Option<String>,

        /// Recursive listing
        #[arg(long)]
        recursive: bool,
    },
}

impl CliArgs {
    /// Create empty CliArgs with defaults
    pub fn new() -> Self {
        Self {
            computer_list: Vec::new(),
            app: String::new(),
            app_args: Vec::new(),
            user: None,
            password: None,
            use_ntlm: false,
            use_kerberos: false,
            use_system_account: false,
            interactive: false,
            session_id: None,
            run_limited: false,
            no_profile: false,
            copy_files: false,
            force_copy: false,
            version_check_copy: false,
            copy_to: None,
            working_dir: None,
            environment: HashMap::new(),
            dont_wait: false,
            timeout_seconds: None,
            terminate_after: None,
            kill_if_hung: false,
            priority: ProcessPriority::Normal,
            processors: None,
            console: false,
            elevate: false,
            accept_eula: false,
            remote_paexec_path: None,
            service_name: None,
            service_display_name: None,
            redirect_stdin: None,
            redirect_stdout: None,
            redirect_stderr: None,
            background: false,
            show_help: false,
            show_usage: false,
        }
    }
}

/// Parse command line arguments (PsExec-compatible)
pub fn parse_command_line() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        let mut cli = CliArgs::new();
        cli.show_usage = true;
        return cli;
    }

    // Skip program name and pass to new parser
    match parse_command_line_args(&args[1..]) {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("Error: {}", e);
            let mut cli = CliArgs::new();
            cli.show_usage = true;
            cli
        }
    }
}

/// Parse command line arguments with error handling
pub fn parse_command_line_args(args: &[String]) -> Result<CliArgs, String> {
    let mut cli = CliArgs::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            // Help options
            "-?" | "/?" | "-h" | "--help" => {
                cli.show_help = true;
                return Ok(cli);
            }

            // Service mode (legacy)
            "-service" => {
                cli.app = "-service".to_string();
                return Ok(cli);
            }

            // Computer list
            arg if arg.starts_with("\\\\") => {
                let computers: Vec<String> = arg[2..].split(',')
                    .map(|s| s.to_string())
                    .collect();
                cli.computer_list = computers;
            }

            // File with computer list
            "-c" if i + 1 < args.len() && args[i + 1].starts_with('@') => {
                i += 1;
                let file_path = &args[i][1..];
                cli.computer_list = read_computer_list(file_path)?;
            }

            // Authentication
            "-u" | "-user" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a value", arg));
                }
                i += 1;
                cli.user = Some(args[i].clone());
            }

            "-p" | "-pass" | "-password" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a value", arg));
                }
                i += 1;
                cli.password = Some(args[i].clone());
            }

            "-ntlm" => {
                cli.use_ntlm = true;
            }

            "-k" | "-kerberos" => {
                cli.use_kerberos = true;
            }

            // System account
            "-s" | "-system" => {
                cli.use_system_account = true;
            }

            // Interactive mode
            "-i" => {
                cli.interactive = true;
                if i + 1 < args.len() {
                    if let Ok(session) = args[i + 1].parse::<i32>() {
                        cli.session_id = Some(session);
                        i += 1;
                    }
                }
            }

            // Run limited (low integrity)
            "-l" | "-limited" => {
                cli.run_limited = true;
            }

            // No profile loading
            "-noprofile" | "-np" => {
                cli.no_profile = true;
            }

            // File copy options
            "-c" | "-copy" => {
                cli.copy_files = true;

                if i + 1 < args.len() {
                    match args[i + 1].as_str() {
                        "-f" | "-force" => {
                            cli.force_copy = true;
                            i += 1;
                        }
                        "-v" | "-verify" => {
                            cli.version_check_copy = true;
                            i += 1;
                        }
                        _ => {}
                    }
                }
            }

            "-csrc" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a path", arg));
                }
                i += 1;
                cli.copy_to = Some(args[i].clone());
            }

            // Working directory
            "-w" | "-workingdir" | "-workingdirectory" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a path", arg));
                }
                i += 1;
                cli.working_dir = Some(args[i].clone());
            }

            // Environment variables
            "-e" | "-env" | "-environment" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a value", arg));
                }
                i += 1;
                let env_str = &args[i];
                if let Some(eq_pos) = env_str.find('=') {
                    let key = env_str[..eq_pos].to_string();
                    let value = env_str[eq_pos + 1..].to_string();
                    cli.environment.insert(key, value);
                } else {
                    return Err(format!("Environment variable must be VAR=VALUE: {}", env_str));
                }
            }

            // Don't wait
            "-d" | "-nowait" | "-dontwait" => {
                cli.dont_wait = true;
            }

            // Timeout
            "-n" | "-timeout" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires seconds", arg));
                }
                i += 1;
                cli.timeout_seconds = Some(args[i].parse().map_err(|_| "Invalid timeout")?);
            }

            // Terminate after
            "-t" | "-terminate" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires seconds", arg));
                }
                i += 1;
                cli.terminate_after = Some(args[i].parse().map_err(|_| "Invalid terminate")?);
            }

            // Kill if hung
            "-h" | "-killhung" => {
                cli.kill_if_hung = true;
            }

            // Priority options
            "-realtime" | "-r" => {
                cli.priority = ProcessPriority::Realtime;
            }
            "-high" => {
                cli.priority = ProcessPriority::High;
            }
            "-abovenormal" | "-an" => {
                cli.priority = ProcessPriority::AboveNormal;
            }
            "-belownormal" | "-bn" => {
                cli.priority = ProcessPriority::BelowNormal;
            }
            "-low" | "-idle" => {
                cli.priority = ProcessPriority::Idle;
            }

            // Processor affinity
            "-a" | "-affinity" | "-processors" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires list", arg));
                }
                i += 1;
                let procs: Result<Vec<u16>, _> = args[i].split(',')
                    .map(|s| s.parse().map_err(|_| format!("Invalid processor: {}", s)))
                    .collect();
                cli.processors = Some(procs?);
            }

            // Console options
            "-x" | "-console" => {
                cli.console = true;
            }
            "-elevated" | "-elevate" => {
                cli.elevate = true;
            }

            // EULA
            "-accepteula" => {
                cli.accept_eula = true;
            }

            // Remote PAExec
            "-rpath" | "-remotepath" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a path", arg));
                }
                i += 1;
                cli.remote_paexec_path = Some(args[i].clone());
            }
            "-sname" | "-servicename" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a name", arg));
                }
                i += 1;
                cli.service_name = Some(args[i].clone());
            }
            "-sdname" | "-servicedisplayname" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a name", arg));
                }
                i += 1;
                cli.service_display_name = Some(args[i].clone());
            }

            // I/O Redirection
            "-stdin" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a file", arg));
                }
                i += 1;
                cli.redirect_stdin = Some(args[i].clone());
            }
            "-stdout" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a file", arg));
                }
                i += 1;
                cli.redirect_stdout = Some(args[i].clone());
            }
            "-stderr" => {
                if i + 1 >= args.len() {
                    return Err(format!("{} requires a file", arg));
                }
                i += 1;
                cli.redirect_stderr = Some(args[i].clone());
            }

            // Background
            "-background" | "-bg" => {
                cli.background = true;
            }

            // Unknown option
            arg if arg.starts_with('-') || arg.starts_with('/') => {
                return Err(format!("Unknown option: {}", arg));
            }

            // Application and arguments
            _ => {
                if cli.app.is_empty() {
                    cli.app = arg.clone();
                } else {
                    cli.app_args.push(arg.clone());
                }
            }
        }

        i += 1;
    }

    // Validate required arguments
    if cli.computer_list.is_empty() && !cli.show_help {
        return Err("No target computer specified. Use \\\\computer or -c @file".to_string());
    }

    if cli.app.is_empty() && !cli.show_help {
        return Err("No application specified".to_string());
    }

    Ok(cli)
}

/// Read computer list from file
fn read_computer_list(path: &str) -> Result<Vec<String>, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read computer list file: {}", e))?
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            if line.starts_with("\\\\") {
                Ok(line[2..].to_string())
            } else {
                Ok(line)
            }
        })
        .collect()
}

pub fn print_usage() {
    println!();
    println!("PAExec-rs - Execute programs on remote systems");
    println!("Usage: psexec-rs [\\\\computer[,computer2[,...]] | @file]");
    println!("                [-u user [-p password] | -s] [-i [session]]");
    println!("                [-c [-f | -v]] [-d] [-w directory] [-high | -low | etc.]");
    println!("                [-a processors] [-n timeout] program [arguments]");
    println!();
    println!("Type: psexec-rs -? for detailed help");
    println!();
}

pub fn print_help() {
    println!("PaExec-rs - Execute processes remotely");
    println!();
    println!("Usage: psexec-rs [\\\\computer[,computer[,...]] | @file] [options] program [arguments]");
    println!();
    println!("Options:");
    println!("  -? /? -h --help          Show this help");
    println!("  -u user                  Username for authentication");
    println!("  -p password              Password for authentication");
    println!("  -ntlm                    Use NTLM authentication");
    println!("  -k / -kerberos           Use Kerberos authentication");
    println!();
    println!("Process Execution:");
    println!("  -s                       Run as SYSTEM account");
    println!("  -i [session]             Run interactively (optionally specify session ID)");
    println!("  -l                       Run with limited privileges (low integrity)");
    println!("  -noprofile               Do not load user profile");
    println!();
    println!("File Copy:");
    println!("  -c [-f|-v]               Copy program to remote and execute");
    println!("                           -f: Force copy (overwrite existing)");
    println!("                           -v: Copy only if newer or different version");
    println!("  -csrc path               Copy from this local path");
    println!();
    println!("Working Environment:");
    println!("  -w directory             Set working directory");
    println!("  -e VAR=VALUE             Set environment variable");
    println!();
    println!("Process Control:");
    println!("  -d                       Don't wait for process to terminate");
    println!("  -n seconds               Timeout for connection (default: 60)");
    println!("  -t seconds               Terminate process after specified seconds");
    println!("  -h                       Kill process if hung");
    println!();
    println!("Priority:");
    println!("  -realtime / -r           Run with REALTIME priority");
    println!("  -high                    Run with HIGH priority");
    println!("  -abovenormal / -an       Run with ABOVE_NORMAL priority");
    println!("  -belownormal / -bn       Run with BELOW_NORMAL priority");
    println!("  -low / -idle             Run with IDLE priority");
    println!();
    println!("Processor Affinity:");
    println!("  -a processors            Run on specified processors (e.g., 0,1,2 or 0-3)");
    println!();
    println!("Console:");
    println!("  -x                       Run on Winlogon desktop (interactive only)");
    println!("  -elevated                Run with elevated privileges");
    println!();
    println!("Service:");
    println!("  -sname name              Use this service name");
    println!("  -sdname name             Use this service display name");
    println!("  -rpath path              Path to PAExec on remote");
    println!();
    println!("I/O Redirection:");
    println!("  -stdin file              Redirect stdin from file");
    println!("  -stdout file             Redirect stdout to file");
    println!("  -stderr file             Redirect stderr to file");
    println!();
    println!("Other:");
    println!("  -accepteula              Accept EULA");
    println!("  -background / -bg        Run in background mode");
    println!();
    println!("Examples:");
    println!("  psexec-rs \\\\server cmd");
    println!("  psexec-rs \\\\server1,server2 -u admin -p pass -s cmd");
    println!("  psexec-rs \\\\server -i 1 -d notepad");
    println!("  psexec-rs \\\\server -c -f app.exe arg1 arg2");
    println!("  psexec-rs @computers.txt -u domain\\\\user -p pass -s powershell");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let args = vec![
            "\\\\server".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.computer_list, vec!["server"]);
        assert_eq!(cli.app, "cmd");
    }

    #[test]
    fn test_parse_system_account() {
        let args = vec![
            "\\\\server".to_string(),
            "-s".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert!(cli.use_system_account);
    }

    #[test]
    fn test_parse_interactive() {
        let args = vec![
            "\\\\server".to_string(),
            "-i".to_string(),
            "1".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert!(cli.interactive);
        assert_eq!(cli.session_id, Some(1));
    }

    #[test]
    fn test_parse_copy_force() {
        let args = vec![
            "\\\\server".to_string(),
            "-c".to_string(),
            "-f".to_string(),
            "app.exe".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert!(cli.copy_files);
        assert!(cli.force_copy);
    }

    #[test]
    fn test_parse_working_dir() {
        let args = vec![
            "\\\\server".to_string(),
            "-w".to_string(),
            "C:\\temp".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.working_dir, Some("C:\\temp".to_string()));
    }

    #[test]
    fn test_parse_priority() {
        let args = vec![
            "\\\\server".to_string(),
            "-high".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.priority, ProcessPriority::High);
    }

    #[test]
    fn test_parse_dont_wait() {
        let args = vec![
            "\\\\server".to_string(),
            "-d".to_string(),
            "notepad".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert!(cli.dont_wait);
    }

    #[test]
    fn test_parse_timeout() {
        let args = vec![
            "\\\\server".to_string(),
            "-n".to_string(),
            "30".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.timeout_seconds, Some(30));
    }

    #[test]
    fn test_parse_multiple_computers() {
        let args = vec![
            "\\\\server1,server2,server3".to_string(),
            "cmd".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.computer_list, vec!["server1", "server2", "server3"]);
    }

    #[test]
    fn test_parse_complex() {
        let args = vec![
            "\\\\server1,server2".to_string(),
            "-u".to_string(),
            "admin".to_string(),
            "-p".to_string(),
            "pass".to_string(),
            "-s".to_string(),
            "-i".to_string(),
            "0".to_string(),
            "-c".to_string(),
            "-f".to_string(),
            "-w".to_string(),
            "C:\\temp".to_string(),
            "-d".to_string(),
            "-high".to_string(),
            "app.exe".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];

        let cli = parse_command_line_args(&args).unwrap();
        assert_eq!(cli.computer_list, vec!["server1", "server2"]);
        assert_eq!(cli.user, Some("admin".to_string()));
        assert_eq!(cli.password, Some("pass".to_string()));
        assert!(cli.use_system_account);
        assert!(cli.interactive);
        assert_eq!(cli.session_id, Some(0));
        assert!(cli.copy_files);
        assert!(cli.force_copy);
        assert_eq!(cli.working_dir, Some("C:\\temp".to_string()));
        assert!(cli.dont_wait);
        assert_eq!(cli.priority, ProcessPriority::High);
        assert_eq!(cli.app, "app.exe");
        assert_eq!(cli.app_args, vec!["arg1", "arg2"]);
    }
}
