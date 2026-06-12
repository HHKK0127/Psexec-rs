mod analyzer;
mod ui;
mod winapi_utils;
mod remote_executor;

use std::env;
use clap::Parser;
use psexec_rs::cli::{ModernCli, parse_command_line};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Special case: service mode (-service flag) - must be handled before clap parsing
    if args.len() == 2 && args[1] == "-service" {
        return run_service_mode();
    }

    // Special case: no arguments or --gui flag - GUI mode
    if args.len() == 1 || (args.len() == 2 && args[1] == "--gui") {
        return run_gui().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
    }

    // Special case: legacy PsExec-compatible syntax (\\computer ...)
    if args.len() > 1 && args[1].starts_with("\\\\") {
        return run_legacy_cli(&args);
    }

    // Default: try modern clap-based CLI
    match ModernCli::try_parse() {
        Ok(cli) => {
            env_logger::init();
            run_modern_cli(cli)?;
            Ok(())
        }
        Err(_) => {
            // Fall back to legacy CLI parsing
            run_legacy_cli(&args)?;
            Ok(())
        }
    }
}

/// Service mode: Run as Windows service
fn run_service_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("Service mode not yet implemented");
    Ok(())
}

/// Modern clap-based CLI
fn run_modern_cli(cli: ModernCli) -> Result<(), Box<dyn std::error::Error>> {
    use psexec_rs::cli::{Commands, ServiceCommands, RegistryCommands};
    use psexec_rs::cli_handlers::*;

    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        match cli.command {
            Commands::Exec { host, command, method, working_dir, copy } => {
                handle_exec(host, command, method, working_dir, copy).await?;
            }

            Commands::Service(cmd) => {
                handle_service_command(None, cmd).await?;
            }

            Commands::Registry(cmd) => {
                handle_registry_command(None, cmd).await?;
            }

            Commands::Script { r#type, file, host, policy, args } => {
                handle_script_command(host, r#type, file, policy, args).await?;
            }

            Commands::Transfer { direction, source, destination, host } => {
                handle_transfer_command(host, direction, source, destination).await?;
            }

            Commands::Shell { host, timeout } => {
                handle_shell_command(host, timeout).await?;
            }

            Commands::ServiceMode => {
                println!("Service mode");
            }
        }
        Ok(())
    })
}

/// Legacy PsExec-compatible CLI
fn run_legacy_cli(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use psexec_rs::settings::RemoteSettings;
    use psexec_rs::cli::ProcessPriority;

    let cli = parse_command_line();

    if cli.show_usage {
        psexec_rs::cli::print_usage();
        return Ok(());
    }

    if cli.show_help {
        psexec_rs::cli::print_usage();
        return Ok(());
    }

    if cli.computer_list.is_empty() {
        eprintln!("Error: No computers specified");
        psexec_rs::cli::print_usage();
        return Err("Missing computers".into());
    }

    if cli.app.is_empty() {
        eprintln!("Error: No command specified");
        psexec_rs::cli::print_usage();
        return Err("Missing command".into());
    }

    // Map CliArgs to RemoteSettings (Phase 2)
    let mut settings = RemoteSettings::default();
    settings.computer_list = cli.computer_list.clone();
    settings.app = cli.app.clone();
    settings.app_args = cli.app_args.join(" ");
    settings.user = cli.user.unwrap_or_default();
    settings.password = cli.password.unwrap_or_default();
    settings.use_system_account = cli.use_system_account;
    settings.interactive = cli.interactive;
    settings.session_to_interact_with = cli.session_id.unwrap_or(-1);
    settings.run_limited = cli.run_limited;
    settings.dont_load_profile = cli.no_profile;
    settings.copy_files = cli.copy_files;
    settings.force_copy = cli.force_copy;
    settings.copy_if_newer_or_higher_ver = cli.version_check_copy;
    settings.working_dir = cli.working_dir.unwrap_or_default();
    settings.dont_wait_for_terminate = cli.dont_wait;
    settings.timeout_seconds = cli.timeout_seconds.unwrap_or(0);
    settings.priority = priority_to_windows_class(cli.priority);
    settings.allowed_processors = cli.processors.unwrap_or_default();
    settings.run_elevated = cli.elevate;
    settings.service_name = cli.service_name.unwrap_or_default();

    // Display settings
    println!("Executing command on: {}", settings.computer_list.join(","));
    println!("Command: {}", settings.app);
    if !settings.app_args.is_empty() {
        println!("Args: {}", settings.app_args);
    }
    if settings.use_system_account {
        println!("Mode: System account (-s)");
    }
    if settings.interactive {
        println!("Mode: Interactive (-i)");
    }
    if !settings.working_dir.is_empty() {
        println!("Working directory: {}", settings.working_dir);
    }
    if settings.timeout_seconds > 0 {
        println!("Timeout: {} seconds", settings.timeout_seconds);
    }
    println!();

    let receiver = remote_executor::execute_remote_command(
        &settings.computer_list.join(","),
        &settings.app,
        !settings.user.is_empty(),
        &settings.user,
        &settings.password,
    );

    // Stream output from background thread
    while let Ok(line) = receiver.recv() {
        println!("{}", line);
    }

    Ok(())
}

