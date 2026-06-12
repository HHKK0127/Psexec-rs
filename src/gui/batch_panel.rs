//! Batch execution panel for GUI with local command execution support

use eframe::egui;
use std::process::Command;
use chrono::Local;
use crate::executor::logging::ExecutionLogEntry;

/// Progress update from async execution
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub completed: usize,
    pub total: usize,
    pub current_computer: String,
    pub status: String,
    pub success: bool,
}

/// Batch execution panel
pub struct BatchPanel {
    // Input fields
    pub computers: String,
    pub command: String,
    pub arguments: String,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,

    // Execution state
    pub status: String,
    pub results: Vec<(String, String, bool)>, // (computer, status, success)
    pub is_executing: bool,
    pub progress: (usize, usize), // (completed, total)

    // Log callback
    log_callback: Option<Box<dyn Fn(ExecutionLogEntry) + Send>>,
}

impl BatchPanel {
    pub fn new() -> Self {
        Self {
            computers: String::new(),
            command: String::new(),
            arguments: String::new(),
            max_concurrent: 10,
            timeout_seconds: 60,
            status: "Ready".to_string(),
            results: Vec::new(),
            is_executing: false,
            progress: (0, 0),
            log_callback: None,
        }
    }

    /// Set callback for logging
    pub fn on_log<F>(&mut self, callback: F)
    where
        F: Fn(ExecutionLogEntry) + Send + 'static,
    {
        self.log_callback = Some(Box::new(callback));
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Batch Execution");

        // Computers input
        ui.horizontal(|ui| {
            ui.label("Computers:");
            ui.text_edit_multiline(&mut self.computers);
        });
        ui.label("Enter one per line or comma-separated");

        // Command input
        ui.horizontal(|ui| {
            ui.label("Command:");
            ui.text_edit_singleline(&mut self.command);
        });

        // Arguments input
        ui.horizontal(|ui| {
            ui.label("Arguments:");
            ui.text_edit_singleline(&mut self.arguments);
        });

        // Settings
        ui.collapsing("Settings", |ui| {
            ui.horizontal(|ui| {
                ui.label("Max Concurrent:");
                ui.add(egui::DragValue::new(&mut self.max_concurrent)
                    .clamp_range(1..=100));
            });
            ui.horizontal(|ui| {
                ui.label("Timeout (s):");
                ui.add(egui::DragValue::new(&mut self.timeout_seconds)
                    .clamp_range(1..=3600));
            });
        });

        // Execute button
        ui.horizontal(|ui| {
            if ui.button("Execute").clicked() && !self.is_executing {
                self.start_execution();
            }
            if ui.button("Clear").clicked() {
                self.results.clear();
                self.status = "Ready".to_string();
                self.progress = (0, 0);
            }
        });

        // Progress
        if self.is_executing {
            ui.label(format!("Progress: {}/{}", self.progress.0, self.progress.1));
            let progress_pct = if self.progress.1 > 0 {
                self.progress.0 as f32 / self.progress.1 as f32
            } else {
                0.0
            };
            ui.add(egui::ProgressBar::new(progress_pct).text(format!("{:.0}%", progress_pct * 100.0)));
        }

        // Status
        ui.label(&self.status);

        // Results table
        if !self.results.is_empty() {
            ui.separator();
            ui.label("Results:");
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                egui::Grid::new("results_grid")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Computer");
                        ui.label("Status");
                        ui.label("Result");
                        ui.end_row();

                        for (computer, status, success) in &self.results {
                            ui.label(computer);
                            let color = if *success {
                                egui::Color32::GREEN
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(color, status);
                            ui.colored_label(color, if *success { "Success" } else { "Failed" });
                            ui.end_row();
                        }
                    });
            });
        }
    }

    /// Poll for progress updates (simplified for local execution)
    pub fn poll_progress(&mut self) {
        // In this simplified version, we update results synchronously
        // In a full implementation, this would poll async results from a channel
    }

    fn start_execution(&mut self) {
        self.is_executing = true;
        self.status = "Executing...".to_string();
        self.results.clear();
        self.progress = (0, 0);

        // Parse computers
        let computers: Vec<String> = self.computers
            .split(&[',', '\n'][..])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if computers.is_empty() {
            self.status = "Error: No computers specified".to_string();
            self.is_executing = false;
            return;
        }

        // Execute command for each computer
        let total = computers.len();
        for (idx, computer) in computers.iter().enumerate() {
            self.progress = (idx, total);

            // For localhost/local execution
            if computer == "localhost" || computer == "127.0.0.1" || computer == "." {
                let output = Command::new(&self.command)
                    .args(self.arguments.split_whitespace())
                    .output();

                let (success, status) = match output {
                    Ok(output) => {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        (exit_code == 0, format!("Exit code: {}", exit_code))
                    }
                    Err(e) => {
                        (false, format!("Error: {}", e))
                    }
                };

                self.results.push((computer.clone(), status, success));

                // Log entry
                if let Some(ref callback) = self.log_callback {
                    let entry = ExecutionLogEntry {
                        timestamp: Local::now(),
                        computer: computer.clone(),
                        command: self.command.clone(),
                        arguments: self.arguments.split_whitespace().map(|s| s.to_string()).collect(),
                        exit_code: if success { 0 } else { 1 },
                        stdout: self.arguments.clone(),
                        stderr: if success { String::new() } else { "Command failed".to_string() },
                        duration_ms: 0,
                        success,
                    };
                    callback(entry);
                }
            } else {
                // Remote execution not implemented in this simplified version
                self.results.push((computer.clone(), "Not supported (use localhost)".to_string(), false));
            }
        }

        self.progress = (total, total);
        self.is_executing = false;
        self.status = format!("Completed: {}/{} succeeded",
            self.results.iter().filter(|(_, _, s)| *s).count(),
            self.results.len()
        );
    }
}

impl Default for BatchPanel {
    fn default() -> Self {
        Self::new()
    }
}
