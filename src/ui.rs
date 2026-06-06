use crate::analyzer::AnalysisResult;
use eframe::egui;
use std::sync::mpsc;
use psexec_rs::cli_response::{ServiceListResponse, RegistryListResponse, ScriptExecResult, OperationResult};
use chrono;

pub struct AnalyzerApp {
    pub result: Option<AnalysisResult>,
    pub selected_tab: Tab,
    pub status: String,
    pub filter: String,

    // Remote Execution 関連フィールド
    pub remote_computers: String,
    pub remote_command: String,
    pub remote_output: Vec<String>,
    pub command_history: Vec<String>,
    pub use_custom_auth: bool,
    pub remote_username: String,
    pub remote_password: String,
    pub is_executing: bool,

    // Service Management フィールド
    pub service_host: String,
    pub service_host_history: Vec<String>,
    pub services: Vec<(String, String)>,
    pub selected_service: Option<usize>,
    pub service_status_message: String,

    // Service Create Dialog フィールド
    pub service_create_open: bool,
    pub service_create_name: String,
    pub service_create_path: String,
    pub service_create_display_name: String,
    pub service_create_startup_type: String,

    // Registry Browser フィールド
    pub registry_host: String,
    pub registry_path: String,
    pub registry_entries: Vec<(String, String, String)>,
    pub selected_registry_entry: Option<usize>,
    pub registry_status_message: String,

    // Registry Edit Dialog フィールド
    pub registry_edit_open: bool,
    pub registry_edit_name: String,
    pub registry_edit_type: String,
    pub registry_edit_value: String,

    // Script Executor フィールド
    pub script_type: String,
    pub script_content: String,
    pub script_host: String,
    pub script_arguments: String,
    pub script_output: String,
    pub script_status_message: String,

    // Async task state
    pub service_list_loading: bool,
    pub service_list_result: Option<Result<ServiceListResponse, String>>,
    pub service_list_rx: Option<mpsc::Receiver<Result<ServiceListResponse, String>>>,
    pub registry_list_loading: bool,
    pub registry_list_result: Option<Result<RegistryListResponse, String>>,
    pub registry_list_rx: Option<mpsc::Receiver<Result<RegistryListResponse, String>>>,
    pub script_exec_loading: bool,
    pub script_exec_result: Option<Result<ScriptExecResult, String>>,
    pub script_exec_rx: Option<mpsc::Receiver<Result<ScriptExecResult, String>>>,
    pub service_op_result: Option<Result<OperationResult, String>>,
    pub registry_op_result: Option<Result<OperationResult, String>>,

    // Timeout & Performance settings
    pub timeout_seconds: u32,
    pub enable_caching: bool,

    // Batch operations state
    pub batch_select_mode: bool,
    pub batch_selected_services: Vec<usize>,
    pub batch_operation: String, // "start", "stop", "restart", "delete"

    // Output streaming receiver (not serialized)
    pub output_receiver: Option<std::sync::mpsc::Receiver<String>>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Tab {
    Overview,
    PeInfo,
    Imports,
    Strings,
    Signature,
    RemoteExecution,
    ServiceManagement,
    Registry,
    Script,
}

impl Default for AnalyzerApp {
    fn default() -> Self {
        Self {
            result: None,
            selected_tab: Tab::Overview,
            status: "Ready".to_string(),
            filter: String::new(),
            remote_computers: String::new(),
            remote_command: String::new(),
            remote_output: Vec::new(),
            command_history: Vec::new(),
            use_custom_auth: false,
            remote_username: String::new(),
            remote_password: String::new(),
            is_executing: false,
            service_host: "localhost".to_string(),
            service_host_history: vec!["localhost".to_string()],
            services: Vec::new(),
            selected_service: None,
            service_status_message: "Click 'Refresh' to load services".to_string(),
            service_create_open: false,
            service_create_name: String::new(),
            service_create_path: String::new(),
            service_create_display_name: String::new(),
            service_create_startup_type: "Automatic".to_string(),
            registry_host: "localhost".to_string(),
            registry_path: "HKEY_LOCAL_MACHINE".to_string(),
            registry_entries: Vec::new(),
            selected_registry_entry: None,
            registry_status_message: "Enter registry path and click 'Browse'".to_string(),
            registry_edit_open: false,
            registry_edit_name: String::new(),
            registry_edit_type: "REG_SZ".to_string(),
            registry_edit_value: String::new(),
            script_type: "powershell".to_string(),
            script_content: String::new(),
            script_host: "localhost".to_string(),
            script_arguments: String::new(),
            script_output: String::new(),
            script_status_message: "Ready to execute script".to_string(),
            service_list_loading: false,
            service_list_result: None,
            service_list_rx: None,
            registry_list_loading: false,
            registry_list_result: None,
            registry_list_rx: None,
            script_exec_loading: false,
            script_exec_result: None,
            script_exec_rx: None,
            service_op_result: None,
            registry_op_result: None,
            timeout_seconds: 30,
            enable_caching: false,
            batch_select_mode: false,
            batch_selected_services: Vec::new(),
            batch_operation: String::new(),
            output_receiver: None,
        }
    }
}

// Async task execution helper for GUI operations
fn spawn_service_list_task(
    host: String,
) -> mpsc::Receiver<Result<ServiceListResponse, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::get_service_list(Some(host), None, false)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_registry_list_task(
    host: String,
    path: String,
) -> mpsc::Receiver<Result<RegistryListResponse, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::get_registry_entries(Some(host), path)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_script_exec_task(
    host: String,
    script_type: String,
    content: String,
    arguments: Option<String>,
) -> mpsc::Receiver<Result<ScriptExecResult, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::execute_script_op(Some(host), script_type, content, arguments)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_service_start_task(
    host: String,
    service_name: String,
) -> mpsc::Receiver<Result<OperationResult, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::start_service_op(Some(host), service_name)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_service_stop_task(
    host: String,
    service_name: String,
) -> mpsc::Receiver<Result<OperationResult, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::stop_service_op(Some(host), service_name)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_service_restart_task(
    host: String,
    service_name: String,
) -> mpsc::Receiver<Result<OperationResult, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            psexec_rs::cli_handlers::restart_service_op(Some(host), service_name)
                .await
                .map_err(|e| e.to_string())
        });
        let _ = tx.send(result);
    });

    rx
}

