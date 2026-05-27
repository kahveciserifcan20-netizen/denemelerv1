use eframe::egui;
use super::super::MonolithGui;

use crate::gui::lang;

pub fn page_ocr(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("👁").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("OCR & Kilit Ayarları", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Captcha OCR alanı ve hedef kilit konfigürasyonu", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);



    ui.columns(2, |cols| {
        // OCR Alanı
        gui.card_frame().show(&mut cols[0], |ui| {
            gui.section_title(ui, "📝", &lang::t("CAPTCHA OCR ALANI (SABİT)", gui.dil_idx));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("X1:313 Y1:153 X2:457 Y2:168").size(12.0).color(gui.t.text));
                ui.label(egui::RichText::new(lang::t("(Sabit)", gui.dil_idx)).size(10.0).color(gui.t.text_muted));
            });
            ui.label(egui::RichText::new(lang::t("OCR alanı bu koordinatlarda çalışır.", gui.dil_idx)).size(10.0).color(gui.t.text_muted));
        });

        // Kilit Ayarları
        gui.card_frame().show(&mut cols[1], |ui| {
            gui.section_title(ui, "🔒", &lang::t("HEDEF KİLİT AYARLARI", gui.dil_idx));
            ui.label(egui::RichText::new(format!("📄 {}", gui.kilit_path)).size(10.0).color(gui.t.text_dim));
            if gui.dim_btn(ui, &lang::t("📂 Şablon Seç", gui.dil_idx)) {
                if let Some(p) = rfd::FileDialog::new().add_filter("PNG",&["png"]).pick_file() {
                    gui.kilit_path = p.to_string_lossy().to_string();
                    let _ = gui.cmd_tx.send(format!("LOAD_KILIT:{}", gui.kilit_path));
                }
            }
            ui.add_space(4.0);
            if gui.kilit_select_mode {
                ui.label(egui::RichText::new(lang::t("⏳ 3:Sol üst  4:Sağ alt", gui.dil_idx)).size(11.0).color(gui.t.yellow));
            } else if gui.dim_btn(ui, &lang::t("🔒 Bölge Seç (3/4)", gui.dil_idx)) {
                gui.kilit_select_mode = true;
            }
            ui.horizontal(|ui| {
                for (l,v) in [("X1",&mut gui.kilit_x1),("Y1",&mut gui.kilit_y1),("X2",&mut gui.kilit_x2),("Y2",&mut gui.kilit_y2)] {
                    ui.label(egui::RichText::new(l).size(10.0).color(gui.t.text_dim));
                    ui.add(egui::TextEdit::singleline(v).desired_width(35.0));
                }
            });
        });
    });
    ui.add_space(12.0);

    // Kaydet + Test
    ui.horizontal(|ui| {
        if gui.accent_btn(ui, &lang::t("💾 Ayarları Kaydet", gui.dil_idx)) {
            let _ = gui.cmd_tx.send(format!("UPDATE_OCR:{},{},{},{}", gui.ocr_x1, gui.ocr_y1, gui.ocr_x2, gui.ocr_y2));
            let _ = gui.cmd_tx.send(format!("UPDATE_KILIT_REGION:{},{},{},{}", gui.kilit_x1, gui.kilit_y1, gui.kilit_x2, gui.kilit_y2));
            let _ = gui.cmd_tx.send("SAVE_CONFIG".into());
            let mut cfg = crate::config::AppConfig::load();
            cfg.kilit_path = gui.kilit_path.clone();
            cfg.kilit_x1 = gui.kilit_x1.parse().unwrap_or(0);
            cfg.kilit_y1 = gui.kilit_y1.parse().unwrap_or(0);
            cfg.kilit_x2 = gui.kilit_x2.parse().unwrap_or(0);
            cfg.kilit_y2 = gui.kilit_y2.parse().unwrap_or(0);
            cfg.save();
        }
        if gui.dim_btn(ui, &lang::t("🎯 Test Tıklama", gui.dil_idx)) {
            let _ = gui.cmd_tx.send("TEST_CLICK".into());
        }
    });
}
