use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_yetenekler(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("⚡").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Yetenek Yönetimi", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Savaş rotasyonu ve yetenek konfigürasyonu", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    gui.card_frame().show(ui, |ui| {
        gui.section_title(ui, "🔄", &lang::t("YETENEK ROTASYONU", gui.dil_idx));
        ui.add_space(8.0);
        // Tablo başlığı
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(lang::t("SIRA", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(20.0);
            ui.label(egui::RichText::new(lang::t("YETENEK", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(60.0);
            ui.label(egui::RichText::new(lang::t("TUŞ", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(20.0);
            ui.label(egui::RichText::new(lang::t("BEKLEME", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(20.0);
            ui.label(egui::RichText::new(lang::t("KOŞUL", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(40.0);
            ui.label(egui::RichText::new(lang::t("DURUM", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
        });
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(12.0);

        // Boş durum — ileride doldurulacak
        let h = 200.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
            lang::t("Yetenek rotasyonu ileride eklenecektir.", gui.dil_idx),
            egui::FontId::proportional(13.0), gui.t.text_muted);
    });
}
