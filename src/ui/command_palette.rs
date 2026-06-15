use eframe::egui;

#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub items: Vec<PaletteItem>,
}

#[derive(Debug, Clone)]
pub enum PaletteItem {
    Command {
        label: String,
        action: String,
    },
    QuickAction {
        id: String,
        label: String,
    },
    Template {
        label: String,
        template_id: String,
    },
    HistoryEntry {
        label: String,
        entry_id: String,
    },
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            items: vec![],
        }
    }

    pub fn open(&mut self) {}
}

pub fn render_palette(
    _ctx: &egui::Context,
    _palette: &mut CommandPalette,
) -> (bool, Option<PaletteItem>) {
    (false, None)
}
