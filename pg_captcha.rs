use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_captcha(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🔐").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Captcha İstatistikleri", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Captcha çözüm başarı oranları", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    let total = gui.captcha_basarili + gui.captcha_basarisiz;
    let success_pct = if total > 0 { (gui.captcha_basarili as f32 / total as f32) * 100.0 } else { 0.0 };
    let fail_pct = if total > 0 { (gui.captcha_basarisiz as f32 / total as f32) * 100.0 } else { 0.0 };

    ui.columns(3, |cols| {
        // Başarılı
        gui.card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("✅ {}", lang::t("BAŞARILI", gui.dil_idx))).size(11.0).strong().color(gui.t.green));
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", gui.captcha_basarili)).size(36.0).strong().color(gui.t.green));
            ui.label(egui::RichText::new(format!("%{:.1}", success_pct)).size(13.0).color(gui.t.text_dim));
        });

        // Başarısız
        gui.card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("❌ {}", lang::t("BAŞARISIZ", gui.dil_idx))).size(11.0).strong().color(gui.t.red));
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", gui.captcha_basarisiz)).size(36.0).strong().color(gui.t.red));
            ui.label(egui::RichText::new(format!("%{:.1}", fail_pct)).size(13.0).color(gui.t.text_dim));
        });

        // Toplam
        gui.card_frame().show(&mut cols[2], |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("📊 {}", lang::t("TOPLAM", gui.dil_idx))).size(11.0).strong().color(gui.t.accent));
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", total)).size(36.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Toplam captcha", gui.dil_idx)).size(13.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    // Başarı oranı bar
    gui.card_frame().show(ui, |ui| {
        gui.section_title(ui, "📈", &lang::t("BAŞARI ORANI", gui.dil_idx));
        let bar_h = 24.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), bar_h), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 6.0, gui.t.red);
        if total > 0 {
            let sw = rect.width() * (success_pct / 100.0);
            let sr = egui::Rect::from_min_size(rect.min, egui::vec2(sw, bar_h));
            painter.rect_filled(sr, 6.0, gui.t.green);
        }
        painter.text(rect.center(), egui::Align2::CENTER_CENTER,
            format!("{} %{:.1} | {} %{:.1}", lang::t("Başarı:", gui.dil_idx), success_pct, lang::t("Başarısız:", gui.dil_idx), fail_pct),
            egui::FontId::proportional(11.0), egui::Color32::WHITE);

        ui.add_space(12.0);
        if gui.accent_btn(ui, &format!("🗑  {}", lang::t("Captcha Loglarını Sıfırla", gui.dil_idx))) {
            gui.captcha_basarili = 0;
            gui.captcha_basarisiz = 0;
            gui.captcha_alert_count = 0;
        }
    });
}
