//! Log viewer panel for GUI

use crate::executor::logging::ExecutionLogEntry;
use eframe::egui;
use chrono::Local;

pub struct LogViewerPanel {
    filter: String,
    level_filter: LogLevelFilter,
    auto_scroll: bool,
    entries: Vec<ExecutionLogEntry>,
    selected_entry: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LogLevelFilter {
    All,
    Success,
    Failed,
}

impl LogViewerPanel {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            level_filter: LogLevelFilter::All,
            auto_scroll: true,
            entries: Vec::new(),
            selected_entry: None,
        }
    }

    pub fn add_entry(&mut self, entry: ExecutionLogEntry) {
        self.entries.push(entry);
        // Keep only last 1000 entries to prevent memory bloat
        if self.entries.len() > 1000 {
            self.entries.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected_entry = None;
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Execution Log");

        // Filters
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);

            ui.label("Show:");
            egui::ComboBox::from_id_source("level_filter")
                .selected_text(format!("{:?}", self.level_filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.level_filter, LogLevelFilter::All, "All");
                    ui.selectable_value(&mut self.level_filter, LogLevelFilter::Success, "Success");
                    ui.selectable_value(&mut self.level_filter, LogLevelFilter::Failed, "Failed");
                });

            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

            if ui.button("Clear").clicked() {
                self.clear();
            }
        });

        // Stats
        let total = self.entries.len();
        let success = self.entries.iter().filter(|e| e.success).count();
        let failed = total - success;
        ui.label(format!("Total: {} | Success: {} | Failed: {}", total, success, failed));

        // Log entries
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                for (idx, entry) in self.entries.iter().enumerate().rev() {
                    // Apply filters
                    if !self.filter.is_empty() {
                        if !entry.computer.contains(&self.filter) &&
                           !entry.command.contains(&self.filter) &&
                           !entry.stdout.contains(&self.filter) {
                            continue;
                        }
                    }

                    match self.level_filter {
                        LogLevelFilter::Success if !entry.success => continue,
                        LogLevelFilter::Failed if entry.success => continue,
                        _ => {}
                    }

                    // Render entry
                    let color = if entry.success {
                        egui::Color32::from_rgb(0, 200, 0)
                    } else {
                        egui::Color32::from_rgb(200, 0, 0)
                    };

                    let is_selected = self.selected_entry == Some(idx);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_selected,
                                format!("{} | {}",
                                    entry.timestamp.format("%H:%M:%S"),
                                    entry.computer
                                )
                            ).clicked() {
                                self.selected_entry = Some(idx);
                            }

                            ui.colored_label(color, if entry.success { "✓" } else { "✗" });
                            ui.label(&entry.command);
                            ui.label(format!("({}ms)", entry.duration_ms));
                        });

                        if is_selected {
                            ui.separator();
                            if !entry.stdout.is_empty() {
                                ui.label("STDOUT:");
                                ui.monospace(&entry.stdout);
                            }
                            if !entry.stderr.is_empty() {
                                ui.colored_label(egui::Color32::RED, "STDERR:");
                                ui.monospace(&entry.stderr);
                            }
                        }
                    });
                }
            });
    }
}

impl Default for LogViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}
