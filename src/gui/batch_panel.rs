//! Batch execution panel for GUI

use crate::executor::batch::{BatchConfig, execute_batch_with_progress};
use crate::executor::ExecutionMethod;
use crate::auth::AuthContext;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BatchPanel {
    computers: String,
    command: String,
    arguments: String,
    max_concurrent: usize,
    timeout_seconds: u64,
    status: String,
    results: Vec<(String, String)>,
    is_executing: bool,
    progress: (usize, usize),
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
        }
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
            }
        });

        // Progress
        if self.is_executing {
            ui.label(format!("Progress: {}/{}", self.progress.0, self.progress.1));
            ui.add(egui::ProgressBar::new(
                if self.progress.1 > 0 {
                    self.progress.0 as f32 / self.progress.1 as f32
                } else {
                    0.0
                }
            ));
        }

        // Status
        ui.label(&self.status);

        // Results table
        if !self.results.is_empty() {
            ui.separator();
            ui.label("Results:");
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                egui::Grid::new("results_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Computer");
                        ui.label("Status");
                        ui.end_row();

                        for (computer, status) in &self.results {
                            ui.label(computer);
                            ui.colored_label(
                                if status == "Success" {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::RED
                                },
                                status,
                            );
                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn start_execution(&mut self) {
        self.is_executing = true;
        self.status = "Executing...".to_string();
        self.results.clear();

        let computers: Vec<String> = self.computers
            .split(&[',', '\n'][..])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        self.progress.1 = computers.len();

        let args: Vec<String> = self.arguments
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let config = BatchConfig::default()
            .with_concurrency(self.max_concurrent)
            .with_timeout(self.timeout_seconds);

        self.status = "Starting batch execution...".to_string();
    }
}

impl Default for BatchPanel {
    fn default() -> Self {
        Self::new()
    }
}
