//! Profile persistence - Save, load, export, import

use super::ExecutionProfile;
use std::fs;
use std::path::{Path, PathBuf};

/// Save profile to JSON file
pub fn save_profile(profile: &ExecutionProfile, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(profile)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load profile from JSON file
pub fn load_profile(path: &Path) -> Result<ExecutionProfile, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let profile = serde_json::from_str(&json)?;
    Ok(profile)
}

/// Export profile to directory with metadata
pub fn export_profile(
    profile: &ExecutionProfile,
    export_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if !export_dir.exists() {
        fs::create_dir_all(export_dir)?;
    }

    let filename = format!("profile_{}.json", profile.id);
    let filepath = export_dir.join(&filename);

    save_profile(profile, &filepath)?;
    Ok(filepath)
}

/// Import profile from file
pub fn import_profile(import_path: &Path) -> Result<ExecutionProfile, Box<dyn std::error::Error>> {
    load_profile(import_path)
}

/// List all profiles in directory
pub fn list_profiles_in_dir(dir: &Path) -> Result<Vec<ExecutionProfile>, Box<dyn std::error::Error>> {
    let mut profiles = vec![];

    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                if let Ok(profile) = load_profile(&path) {
                    profiles.push(profile);
                }
            }
        }
    }

    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_profile() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("test_profile.json");

        let mut profile = ExecutionProfile::new("Test Profile");
        profile.description = Some("Test description".to_string());

        // Save
        save_profile(&profile, &profile_path).unwrap();
        assert!(profile_path.exists());

        // Load
        let loaded = load_profile(&profile_path).unwrap();
        assert_eq!(loaded.name, "Test Profile");
        assert_eq!(loaded.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_export_profile() {
        let temp_dir = TempDir::new().unwrap();
        let profile = ExecutionProfile::new("Export Test");

        let export_path = export_profile(&profile, temp_dir.path()).unwrap();
        assert!(export_path.exists());
    }

    #[test]
    fn test_import_profile() {
        let temp_dir = TempDir::new().unwrap();
        let original = ExecutionProfile::new("Import Test");

        let export_path = export_profile(&original, temp_dir.path()).unwrap();
        let imported = import_profile(&export_path).unwrap();

        assert_eq!(imported.name, "Import Test");
        assert_eq!(imported.id, original.id);
    }

    #[test]
    fn test_list_profiles_in_dir() {
        let temp_dir = TempDir::new().unwrap();

        let profile1 = ExecutionProfile::new("Profile 1");
        let profile2 = ExecutionProfile::new("Profile 2");

        save_profile(&profile1, &temp_dir.path().join("profile1.json")).unwrap();
        save_profile(&profile2, &temp_dir.path().join("profile2.json")).unwrap();

        let profiles = list_profiles_in_dir(temp_dir.path()).unwrap();
        assert_eq!(profiles.len(), 2);
    }
}
