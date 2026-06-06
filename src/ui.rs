use crate::analyzer::AnalysisResult;
use eframe::egui;

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
            output_receiver: None,
        }
    }
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

            // Remote Execution タブは常に表示
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::RemoteExecution, "Remote Execution");
                if self.result.is_some() {
                    ui.separator();
                    ui.selectable_value(&mut self.selected_tab, Tab::Overview, "Overview");
                    ui.selectable_value(&mut self.selected_tab, Tab::PeInfo, "PE Info");
                    ui.selectable_value(&mut self.selected_tab, Tab::Imports, "Imports");
                    ui.selectable_value(&mut self.selected_tab, Tab::Strings, "Strings");
                    ui.selectable_value(&mut self.selected_tab, Tab::Signature, "Signature");
                }
            });

            ui.separator();

            match self.selected_tab {
                Tab::RemoteExecution => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show_remote_execution(ui, self);
                    });
                }
                _ => {
                    if let Some(res) = &self.result {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            match self.selected_tab {
                                Tab::Overview => show_overview(ui, res),
                                Tab::PeInfo => show_pe_info(ui, res),
                                Tab::Imports => show_imports(ui, res, &mut self.filter),
                                Tab::Strings => show_strings(ui, res, &mut self.filter),
                                Tab::Signature => show_signature(ui, res),
                                Tab::RemoteExecution => {} // handled above
                            }
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a PE file to analyze.");
                        });
                    }
                }
            }
        });
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

