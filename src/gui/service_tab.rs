//! Service management tab for GUI

use egui::{Color32, RichText};
use crate::service::{ServiceInfo, ServiceState};

pub struct ServiceTab {
    host: String,
    services: Vec<ServiceInfo>,
    selected_service: Option<usize>,
    status_message: String,
    is_loading: bool,
    show_details: bool,
}

impl ServiceTab {
    pub fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            services: Vec::new(),
            selected_service: None,
            status_message: "Click 'Refresh' to load services".to_string(),
            is_loading: false,
            show_details: false,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Service Management");
        ui.separator();

        // Host input
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut self.host);
            if ui.button("🔄 Refresh").clicked() {
                self.load_services();
            }
        });

        ui.separator();

        // Service list
        ui.label("Services:");
        let available_height = ui.available_height() - 120.0;

        egui::ScrollArea::vertical()
            .max_height(available_height * 0.6)
            .show(ui, |ui| {
                for (idx, service) in self.services.iter().enumerate() {
                    let is_selected = self.selected_service == Some(idx);

                    let (color, status_text) = match service.state {
                        ServiceState::Running => (Color32::GREEN, "▶ Running"),
                        ServiceState::Stopped => (Color32::RED, "⏹ Stopped"),
                        ServiceState::StartPending => (Color32::YELLOW, "⏳ Starting"),
                        ServiceState::StopPending => (Color32::YELLOW, "⏳ Stopping"),
                        _ => (Color32::GRAY, "⚙ Other"),
                    };

                    let label = format!("{} - {}", service.name, status_text);

                    if ui
                        .selectable_label(
                            is_selected,
                            RichText::new(&label).color(color),
                        )
                        .clicked()
                    {
                        self.selected_service = Some(idx);
                        self.show_details = true;
                    }
                }
            });

        ui.separator();

        // Service details (if selected)
        if self.show_details {
            if let Some(idx) = self.selected_service {
                if let Some(service) = self.services.get(idx) {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Service Details").strong());
                            ui.label(format!("Name: {}", service.name));
                            ui.label(format!("Display: {}", service.display_name));
                            ui.label(format!("Path: {}", service.path));
                            ui.label(format!("Account: {}", service.account));
                            ui.label(format!("State: {:?}", service.state));
                            ui.label(format!("Startup: {:?}", service.startup_type));
                        });
                    });
                }
            }
        }

        ui.separator();

        // Action buttons
        if self.selected_service.is_some() {
            ui.horizontal(|ui| {
                if ui.button("▶ Start").clicked() {
                    if let Some(idx) = self.selected_service {
                        if let Some(service) = self.services.get(idx) {
                            self.status_message =
                                format!("Starting service: {}", service.name);
                        }
                    }
                }

                if ui.button("⏹ Stop").clicked() {
                    if let Some(idx) = self.selected_service {
                        if let Some(service) = self.services.get(idx) {
                            self.status_message = format!("Stopping service: {}", service.name);
                        }
                    }
                }

                if ui.button("↻ Restart").clicked() {
                    if let Some(idx) = self.selected_service {
                        if let Some(service) = self.services.get(idx) {
                            self.status_message =
                                format!("Restarting service: {}", service.name);
                        }
                    }
                }

                if ui.button("🗑 Delete").clicked() {
                    if let Some(idx) = self.selected_service {
                        if let Some(service) = self.services.get(idx) {
                            self.status_message = format!("Deleting service: {}", service.name);
                        }
                    }
                }
            });
        }

        ui.separator();

        // Status message
        if !self.status_message.is_empty() {
            ui.label(RichText::new(&self.status_message).color(Color32::LIGHT_BLUE));
        }
    }

    fn load_services(&mut self) {
        // Mock implementation - in real version, would call async service listing
        self.is_loading = true;
        self.status_message = "Loading services...".to_string();
        self.services.clear();

        // Example services (placeholder)
        // In real implementation, would call ServiceContext API
        self.status_message = format!("Loaded services from {}", self.host);
        self.is_loading = false;
    }
}

impl Default for ServiceTab {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_tab_creation() {
        let tab = ServiceTab::new();
        assert_eq!(tab.host, "localhost");
        assert!(tab.services.is_empty());
        assert!(!tab.is_loading);
    }

    #[test]
    fn test_service_selection() {
        let mut tab = ServiceTab::new();
        tab.selected_service = Some(0);
        assert_eq!(tab.selected_service, Some(0));
    }

    #[test]
    fn test_status_message() {
        let mut tab = ServiceTab::new();
        tab.status_message = "Test message".to_string();
        assert_eq!(tab.status_message, "Test message");
    }
}
