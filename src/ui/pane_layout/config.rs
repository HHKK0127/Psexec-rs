//! Pane layout configuration persistence

use super::layout::{LayoutNode, PaneLayoutState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutConfig {
    root: LayoutNode,
    active_pane: Option<usize>,
}

impl LayoutConfig {
    fn from_state(state: &PaneLayoutState) -> Self {
        Self {
            root: state.root.clone(),
            active_pane: state.active_pane,
        }
    }

    fn to_state(mut self) -> PaneLayoutState {
        let mut contents = std::collections::HashMap::new();
        let mut ids = Vec::new();
        self.root.collect_panes(&mut ids);

        for id in ids {
            if let Some(node) = self.root.find_mut(id) {
                if let LayoutNode::Pane { content, .. } = node {
                    contents.insert(id, *content);
                }
            }
        }

        PaneLayoutState {
            root: self.root,
            active_pane: self.active_pane,
            resizing_divider: None,
            hover_divider: None,
            next_id: 1000,
            pane_contents: contents,
        }
    }
}

/// Save layout to file
pub fn save_layout(state: &PaneLayoutState, path: &PathBuf) -> io::Result<()> {
    let config = LayoutConfig::from_state(state);
    let json = serde_json::to_string_pretty(&config)?;

    fs::write(path, json)?;

    Ok(())
}

/// Load layout from file
pub fn load_layout(path: &PathBuf) -> io::Result<PaneLayoutState> {
    let json = fs::read_to_string(path)?;
    let config: LayoutConfig = serde_json::from_str(&json)?;

    Ok(config.to_state())
}

/// Get default layout file path
pub fn default_layout_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("psexec-rs")
        .join("layout.json")
}

/// Save to default location
pub fn save_default(state: &PaneLayoutState) -> io::Result<()> {
    let path = default_layout_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    save_layout(state, &path)
}

/// Load from default location
pub fn load_default() -> io::Result<PaneLayoutState> {
    let path = default_layout_path();
    if path.exists() {
        load_layout(&path)
    } else {
        Ok(PaneLayoutState::default())
    }
}
