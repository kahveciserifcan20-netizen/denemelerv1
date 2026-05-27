use eframe::egui;
use super::super::MonolithGui;
use crate::gui::{lang, SelectionType};
use crate::client_runner::{RadarPoint, RadarPointType};
use winapi::shared::windef::POINT;
use winapi::um::winuser::*;

/// Radar widget çiz — 180x180 daire radar
fn draw_radar(ui: &mut egui::Ui, points: &[RadarPoint], _t: &crate::gui::theme::Theme) {
    let size = egui::vec2(180.0, 180.0);
    let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = 85.0;

    // Arka plan daire
    painter.circle_filled(center, radius + 2.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
    painter.circle_stroke(center, radius, egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 200, 100)));

    // Artı çizgileri
    let cross_col = egui::Color32::from_rgba_unmultiplied(0, 200, 100, 60);
    painter.line_segment([egui::pos2(center.x - radius, center.y), egui::pos2(center.x + radius, center.y)], egui::Stroke::new(0.5, cross_col));
    painter.line_segment([egui::pos2(center.x, center.y - radius), egui::pos2(center.x, center.y + radius)], egui::Stroke::new(0.5, cross_col));

    // İç halkalar
    painter.circle_stroke(center, radius * 0.5, egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(0, 200, 100, 30)));
    painter.circle_stroke(center, radius * 0.25, egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(0, 200, 100, 20)));

    // Merkez nokta (oyuncu)
    painter.circle_filled(center, 4.0, egui::Color32::from_rgb(0, 200, 255));

    // Taş noktaları
    for p in points {
        let px = center.x + (p.x - 0.5) * 2.0 * radius;
        let py = center.y + (p.y - 0.5) * 2.0 * radius;

        // Radar dairesi dışındaysa çizme
        let dist = ((px - center.x).powi(2) + (py - center.y).powi(2)).sqrt();
        if dist > radius { continue; }

        let (color, dot_r) = match p.point_type {
            RadarPointType::Stone => (egui::Color32::from_rgb(0, 255, 100), 3.5),
            RadarPointType::BlacklistedStone => (egui::Color32::from_rgb(255, 60, 60), 2.5),
            RadarPointType::TargetStone => (egui::Color32::from_rgb(255, 220, 0), 4.0),
        };

        painter.circle_filled(egui::pos2(px, py), dot_r, color);

        // Confidence göstergesi (parlak halka)
        if p.confidence > 0.80 {
            painter.circle_stroke(egui::pos2(px, py), dot_r + 2.0, egui::Stroke::new(1.0, color.linear_multiply(0.5)));
        }
    }

    // Etiket
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        "RADAR",
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgb(0, 200, 100),
    );
    let stone_count = points.iter().filter(|p| p.point_type == RadarPointType::Stone).count();
    painter.text(
        egui::pos2(rect.right() - 4.0, rect.top() + 4.0),
        egui::Align2::RIGHT_TOP,
        &format!("{}⛏", stone_count),
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgb(200, 200, 200),
    );
}

