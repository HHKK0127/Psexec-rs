//! Settings Panel UI - Phase 5
//! GUI configuration, theme management, profile management

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UI Theme enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
}

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Info",
            LogLevel::Warning => "Warning",
            LogLevel::Error => "Error",
        }
    }
}

/// Settings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub auto_save_profiles: bool,
    pub max_history_items: usize,
    pub default_timeout_seconds: u32,
    pub max_concurrent_tasks: usize,
    pub log_level: LogLevel,
    pub enable_notifications: bool,
    pub dark_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            auto_save_profiles: true,
            max_history_items: 100,
            default_timeout_seconds: 60,
            max_concurrent_tasks: 10,
            log_level: LogLevel::Info,
            enable_notifications: true,
            dark_mode: true,
        }
    }
}

/// Settings panel state
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    pub settings: AppSettings,
    pub active_tab: SettingsTab,
    pub unsaved_changes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Appearance,
    Advanced,
    Profiles,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            settings: AppSettings::default(),
            active_tab: SettingsTab::General,
            unsaved_changes: false,
        }
    }
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_changed(&mut self) {
        self.unsaved_changes = true;
    }

    pub fn save(&mut self) {
        self.unsaved_changes = false;
        // TODO: Persist to config file
    }
}

/// Render settings panel
pub fn render_settings_panel(ctx: &egui::Context, settings_panel: &mut SettingsPanel) {
    egui::SidePanel::right("settings_panel")
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("⚙️ Settings");

            // Tab selector
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings_panel.active_tab, SettingsTab::General, "General");
                ui.selectable_value(&mut settings_panel.active_tab, SettingsTab::Appearance, "Appearance");
                ui.selectable_value(&mut settings_panel.active_tab, SettingsTab::Advanced, "Advanced");
                ui.selectable_value(&mut settings_panel.active_tab, SettingsTab::Profiles, "Profiles");
            });

            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    match settings_panel.active_tab {
                        SettingsTab::General => render_general_settings(ui, settings_panel),
                        SettingsTab::Appearance => render_appearance_settings(ui, settings_panel),
                        SettingsTab::Advanced => render_advanced_settings(ui, settings_panel),
                        SettingsTab::Profiles => render_profile_settings(ui, settings_panel),
                    }
                });

            ui.separator();

            // Save/Reset buttons
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    settings_panel.save();
                }
                if ui.button("↩️ Reset").clicked() {
                    *settings_panel = SettingsPanel::new();
                }
                if settings_panel.unsaved_changes {
                    ui.label("⚠️ Unsaved changes");
                }
            });
        });
}

fn render_general_settings(ui: &mut egui::Ui, settings_panel: &mut SettingsPanel) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("General Settings").strong());

        ui.label("Default Timeout (seconds):");
        if ui.add(egui::Slider::new(
            &mut settings_panel.settings.default_timeout_seconds,
            10..=600,
        )).changed()
        {
            settings_panel.mark_changed();
        }

        ui.label("Max Concurrent Tasks:");
        if ui.add(egui::Slider::new(
            &mut settings_panel.settings.max_concurrent_tasks,
            1..=100,
        )).changed()
        {
            settings_panel.mark_changed();
        }

        ui.label("Max History Items:");
        if ui.add(egui::Slider::new(
            &mut settings_panel.settings.max_history_items,
            10..=500,
        )).changed()
        {
            settings_panel.mark_changed();
        }
    });
}

fn render_appearance_settings(ui: &mut egui::Ui, settings_panel: &mut SettingsPanel) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Appearance").strong());

        if ui.checkbox(&mut settings_panel.settings.dark_mode, "Dark Mode").changed() {
            settings_panel.mark_changed();
        }

        ui.label("Theme:");
        let theme_str = settings_panel.settings.theme.as_str().to_string();
        ui.label(&theme_str);
    });
}

fn render_advanced_settings(ui: &mut egui::Ui, settings_panel: &mut SettingsPanel) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Advanced Settings").strong());

        ui.label("Log Level:");
        let log_level_options = vec!["Debug", "Info", "Warning", "Error"];
        let current_level = settings_panel.settings.log_level.as_str();

        for option in &log_level_options {
            if ui.radio(current_level == *option, *option).clicked() {
                settings_panel.settings.log_level = match *option {
                    "Debug" => LogLevel::Debug,
                    "Info" => LogLevel::Info,
                    "Warning" => LogLevel::Warning,
                    "Error" => LogLevel::Error,
                    _ => LogLevel::Info,
                };
                settings_panel.mark_changed();
            }
        }

        if ui.checkbox(&mut settings_panel.settings.enable_notifications, "Enable Notifications").changed() {
            settings_panel.mark_changed();
        }
    });
}

fn render_profile_settings(ui: &mut egui::Ui, settings_panel: &mut SettingsPanel) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Profile Settings").strong());

        if ui.checkbox(&mut settings_panel.settings.auto_save_profiles, "Auto-save profiles").changed() {
            settings_panel.mark_changed();
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("📤 Export Profile").clicked() {
                // TODO: Implement export functionality
            }
            if ui.button("📥 Import Profile").clicked() {
                // TODO: Implement import functionality
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.default_timeout_seconds, 60);
        assert_eq!(settings.max_concurrent_tasks, 10);
        assert!(settings.auto_save_profiles);
    }

    #[test]
    fn test_theme_as_str() {
        assert_eq!(Theme::Light.as_str(), "Light");
        assert_eq!(Theme::Dark.as_str(), "Dark");
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "Debug");
        assert_eq!(LogLevel::Info.as_str(), "Info");
        assert_eq!(LogLevel::Warning.as_str(), "Warning");
        assert_eq!(LogLevel::Error.as_str(), "Error");
    }

    #[test]
    fn test_settings_panel_creation() {
        let panel = SettingsPanel::new();
        assert_eq!(panel.active_tab, SettingsTab::General);
        assert!(!panel.unsaved_changes);
    }

    #[test]
    fn test_mark_changed() {
        let mut panel = SettingsPanel::new();
        assert!(!panel.unsaved_changes);
        panel.mark_changed();
        assert!(panel.unsaved_changes);
    }

    #[test]
    fn test_settings_modification() {
        let mut panel = SettingsPanel::new();
        let original_timeout = panel.settings.default_timeout_seconds;

        panel.settings.default_timeout_seconds = 120;
        panel.mark_changed();

        assert_eq!(panel.settings.default_timeout_seconds, 120);
        assert_ne!(panel.settings.default_timeout_seconds, original_timeout);
        assert!(panel.unsaved_changes);
    }
}
