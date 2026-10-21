use crate::theme::c_tx;
use egui::{FontId, TextEdit, Ui};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct YamlEditor {
    pub path: Option<PathBuf>,
    pub content: String,
}

impl YamlEditor {
    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        let c = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.content = c;
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(p) = &self.path {
            std::fs::write(p, &self.content).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Файл не выбран".into())
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut self.content)
                    .font(FontId::monospace(13.0))
                    .desired_width(f32::INFINITY)
                    .text_color(c_tx()),
            );
        });
    }
}
