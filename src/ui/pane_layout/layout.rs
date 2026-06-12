//! Pane layout data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for layout nodes
pub type NodeId = usize;

/// Content type for each pane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneContent {
    BatchExecutor,
    LogViewer,
    Settings,
    Terminal,
    Placeholder,
}

impl PaneContent {
    pub fn title(&self) -> &'static str {
        match self {
            PaneContent::BatchExecutor => "Batch Executor",
            PaneContent::LogViewer => "Log Viewer",
            PaneContent::Settings => "Settings",
            PaneContent::Terminal => "Terminal",
            PaneContent::Placeholder => "Empty",
        }
    }
}

/// Layout tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    /// Leaf node containing a pane
    Pane {
        id: NodeId,
        content: PaneContent,
    },
    /// Internal node splitting space between two children
    Split {
        id: NodeId,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
        divider_pos: f32,
        is_horizontal: bool,
    },
}

impl LayoutNode {
    /// Create a new pane node
    pub fn pane(id: NodeId, content: PaneContent) -> Self {
        LayoutNode::Pane { id, content }
    }

    /// Create a new split node
    pub fn split(
        id: NodeId,
        left: LayoutNode,
        right: LayoutNode,
        divider_pos: f32,
        is_horizontal: bool,
    ) -> Self {
        LayoutNode::Split {
            id,
            left: Box::new(left),
            right: Box::new(right),
            divider_pos: divider_pos.clamp(0.1, 0.9),
            is_horizontal,
        }
    }

    /// Get node ID
    pub fn id(&self) -> NodeId {
        match self {
            LayoutNode::Pane { id, .. } => *id,
            LayoutNode::Split { id, .. } => *id,
        }
    }

    /// Find mutable reference to node by ID
    pub fn find_mut(&mut self, target_id: NodeId) -> Option<&mut LayoutNode> {
        if self.id() == target_id {
            return Some(self);
        }

        match self {
            LayoutNode::Split { left, right, .. } => {
                left.find_mut(target_id).or_else(|| right.find_mut(target_id))
            }
            _ => None,
        }
    }

    /// Check if this node is a divider (split node)
    pub fn is_split(&self) -> bool {
        matches!(self, LayoutNode::Split { .. })
    }

    /// Get all pane IDs in this subtree
    pub fn collect_panes(&self, ids: &mut Vec<NodeId>) {
        match self {
            LayoutNode::Pane { id, .. } => ids.push(*id),
            LayoutNode::Split { left, right, .. } => {
                left.collect_panes(ids);
                right.collect_panes(ids);
            }
        }
    }
}

/// Divider being dragged
#[derive(Debug, Clone, Copy)]
pub struct ActiveDivider {
    pub node_id: NodeId,
    pub original_pos: f32,
    pub drag_start: egui::Pos2,
}

/// Pane layout state
#[derive(Debug)]
pub struct PaneLayoutState {
    /// Root of the layout tree
    pub root: LayoutNode,
    /// Currently active (focused) pane
    pub active_pane: Option<NodeId>,
    /// Currently dragged divider
    pub resizing_divider: Option<ActiveDivider>,
    /// Hovering divider (for cursor change)
    pub hover_divider: Option<NodeId>,
    /// Next unique ID
    pub next_id: NodeId,
    /// Pane content cache
    pub pane_contents: HashMap<NodeId, PaneContent>,
}

impl PaneLayoutState {
    /// Create default layout with two panes side by side
    pub fn new() -> Self {
        let left = LayoutNode::pane(0, PaneContent::BatchExecutor);
        let right = LayoutNode::pane(1, PaneContent::LogViewer);
        let root = LayoutNode::split(2, left, right, 0.5, true);

        let mut contents = HashMap::new();
        contents.insert(0, PaneContent::BatchExecutor);
        contents.insert(1, PaneContent::LogViewer);

        Self {
            root,
            active_pane: Some(0),
            resizing_divider: None,
            hover_divider: None,
            next_id: 3,
            pane_contents: contents,
        }
    }

    /// Generate new unique ID
    pub fn next_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Get content for a pane
    pub fn get_content(&self, id: NodeId) -> Option<&PaneContent> {
        self.pane_contents.get(&id)
    }

    /// Set content for a pane
    pub fn set_content(&mut self, id: NodeId, content: PaneContent) {
        self.pane_contents.insert(id, content);

        if let Some(node) = self.root.find_mut(id) {
            if let LayoutNode::Pane { content: c, .. } = node {
                *c = content;
            }
        }
    }

    /// Update divider position
    pub fn update_divider(&mut self, node_id: NodeId, new_pos: f32) {
        if let Some(LayoutNode::Split { divider_pos, .. }) = self.root.find_mut(node_id) {
            *divider_pos = new_pos.clamp(0.1, 0.9);
        }
    }

    /// Get all pane IDs
    pub fn all_pane_ids(&self) -> Vec<NodeId> {
        let mut ids = Vec::new();
        self.root.collect_panes(&mut ids);
        ids
    }
}

impl Default for PaneLayoutState {
    fn default() -> Self {
        Self::new()
    }
}