fn spawn_service_create_task(
    host: String,
    name: String,
    display_name: String,
    path: String,
) -> mpsc::Receiver<Result<OperationResult, String>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            // Use the CLI handler's create_service API
            let ctx = psexec_rs::ServiceContext::new(&host);
            match psexec_rs::service::create_service(
                &ctx,
                &name,
                &display_name,
                &path,
                psexec_rs::ServiceStartupType::Automatic,
            )
            .await
            {
                Ok(result) => {
                    if result.success {
                        Ok(OperationResult::success(format!(
                            "Service '{}' created successfully",
                            name
                        )))
                    } else {
                        Ok(OperationResult::error(
                            result
                                .error_message
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        ))
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        });
        let _ = tx.send(result);
    });

    rx
}

impl eframe::App for AnalyzerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for output from background thread
        if let Some(ref receiver) = self.output_receiver {
            while let Ok(line) = receiver.try_recv() {
                self.remote_output.push(line);
            }
        } else if self.is_executing {
            // If receiver is None and we're supposed to be executing, stop
            self.is_executing = false;
        }

        // Check for service list async results
        if let Some(ref rx) = self.service_list_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.service_list_loading = false;
                    self.service_list_result = Some(result.clone());

                    match result {
                        Ok(response) => {
                            // Convert to display format
                            self.services = response.services
                                .iter()
                                .map(|s| (s.name.clone(), s.state.to_string()))
                                .collect();
                            self.service_status_message = format!("✓ Loaded {} service(s)", response.count);
                        }
                        Err(e) => {
                            self.service_status_message = format!("❌ Error: {}", e);
                        }
                    }
                    self.service_list_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still loading, request repaint
                    ctx.request_repaint();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, stop loading
                    self.service_list_loading = false;
                    self.service_list_rx = None;
                }
            }
        }

        // Check for registry list async results
        if let Some(ref rx) = self.registry_list_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.registry_list_loading = false;
                    self.registry_list_result = Some(result.clone());

                    match result {
                        Ok(response) => {
                            // Convert to display format
                            self.registry_entries = response.entries
                                .iter()
                                .map(|e| (e.name.clone(), e.value_type.clone(), e.data.clone()))
                                .collect();
                            self.registry_status_message = format!("✓ Loaded {} entries", response.count);
                        }
                        Err(e) => {
                            self.registry_status_message = format!("❌ Error: {}", e);
                        }
                    }
                    self.registry_list_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still loading, request repaint
                    ctx.request_repaint();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, stop loading
                    self.registry_list_loading = false;
                    self.registry_list_rx = None;
                }
            }
        }

        // Check for script exec async results
        if let Some(ref rx) = self.script_exec_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.script_exec_loading = false;
                    self.script_exec_result = Some(result.clone());

                    match result {
                        Ok(response) => {
                            self.script_output = format!(
                                "Exit Code: {}\n\n[STDOUT]\n{}\n\n[STDERR]\n{}",
                                response.exit_code, response.stdout, response.stderr
                            );
                            self.script_status_message = format!("✓ Execution completed ({}ms)", response.execution_time_ms);
                        }
                        Err(e) => {
                            self.script_status_message = format!("❌ Error: {}", e);
                        }
                    }
                    self.script_exec_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still loading, request repaint
                    ctx.request_repaint();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Channel closed, stop loading
                    self.script_exec_loading = false;
                    self.script_exec_rx = None;
                }
            }
        }

        // Request repaint if still executing
        if self.is_executing {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PE File Analyzer");
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.status = format!("Analyzing: {}", path.display());
                        match crate::analyzer::analyze_file(&path) {
                            Ok(res) => {
                                self.result = Some(res);
                                self.status = "Analysis complete".to_string();
                            }
                            Err(e) => {
                                self.status = format!("Error: {}", e);
                                self.result = None;
                            }
                        }
                    }
                }
                ui.label(&self.status);
            });

            ui.separator();

            // タブナビゲーション
            ui.horizontal(|ui| {
                // PE Analyzer tabs
                if self.result.is_some() {
                    ui.selectable_value(&mut self.selected_tab, Tab::Overview, "Overview");
                    ui.selectable_value(&mut self.selected_tab, Tab::PeInfo, "PE Info");
                    ui.selectable_value(&mut self.selected_tab, Tab::Imports, "Imports");
                    ui.selectable_value(&mut self.selected_tab, Tab::Strings, "Strings");
                    ui.selectable_value(&mut self.selected_tab, Tab::Signature, "Signature");
                    ui.separator();
                }

                // Management tabs (常に表示)
                ui.selectable_value(&mut self.selected_tab, Tab::RemoteExecution, "🖥️ Remote Exec");
                ui.selectable_value(&mut self.selected_tab, Tab::ServiceManagement, "⚙️ Services");
                ui.selectable_value(&mut self.selected_tab, Tab::Registry, "📋 Registry");
                ui.selectable_value(&mut self.selected_tab, Tab::Script, "📝 Script");
            });

            ui.separator();

            match self.selected_tab {
                // PE Analysis tabs
                Tab::Overview | Tab::PeInfo | Tab::Imports | Tab::Strings | Tab::Signature => {
                    if let Some(res) = &self.result {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            match self.selected_tab {
                                Tab::Overview => show_overview(ui, res),
                                Tab::PeInfo => show_pe_info(ui, res),
                                Tab::Imports => show_imports(ui, res, &mut self.filter),
                                Tab::Strings => show_strings(ui, res, &mut self.filter),
                                Tab::Signature => show_signature(ui, res),
                                _ => {}
                            }
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a PE file to analyze.");
                        });
                    }
                }
                // Management tabs
                Tab::RemoteExecution => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_remote_execution(ui, self);
                    });
                }
                Tab::ServiceManagement => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_service_management(ui, self);
                    });
                }
                Tab::Registry => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_registry_browser(ui, self);
                    });
                }
                Tab::Script => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_script_executor(ui, self);
                    });
                }
            }
        });

        // Show service create dialog if open
        show_service_create_dialog(ctx, self);

        // Show registry edit dialog if open
        show_registry_edit_dialog(ctx, self);
    }
}

