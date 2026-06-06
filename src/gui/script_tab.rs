//! Script execution tab for GUI

use egui::{Color32, RichText};

pub struct ScriptTab {
    host: String,
    script_type: String,
    script_content: String,
    arguments: String,
    output: String,
    status_message: String,
    is_loading: bool,
}

impl ScriptTab {
    pub fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            script_type: "powershell".to_string(),
            script_content: String::new(),
            arguments: String::new(),
            output: String::new(),
            status_message: "Ready to execute script".to_string(),
            is_loading: false,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Script Execution");
        ui.separator();

        // Host input
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut self.host);
        });

        // Script type selector
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_label("")
                .selected_text(self.script_type.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.script_type, "powershell".to_string(), "PowerShell");
                    ui.selectable_value(&mut self.script_type, "vbscript".to_string(), "VBScript");
                    ui.selectable_value(&mut self.script_type, "batch".to_string(), "Batch");
                    ui.selectable_value(&mut self.script_type, "javascript".to_string(), "JavaScript");
                });
        });

        ui.separator();

        // Script content editor
        ui.label("Script:");
        let available_height = ui.available_height() - 200.0;

        egui::ScrollArea::vertical()
            .max_height(available_height * 0.5)
            .show(ui, |ui| {
                ui.text_edit_multiline(&mut self.script_content);
            });

        ui.separator();

        // Arguments input
        ui.horizontal(|ui| {
            ui.label("Arguments:");
            ui.text_edit_singleline(&mut self.arguments);
        });

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("▶ Execute").clicked() {
                self.status_message = "Executing script...".to_string();
            }

            if ui.button("📂 Load File").clicked() {
                self.status_message = "File picker not yet implemented".to_string();
            }

            if ui.button("💾 Save Output").clicked() {
                self.status_message = "Saving output...".to_string();
            }

            if ui.button("🗑 Clear").clicked() {
                self.output.clear();
                self.status_message = "Output cleared".to_string();
            }
        });

        ui.separator();

        // Output display
        ui.label("Output:");
        egui::ScrollArea::vertical()
            .max_height(available_height * 0.3)
            .show(ui, |ui| {
                if self.output.is_empty() {
                    ui.label(RichText::new("[No output yet]").color(Color32::GRAY));
                } else {
                    ui.label(&self.output);
                }
            });

        ui.separator();

        // Status message
        if !self.status_message.is_empty() {
            ui.label(RichText::new(&self.status_message).color(Color32::LIGHT_BLUE));
        }
    }
}

impl Default for ScriptTab {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_tab_creation() {
        let tab = ScriptTab::new();
        assert_eq!(tab.host, "localhost");
        assert_eq!(tab.script_type, "powershell");
        assert!(tab.script_content.is_empty());
    }

    #[test]
    fn test_script_type_change() {
        let mut tab = ScriptTab::new();
        tab.script_type = "batch".to_string();
        assert_eq!(tab.script_type, "batch");
    }

    #[test]
    fn test_output_display() {
        let mut tab = ScriptTab::new();
        tab.output = "Test output".to_string();
        assert_eq!(tab.output, "Test output");
    }
}
