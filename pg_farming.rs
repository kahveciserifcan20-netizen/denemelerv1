use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_farming(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🌾").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Farming", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Savaş, hedef ve bölge konfigürasyonu", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    ui.columns(2, |cols| {
        let display_model = gui.clients.iter().find(|c| c.active).map(|c| c.model.clone()).unwrap_or_else(|| gui.selected_model.clone());

        // Sol — Hedef Metin Taşları
        gui.card_frame().show(&mut cols[0], |ui| {
            gui.section_title(ui, "💎", &lang::t("HEDEF METİN TAŞLARI", gui.dil_idx));
            let stones = MonolithGui::stones_for_model(&display_model);
            for (name, level) in &stones {
                egui::Frame::none()
                    .fill(gui.t.bg3)
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("■").size(10.0).color(gui.t.accent));
                            ui.label(egui::RichText::new(lang::t(name, gui.dil_idx)).size(12.0).strong().color(gui.t.text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(*level).size(10.0).color(gui.t.accent));
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });

        // Sağ — Toplama Ayarları
        gui.card_frame().show(&mut cols[1], |ui| {
            gui.section_title(ui, "🎒", &lang::t("EŞYA TOPLAMA", gui.dil_idx));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("Topla", gui.dil_idx)).size(12.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::gui::draw_toggle(ui, &mut gui.toplama_aktif, &gui.t.clone());
                });
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(lang::t("Toplama Tuşu", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let z_sel = gui.toplama_tusu == "Z";
                let q_sel = gui.toplama_tusu == "\"";
                let z_col = if z_sel { gui.t.accent } else { gui.t.bg3 };
                let q_col = if q_sel { gui.t.accent } else { gui.t.bg3 };
                let z_tc = if z_sel { egui::Color32::BLACK } else { gui.t.text };
                let q_tc = if q_sel { egui::Color32::BLACK } else { gui.t.text };
                if ui.add(egui::Button::new(egui::RichText::new("Z").size(13.0).strong().color(z_tc))
                    .fill(z_col).rounding(egui::Rounding::same(6.0)).min_size(egui::vec2(50.0, 30.0))).clicked() {
                    gui.toplama_tusu = "Z".into();
                }
                if ui.add(egui::Button::new(egui::RichText::new("\"").size(13.0).strong().color(q_tc))
                    .fill(q_col).rounding(egui::Rounding::same(6.0)).min_size(egui::vec2(50.0, 30.0))).clicked() {
                    gui.toplama_tusu = "\"".into();
                }
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Her taş kesimi sonrasında seçilen tuşa 4-5 kere basarak yerdeki eşyaları toplar.").size(10.0).color(gui.t.text_muted));
        });
    });
}