fn show_overview(ui: &mut egui::Ui, res: &AnalysisResult) {
    ui.heading("Overview");
    ui.monospace(format!("File Name:        {}", res.file_name));
    ui.monospace(format!("File Path:        {}", res.file_path));
    ui.monospace(format!("File Size:        {} bytes ({:.2} KB)", res.file_size, res.file_size as f64 / 1024.0));
    ui.monospace(format!("Created:          {}", res.created));
    ui.monospace(format!("Modified:         {}", res.modified));
    ui.monospace(format!("SHA256:           {}", res.sha256));

    ui.separator();
    ui.heading("Version Information");
    let vi = &res.version_info;
    if vi.file_description.is_empty() && vi.company_name.is_empty() {
        ui.label("No version information available.");
    } else {
        if !vi.file_description.is_empty() { ui.monospace(format!("Description:      {}", vi.file_description)); }
        if !vi.file_version.is_empty() { ui.monospace(format!("File Version:     {}", vi.file_version)); }
        if !vi.product_version.is_empty() { ui.monospace(format!("Product Version:  {}", vi.product_version)); }
        if !vi.product_name.is_empty() { ui.monospace(format!("Product Name:     {}", vi.product_name)); }
        if !vi.company_name.is_empty() { ui.monospace(format!("Company:          {}", vi.company_name)); }
        if !vi.internal_name.is_empty() { ui.monospace(format!("Internal Name:    {}", vi.internal_name)); }
        if !vi.original_filename.is_empty() { ui.monospace(format!("Original File:    {}", vi.original_filename)); }
        if !vi.copyright.is_empty() { ui.monospace(format!("Copyright:        {}", vi.copyright)); }
    }
}

fn show_pe_info(ui: &mut egui::Ui, res: &AnalysisResult) {
    ui.heading("PE Information");
    let pe = &res.pe_info;
    ui.monospace(format!("Architecture:     {}", if pe.is_64bit { "x86-64 (PE32+)" } else { "x86 (PE32)" }));
    ui.monospace(format!("Machine:          {}", pe.machine));
    ui.monospace(format!("Subsystem:        {}", pe.subsystem));
    ui.monospace(format!("Entry Point:      {}", pe.entry_point));
    ui.monospace(format!("Image Base:       {}", pe.image_base));
    ui.monospace(format!("Timestamp:        {} (Unix timestamp)", pe.timestamp));

    ui.separator();
    ui.heading("Sections");
    if pe.sections.is_empty() {
        ui.label("No sections found.");
    } else {
        for sec in &pe.sections {
            ui.monospace(sec);
        }
    }
}

fn show_imports(ui: &mut egui::Ui, res: &AnalysisResult, filter: &mut String) {
    ui.heading("Imports");
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(filter);
    });

    let filter_lower = filter.to_lowercase();
    let should_filter = !filter_lower.is_empty();

    for dll in &res.imports {
        if should_filter && !dll.name.to_lowercase().contains(&filter_lower) {
            continue;
        }
        ui.collapsing(format!("{} ({} functions)", dll.name, dll.functions.len()), |ui| {
            for func in &dll.functions {
                if should_filter && !func.to_lowercase().contains(&filter_lower) {
                    continue;
                }
                ui.monospace(format!("  {}", func));
            }
        });
    }
}