pub fn page_coklu_client(gui: &mut MonolithGui, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🖥").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Çoklu Client Yönetimi", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Birden fazla oyun penceresini yönetin", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    gui.card_frame().show(ui, |ui| {
        gui.section_title(ui, "📋", &lang::t("CLIENT LİSTESİ", gui.dil_idx));
        
        if gui.clients.is_empty() {
            ui.label(egui::RichText::new(lang::t("Henüz client eklenmedi.", gui.dil_idx)).size(12.0).color(gui.t.text_muted));
        }

        let mut remove_idx: Option<usize> = None;
        let mut toggle_preview: Option<usize> = None;
        let mut toggle_radar: Option<usize> = None;
        // 🎯 Anlık görüntü isteği - borrow checker için döngü dışında işlem yapılacak
        let mut screenshot_request: Option<(isize, usize, SelectionType)> = None;

        for (i, c) in gui.clients.iter_mut().enumerate() {
            egui::Frame::none()
                .fill(gui.t.bg3)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Başlık satırı
                        ui.horizontal(|ui| {
                            // Per-client stats kontrolü
                            let client_running = gui.per_client_stats.get(&i).map_or(false, |s| s.is_running);
                            let dot_col = if client_running { gui.t.green } else if c.active { gui.t.yellow } else { gui.t.red };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, dot_col);
                            ui.label(egui::RichText::new(&c.name).size(13.0).strong().color(gui.t.text));
                            
                            if c.waiting_for_pid {
                                ui.label(egui::RichText::new(lang::t("⏳ Shift ile seç...", gui.dil_idx)).size(10.0).color(gui.t.yellow));
                            } else {
                                let btn_text = if c.hwnd.is_empty() || c.hwnd == "0" {
                                    lang::t("🎯 PID Seç", gui.dil_idx)
                                } else {
                                    format!("🎯 HWND: {}", c.hwnd)
                                };
                                let btn_col = if c.hwnd.is_empty() || c.hwnd == "0" { gui.t.text } else { gui.t.accent };
                                if ui.add(egui::Button::new(egui::RichText::new(btn_text).size(10.0).color(btn_col))
                                    .fill(gui.t.bg2).rounding(egui::Rounding::same(4.0))).clicked() {
                                    c.waiting_for_pid = true;
                                }
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🗑").size(12.0).color(gui.t.red))
                                    .frame(false)).clicked() {
                                    remove_idx = Some(i);
                                }
                                
                                // Başlat/Durdur — client_runner thread
                                let is_running = gui.per_client_stats.get(&i).map_or(false, |s| s.is_running);
                                if is_running {
                                    if ui.add(egui::Button::new(egui::RichText::new("⏹ Durdur").size(10.0).color(egui::Color32::WHITE))
                                        .fill(gui.t.red).rounding(egui::Rounding::same(4.0))).clicked() {
                                        let _ = gui.cmd_tx.send(format!("STOP_CLIENT:{}", i));
                                    }
                                } else {
                                if ui.add(egui::Button::new(egui::RichText::new("▶ Başlat").size(10.0).color(egui::Color32::BLACK))
                                    .fill(gui.t.green).rounding(egui::Rounding::same(4.0))).clicked() {
                                    // Debug log
                                    gui.logs.push_back(format!("[{}] 🔧 Başlat tıklandı - Client {}: hwnd={}, kilit={}", 
                                        chrono::Local::now().format("%H:%M:%S"), i, c.hwnd, c.kilit_path));
                                    
                                    if c.hwnd.is_empty() || c.hwnd == "0" {
                                        gui.logs.push_back(format!("[{}] ❌ Önce HWND seçin! (PID Seç butonuna tıklayın)", 
                                            chrono::Local::now().format("%H:%M:%S")));
                                    } else if c.kilit_path.is_empty() || c.kilit_path == "0" {
                                        gui.logs.push_back(format!("[{}] ❌ Kilit görseli seçilmemiş! (📂 butonuna tıklayın)", 
                                            chrono::Local::now().format("%H:%M:%S")));
                                } else {
                                    // Skill tuşlarını virgülle ayrılmış string olarak gönder
                                    let skill_tuslari_str = if c.olum_skill_tuslari.is_empty() {
                                        "EMPTY".to_string()
                                    } else {
                                        c.olum_skill_tuslari.join(",")
                                    };
                                    
                                    let cmd = format!("START_CLIENT:{}:{}:{}:{}:{}:{},{},{},{}:{},{},{},{}:{}:{}:{}:{}:{}:{}:{},{},{},{}",
                                            i, c.hwnd, c.model, c.driver, c.kilit_path,
                                            c.search_x1, c.search_y1, c.search_x2, c.search_y2,
                                            c.ocr_x1, c.ocr_y1, c.ocr_x2, c.ocr_y2,
                                            if c.olum_skill_aktif { "1" } else { "0" },
                                            skill_tuslari_str,
                                            c.olum_skill_bekleme,
                                            if c.olum_binek_aktif { "1" } else { "0" },
                                            c.olum_binek_tusu,
                                            c.olum_binek_bekleme,
                                            c.captcha_buton_x1, c.captcha_buton_y1, c.captcha_buton_x2, c.captcha_buton_y2);
                                        gui.logs.push_back(format!("[{}] 📤 Komut gönderiliyor: {}", 
                                            chrono::Local::now().format("%H:%M:%S"), cmd));
                                        let _ = gui.cmd_tx.send(cmd);
                                    }
                                }
                                }
                                
                                // Radar toggle
                                let radar_txt = if c.radar_aktif { "📡 Radar ✓" } else { "📡" };
                                let radar_col = if c.radar_aktif { gui.t.accent } else { gui.t.bg2 };
                                if ui.add(egui::Button::new(egui::RichText::new(radar_txt).size(10.0).color(gui.t.text))
                                    .fill(radar_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                    toggle_radar = Some(i);
                                }

                                let is_preview = gui.active_client_preview == Some(i);
                                let preview_txt = if is_preview { lang::t("📺 Kapat", gui.dil_idx) } else { lang::t("📺 İzle", gui.dil_idx) };
                                let preview_col = if is_preview { gui.t.accent } else { gui.t.bg2 };
                                if ui.add(egui::Button::new(egui::RichText::new(preview_txt).size(10.0).color(gui.t.text))
                                    .fill(preview_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                    toggle_preview = Some(i);
                                }
                            });
                        });
                        
                        ui.add_space(4.0);

                        // Per-client istatistikler
                        if let Some(stats) = gui.per_client_stats.get(&i) {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("⛏{} ❌{} 🕐{:.0}s", 
                                    stats.stones_mined, stats.stones_missed, stats.uptime_secs))
                                    .size(10.0).color(gui.t.text_dim));
                                ui.label(egui::RichText::new(&stats.state_name).size(10.0).color(
                                    if stats.is_running { gui.t.green } else { gui.t.text_muted }
                                ));
                            });
                            ui.add_space(2.0);
                        }
                        
                        // Model ve Driver satırı
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("YOLO:").size(10.0).color(gui.t.text_dim));
                            egui::ComboBox::from_id_source(format!("mdl_{}", i))
                                .width(120.0).selected_text(&c.model)
                                .show_ui(ui, |ui| {
                                    for m in &gui.available_models.clone() {
                                        ui.selectable_value(&mut c.model, m.clone(), m);
                                    }
                                });
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(lang::t("Fare:", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
                            egui::ComboBox::from_id_source(format!("mse_{}", i))
                                .width(100.0).selected_text(&c.driver)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut c.driver, "Arduino_AUTO".into(), "Arduino");
                                    ui.selectable_value(&mut c.driver, "WinAPI".into(), "WinAPI");
                                });
                        });
                        
                        ui.add_space(6.0);
                        
                        // Kilit ayarları (kompakt)
                        egui::Frame::none()
                            .fill(gui.t.bg2)
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🔒").size(10.0).color(gui.t.accent));
                                    ui.label(egui::RichText::new(&c.kilit_path).size(9.0).color(gui.t.text));
                                    if ui.add(egui::Button::new(egui::RichText::new("📂").size(9.0)).fill(gui.t.bg3)).clicked() {
                                        if let Some(p) = rfd::FileDialog::new().add_filter("PNG",&["png"]).pick_file() {
                                            c.kilit_path = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🔍").size(9.0));
                                    ui.label(egui::RichText::new(format!("{} {} {} {}", 
                                        c.search_x1, c.search_y1, c.search_x2, c.search_y2))
                                        .size(9.0).color(gui.t.text_dim));
                                    
                                    // 🎯 ANLIK GÖRÜNTÜDEN SEÇİM BUTONU - Arama Bölgesi
                                    if ui.add(egui::Button::new(egui::RichText::new("🎯 Seç").size(9.0))
                                        .fill(gui.t.accent).rounding(egui::Rounding::same(4.0)))
                                        .clicked() {
                                        if let Ok(hwnd) = c.hwnd.parse::<isize>() {
                                            if hwnd > 0 {
                                                // Borrow checker için request'i kaydet, döngü dışında işle
                                                screenshot_request = Some((hwnd, i, SelectionType::SearchRegion));
                                            } else {
                                                gui.logs.push_back(format!("[{}] ❌ Önce oyun penceresi seçin!", 
                                                    chrono::Local::now().format("%H:%M:%S")));
                                            }
                                        }
                                    }
                                });
                            });
                        
                        ui.add_space(6.0);
                        
                        // ── ÖLÜM SONRASI YENİDEN BAŞLATMA ─────────────────────────
                        egui::Frame::none()
                            .fill(gui.t.bg2)
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("💀").size(10.0).color(gui.t.red));
                                    ui.label(egui::RichText::new("Ölüm Sonrası:").size(9.0).color(gui.t.text_dim));
                                    
                                    // Aktif/Pasif toggle
                                    let toggle_text = if c.olum_aktif { "✓ Aktif" } else { "✗ Pasif" };
                                    let toggle_col = if c.olum_aktif { gui.t.green } else { gui.t.bg3 };
                                    if ui.add(egui::Button::new(egui::RichText::new(toggle_text).size(9.0).color(gui.t.text))
                                        .fill(toggle_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                        c.olum_aktif = !c.olum_aktif;
                                    }
                                    
                                    ui.add_space(8.0);
                                    
                                    // Mod seçimi butonları
                                    let burada_col = if c.olum_modu == crate::gui::OlumModu::Burada { gui.t.accent } else { gui.t.bg3 };
                                    let sehirde_col = if c.olum_modu == crate::gui::OlumModu::Sehirde { gui.t.accent } else { gui.t.bg3 };
                                    
                                    if ui.add(egui::Button::new(egui::RichText::new("📍 Burada").size(9.0).color(gui.t.text))
                                        .fill(burada_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                        c.olum_modu = crate::gui::OlumModu::Burada;
                                    }
                                    if ui.add(egui::Button::new(egui::RichText::new("🏙️ Şehirde").size(9.0).color(gui.t.text))
                                        .fill(sehirde_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                        c.olum_modu = crate::gui::OlumModu::Sehirde;
                                    }
                                });
                                
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("⏱️ Bekleme:").size(9.0).color(gui.t.text_dim));
                                    let mut bekle_str = c.olum_bekleme_suresi.to_string();
                                    ui.add(egui::TextEdit::singleline(&mut bekle_str).desired_width(40.0));
                                    ui.label(egui::RichText::new("sn").size(9.0).color(gui.t.text_dim));
                                    if let Ok(v) = bekle_str.parse::<u64>() {
                                        if v >= 5 && v <= 60 {
                                            c.olum_bekleme_suresi = v;
                                        }
                                    }
                                });
                                
                                ui.add_space(4.0);
                                
                                // ── SKILL AYARLARI (3 Tuş Kombinasyonu) ─────────────────────────
                                ui.horizontal(|ui| {
                                    // Skill Aktif/Pasif
                                    let skill_toggle_text = if c.olum_skill_aktif { "⚔️ 3 Skill ✓" } else { "⚔️ 3 Skill ✗" };
                                    let skill_toggle_col = if c.olum_skill_aktif { gui.t.accent } else { gui.t.bg3 };
                                    if ui.add(egui::Button::new(egui::RichText::new(skill_toggle_text).size(9.0).color(gui.t.text))
                                        .fill(skill_toggle_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                        c.olum_skill_aktif = !c.olum_skill_aktif;
                                    }
                                    
                                    if c.olum_skill_aktif {
                                        ui.label(egui::RichText::new("Bekle:").size(9.0).color(gui.t.text_dim));
                                        let mut skill_bekle_str = c.olum_skill_bekleme.to_string();
                                        ui.add(egui::TextEdit::singleline(&mut skill_bekle_str).desired_width(30.0));
                                        ui.label(egui::RichText::new("sn").size(9.0).color(gui.t.text_dim));
                                        if let Ok(v) = skill_bekle_str.parse::<u64>() {
                                            if v >= 1 && v <= 30 {
                                                c.olum_skill_bekleme = v;
                                            }
                                        }
                                    }
                                });
                                
                                // 3 Skill tuşu gösterimi ve yakalama
                                if c.olum_skill_aktif {
                                    ui.horizontal(|ui| {
                                        for i in 0..3 {
                                            let btn_text = if c.skill_tus_yakalama_modu && c.skill_tus_yakalama_idx == i {
                                                format!("⏳ Skill {}", i + 1)
                                            } else if i < c.olum_skill_tuslari.len() {
                                                c.olum_skill_tuslari[i].clone()
                                            } else {
                                                format!("Skill {}", i + 1)
                                            };
                                            
                                            let btn_col = if c.skill_tus_yakalama_modu && c.skill_tus_yakalama_idx == i {
                                                gui.t.yellow
                                            } else if i < c.olum_skill_tuslari.len() && !c.olum_skill_tuslari[i].is_empty() {
                                                gui.t.green
                                            } else {
                                                gui.t.bg3
                                            };
                                            
                                            if ui.add(egui::Button::new(egui::RichText::new(&btn_text).size(9.0).color(gui.t.text))
                                                .fill(btn_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                                c.skill_tus_yakalama_modu = true;
                                                c.skill_tus_yakalama_idx = i;
                                                c.binek_tus_yakalama_modu = false; // Diğer modu kapat
                                                gui.logs.push_back(format!("[{}] ⏳ Skill {} tuşu yakalanıyor... Tuşa basın!", 
                                                    chrono::Local::now().format("%H:%M:%S"), i + 1));
                                            }
                                            
                                            if i < 2 { ui.add_space(4.0); }
                                        }
                                        
                                        // Sıfırla butonu
                                        if ui.small_button("🔄").clicked() {
                                            c.olum_skill_tuslari.clear();
                                            c.skill_tus_yakalama_modu = false;
                                            gui.logs.push_back(format!("[{}] 🔄 Skill tuşları sıfırlandı", 
                                                chrono::Local::now().format("%H:%M:%S")));
                                        }
                                    });
                                }
                                
                                // ── BİNEK AYARLARI ─────────────────────────
                                ui.horizontal(|ui| {
                                    // Binek Aktif/Pasif
                                    let binek_toggle_text = if c.olum_binek_aktif { "🐴 Binek ✓" } else { "🐴 Binek ✗" };
                                    let binek_toggle_col = if c.olum_binek_aktif { gui.t.accent } else { gui.t.bg3 };
                                    if ui.add(egui::Button::new(egui::RichText::new(binek_toggle_text).size(9.0).color(gui.t.text))
                                        .fill(binek_toggle_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                        c.olum_binek_aktif = !c.olum_binek_aktif;
                                    }
                                    
                                    if c.olum_binek_aktif {
                                        ui.label(egui::RichText::new("Bekle:").size(9.0).color(gui.t.text_dim));
                                        let mut binek_bekle_str = c.olum_binek_bekleme.to_string();
                                        ui.add(egui::TextEdit::singleline(&mut binek_bekle_str).desired_width(30.0));
                                        ui.label(egui::RichText::new("sn").size(9.0).color(gui.t.text_dim));
                                        if let Ok(v) = binek_bekle_str.parse::<u64>() {
                                            if v >= 1 && v <= 30 {
                                                c.olum_binek_bekleme = v;
                                            }
                                        }
                                    }
                                });
                                
                                // Binek tuşu gösterimi ve yakalama
                                if c.olum_binek_aktif {
                                    ui.horizontal(|ui| {
                                        let btn_text = if c.binek_tus_yakalama_modu {
                                            "⏳ Tuş Yakalanıyor...".to_string()
                                        } else if !c.olum_binek_tusu.is_empty() {
                                            c.olum_binek_tusu.clone()
                                        } else {
                                            "Tuş Ayarla".to_string()
                                        };
                                        
                                        let btn_col = if c.binek_tus_yakalama_modu {
                                            gui.t.yellow
                                        } else if !c.olum_binek_tusu.is_empty() {
                                            gui.t.green
                                        } else {
                                            gui.t.bg3
                                        };
                                        
                                        if ui.add(egui::Button::new(egui::RichText::new(&btn_text).size(9.0).color(gui.t.text))
                                            .fill(btn_col).rounding(egui::Rounding::same(4.0))).clicked() {
                                            c.binek_tus_yakalama_modu = true;
                                            c.skill_tus_yakalama_modu = false; // Diğer modu kapat
                                            gui.logs.push_back(format!("[{}] ⏳ Binek tuşu yakalanıyor... Tuşa basın!", 
                                                chrono::Local::now().format("%H:%M:%S")));
                                        }
                                        
                                        // Sıfırla butonu
                                        if ui.small_button("🔄").clicked() {
                                            c.olum_binek_tusu.clear();
                                            c.binek_tus_yakalama_modu = false;
                                            gui.logs.push_back(format!("[{}] 🔄 Binek tuşu sıfırlandı", 
                                                chrono::Local::now().format("%H:%M:%S")));
                                        }
                                    });
                                }
                            });

                        // ── RADAR WIDGET (per-client, açılır/kapanır) ──
                        if c.radar_aktif {
                            ui.add_space(6.0);
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgba_unmultiplied(10, 20, 10, 200))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    let points = gui.per_client_radar.get(&i).cloned().unwrap_or_default();
                                    draw_radar(ui, &points, &gui.t);
                                });
                        }
                    });
                });
            ui.add_space(4.0);
        }

        // 🎯 Anlık görüntü isteğini işle (borrow checker için döngü dışında)
        if let Some((hwnd, client_idx, selection_type)) = screenshot_request {
            gui.start_screenshot_selection(hwnd as _, client_idx, selection_type);
        }

        // Radar toggle
        if let Some(idx) = toggle_radar {
            if idx < gui.clients.len() {
                gui.clients[idx].radar_aktif = !gui.clients[idx].radar_aktif;
            }
        }

        // Hotkey HWND yakalama
        for c in &mut gui.clients {
            if c.template_select_mode {
                let hp = c.hwnd.parse::<usize>().unwrap_or(0) as winapi::shared::windef::HWND;
                unsafe {
                    let s3 = GetAsyncKeyState(0x33) < 0;
                    if s3 && !c.key3_pressed {
                        let mut p = POINT{x:0,y:0}; GetCursorPos(&mut p);
                        if !hp.is_null() { ScreenToClient(hp, &mut p); }
                        c.template_x1 = p.x.max(0); c.template_y1 = p.y.max(0);
                    }
                    c.key3_pressed = s3;
                    let s4 = GetAsyncKeyState(0x34) < 0;
                    if s4 && !c.key4_pressed {
                        let mut p = POINT{x:0,y:0}; GetCursorPos(&mut p);
                        if !hp.is_null() { ScreenToClient(hp, &mut p); }
                        c.template_x2 = p.x.max(0); c.template_y2 = p.y.max(0);
                        c.template_select_mode = false;
                    }
                    c.key4_pressed = s4;
                }
            }
        }

        // Preview toggle
        if let Some(idx) = toggle_preview {
            if gui.active_client_preview == Some(idx) {
                gui.active_client_preview = None;
                let _ = gui.cmd_tx.send("PREVIEW:0".into());
            } else {
                gui.active_client_preview = Some(idx);
                if idx < gui.clients.len() {
                    let hwnd = gui.clients[idx].hwnd.parse::<usize>().unwrap_or(0);
                    if hwnd > 0 { let _ = gui.cmd_tx.send(format!("PREVIEW:{}", hwnd)); }
                }
            }
        }
        if let Some(idx) = remove_idx {
            let _ = gui.cmd_tx.send(format!("STOP_CLIENT:{}", idx));
            gui.clients.remove(idx);
            if gui.active_client_preview == Some(idx) { gui.active_client_preview = None; }
        }

        ui.add_space(8.0);
        if gui.dim_btn(ui, &format!("+ {}", lang::t("Yeni Client Ekle", gui.dil_idx))) {
            let name = format!("Client {:02}", gui.clients.len() + 1);
            let model = if !gui.available_models.is_empty() { gui.available_models[0].clone() } else { "".into() };
            gui.clients.push(crate::gui::ClientData {
                name, hwnd: "0".into(), active: true, model,
                driver: "Arduino_AUTO".into(), waiting_for_pid: false,
                kilit_path: "hedef_kilit.png".into(),
                template_x1: 354, template_y1: 54, template_x2: 398, template_y2: 71,
                search_x1: 300, search_y1: 20, search_x2: 500, search_y2: 90,
                ocr_x1: 313, ocr_y1: 153, ocr_x2: 457, ocr_y2: 168,
                template_select_mode: false, search_select_mode: false,
                key3_pressed: false, key4_pressed: false, key5_pressed: false, key6_pressed: false,
                selecting_region: false, selection_start: None, selection_end: None,
                ocr_selecting_region: false, ocr_selection_start: None, ocr_selection_end: None,
                radar_aktif: false,
                // Ölüm sonrası varsayılan değerler
                olum_modu: crate::gui::OlumModu::Burada,
                olum_bekleme_suresi: 10,
                olum_aktif: false,
                // Ölüm sonrası skill ve binek varsayılan değerler (3 skill tuşu desteği)
                olum_skill_aktif: false,
                olum_skill_tuslari: vec![],
                olum_skill_bekleme: 3,
                olum_binek_aktif: false,
                olum_binek_tusu: String::new(),
                olum_binek_bekleme: 5,
                // Tuş yakalama modları
                skill_tus_yakalama_modu: false,
                binek_tus_yakalama_modu: false,
                skill_tus_yakalama_idx: 0,
                // Captcha buton bölgesi - Varsayılan: (354,420)-(445,449)
                captcha_buton_x1: 354, captcha_buton_y1: 420,
                captcha_buton_x2: 445, captcha_buton_y2: 449,
                captcha_selecting_region: false,
                captcha_selection_start: None,
                captcha_selection_end: None,
                // Auto-PM AI varsayılan değerler
                pm_ai_aktif: false,
                pm_ai_backend: "gemini".to_string(),
                pm_ai_api_key: String::new(),
                pm_system_prompt: "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string(),
                pm_region: (0, 0, 0, 0),
                pm_cooldown_sn: 30,
                pm_daily_limit: 200,
                pm_selecting_region: false,
                pm_selection_start: None,
                pm_selection_end: None,
                // PM Simge bölgesi varsayılan değerler
                pm_simge_x1: 0,
                pm_simge_y1: 0,
                pm_simge_x2: 0,
                pm_simge_y2: 0,
                pm_simge_selecting_region: false,
                pm_simge_selection_start: None,
                pm_simge_selection_end: None,
            });
        }
    });

    // Canlı ekran preview
    if let Some(idx) = gui.active_client_preview {
        if idx < gui.clients.len() {
            ui.add_space(12.0);
            let client_name = gui.clients[idx].name.clone();
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "📺", &format!("{} {}", lang::t("CANLI EKRAN —", gui.dil_idx), client_name));
                
                // Bölge seçimi butonları
                ui.horizontal(|ui| {
                    let c = &mut gui.clients[idx];
                    
                    // Arama Bölgesi Seç butonu
                    let btn_txt = if c.selecting_region { 
                        lang::t("🔍 Seçiliyor...", gui.dil_idx) 
                    } else { 
                        lang::t("🔍 Arama Bölgesi", gui.dil_idx) 
                    };
                    let btn_col = if c.selecting_region { gui.t.accent } else { gui.t.bg3 };
                    if ui.add(egui::Button::new(egui::RichText::new(btn_txt).size(10.0).color(gui.t.text))
                        .fill(btn_col).rounding(egui::Rounding::same(4.0))).clicked() {
                        c.selecting_region = !c.selecting_region;
                        c.selection_start = None;
                        c.selection_end = None;
                        // OCR seçimini kapat
                        c.ocr_selecting_region = false;
                        c.ocr_selection_start = None;
                        c.ocr_selection_end = None;
                    }
                    
                    ui.label(egui::RichText::new(format!("({},{})-({},{})", 
                        c.search_x1, c.search_y1, c.search_x2, c.search_y2))
                        .size(9.0).color(gui.t.text_dim));
                    
                    // OCR Bölgesi Seç butonu
                    let ocr_btn_txt = if c.ocr_selecting_region { 
                        lang::t("📝 Seçiliyor...", gui.dil_idx) 
                    } else { 
                        lang::t("📝 OCR Bölgesi", gui.dil_idx) 
                    };
                    let ocr_btn_col = if c.ocr_selecting_region { gui.t.accent } else { gui.t.bg3 };
                    if ui.add(egui::Button::new(egui::RichText::new(ocr_btn_txt).size(10.0).color(gui.t.text))
                        .fill(ocr_btn_col).rounding(egui::Rounding::same(4.0))).clicked() {
                        c.ocr_selecting_region = !c.ocr_selecting_region;
                        c.ocr_selection_start = None;
                        c.ocr_selection_end = None;
                        // Arama seçimini kapat
                        c.selecting_region = false;
                        c.selection_start = None;
                        c.selection_end = None;
                    }
                    
                    ui.label(egui::RichText::new(format!("({},{})-({},{})", 
                        c.ocr_x1, c.ocr_y1, c.ocr_x2, c.ocr_y2))
                        .size(9.0).color(gui.t.text_dim));
                    
                    // Captcha Buton Bölgesi Seç butonu
                    let captcha_btn_txt = if c.captcha_selecting_region { 
                        lang::t("🎯 Seçiliyor...", gui.dil_idx) 
                    } else { 
                        lang::t("🎯 Captcha Buton", gui.dil_idx) 
                    };
                    let captcha_btn_col = if c.captcha_selecting_region { gui.t.accent } else { gui.t.bg3 };
                    if ui.add(egui::Button::new(egui::RichText::new(captcha_btn_txt).size(10.0).color(gui.t.text))
                        .fill(captcha_btn_col).rounding(egui::Rounding::same(4.0))).clicked() {
                        c.captcha_selecting_region = !c.captcha_selecting_region;
                        c.captcha_selection_start = None;
                        c.captcha_selection_end = None;
                        // Diğer seçimleri kapat
                        c.selecting_region = false;
                        c.selection_start = None;
                        c.selection_end = None;
                        c.ocr_selecting_region = false;
                        c.ocr_selection_start = None;
                        c.ocr_selection_end = None;
                    }
                    
                    let has_captcha = c.captcha_buton_x1 > 0 && c.captcha_buton_y1 > 0;
                    let captcha_info_col = if has_captcha { gui.t.green } else { gui.t.text_dim };
                    ui.label(egui::RichText::new(format!("({},{})-({},{})", 
                        c.captcha_buton_x1, c.captcha_buton_y1, c.captcha_buton_x2, c.captcha_buton_y2))
                        .size(9.0).color(captcha_info_col));
                    
                    // Seçili bölgeleri sıfırla
                    if ui.small_button("🔄").clicked() {
                        c.search_x1 = 300; c.search_y1 = 20;
                        c.search_x2 = 500; c.search_y2 = 90;
                        c.ocr_x1 = 313; c.ocr_y1 = 153;
                        c.ocr_x2 = 457; c.ocr_y2 = 168;
                        c.captcha_buton_x1 = 354; c.captcha_buton_y1 = 420;
                        c.captcha_buton_x2 = 445; c.captcha_buton_y2 = 449;
                    }
                });
                
                ui.add_space(6.0);
                
                if let Some(ref f) = gui.current_frame {
                    let tex = ctx.load_texture("client_preview",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [f.width() as usize, f.height() as usize], f.as_raw()),
                        egui::TextureOptions::LINEAR);
                    let w = ui.available_width().min(600.0);
                    let a = f.height() as f32 / f.width() as f32;
                    let img_size = egui::Vec2::new(w, w * a);
                    
                    // Görüntü ve etkileşim
                    let (rect, response) = ui.allocate_exact_size(img_size, egui::Sense::drag());
                    ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    
                    // Bölge seçimi işlemi - Arama Bölgesi
                    let c = &mut gui.clients[idx];
                    if c.selecting_region {
                        // Fare pozisyonunu görüntü koordinatlarına çevir
                        if response.dragged() {
                            let pos = response.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO);
                            let rel_x = ((pos.x - rect.left()) / rect.width() * f.width() as f32) as i32;
                            let rel_y = ((pos.y - rect.top()) / rect.height() * f.height() as f32) as i32;
                            
                            if c.selection_start.is_none() {
                                c.selection_start = Some((rel_x.max(0), rel_y.max(0)));
                            }
                            c.selection_end = Some((rel_x.max(0), rel_y.max(0)));
                        }
                        
                        // Sürükleme bittiğinde bölgeyi kaydet
                        if response.drag_stopped() {
                            if let (Some((x1, y1)), Some((x2, y2))) = (c.selection_start, c.selection_end) {
                                c.search_x1 = x1.min(x2);
                                c.search_y1 = y1.min(y2);
                                c.search_x2 = x1.max(x2);
                                c.search_y2 = y1.max(y2);
                                c.selecting_region = false;
                                c.selection_start = None;
                                c.selection_end = None;
                                
                                gui.logs.push_back(format!("[{}] 🎯 Arama bölgesi güncellendi: ({},{}) - ({},{})", 
                                    chrono::Local::now().format("%H:%M:%S"),
                                    c.search_x1, c.search_y1, c.search_x2, c.search_y2));
                            }
                        }
                    }
                    
                    // OCR Bölgesi seçimi
                    if c.ocr_selecting_region {
                        // Fare pozisyonunu görüntü koordinatlarına çevir
                        if response.dragged() {
                            let pos = response.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO);
                            let rel_x = ((pos.x - rect.left()) / rect.width() * f.width() as f32) as i32;
                            let rel_y = ((pos.y - rect.top()) / rect.height() * f.height() as f32) as i32;
                            
                            if c.ocr_selection_start.is_none() {
                                c.ocr_selection_start = Some((rel_x.max(0), rel_y.max(0)));
                            }
                            c.ocr_selection_end = Some((rel_x.max(0), rel_y.max(0)));
                        }
                        
                        // Sürükleme bittiğinde bölgeyi kaydet
                        if response.drag_stopped() {
                            if let (Some((x1, y1)), Some((x2, y2))) = (c.ocr_selection_start, c.ocr_selection_end) {
                                c.ocr_x1 = x1.min(x2);
                                c.ocr_y1 = y1.min(y2);
                                c.ocr_x2 = x1.max(x2);
                                c.ocr_y2 = y1.max(y2);
                                c.ocr_selecting_region = false;
                                c.ocr_selection_start = None;
                                c.ocr_selection_end = None;
                                
                                gui.logs.push_back(format!("[{}] 📝 OCR bölgesi güncellendi: ({},{}) - ({},{})", 
                                    chrono::Local::now().format("%H:%M:%S"),
                                    c.ocr_x1, c.ocr_y1, c.ocr_x2, c.ocr_y2));
                            }
                        }
                    }
                    
                    // Captcha Buton Bölgesi seçimi
                    if c.captcha_selecting_region {
                        if response.dragged() {
                            let pos = response.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO);
                            let rel_x = ((pos.x - rect.left()) / rect.width() * f.width() as f32) as i32;
                            let rel_y = ((pos.y - rect.top()) / rect.height() * f.height() as f32) as i32;
                            
                            if c.captcha_selection_start.is_none() {
                                c.captcha_selection_start = Some((rel_x.max(0), rel_y.max(0)));
                            }
                            c.captcha_selection_end = Some((rel_x.max(0), rel_y.max(0)));
                        }
                        
                        if response.drag_stopped() {
                            if let (Some((x1, y1)), Some((x2, y2))) = (c.captcha_selection_start, c.captcha_selection_end) {
                                c.captcha_buton_x1 = x1.min(x2);
                                c.captcha_buton_y1 = y1.min(y2);
                                c.captcha_buton_x2 = x1.max(x2);
                                c.captcha_buton_y2 = y1.max(y2);
                                c.captcha_selecting_region = false;
                                c.captcha_selection_start = None;
                                c.captcha_selection_end = None;
                                
                                gui.logs.push_back(format!("[{}] 🎯 Captcha buton bölgesi güncellendi: ({},{}) - ({},{})", 
                                    chrono::Local::now().format("%H:%M:%S"),
                                    c.captcha_buton_x1, c.captcha_buton_y1, c.captcha_buton_x2, c.captcha_buton_y2));
                            }
                        }
                    }
                    
                    // Arama Bölgesini çiz (sarı dikdörtgen)
                    let c = &gui.clients[idx];
                    let x1_norm = c.search_x1 as f32 / f.width() as f32;
                    let y1_norm = c.search_y1 as f32 / f.height() as f32;
                    let x2_norm = c.search_x2 as f32 / f.width() as f32;
                    let y2_norm = c.search_y2 as f32 / f.height() as f32;
                    
                    let sel_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + x1_norm * rect.width(), rect.top() + y1_norm * rect.height()),
                        egui::pos2(rect.left() + x2_norm * rect.width(), rect.top() + y2_norm * rect.height())
                    );
                    ui.painter().rect_stroke(sel_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 0)));
                    
                    // OCR Bölgesini çiz (mavi dikdörtgen)
                    let ocr_x1_norm = c.ocr_x1 as f32 / f.width() as f32;
                    let ocr_y1_norm = c.ocr_y1 as f32 / f.height() as f32;
                    let ocr_x2_norm = c.ocr_x2 as f32 / f.width() as f32;
                    let ocr_y2_norm = c.ocr_y2 as f32 / f.height() as f32;
                    
                    let ocr_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + ocr_x1_norm * rect.width(), rect.top() + ocr_y1_norm * rect.height()),
                        egui::pos2(rect.left() + ocr_x2_norm * rect.width(), rect.top() + ocr_y2_norm * rect.height())
                    );
                    ui.painter().rect_stroke(ocr_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 255)));
                    
                    // Captcha Buton Bölgesini çiz (yeşil dikdörtgen)
                    if c.captcha_buton_x1 > 0 && c.captcha_buton_y1 > 0 && c.captcha_buton_x2 > c.captcha_buton_x1 {
                        let cb_x1_norm = c.captcha_buton_x1 as f32 / f.width() as f32;
                        let cb_y1_norm = c.captcha_buton_y1 as f32 / f.height() as f32;
                        let cb_x2_norm = c.captcha_buton_x2 as f32 / f.width() as f32;
                        let cb_y2_norm = c.captcha_buton_y2 as f32 / f.height() as f32;
                        
                        let captcha_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.left() + cb_x1_norm * rect.width(), rect.top() + cb_y1_norm * rect.height()),
                            egui::pos2(rect.left() + cb_x2_norm * rect.width(), rect.top() + cb_y2_norm * rect.height())
                        );
                        ui.painter().rect_stroke(captcha_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100)));
                        
                        // Captcha Buton etiketi
                        ui.painter().text(
                            egui::pos2(rect.left() + cb_x1_norm * rect.width(), rect.top() + cb_y1_norm * rect.height() - 12.0),
                            egui::Align2::LEFT_BOTTOM,
                            "🎯 Captcha",
                            egui::FontId::proportional(9.0),
                            egui::Color32::from_rgb(0, 255, 100)
                        );
                    }
                    
                    // Arama seçim sürüklenirken geçici bölgeyi göster
                    if c.selecting_region {
                        if let (Some((sx1, sy1)), Some((sx2, sy2))) = (c.selection_start, c.selection_end) {
                            let sx1_norm = sx1.min(sx2) as f32 / f.width() as f32;
                            let sy1_norm = sy1.min(sy2) as f32 / f.height() as f32;
                            let sx2_norm = sx1.max(sx2) as f32 / f.width() as f32;
                            let sy2_norm = sy1.max(sy2) as f32 / f.height() as f32;
                            
                            let drag_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.left() + sx1_norm * rect.width(), rect.top() + sy1_norm * rect.height()),
                                egui::pos2(rect.left() + sx2_norm * rect.width(), rect.top() + sy2_norm * rect.height())
                            );
                            ui.painter().rect_stroke(drag_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100)));
                            ui.painter().rect_filled(drag_rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 255, 100, 30));
                        }
                    }
                    
                    // OCR seçim sürüklenirken geçici bölgeyi göster
                    if c.ocr_selecting_region {
                        if let (Some((sx1, sy1)), Some((sx2, sy2))) = (c.ocr_selection_start, c.ocr_selection_end) {
                            let sx1_norm = sx1.min(sx2) as f32 / f.width() as f32;
                            let sy1_norm = sy1.min(sy2) as f32 / f.height() as f32;
                            let sx2_norm = sx1.max(sx2) as f32 / f.width() as f32;
                            let sy2_norm = sy1.max(sy2) as f32 / f.height() as f32;
                            
                            let drag_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.left() + sx1_norm * rect.width(), rect.top() + sy1_norm * rect.height()),
                                egui::pos2(rect.left() + sx2_norm * rect.width(), rect.top() + sy2_norm * rect.height())
                            );
                            ui.painter().rect_stroke(drag_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)));
                            ui.painter().rect_filled(drag_rect, 0.0, egui::Color32::from_rgba_unmultiplied(100, 200, 255, 30));
                        }
                    }
                    
                    // Captcha seçim sürüklenirken geçici bölgeyi göster
                    if c.captcha_selecting_region {
                        if let (Some((sx1, sy1)), Some((sx2, sy2))) = (c.captcha_selection_start, c.captcha_selection_end) {
                            let sx1_norm = sx1.min(sx2) as f32 / f.width() as f32;
                            let sy1_norm = sy1.min(sy2) as f32 / f.height() as f32;
                            let sx2_norm = sx1.max(sx2) as f32 / f.width() as f32;
                            let sy2_norm = sy1.max(sy2) as f32 / f.height() as f32;
                            
                            let drag_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.left() + sx1_norm * rect.width(), rect.top() + sy1_norm * rect.height()),
                                egui::pos2(rect.left() + sx2_norm * rect.width(), rect.top() + sy2_norm * rect.height())
                            );
                            ui.painter().rect_stroke(drag_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100)));
                            ui.painter().rect_filled(drag_rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 255, 100, 30));
                        }
                    }
                    
                    // Arama Bölgesi etiketi
                    ui.painter().text(
                        egui::pos2(rect.left() + x1_norm * rect.width(), rect.top() + y1_norm * rect.height() - 12.0),
                        egui::Align2::LEFT_BOTTOM,
                        "🔍 Arama",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(255, 220, 0)
                    );
                    
                    // OCR Bölgesi etiketi
                    ui.painter().text(
                        egui::pos2(rect.left() + ocr_x1_norm * rect.width(), rect.top() + ocr_y1_norm * rect.height() - 12.0),
                        egui::Align2::LEFT_BOTTOM,
                        "📝 OCR",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(0, 200, 255)
                    );
                } else {
                    ui.label(egui::RichText::new(lang::t("Kamera verisi bekleniyor...", gui.dil_idx)).size(12.0).color(gui.t.text_muted));
                }
            });
        }
    }
}
