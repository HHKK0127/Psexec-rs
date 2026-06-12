//! Pane layout system for dynamic panel management

pub mod layout;
pub mod renderer;
pub mod events;
pub mod config;

pub use layout::{LayoutNode, PaneContent, PaneLayoutState, NodeId, ActiveDivider};
pub use renderer::render_pane_layout;
pub use events::handle_pane_events;
pub use config::{save_layout, load_layout, save_default, load_default, default_layout_path};

/// Initialize layout from default or saved configuration
pub fn init() -> PaneLayoutState {
    load_default().unwrap_or_default()
}