fn show_strings(ui: &mut egui::Ui, res: &AnalysisResult, filter: &mut String) {
    ui.heading("Relevant Strings (PSExec Keywords)");
    ui.label("Note: Showing strings matching: psexec, service, process, token, advapi, create, open, etc.");

    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(filter);
    });

    let filter_lower = filter.to_lowercase();
    let should_filter = !filter_lower.is_empty();

    ui.label(format!("ASCII strings: {}", res.strings_ascii.len()));
    egui::Grid::new("ascii_strings").show(ui, |ui| {
        for s in &res.strings_ascii {
            if should_filter && !s.to_lowercase().contains(&filter_lower) {
                continue;
            }
            ui.monospace(s);
            ui.end_row();
        }
    });

    ui.separator();
    ui.label(format!("Unicode strings: {}", res.strings_unicode.len()));
    egui::Grid::new("unicode_strings").show(ui, |ui| {
        for s in &res.strings_unicode {
            if should_filter && !s.to_lowercase().contains(&filter_lower) {
                continue;
            }
            ui.monospace(s);
            ui.end_row();
        }
    });
}

fn show_signature(ui: &mut egui::Ui, res: &AnalysisResult) {
    ui.heading("Digital Signature");
    let sig = &res.signature;
    ui.monospace(format!("Status:           {}", sig.status));

    if !sig.signer.is_empty() {
        ui.monospace(format!("Signer:           {}", sig.signer));
    }
    if !sig.valid_from.is_empty() {
        ui.monospace(format!("Valid From:       {}", sig.valid_from));
    }
    if !sig.valid_to.is_empty() {
        ui.monospace(format!("Valid To:         {}", sig.valid_to));
    }
    if !sig.serial.is_empty() {
        ui.monospace(format!("Serial:           {}", sig.serial));
    }
    if !sig.thumbprint.is_empty() {
        ui.monospace(format!("Thumbprint:       {}", sig.thumbprint));
    }

    ui.separator();
    ui.label("Note: Detailed certificate properties (subject, issuer, serial, thumbprint) require additional certificate chain traversal via WinCrypt APIs.");
}

fn show_remote_execution(ui: &mut egui::Ui, app: &mut AnalyzerApp) {
    ui.heading("Remote Command Execution");

    // Target Computers
    ui.label("Target Computers (comma-separated):");
    ui.text_edit_singleline(&mut app.remote_computers);
    ui.label("Example: server1,server2,server3 or \\\\server1");
    ui.label("Note: For local execution, use 'localhost' or '.', e.g., 'localhost,server2'");

    ui.separator();

    // Command Input with History
    ui.label("Command:");
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut app.remote_command);
        if ui.button("History").clicked() {
            // Show command history as dropdown
            ui.menu_button("History", |ui| {
                for (i, cmd) in app.command_history.iter().enumerate() {
                    if ui.button(format!("{}. {}", i + 1, cmd)).clicked() {
                        app.remote_command = cmd.clone();
                    }
                }
                if app.command_history.is_empty() {
                    ui.label("(No history yet)");
                }
            });
        }
    });
    ui.label("Example: Get-Process | Select-Object Name, Id");
    ui.label("         | dir C:\\Windows | Select-Object -First 20");

    ui.separator();

    // Authentication Section
    ui.heading("Authentication");
    ui.horizontal(|ui| {
        ui.radio_value(&mut app.use_custom_auth, false, "Use current user");
        ui.radio_value(&mut app.use_custom_auth, true, "Use custom credentials");
    });

    if app.use_custom_auth {
        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut app.remote_username);
        });
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.text_edit_singleline(&mut app.remote_password);
        });
    }

    ui.separator();

    // Execute Button
    if !app.is_executing {
        ui.horizontal(|ui| {
            if ui.button("Execute").clicked() {
                if !app.remote_computers.is_empty() && !app.remote_command.is_empty() {
                    // Save to history
                    if !app.command_history.contains(&app.remote_command) {
                        app.command_history.push(app.remote_command.clone());
                        if app.command_history.len() > 20 {
                            app.command_history.remove(0);
                        }
                    }
                    app.is_executing = true;
                    app.remote_output.clear();
                    app.remote_output.push(format!("Starting execution at {}", chrono::Local::now().format("%H:%M:%S")));
                    app.remote_output.push("".to_string());

                    // Start actual execution with remote_executor
                    let receiver = crate::remote_executor::execute_remote_command(
                        &app.remote_computers,
                        &app.remote_command,
                        app.use_custom_auth,
                        &app.remote_username,
                        &app.remote_password,
                    );
                    app.output_receiver = Some(receiver);
                    app.status = "Executing command...".to_string();
                } else {
                    app.status = "Error: Please fill in computers and command".to_string();
                }
            }
            if ui.button("Clear Output").clicked() {
                app.remote_output.clear();
            }
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("⏳ Executing...");
            if ui.button("Stop").clicked() {
                app.is_executing = false;
                app.output_receiver = None;
                app.remote_output.push("".to_string());
                app.remote_output.push("--- Execution cancelled by user ---".to_string());
                app.status = "Execution cancelled".to_string();
            }
        });
    }

    ui.separator();

    // Output Panel
    ui.heading("Output");
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for line in &app.remote_output {
                if line.contains("[ERROR]") || line.contains("Error") {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), line);
                } else if line.contains("[Success]") || line.contains("Completed") {
                    ui.colored_label(egui::Color32::from_rgb(100, 255, 100), line);
                } else if line.starts_with("===") {
                    ui.colored_label(egui::Color32::from_rgb(150, 200, 255), line);
                } else {
                    ui.monospace(line);
                }
            }
        });

    ui.separator();

    // Export Button
    if ui.button("Export Results").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("CSV", &["csv"])
            .save_file()
        {
            let _ = std::fs::write(
                &path,
                app.remote_output.join("\n"),
            );
            app.status = format!("Results exported to {}", path.display());
        }
    }
}

