use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_kayitlar(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📋").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Kayıtlar (Logs)", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Gerçek zamanlı bot etkinlik akışı", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    // Filtre bar
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut gui.log_search).hint_text(lang::t("🔍 Log ara...", gui.dil_idx)).desired_width(250.0));
        ui.add_space(4.0);
        let filters = [lang::t("Tümü", gui.dil_idx), lang::t("Kırılan Taş", gui.dil_idx), "CAPTCHA".into()];
        for (i, f) in filters.iter().enumerate() {
            let sel = gui.log_filter == i;
            let bg = if sel { gui.t.accent } else { gui.t.bg3 };
            let tc = if sel { egui::Color32::BLACK } else { gui.t.text };
            if ui.add(egui::Button::new(egui::RichText::new(f).size(11.0).color(tc))
                .fill(bg).rounding(egui::Rounding::same(4.0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
                gui.log_filter = i;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new(egui::RichText::new(format!("🗑 {}", lang::t("Temizle", gui.dil_idx))).size(11.0).color(gui.t.red))
                .fill(gui.t.bg3).rounding(egui::Rounding::same(4.0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
                gui.logs.clear();
                gui.captcha_alert_count = 0;
            }
        });
    });
    ui.add_space(8.0);

    // Log viewer
    egui::Frame::none().fill(gui.t.bg1).rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0)).stroke(egui::Stroke::new(1.0, gui.t.border))
        .show(ui, |ui| {
            // Terminal header
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, gui.t.red);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, gui.t.yellow);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, gui.t.green);
                ui.add_space(8.0);
                ui.label(egui::RichText::new(format!("kbot_activity.log — {} {}", gui.logs.len(), lang::t("kayıt", gui.dil_idx))).size(10.0).color(gui.t.text_muted));
            });
            ui.add_space(8.0);

            egui::ScrollArea::vertical().auto_shrink([false; 2]).stick_to_bottom(true)
                .max_height(ui.available_height() - 4.0).show(ui, |ui| {
                    if gui.logs.is_empty() {
                        ui.label(egui::RichText::new(lang::t("Log kaydı bulunmadı.", gui.dil_idx)).size(12.0).color(gui.t.text_muted));
                    }
                    for l in &gui.logs {
                        // Filtre
                        let is_stone = l.contains("💎") || l.contains("Taş") || l.contains("kırıldı") || l.contains("Kesiliyor");
                        let is_captcha = l.contains("🚨") || l.contains("Captcha") || l.contains("captcha");
                        
                        let show = match gui.log_filter {
                            1 => is_stone,
                            2 => is_captcha,
                            _ => true,
                        };
                        if !show { continue; }
                        if !gui.log_search.is_empty() && !l.to_lowercase().contains(&gui.log_search.to_lowercase()) { continue; }

                        let c = if l.contains("✅") || l.contains("💎") { gui.t.green }
                            else if l.contains("❌") || l.contains("⚠") { gui.t.red }
                            else if l.contains("🚨") || l.contains("🔒") { gui.t.yellow }
                            else if l.contains("⚔") || l.contains("🎯") { gui.t.cyan }
                            else if l.contains("[STAT]") { gui.t.purple }
                            else { gui.t.text_dim };
                        ui.label(egui::RichText::new(l).size(12.0).color(c).family(egui::FontFamily::Monospace));
                        ui.add_space(2.0);
                    }
                });
        });
}
