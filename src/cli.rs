//! Command-line interface for PAExec-rs
//! Supports both modern clap-based CLI and legacy PsExec-compatible syntax

use clap::{Parser, Subcommand};

// Legacy support struct
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub computer_list: Vec<String>,
    pub app: String,
    pub app_args: String,
    pub user: String,
    pub password: String,
    pub show_help: bool,
    pub show_usage: bool,
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

/// Parse legacy PsExec-compatible command line
pub fn parse_command_line() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut cli = CliArgs {
        computer_list: Vec::new(),
        app: String::new(),
        app_args: String::new(),
        user: String::new(),
        password: String::new(),
        show_help: false,
        show_usage: false,
    };

    if args.len() < 2 {
        cli.show_usage = true;
        return cli;
    }

    let mut i = 1;

    if args[i] == "/?" || args[i] == "-?" || args[i] == "--help" {
        cli.show_help = true;
        return cli;
    }

    if args[i] == "-service" {
        cli.app = "-service".into();
        return cli;
    }

    // Parse computer list
    if args[i].starts_with("\\\\") {
        let target = args[i].trim_start_matches("\\\\");
        if target == "*" {
            eprintln!("\\\\* not yet supported");
            cli.show_usage = true;
            return cli;
        }
        for comp in target.split(',') {
            let comp = comp.trim();
            if !comp.is_empty() {
                cli.computer_list.push(comp.to_string());
            }
        }
        i += 1;
    }

    // Parse options
    while i < args.len() && args[i].starts_with('-') {
        let opt = args[i].trim_start_matches('-');

        match opt {
            "u" => {
                i += 1;
                if i < args.len() {
                    cli.user = args[i].clone();
                }
            }
            "p" => {
                i += 1;
                if i < args.len() {
                    cli.password = args[i].clone();
                }
            }
            "accepteula" => { /* silently accept */ }
            _ => {
                eprintln!("Unknown option: -{}", opt);
                cli.show_usage = true;
                return cli;
            }
        }
        i += 1;
    }

    // Remaining args: command and its arguments
    if i < args.len() {
        cli.app = args[i].clone();
        i += 1;
        if i < args.len() {
            cli.app_args = args[i..].join(" ");
        }
    } else {
        cli.show_usage = true;
    }

    cli
}

pub fn print_usage() {
    println!();
    println!("PAExec-rs - Execute programs on remote systems");
    println!("Usage: psexec-rs [\\\\computer[,computer2[,...]] | @file | \\\\*]");
    println!("                [-u user [-p password] | -s] [-i [session]]");
    println!("                [-c [-f | -v]] [-d] [-w directory] [-low | -belownormal |");
    println!("                -abovenormal | -high | -realtime | -background]");
    println!("                [-a processors] [-n timeout] program [arguments]");
    println!();
    println!("Modern CLI usage:");
    println!("  psexec-rs exec --host <host> <command>");
    println!("  psexec-rs service list --host <host>");
    println!("  psexec-rs registry read --host <host> --key <key> --value <value>");
    println!("  psexec-rs script --type ps --file <file> --host <host>");
    println!();
}