fn show_service_management(ui: &mut egui::Ui, app: &mut AnalyzerApp) {
    ui.heading("⚙️ Service Management");
    ui.separator();

    // Settings panel
    ui.collapsing("⚡ Settings", |ui| {
        ui.horizontal(|ui| {
            ui.label("Timeout (seconds):");
            ui.add(egui::Slider::new(&mut app.timeout_seconds, 5..=300)
                .step_by(5.0)
                .text("sec"));
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut app.enable_caching, "Enable Result Caching");
            ui.label("(Cache results for 1 minute)");
        });

        ui.horizontal(|ui| {
            if ui.button("🔄 Reset Defaults").clicked() {
                app.timeout_seconds = 30;
                app.enable_caching = false;
            }
        });
    });

    ui.separator();

    // Host input with history dropdown
    ui.horizontal(|ui| {
        ui.label("Host:");

        // Host dropdown history
        egui::ComboBox::from_label("")
            .selected_text(&app.service_host)
            .show_ui(ui, |ui| {
                for hist_host in &app.service_host_history.clone() {
                    if ui.selectable_value(&mut app.service_host, hist_host.clone(), hist_host).clicked() {
                        // Host selected from history
                    }
                }
            });

        if app.service_list_loading {
            ui.label("⏳ Loading...");
        } else if ui.button("🔄 Refresh").clicked() {
            let host = app.service_host.clone();
            if !host.is_empty() {
                // Add to history if not already present
                if !app.service_host_history.contains(&host) {
                    app.service_host_history.insert(0, host.clone());
                    // Keep history to max 10 items
                    if app.service_host_history.len() > 10 {
                        app.service_host_history.pop();
                    }
                }

                app.service_status_message = format!("Loading services from {}...", host);
                app.service_list_loading = true;
                let rx = spawn_service_list_task(host.clone());
                app.service_list_rx = Some(rx);
            } else {
                app.service_status_message = "Please enter a host name".to_string();
            }
        }
    });

    ui.separator();

    // Batch mode controls
    ui.horizontal(|ui| {
        if ui.button(if app.batch_select_mode { "✓ Batch Mode" } else { "☐ Batch Mode" }).clicked() {
            app.batch_select_mode = !app.batch_select_mode;
            if !app.batch_select_mode {
                app.batch_selected_services.clear();
            }
        }

        if app.batch_select_mode && !app.batch_selected_services.is_empty() {
            ui.label(format!("Selected: {}", app.batch_selected_services.len()));

            egui::ComboBox::from_label("Batch Op:")
                .selected_text(&app.batch_operation)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.batch_operation, "start".to_string(), "Start");
                    ui.selectable_value(&mut app.batch_operation, "stop".to_string(), "Stop");
                    ui.selectable_value(&mut app.batch_operation, "restart".to_string(), "Restart");
                });

            if ui.button("▶ Execute Batch").clicked() {
                if !app.batch_operation.is_empty() {
                    app.service_status_message = format!(
                        "Executing batch {}: {} services",
                        app.batch_operation,
                        app.batch_selected_services.len()
                    );
                    // TODO: Implement batch operation execution
                }
            }
        }
    });

    ui.separator();

    // Services list
    ui.label(format!("Services ({} found, {} selected):", app.services.len(), app.batch_selected_services.len()));
    let available_height = ui.available_height() - 140.0;
    egui::ScrollArea::vertical()
        .max_height(available_height * 0.6)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if app.services.is_empty() && !app.service_list_loading {
                ui.colored_label(egui::Color32::GRAY, "[No services loaded]");
            }

            for (idx, (name, state)) in app.services.iter().enumerate() {
                let is_selected = app.selected_service == Some(idx);
                let is_batch_selected = app.batch_selected_services.contains(&idx);
                let color = match state.as_str() {
                    "Running" => egui::Color32::GREEN,
                    "Stopped" => egui::Color32::RED,
                    _ => egui::Color32::YELLOW,
                };

                if app.batch_select_mode {
                    // Batch selection mode: show checkboxes
                    ui.horizontal(|ui| {
                        let mut checked = is_batch_selected;
                        if ui.checkbox(&mut checked, format!("⚙️ {} [{}]", name, state)).clicked() {
                            if checked {
                                if !app.batch_selected_services.contains(&idx) {
                                    app.batch_selected_services.push(idx);
                                }
                            } else {
                                app.batch_selected_services.retain(|&i| i != idx);
                            }
                        }
                    });
                } else {
                    // Normal selection mode: selectable label
                    let label = egui::RichText::new(format!("⚙️ {} [{}]", name, state)).color(color);
                    if ui.selectable_label(is_selected, label).clicked() {
                        app.selected_service = Some(idx);
                    }
                }
            }
        });

    ui.separator();

    // Service details
    if let Some(idx) = app.selected_service {
        if let Some((name, state)) = app.services.get(idx) {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Service Details").strong());
                    ui.label(format!("Name: {}", name));
                    ui.label(format!("State: {}", state));
                });
            });
        }
    }

    ui.separator();

    // Action buttons
    if app.selected_service.is_some() && !app.service_list_loading {
        ui.horizontal(|ui| {
            if ui.button("▶ Start").clicked() {
                if let Some((name, _)) = app.services.get(app.selected_service.unwrap()) {
                    let host = app.service_host.clone();
                    let service_name = name.clone();
                    app.service_status_message = format!("Starting service: {}", name);
                    // Spawn async task for service start
                    let _rx = spawn_service_start_task(host, service_name);
                    // Note: For now, result is fire-and-forget; could enhance with result tracking
                }
            }
            if ui.button("⏹ Stop").clicked() {
                if let Some((name, _)) = app.services.get(app.selected_service.unwrap()) {
                    let host = app.service_host.clone();
                    let service_name = name.clone();
                    app.service_status_message = format!("Stopping service: {}", name);
                    // Spawn async task for service stop
                    let _rx = spawn_service_stop_task(host, service_name);
                }
            }
            if ui.button("↻ Restart").clicked() {
                if let Some((name, _)) = app.services.get(app.selected_service.unwrap()) {
                    let host = app.service_host.clone();
                    let service_name = name.clone();
                    app.service_status_message = format!("Restarting service: {}", name);
                    // Spawn async task for service restart
                    let _rx = spawn_service_restart_task(host, service_name);
                }
            }
            if ui.button("🗑 Delete").clicked() {
                if let Some((name, _)) = app.services.get(app.selected_service.unwrap()) {
                    app.service_status_message = format!("Deleting service: {}", name);
                    // Delete operation would require additional handler
                }
            }
        });
    }

    // Create Service button (always available)
    if !app.service_list_loading {
        if ui.button("➕ Create New Service").clicked() {
            app.service_create_open = true;
        }
    }

    ui.separator();

    let status_color = if app.service_status_message.contains("Error") || app.service_status_message.contains("error") {
        egui::Color32::from_rgb(255, 100, 100)
    } else if app.service_status_message.contains("✓") {
        egui::Color32::from_rgb(100, 255, 100)
    } else {
        egui::Color32::LIGHT_BLUE
    };

    ui.colored_label(status_color, &app.service_status_message);
}

