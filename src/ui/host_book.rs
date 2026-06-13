//! Host Book Management UI - Phase 5
//! Manages remote host list, grouping, favorites, and search

use eframe::egui;
use serde::{Deserialize, Serialize};

/// Host entry in the host book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub group: String,
    pub username: Option<String>,
    pub description: Option<String>,
    pub is_favorite: bool,
    pub port: u16,
}

impl HostEntry {
    pub fn new(name: &str, address: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            address: address.to_string(),
            group: "Default".to_string(),
            username: None,
            description: None,
            is_favorite: false,
            port: 445,
        }
    }
}

/// Host book state
#[derive(Debug, Clone)]
pub struct HostBook {
    pub hosts: Vec<HostEntry>,
    pub groups: Vec<String>,
    pub selected_host: Option<String>,
    pub search_filter: String,
    pub show_favorites_only: bool,
    pub editing_host: Option<HostEntry>,
}

impl Default for HostBook {
    fn default() -> Self {
        Self {
            hosts: vec![],
            groups: vec!["Default".to_string()],
            selected_host: None,
            search_filter: String::new(),
            show_favorites_only: false,
            editing_host: None,
        }
    }
}

impl HostBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get filtered hosts based on search and favorites filter
    pub fn filtered_hosts(&self) -> Vec<HostEntry> {
        self.hosts
            .iter()
            .filter(|host| {
                let matches_search = self.search_filter.is_empty()
                    || host.name.to_lowercase().contains(&self.search_filter.to_lowercase())
                    || host.address.to_lowercase().contains(&self.search_filter.to_lowercase());

                let matches_favorites =
                    !self.show_favorites_only || host.is_favorite;

                matches_search && matches_favorites
            })
            .cloned()
            .collect()
    }

    /// Add new host
    pub fn add_host(&mut self, host: HostEntry) {
        if !self.groups.contains(&host.group) {
            self.groups.push(host.group.clone());
        }
        self.hosts.push(host);
    }

    /// Update existing host
    pub fn update_host(&mut self, id: &str, updated: HostEntry) {
        if let Some(pos) = self.hosts.iter().position(|h| h.id == id) {
            if !self.groups.contains(&updated.group) {
                self.groups.push(updated.group.clone());
            }
            self.hosts[pos] = updated;
        }
    }

    /// Delete host by id
    pub fn delete_host(&mut self, id: &str) {
        self.hosts.retain(|h| h.id != id);
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&mut self, id: &str) {
        if let Some(host) = self.hosts.iter_mut().find(|h| h.id == id) {
            host.is_favorite = !host.is_favorite;
        }
    }

    /// Get hosts by group
    pub fn hosts_by_group(&self, group: &str) -> Vec<HostEntry> {
        self.hosts
            .iter()
            .filter(|h| h.group == group)
            .cloned()
            .collect()
    }
}

/// Render host book panel
pub fn render_host_book(ctx: &egui::Context, host_book: &mut HostBook) {
    egui::SidePanel::left("host_book_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("🖥️ Host Book");

            // Search bar
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut host_book.search_filter);
            });

            // Favorites filter
            ui.checkbox(&mut host_book.show_favorites_only, "⭐ Favorites only");

            ui.separator();

            // Host list with groups
            let filtered = host_book.filtered_hosts();

            if filtered.is_empty() {
                ui.label("No hosts found");
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for group in &host_book.groups.clone() {
                            let group_hosts: Vec<_> =
                                filtered.iter().filter(|h| h.group == *group).collect();

                            if !group_hosts.is_empty() {
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new(group)
                                        .strong()
                                        .color(egui::Color32::LIGHT_BLUE));

                                    for host in group_hosts {
                                        let selected = host_book.selected_host
                                            .as_ref()
                                            .map(|id| id == &host.id)
                                            .unwrap_or(false);

                                        if ui.selectable_label(selected, &host.name).clicked() {
                                            host_book.selected_host = Some(host.id.clone());
                                        }

                                        // Favorite button
                                        if ui.small_button(if host.is_favorite { "⭐" } else { "☆" }).clicked()
                                        {
                                            host_book.toggle_favorite(&host.id);
                                        }
                                    }
                                });
                            }
                        }
                    });
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("➕ New Host").clicked() {
                    host_book.editing_host = Some(HostEntry::new("", ""));
                }
                // Clone host_id to avoid borrow checker issues
                if let Some(host_id) = host_book.selected_host.clone() {
                    if ui.button("✏️ Edit").clicked() {
                        if let Some(host) = host_book.hosts.iter().find(|h| h.id == host_id) {
                            host_book.editing_host = Some(host.clone());
                        }
                    }
                    if ui.button("🗑️ Delete").clicked() {
                        host_book.delete_host(&host_id);
                        host_book.selected_host = None;
                    }
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_book_creation() {
        let book = HostBook::new();
        assert!(book.hosts.is_empty());
        assert!(book.groups.contains(&"Default".to_string()));
    }

    #[test]
    fn test_add_host() {
        let mut book = HostBook::new();
        let host = HostEntry::new("server1", "192.168.1.10");
        book.add_host(host.clone());
        assert_eq!(book.hosts.len(), 1);
        assert_eq!(book.hosts[0].name, "server1");
    }

    #[test]
    fn test_search_filter() {
        let mut book = HostBook::new();
        book.add_host(HostEntry::new("web-server", "192.168.1.10"));
        book.add_host(HostEntry::new("db-server", "192.168.1.20"));

        book.search_filter = "web".to_string();
        let filtered = book.filtered_hosts();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "web-server");
    }

    #[test]
    fn test_favorites_filter() {
        let mut book = HostBook::new();
        let mut host1 = HostEntry::new("server1", "192.168.1.10");
        let mut host2 = HostEntry::new("server2", "192.168.1.20");

        host1.is_favorite = true;
        book.add_host(host1);
        book.add_host(host2);

        book.show_favorites_only = true;
        let filtered = book.filtered_hosts();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "server1");
    }

    #[test]
    fn test_toggle_favorite() {
        let mut book = HostBook::new();
        let host = HostEntry::new("server1", "192.168.1.10");
        let host_id = host.id.clone();
        book.add_host(host);

        assert!(!book.hosts[0].is_favorite);
        book.toggle_favorite(&host_id);
        assert!(book.hosts[0].is_favorite);
    }

    #[test]
    fn test_hosts_by_group() {
        let mut book = HostBook::new();
        let mut host1 = HostEntry::new("server1", "192.168.1.10");
        let mut host2 = HostEntry::new("server2", "192.168.1.20");

        host1.group = "Production".to_string();
        host2.group = "Development".to_string();

        book.add_host(host1);
        book.add_host(host2);

        let prod_hosts = book.hosts_by_group("Production");
        assert_eq!(prod_hosts.len(), 1);
        assert_eq!(prod_hosts[0].name, "server1");
    }
}
