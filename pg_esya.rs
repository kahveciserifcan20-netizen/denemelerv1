use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_esyalar(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📦").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Eşya Filtresi", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Düşen eşya toplama kuralları", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    gui.card_frame().show(ui, |ui| {
        let h = 250.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
            lang::t("Bu bölüm ileride eklenecektir.", gui.dil_idx),
            egui::FontId::proportional(13.0), gui.t.text_muted);
    });
}