fn show_service_create_dialog(ctx: &egui::Context, app: &mut AnalyzerApp) {
    if app.service_create_open {
        egui::Window::new("Create New Service")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Service Name:");
                    ui.text_edit_singleline(&mut app.service_create_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Display Name:");
                    ui.text_edit_singleline(&mut app.service_create_display_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Executable Path:");
                    ui.text_edit_singleline(&mut app.service_create_path);
                });

                ui.horizontal(|ui| {
                    ui.label("Startup Type:");
                    egui::ComboBox::from_label("")
                        .selected_text(&app.service_create_startup_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.service_create_startup_type,
                                "Automatic".to_string(),
                                "Automatic",
                            );
                            ui.selectable_value(
                                &mut app.service_create_startup_type,
                                "Manual".to_string(),
                                "Manual",
                            );
                            ui.selectable_value(
                                &mut app.service_create_startup_type,
                                "Disabled".to_string(),
                                "Disabled",
                            );
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Create").clicked() {
                        if !app.service_create_name.is_empty() && !app.service_create_path.is_empty() {
                            let host = app.service_host.clone();
                            let name = app.service_create_name.clone();
                            let display_name = if app.service_create_display_name.is_empty() {
                                app.service_create_name.clone()
                            } else {
                                app.service_create_display_name.clone()
                            };
                            let path = app.service_create_path.clone();

                            app.service_status_message = format!("Creating service: {}", name);

                            // Spawn async task for service creation
                            let _rx = spawn_service_create_task(host, name, display_name, path);

                            app.service_create_open = false;
                            app.service_create_name.clear();
                            app.service_create_display_name.clear();
                            app.service_create_path.clear();
                            app.service_create_startup_type = "Automatic".to_string();
                        } else {
                            app.service_status_message = "Service Name and Path are required".to_string();
                        }
                    }

                    if ui.button("✗ Cancel").clicked() {
                        app.service_create_open = false;
                        app.service_create_name.clear();
                        app.service_create_display_name.clear();
                        app.service_create_path.clear();
                        app.service_create_startup_type = "Automatic".to_string();
                    }
                });
            });
    }
}

