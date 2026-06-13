//! Profile Management System - Phase 5
//! Save, load, export, and import execution profiles

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod persistence;

/// Execution profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub settings: ProfileSettings,
    pub commands: Vec<SavedCommand>,
    pub hosts: Vec<ProfileHost>,
}

/// Profile settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSettings {
    pub timeout_seconds: u32,
    pub max_concurrent: usize,
    pub use_kerberos: bool,
    pub use_ntlm: bool,
    pub retry_count: u32,
    pub environment_vars: HashMap<String, String>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            timeout_seconds: 60,
            max_concurrent: 10,
            use_kerberos: false,
            use_ntlm: false,
            retry_count: 3,
            environment_vars: HashMap::new(),
        }
    }
}

/// Saved command in profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl SavedCommand {
    pub fn new(name: &str, command: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            description: None,
            tags: vec![],
        }
    }
}

/// Profile host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileHost {
    pub name: String,
    pub address: String,
    pub username: Option<String>,
    pub port: u16,
}

impl ExecutionProfile {
    pub fn new(name: &str) -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            created_at: now.clone(),
            updated_at: now,
            settings: ProfileSettings::default(),
            commands: vec![],
            hosts: vec![],
        }
    }

    /// Add command to profile
    pub fn add_command(&mut self, cmd: SavedCommand) {
        self.commands.push(cmd);
        self.update_timestamp();
    }

    /// Add host to profile
    pub fn add_host(&mut self, host: ProfileHost) {
        self.hosts.push(host);
        self.update_timestamp();
    }

    /// Remove command by id
    pub fn remove_command(&mut self, id: &str) {
        self.commands.retain(|c| c.id != id);
        self.update_timestamp();
    }

    /// Update timestamp
    fn update_timestamp(&mut self) {
        self.updated_at = chrono::Local::now().to_rfc3339();
    }
}

/// Profile manager
pub struct ProfileManager {
    pub profiles: Vec<ExecutionProfile>,
    pub active_profile: Option<String>,
    pub profile_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(profile_dir: PathBuf) -> Self {
        Self {
            profiles: vec![],
            active_profile: None,
            profile_dir,
        }
    }

    /// Create new profile
    pub fn create_profile(&mut self, name: &str) -> ExecutionProfile {
        let profile = ExecutionProfile::new(name);
        self.profiles.push(profile.clone());
        profile
    }

    /// Get profile by id
    pub fn get_profile(&self, id: &str) -> Option<&ExecutionProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Get mutable profile by id
    pub fn get_profile_mut(&mut self, id: &str) -> Option<&mut ExecutionProfile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Delete profile by id
    pub fn delete_profile(&mut self, id: &str) {
        self.profiles.retain(|p| p.id != id);
        if self.active_profile.as_ref() == Some(&id.to_string()) {
            self.active_profile = None;
        }
    }

    /// Set active profile
    pub fn set_active_profile(&mut self, id: &str) {
        if self.get_profile(id).is_some() {
            self.active_profile = Some(id.to_string());
        }
    }

    /// Get active profile
    pub fn active_profile(&self) -> Option<&ExecutionProfile> {
        self.active_profile
            .as_ref()
            .and_then(|id| self.get_profile(id))
    }

    /// List all profile names
    pub fn list_profiles(&self) -> Vec<(String, String)> {
        self.profiles
            .iter()
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_profile_creation() {
        let profile = ExecutionProfile::new("Test Profile");
        assert_eq!(profile.name, "Test Profile");
        assert!(profile.commands.is_empty());
        assert!(profile.hosts.is_empty());
    }

    #[test]
    fn test_add_command_to_profile() {
        let mut profile = ExecutionProfile::new("Test Profile");
        let cmd = SavedCommand::new("List Files", "ls -la");

        profile.add_command(cmd.clone());
        assert_eq!(profile.commands.len(), 1);
        assert_eq!(profile.commands[0].name, "List Files");
    }

    #[test]
    fn test_add_host_to_profile() {
        let mut profile = ExecutionProfile::new("Test Profile");
        let host = ProfileHost {
            name: "server1".to_string(),
            address: "192.168.1.10".to_string(),
            username: Some("admin".to_string()),
            port: 445,
        };

        profile.add_host(host);
        assert_eq!(profile.hosts.len(), 1);
        assert_eq!(profile.hosts[0].name, "server1");
    }

    #[test]
    fn test_profile_manager_create() {
        let mut manager = ProfileManager::new(PathBuf::from("/tmp"));
        let profile = manager.create_profile("MyProfile");

        assert_eq!(manager.profiles.len(), 1);
        assert_eq!(manager.profiles[0].name, "MyProfile");
    }

    #[test]
    fn test_profile_manager_set_active() {
        let mut manager = ProfileManager::new(PathBuf::from("/tmp"));
        let profile = manager.create_profile("MyProfile");
        let profile_id = profile.id.clone();

        manager.set_active_profile(&profile_id);
        assert_eq!(manager.active_profile.as_ref(), Some(&profile_id));
    }

    #[test]
    fn test_profile_manager_delete() {
        let mut manager = ProfileManager::new(PathBuf::from("/tmp"));
        let profile = manager.create_profile("MyProfile");
        let profile_id = profile.id.clone();

        assert_eq!(manager.profiles.len(), 1);
        manager.delete_profile(&profile_id);
        assert_eq!(manager.profiles.len(), 0);
    }

    #[test]
    fn test_profile_manager_list() {
        let mut manager = ProfileManager::new(PathBuf::from("/tmp"));
        manager.create_profile("Profile1");
        manager.create_profile("Profile2");

        let profiles = manager.list_profiles();
        assert_eq!(profiles.len(), 2);
    }
}
