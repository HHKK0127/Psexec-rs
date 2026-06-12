//! Log viewer panel for GUI

use crate::executor::logging::ExecutionLogEntry;
use eframe::egui;

pub struct LogViewerPanel {
    filter: String,
    level_filter: LogLevelFilter,
    auto_scroll: bool,
    entries: Vec<ExecutionLogEntry>,
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
        }
    }

    pub fn add_entry(&mut self, entry: ExecutionLogEntry) {
        self.entries.push(entry);
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
                self.entries.clear();
            }
        });

        // Log entries
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                for entry in &self.entries {
                    // Apply filters
                    if !self.filter.is_empty() {
                        if !entry.computer.contains(&self.filter) &&
                           !entry.command.contains(&self.filter) {
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
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };

                    ui.horizontal(|ui| {
                        ui.label(entry.timestamp.format("%H:%M:%S").to_string());
                        ui.colored_label(color, &entry.computer);
                        ui.label(&entry.command);
                        ui.label(format!("({}ms)", entry.duration_ms));
                    });

                    if !entry.stdout.is_empty() {
                        ui.monospace(&entry.stdout);
                    }
                    if !entry.stderr.is_empty() {
                        ui.colored_label(egui::Color32::RED, &entry.stderr);
                    }
                }
            });
    }
}

impl Default for LogViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}