fn show_registry_edit_dialog(ctx: &egui::Context, app: &mut AnalyzerApp) {
    if app.registry_edit_open {
        egui::Window::new("Edit Registry Entry")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.registry_edit_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_label("")
                        .selected_text(&app.registry_edit_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.registry_edit_type, "REG_SZ".to_string(), "REG_SZ (String)");
                            ui.selectable_value(&mut app.registry_edit_type, "REG_DWORD".to_string(), "REG_DWORD (32-bit)");
                            ui.selectable_value(&mut app.registry_edit_type, "REG_QWORD".to_string(), "REG_QWORD (64-bit)");
                            ui.selectable_value(&mut app.registry_edit_type, "REG_BINARY".to_string(), "REG_BINARY");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.text_edit_singleline(&mut app.registry_edit_value);
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("✓ Save").clicked() {
                        if !app.registry_edit_name.is_empty() && !app.registry_edit_value.is_empty() {
                            let host = app.registry_host.clone();
                            let path = app.registry_path.clone();
                            let value_name = app.registry_edit_name.clone();
                            let value_data = app.registry_edit_value.clone();
                            let value_type = app.registry_edit_type.clone();

                            app.registry_status_message = format!(
                                "Saving registry value: {}\\{}",
                                path, value_name
                            );

                            // Spawn async task for registry write
                            let (tx, _rx) = mpsc::channel();
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let result = rt.block_on(async {
                                    psexec_rs::cli_handlers::write_registry_op(
                                        Some(host),
                                        path,
                                        value_name,
                                        value_data,
                                        value_type,
                                    )
                                    .await
                                    .map_err(|e| e.to_string())
                                });
                                let _ = tx.send(result);
                            });

                            app.registry_edit_open = false;
                        } else {
                            app.registry_status_message = "Name and Value are required".to_string();
                        }
                    }

                    if ui.button("✗ Cancel").clicked() {
                        app.registry_edit_open = false;
                    }
                });
            });
    }
}

