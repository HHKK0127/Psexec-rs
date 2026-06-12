use eframe::egui;
use crate::ui::command_palette::{CommandPalette, PaletteItem};
use crate::ui::pane_layout::PaneLayoutState;

pub struct AnalyzerApp {
    pub command_palette: CommandPalette,
    pub palette_visible: bool,
    pub pane_layout: PaneLayoutState,
}

impl Default for AnalyzerApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerApp {
    pub fn new() -> Self {
        let palette = CommandPalette::new();
        let layout = crate::ui::pane_layout::init();
        Self {
            command_palette: palette,
            palette_visible: false,
            pane_layout: layout,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        // Handle pane layout events (divider dragging, etc.)
        crate::ui::pane_layout::handle_pane_events(ctx, &mut self.pane_layout);

        // Check for Ctrl+P (Command Palette trigger) - Phase 4.4
        if ctx.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.ctrl) {
            self.palette_visible = true;
            self.command_palette.open();
        }

        // Render palette if visible
        if self.palette_visible {
            let (still_open, selected_item) = crate::ui::command_palette::render_palette(
                ctx,
                &mut self.command_palette,
            );
            self.palette_visible = still_open;

            // Handle selected item
            if let Some(item) = selected_item {
                self.handle_palette_selection(item);
            }
        }
    }

    fn handle_palette_selection(&mut self, item: PaletteItem) {
        match item {
            PaletteItem::QuickAction { id, .. } => {
                match id.as_str() {
                    "export_csv" => {
                        // Handle CSV export
                    }
                    "export_json" => {
                        // Handle JSON export
                    }
                    "open_settings" => {
                        // Handle settings
                    }
                    _ => {}
                }
            }
            PaletteItem::Template { .. } => {
                // Handle template selection
            }
            PaletteItem::HistoryEntry { .. } => {
                // Handle history entry
            }
            _ => {}
        }
    }
}

impl eframe::App for AnalyzerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Render pane layout (Phase 4.5)
        crate::ui::pane_layout::render_pane_layout(
            ctx,
            &mut self.pane_layout,
            ctx.available_rect(),
        );

        // Call the custom update method for palette handling
        self.update(ctx);

        // Command Palette rendered as overlay (Phase 4.4)
        if self.palette_visible {
            // Palette is already rendered in self.update(ctx) above
        }
    }
}
