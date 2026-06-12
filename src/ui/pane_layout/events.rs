//! Pane layout event handling

use super::layout::PaneLayoutState;

/// Handle all pane layout events
pub fn handle_pane_events(ctx: &egui::Context, state: &mut PaneLayoutState) -> bool {
    let mut changed = false;

    // Handle divider dragging
    if let Some(active) = state.resizing_divider {
        if ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary)) {
            if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                let delta = pointer_pos - active.drag_start;

                if let Some(super::layout::LayoutNode::Split {
                    is_horizontal,
                    divider_pos: _,
                    ..
                }) = state.root.find_mut(active.node_id)
                {
                    let delta_factor = if *is_horizontal {
                        delta.x / 1000.0
                    } else {
                        delta.y / 1000.0
                    };

                    let new_pos = (active.original_pos + delta_factor).clamp(0.1, 0.9);
                    state.update_divider(active.node_id, new_pos);
                    changed = true;
                }
            }
        } else {
            state.resizing_divider = None;
            changed = true;
        }
    }

    // Clear hover if not dragging
    if state.resizing_divider.is_none() && !ctx.input(|i| i.pointer.is_moving()) {
        state.hover_divider = None;
    }

    changed
}
