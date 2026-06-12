//! Pane layout rendering

use super::layout::{LayoutNode, PaneContent, PaneLayoutState};
use egui::{Rect, Vec2, Color32, Stroke, CursorIcon};

const MIN_PANE_SIZE: f32 = 200.0;
const DIVIDER_THICKNESS: f32 = 4.0;
const DIVIDER_HIT_THICKNESS: f32 = 10.0;

/// Render the entire pane layout
pub fn render_pane_layout(
    ctx: &egui::Context,
    state: &mut PaneLayoutState,
    available_rect: Rect,
) {
    let mut root = state.root.clone();

    render_node(ctx, state, &mut root, available_rect);
    state.root = root;
}

fn render_node(
    ctx: &egui::Context,
    state: &mut PaneLayoutState,
    node: &mut LayoutNode,
    rect: Rect,
) {
    match node {
        LayoutNode::Pane { id, content } => {
            render_pane(ctx, state, *id, *content, rect);
        }
        LayoutNode::Split {
            id,
            left,
            right,
            divider_pos,
            is_horizontal,
        } => {
            render_split(
                ctx,
                state,
                *id,
                left,
                right,
                *divider_pos,
                *is_horizontal,
                rect,
            );
        }
    }
}

fn render_pane(
    ctx: &egui::Context,
    state: &mut PaneLayoutState,
    id: usize,
    content: PaneContent,
    rect: Rect,
) {
    let painter = ctx.layer_painter(egui::LayerId::background());

    let bg_color = if state.active_pane == Some(id) {
        Color32::from_gray(45)
    } else {
        Color32::from_gray(35)
    };

    painter.rect_filled(rect, 0.0, bg_color);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(60)));

    let header_height = 24.0;
    let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_height));

    painter.rect_filled(header_rect, 0.0, Color32::from_gray(50));

    painter.text(
        header_rect.min + Vec2::new(8.0, 4.0),
        egui::Align2::LEFT_TOP,
        content.title(),
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );
}

fn render_split(
    ctx: &egui::Context,
    state: &mut PaneLayoutState,
    node_id: usize,
    left: &mut LayoutNode,
    right: &mut LayoutNode,
    divider_pos: f32,
    is_horizontal: bool,
    rect: Rect,
) {
    let (first_rect, divider_rect, second_rect) = calculate_split_rects(
        rect,
        divider_pos,
        is_horizontal,
    );

    render_node(ctx, state, left, first_rect);
    render_node(ctx, state, right, second_rect);

    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new(("dividers", node_id))));

    let is_hovering = ctx.is_pointer_over_area() && divider_rect.contains(ctx.pointer_latest_pos().unwrap_or_default());
    let is_dragging = state.resizing_divider.map(|d| d.node_id == node_id).unwrap_or(false);

    let divider_color = if is_dragging {
        Color32::from_rgb(100, 150, 255)
    } else if is_hovering {
        Color32::from_rgb(150, 150, 150)
    } else {
        Color32::from_gray(60)
    };

    painter.rect_filled(divider_rect, 0.0, divider_color);

    if is_hovering || is_dragging {
        state.hover_divider = Some(node_id);

        let cursor = if is_horizontal {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        };
        ctx.set_cursor_icon(cursor);
    }

    let hit_rect = expand_rect(divider_rect, DIVIDER_HIT_THICKNESS / 2.0);

    if is_hovering {
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
            state.resizing_divider = Some(super::layout::ActiveDivider {
                node_id,
                original_pos: divider_pos,
                drag_start: ctx.pointer_latest_pos().unwrap_or_default(),
            });
        }
    }
}

fn calculate_split_rects(
    rect: Rect,
    divider_pos: f32,
    is_horizontal: bool,
) -> (Rect, Rect, Rect) {
    if is_horizontal {
        let total_width = rect.width() - DIVIDER_THICKNESS;
        let left_width = (total_width * divider_pos).max(MIN_PANE_SIZE).min(total_width - MIN_PANE_SIZE);

        let left = Rect::from_min_size(
            rect.min,
            Vec2::new(left_width, rect.height()),
        );

        let divider = Rect::from_min_size(
            rect.min + Vec2::new(left_width, 0.0),
            Vec2::new(DIVIDER_THICKNESS, rect.height()),
        );

        let right = Rect::from_min_size(
            rect.min + Vec2::new(left_width + DIVIDER_THICKNESS, 0.0),
            Vec2::new(total_width - left_width, rect.height()),
        );

        (left, divider, right)
    } else {
        let total_height = rect.height() - DIVIDER_THICKNESS;
        let top_height = (total_height * divider_pos).max(MIN_PANE_SIZE).min(total_height - MIN_PANE_SIZE);

        let top = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width(), top_height),
        );

        let divider = Rect::from_min_size(
            rect.min + Vec2::new(0.0, top_height),
            Vec2::new(rect.width(), DIVIDER_THICKNESS),
        );

        let bottom = Rect::from_min_size(
            rect.min + Vec2::new(0.0, top_height + DIVIDER_THICKNESS),
            Vec2::new(rect.width(), total_height - top_height),
        );

        (top, divider, bottom)
    }
}

fn expand_rect(rect: Rect, margin: f32) -> Rect {
    Rect::from_min_max(
        rect.min - Vec2::new(margin, margin),
        rect.max + Vec2::new(margin, margin),
    )
}
