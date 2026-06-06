//! Registry browser tab for GUI

use egui::{Color32, RichText};

pub struct RegistryTab {
    host: String,
    current_path: String,
    entries: Vec<RegistryEntry>,
    selected_entry: Option<usize>,
    status_message: String,
    is_loading: bool,
}

#[derive(Clone, Debug)]
pub struct RegistryEntry {
    pub name: String,
    pub value_type: String,
    pub data: String,
}

impl RegistryTab {
    pub fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            current_path: "HKEY_LOCAL_MACHINE".to_string(),
            entries: Vec::new(),
            selected_entry: None,
            status_message: "Enter registry path and click 'Browse'".to_string(),
            is_loading: false,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Registry Browser");
        ui.separator();

        // Host input
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut self.host);
        });

        // Path input
        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.current_path);
            if ui.button("🔍 Browse").clicked() {
                self.load_entries();
            }
        });

        ui.separator();

        // Registry entries
        ui.label("Entries:");
        let available_height = ui.available_height() - 150.0;

        egui::ScrollArea::vertical()
            .max_height(available_height * 0.5)
            .show(ui, |ui| {
                for (idx, entry) in self.entries.iter().enumerate() {
                    let is_selected = self.selected_entry == Some(idx);

                    let type_color = match entry.value_type.as_str() {
                        "REG_SZ" => Color32::BLUE,
                        "REG_DWORD" => Color32::GREEN,
                        "REG_BINARY" => Color32::YELLOW,
                        _ => Color32::GRAY,
                    };

                    let label = format!("{} ({})", entry.name, entry.value_type);

                    if ui
                        .selectable_label(
                            is_selected,
                            RichText::new(&label).color(type_color),
                        )
                        .clicked()
                    {
                        self.selected_entry = Some(idx);
                    }
                }
            });

        ui.separator();

        // Entry details
        if let Some(idx) = self.selected_entry {
            if let Some(entry) = self.entries.get(idx) {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Entry Details").strong());
                        ui.label(format!("Name: {}", entry.name));
                        ui.label(format!("Type: {}", entry.value_type));

                        ui.label("Value:");
                        ui.text_edit_multiline(&mut entry.data.clone());
                    });
                });
            }
        }

        ui.separator();

        // Action buttons
        if self.selected_entry.is_some() {
            ui.horizontal(|ui| {
                if ui.button("✏ Edit").clicked() {
                    if let Some(idx) = self.selected_entry {
                        if let Some(entry) = self.entries.get(idx) {
                            self.status_message =
                                format!("Editing: {}", entry.name);
                        }
                    }
                }

                if ui.button("🗑 Delete").clicked() {
                    if let Some(idx) = self.selected_entry {
                        if let Some(entry) = self.entries.get(idx) {
                            self.status_message =
                                format!("Deleting: {}", entry.name);
                        }
                    }
                }

                if ui.button("➕ New").clicked() {
                    self.status_message = "Creating new value...".to_string();
                }
            });
        }

        ui.separator();

        // Status message
        if !self.status_message.is_empty() {
            ui.label(RichText::new(&self.status_message).color(Color32::LIGHT_BLUE));
        }
    }

    fn load_entries(&mut self) {
        self.is_loading = true;
        self.status_message = format!("Loading: {}", self.current_path);
        self.entries.clear();

        // Example entries (placeholder)
        self.entries = vec![
            RegistryEntry {
                name: "Example1".to_string(),
                value_type: "REG_SZ".to_string(),
                data: "Value1".to_string(),
            },
            RegistryEntry {
                name: "Example2".to_string(),
                value_type: "REG_DWORD".to_string(),
                data: "12345".to_string(),
            },
        ];

        self.status_message = format!("Loaded {} entries", self.entries.len());
        self.is_loading = false;
    }
}

impl Default for RegistryTab {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_tab_creation() {
        let tab = RegistryTab::new();
        assert_eq!(tab.host, "localhost");
        assert_eq!(tab.current_path, "HKEY_LOCAL_MACHINE");
        assert!(tab.entries.is_empty());
    }

    #[test]
    fn test_registry_entry_creation() {
        let entry = RegistryEntry {
            name: "Test".to_string(),
            value_type: "REG_SZ".to_string(),
            data: "TestValue".to_string(),
        };
        assert_eq!(entry.name, "Test");
        assert_eq!(entry.value_type, "REG_SZ");
    }

    #[test]
    fn test_entry_selection() {
        let mut tab = RegistryTab::new();
        tab.selected_entry = Some(0);
        assert_eq!(tab.selected_entry, Some(0));
    }
}
