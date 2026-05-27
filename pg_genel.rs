use eframe::egui;
use super::super::MonolithGui;
use crate::gui::lang;

pub fn page_genel_bakis(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    // Başlık
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📊").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Genel Bakış", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Gerçek zamanlı bot durumu ve istatistikler", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let up = MonolithGui::uptime_str(gui.current_stats.uptime_secs);
            gui.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⏱").size(12.0).color(gui.t.accent));
                    ui.label(egui::RichText::new(&up).size(14.0).strong().color(gui.t.text).family(egui::FontFamily::Monospace));
                    ui.label(egui::RichText::new(lang::t("Çalışma Süresi", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
                });
            });
        });
    });
    ui.add_space(16.0);

    // Kırılan Taş + Mevcut Harita kartları
    ui.columns(2, |cols| {
        // Kırılan Taş
        gui.card_frame().show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("⛏ {}", lang::t("Kırılan Taş", gui.dil_idx).to_uppercase())).size(11.0).strong().color(gui.t.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("💎").size(16.0).color(gui.t.accent));
                });
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("{}", gui.current_stats.stones_mined)).size(32.0).strong().color(gui.t.text));

            // Son saatte gerçek değer — taş geçmişinden hesapla
            let stones_last_hour = gui.stone_history_last_hour();
            ui.label(egui::RichText::new(format!("+ {} {}", stones_last_hour, lang::t("son saatte", gui.dil_idx))).size(11.0).color(gui.t.text_dim));
        });

        // Mevcut Harita — model dosya ismi GÖSTERME
        gui.card_frame().show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("🗺 {}", lang::t("MEVCUT HARİTA", gui.dil_idx))).size(11.0).strong().color(gui.t.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("📍").size(16.0).color(gui.t.accent));
                });
            });
            ui.add_space(8.0);
            let name = MonolithGui::model_display_name(&gui.selected_model);
            ui.label(egui::RichText::new(&name).size(24.0).strong().color(gui.t.accent));
        });
    });
    ui.add_space(12.0);

    // Kırılan Taş Grafiği — gerçek veri
    gui.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(lang::t("Kırılan Taş Grafiği", gui.dil_idx).to_uppercase()).size(11.0).strong().color(gui.t.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("◆ {}", lang::t("Canlı", gui.dil_idx))).size(10.0).color(gui.t.green));
            });
        });
        ui.add_space(8.0);

        let h = 140.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, gui.t.bg0);

        // Grid çizgileri
        for i in 0..5 {
            let y = rect.top() + (i as f32 / 4.0) * h;
            painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(0.5, gui.t.border));
        }

        // Taş grafiği — son 60 veri noktası (dakika bazlı)
        let history = &gui.stone_history;
        if history.len() >= 2 {
            let display_count = history.len().min(60);
            let data: Vec<u32> = history.iter().rev().take(display_count).cloned().collect::<Vec<_>>().into_iter().rev().collect();
            let max_val = data.iter().cloned().max().unwrap_or(1).max(1) as f32;
            let bar_w = rect.width() / display_count as f32;

            for (i, &val) in data.iter().enumerate() {
                let bar_h = (val as f32 / max_val) * (h - 20.0);
                let x = rect.left() + i as f32 * bar_w;
                let y = rect.bottom() - bar_h;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(x + 1.0, y),
                    egui::vec2(bar_w - 2.0, bar_h),
                );
                let alpha = 120 + (135.0 * (val as f32 / max_val)) as u8;
                let col = egui::Color32::from_rgba_premultiplied(
                    gui.t.accent.r(), gui.t.accent.g(), gui.t.accent.b(), alpha
                );
                painter.rect_filled(bar_rect, 2.0, col);
            }

            // Y ekseni etiketi
            painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 4.0), egui::Align2::LEFT_TOP,
                format!("{} {}", lang::t("Max:", gui.dil_idx), max_val as u32), egui::FontId::proportional(9.0), gui.t.text_muted);
        } else {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER,
                &lang::t("Veri toplanıyor...", gui.dil_idx),
                egui::FontId::proportional(12.0), gui.t.text_muted);
        }

        // Alt etiket — Fix #10: _son_dk_label ölü değişken kaldırıldı
        painter.text(egui::pos2(rect.center().x, rect.bottom() - 4.0), egui::Align2::CENTER_BOTTOM,
            format!("{} {} {}", lang::t("Son", gui.dil_idx), history.len().min(60), lang::t("dakika", gui.dil_idx)),
            egui::FontId::proportional(9.0), gui.t.text_muted);
    });
    
    ui.add_space(12.0);
    
    // ── ONLINE LEARNING KARTI ─────────────────────────────────────────
    gui.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🧠").size(14.0).color(gui.t.accent));
            ui.label(egui::RichText::new("ONLINE LEARNING").size(11.0).strong().color(gui.t.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                super::super::draw_toggle(ui, &mut gui.learning_enabled, &gui.t);
            });
        });
        ui.add_space(8.0);
        
        if gui.learning_enabled {
            // İstatistikler
            ui.columns(3, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(egui::RichText::new(format!("{}", gui.learning_samples)).size(20.0).strong().color(gui.t.text));
                    ui.label(egui::RichText::new("Toplam Örnek").size(9.0).color(gui.t.text_dim));
                });
                cols[1].vertical(|ui| {
                    ui.label(egui::RichText::new(format!("{}", gui.learning_positive)).size(20.0).strong().color(gui.t.green));
                    ui.label(egui::RichText::new("Metin Taşı").size(9.0).color(gui.t.text_dim));
                });
                cols[2].vertical(|ui| {
                    ui.label(egui::RichText::new(format!("{}", gui.learning_negative)).size(20.0).strong().color(gui.t.red));
                    ui.label(egui::RichText::new("Kesilemez").size(9.0).color(gui.t.text_dim));
                });
            });
            
            ui.add_space(8.0);
            
            // Feedback butonları
            ui.horizontal(|ui| {
                if gui.pending_feedback {
                    ui.label(egui::RichText::new("Son tespit:").size(10.0).color(gui.t.text_dim));
                    if ui.add(egui::Button::new(egui::RichText::new("✓ Evet, Metin Taşı").size(11.0).color(egui::Color32::BLACK))
                        .fill(gui.t.green).rounding(egui::Rounding::same(6.0))).clicked() {
                        let _ = gui.cmd_tx.send("FEEDBACK_YES".into());
                        gui.pending_feedback = false;
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("✗ Hayır, Kesilemez").size(11.0).color(egui::Color32::WHITE))
                        .fill(gui.t.red).rounding(egui::Rounding::same(6.0))).clicked() {
                        let _ = gui.cmd_tx.send("FEEDBACK_NO".into());
                        gui.pending_feedback = false;
                    }
                } else {
                    ui.label(egui::RichText::new("Bekleyen feedback yok").size(10.0).color(gui.t.text_muted));
                }
            });
            
            ui.add_space(8.0);
            
            // Kontrol butonları
            ui.horizontal(|ui| {
                if ui.small_button("📊 İstatistik").clicked() {
                    let _ = gui.cmd_tx.send("LEARNING_STATS".into());
                }
                if ui.small_button("🔄 Sıfırla").clicked() {
                    let _ = gui.cmd_tx.send("LEARNING_RESET".into());
                    gui.learning_samples = 0;
                    gui.learning_positive = 0;
                    gui.learning_negative = 0;
                }
            });
        } else {
            ui.label(egui::RichText::new("Online Learning devre dışı").size(11.0).color(gui.t.text_dim));
        }
    });
}
