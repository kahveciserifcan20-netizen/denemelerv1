use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn render_topbar(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    let page_names = ["Genel Bakış","Farming","Yetenekler","Eşyalar","Captcha","OCR","Çoklu Client","Ayarlar","Kayıtlar"];
    let page = page_names.get(gui.nav).unwrap_or(&"");
    let page_translated = lang::t(page, gui.dil_idx);

    egui::Frame::none()
        .fill(gui.t.bg1)
        .inner_margin(egui::Margin::symmetric(20.0, 10.0))
        .stroke(egui::Stroke::new(1.0, gui.t.border))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("Dashboard", gui.dil_idx)).size(12.0).color(gui.t.text_dim));
                ui.label(egui::RichText::new("/").size(12.0).color(gui.t.text_muted));
                ui.label(egui::RichText::new(page_translated).size(12.0).strong().color(gui.t.text));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Kullanıcı
                    ui.label(egui::RichText::new("K-User").size(11.0).color(gui.t.text_dim));
                    ui.label(egui::RichText::new("👤").size(14.0).color(gui.t.text_dim));
                    ui.add_space(8.0);

                    // Captcha badge
                    let count = gui.captcha_alert_count;
                    if count > 0 {
                        ui.label(egui::RichText::new(format!("{}", count)).size(10.0).color(gui.t.red).strong());
                    }
                    ui.label(egui::RichText::new("🔔").size(14.0).color(gui.t.text_dim));
                    ui.add_space(8.0);

                    // Ping
                    ui.label(egui::RichText::new(format!("{}ms", gui.ping_ms)).size(11.0).color(gui.t.text_dim));
                    ui.label(egui::RichText::new("📶").size(12.0).color(gui.t.text_dim));
                    ui.add_space(8.0);

                    // Bağlantı durumu — her zaman bağlı göster
                    ui.add(egui::Button::new(egui::RichText::new(format!("⚡ {}", lang::t("Bağlı", gui.dil_idx))).size(10.0).color(egui::Color32::BLACK))
                        .fill(gui.t.green).rounding(egui::Rounding::same(10.0)));
                });
            });
        });
}