fn show_registry_browser(ui: &mut egui::Ui, app: &mut AnalyzerApp) {
    ui.heading("📋 Registry Browser");
    ui.separator();

    // Host & Path input
    ui.horizontal(|ui| {
        ui.label("Host:");
        ui.text_edit_singleline(&mut app.registry_host);
    });

    ui.horizontal(|ui| {
        ui.label("Path:");
        ui.text_edit_singleline(&mut app.registry_path);

        if app.registry_list_loading {
            ui.label("⏳ Loading...");
        } else if ui.button("🔍 Browse").clicked() {
            let host = app.registry_host.clone();
            let path = app.registry_path.clone();

            if !host.is_empty() && !path.is_empty() {
                app.registry_status_message = format!("Loading: {}", path);
                app.registry_list_loading = true;
                let rx = spawn_registry_list_task(host.clone(), path.clone());
                app.registry_list_rx = Some(rx);
            } else {
                app.registry_status_message = "Please enter both host and path".to_string();
            }
        }
    });

    ui.separator();

    // Registry entries
    ui.label(format!("Entries ({} found):", app.registry_entries.len()));
    let available_height = ui.available_height() - 160.0;
    egui::ScrollArea::vertical()
        .max_height(available_height * 0.5)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if app.registry_entries.is_empty() && !app.registry_list_loading {
                ui.colored_label(egui::Color32::GRAY, "[No entries loaded]");
            }

            for (idx, (name, value_type, _)) in app.registry_entries.iter().enumerate() {
                let is_selected = app.selected_registry_entry == Some(idx);

                let type_color = match value_type.as_str() {
                    "REG_SZ" => egui::Color32::BLUE,
                    "REG_DWORD" => egui::Color32::GREEN,
                    "REG_BINARY" => egui::Color32::YELLOW,
                    _ => egui::Color32::GRAY,
                };

                let label = egui::RichText::new(format!("📄 {} [{}]", name, value_type))
                    .color(type_color);
                if ui.selectable_label(is_selected, label).clicked() {
                    app.selected_registry_entry = Some(idx);
                }
            }
        });

    ui.separator();

    // Entry details
    if let Some(idx) = app.selected_registry_entry {
        if let Some((name, value_type, data)) = app.registry_entries.get(idx) {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Entry Details").strong());
                    ui.label(format!("Name: {}", name));
                    ui.label(format!("Type: {}", value_type));
                    ui.label(format!("Data: {}", data));
                });
            });
        }
    }

    ui.separator();

    // Action buttons
    if app.selected_registry_entry.is_some() && !app.registry_list_loading {
        ui.horizontal(|ui| {
            if ui.button("✏ Edit").clicked() {
                if let Some((name, value_type, data)) = app.registry_entries.get(app.selected_registry_entry.unwrap()) {
                    app.registry_edit_name = name.clone();
                    app.registry_edit_type = value_type.clone();
                    app.registry_edit_value = data.clone();
                    app.registry_edit_open = true;
                }
            }
            if ui.button("🗑 Delete").clicked() {
                if let Some((name, _, _)) = app.registry_entries.get(app.selected_registry_entry.unwrap()) {
                    let host = app.registry_host.clone();
                    let path = app.registry_path.clone();
                    let value_name = name.clone();

                    app.registry_status_message = format!("Deleting: {}", name);

                    // Spawn async delete task
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async {
                            psexec_rs::cli_handlers::delete_registry_op(Some(host), path, value_name)
                                .await
                                .map_err(|e| e.to_string())
                        });
                        let _ = tx.send(result);
                    });

                    // Try to get result immediately (or will be handled next frame)
                    if let Ok(result) = rx.try_recv() {
                        match result {
                            Ok(op_result) => {
                                if op_result.success {
                                    app.registry_status_message = format!("✓ {}", op_result.message);
                                } else {
                                    app.registry_status_message = format!("❌ {}", op_result.message);
                                }
                            }
                            Err(e) => {
                                app.registry_status_message = format!("❌ Error: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    ui.separator();

    let status_color = if app.registry_status_message.contains("Error") || app.registry_status_message.contains("error") {
        egui::Color32::from_rgb(255, 100, 100)
    } else if app.registry_status_message.contains("✓") {
        egui::Color32::from_rgb(100, 255, 100)
    } else {
        egui::Color32::LIGHT_BLUE
    };

    ui.colored_label(status_color, &app.registry_status_message);
}

fn show_script_executor(ui: &mut egui::Ui, app: &mut AnalyzerApp) {
    ui.heading("📝 Script Executor");
    ui.separator();

    // Host input
    ui.horizontal(|ui| {
        ui.label("Host:");
        ui.text_edit_singleline(&mut app.script_host);
    });

    // Script type selector
    ui.horizontal(|ui| {
        ui.label("Type:");
        if ui.button(if app.script_type == "powershell" { "✓ PowerShell" } else { "PowerShell" }).clicked() {
            app.script_type = "powershell".to_string();
        }
        if ui.button(if app.script_type == "vbscript" { "✓ VBScript" } else { "VBScript" }).clicked() {
            app.script_type = "vbscript".to_string();
        }
        if ui.button(if app.script_type == "batch" { "✓ Batch" } else { "Batch" }).clicked() {
            app.script_type = "batch".to_string();
        }
        if ui.button(if app.script_type == "javascript" { "✓ JavaScript" } else { "JavaScript" }).clicked() {
            app.script_type = "javascript".to_string();
        }
    });

    ui.label(egui::RichText::new(format!("Selected: {}", app.script_type)).strong());
    ui.separator();

    // Script editor
    ui.label("Script Content:");
    let available_height = ui.available_height() - 220.0;
    egui::ScrollArea::vertical()
        .max_height(available_height * 0.4)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.text_edit_multiline(&mut app.script_content);
        });

    ui.separator();

    // Arguments input
    ui.horizontal(|ui| {
        ui.label("Arguments:");
        ui.text_edit_singleline(&mut app.script_arguments);
    });

    ui.separator();

    // Execute button
    if app.script_exec_loading {
        ui.label("⏳ Executing...");
    } else if ui.button("▶ Execute Script").clicked() {
        if app.script_content.is_empty() {
            app.script_status_message = "❌ Error: Script content is empty".to_string();
        } else if app.script_host.is_empty() {
            app.script_status_message = "❌ Error: Host is required".to_string();
        } else {
            app.script_status_message = format!(
                "Executing {} script on {}...",
                app.script_type, app.script_host
            );
            app.script_exec_loading = true;

            let host = app.script_host.clone();
            let script_type = app.script_type.clone();
            let content = app.script_content.clone();
            let args = if app.script_arguments.is_empty() {
                None
            } else {
                Some(app.script_arguments.clone())
            };

            let rx = spawn_script_exec_task(host, script_type, content, args);
            app.script_exec_rx = Some(rx);
        }
    }

    ui.separator();

    // Output display header with export button
    ui.horizontal(|ui| {
        ui.label("Output:");

        if !app.script_output.is_empty() {
            if ui.button("💾 Export Output").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt"])
                    .add_filter("Log", &["log"])
                    .set_file_name(&format!("script_output_{}.txt",
                        chrono::Local::now().format("%Y%m%d_%H%M%S")))
                    .save_file()
                {
                    match std::fs::write(&path, &app.script_output) {
                        Ok(_) => {
                            app.script_status_message = format!("✓ Output saved to {}", path.display());
                        }
                        Err(e) => {
                            app.script_status_message = format!("❌ Error saving file: {}", e);
                        }
                    }
                }
            }
        }

        if !app.script_output.is_empty() {
            if ui.button("🗑 Clear Output").clicked() {
                app.script_output.clear();
                app.script_status_message = "Output cleared".to_string();
            }
        }
    });

    // Output display
    egui::ScrollArea::vertical()
        .max_height(available_height * 0.35)
        .auto_shrink([false; 2])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if app.script_output.is_empty() {
                ui.colored_label(egui::Color32::GRAY, "[No output yet]");
            } else {
                ui.monospace(&app.script_output);
            }
        });

    ui.separator();

    let status_color = if app.script_status_message.contains("Error") || app.script_status_message.contains("error") {
        egui::Color32::from_rgb(255, 100, 100)
    } else if app.script_status_message.contains("✓") || app.script_status_message.contains("Executing") {
        egui::Color32::from_rgb(100, 200, 255)
    } else {
        egui::Color32::LIGHT_BLUE
    };

    ui.colored_label(status_color, &app.script_status_message);
}