/// Convert ProcessPriority enum to Windows priority class value
fn priority_to_windows_class(priority: psexec_rs::cli::ProcessPriority) -> u32 {
    use psexec_rs::cli::ProcessPriority;
    match priority {
        ProcessPriority::Realtime => 0x100,        // REALTIME_PRIORITY_CLASS
        ProcessPriority::High => 0x80,             // HIGH_PRIORITY_CLASS
        ProcessPriority::AboveNormal => 0x8000,    // ABOVE_NORMAL_PRIORITY_CLASS
        ProcessPriority::Normal => 0x20,           // NORMAL_PRIORITY_CLASS
        ProcessPriority::BelowNormal => 0x4000,    // BELOW_NORMAL_PRIORITY_CLASS
        ProcessPriority::Idle => 0x40,             // IDLE_PRIORITY_CLASS
    }
}

fn print_usage() {
    println!("PAExec-rs - PE File Analyzer & Remote Command Executor");
    println!();
    println!("Usage:");
    println!("  psexec-rs                           # Start GUI (PE analyzer)");
    println!("  psexec-rs \\\\server1,server2 command  # Execute remote command");
    println!("  psexec-rs -h                        # Show help");
    println!();
}

fn print_help() {
    println!("PAExec-rs - Remote Command Executor");
    println!();
    println!("USAGE:");
    println!("  psexec-rs [OPTIONS] <computers> <command>");
    println!();
    println!("ARGUMENTS:");
    println!("  <computers>     Target computer(s) separated by commas");
    println!("                  Examples: server1,server2,server3");
    println!("                           localhost");
    println!("                           \\\\server1");
    println!();
    println!("  <command>       PowerShell command to execute");
    println!("                  Examples: Get-Process");
    println!("                           ipconfig");
    println!("                           'dir C:\\ | Select-Object -First 10'");
    println!();
    println!("OPTIONS:");
    println!("  -u, --user <username>     Domain username (e.g., DOMAIN\\username)");
    println!("  -p, --password <password> Password for authentication");
    println!("  -h, --help                Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  # Get processes from localhost");
    println!("  psexec-rs localhost \"Get-Process | Select-Object Name, Id\"");
    println!();
    println!("  # Get IP configuration from multiple servers");
    println!("  psexec-rs server1,server2 \"Get-NetIPAddress -AddressFamily IPv4\"");
    println!();
    println!("  # Execute with custom credentials");
    println!("  psexec-rs server1 -u DOMAIN\\\\admin -p password \"whoami\"");
    println!();
    println!("NOTES:");
    println!("  - Remote execution requires PowerShell remoting to be enabled");
    println!("  - Current user credentials are used if -u/-p are not specified");
    println!("  - UTF-8 output encoding is automatically applied");
    println!("  - Character encoding is automatically detected for multi-byte characters");
    println!();
}

fn run_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PAExec-rs - PE Analyzer & Remote Execution Tool",
        options,
        Box::new(|cc| {
            // Configure egui style for better readability
            let mut style = egui::Style::default();

            // Increase base font size
            style.text_styles = std::collections::BTreeMap::from_iter(vec![
                (egui::TextStyle::Body, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Heading, egui::FontId::new(22.0, egui::FontFamily::Proportional)),
                (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
            ]);

            // Increase button and widget sizes
            style.spacing.button_padding = egui::vec2(12.0, 8.0);
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.icon_width = 24.0;

            // Improve contrast and colors
            style.visuals = egui::Visuals::dark();
            style.visuals.override_text_color = Some(egui::Color32::from_rgb(240, 240, 240));

            cc.egui_ctx.set_style(style);

            Box::new(ui::AnalyzerApp::default())
        }),
    )
}
