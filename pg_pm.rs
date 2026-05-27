// pg_pm.rs — Auto-PM AI Cevaplama Sayfası

use eframe::egui;
use super::super::MonolithGui;
use crate::gui::draw_toggle;

pub fn page_auto_pm(gui: &mut MonolithGui, ui: &mut egui::Ui) {
    // ── Her frame: API test sonucunu kontrol et ──────────────────────────────
    let mut test_done = false;
    if let Some(ref rx) = gui.pm_test_rx {
        match rx.try_recv() {
            Ok(result) => {
                gui.pm_test_sonucu = result;
                test_done = true;
                ui.ctx().request_repaint(); // GUI'yi hemen yenile
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if gui.pm_test_sonucu.contains("🔄") {
                    gui.pm_test_sonucu = "❌ Thread beklenmedik şekilde kapandı".into();
                }
                test_done = true;
            }
        }
    }
    if test_done { gui.pm_test_rx = None; }

    // Başlık
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("💬").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Auto-PM AI Cevaplama").size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new("Oyun içi özel mesajlara yapay zeka ile otomatik yanıt ver").size(11.0).color(gui.t.text_dim));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let status = if gui.pm_ai_aktif { ("🟢 Aktif", gui.t.green) } else { ("🔴 Pasif", gui.t.red) };
            ui.label(egui::RichText::new(status.0).size(12.0).color(status.1));
        });
    });
    ui.add_space(16.0);

    ui.columns(2, |cols| {
        // ── Sol Sütun: Ayarlar ──────────────────────────────────────────────
        cols[0].vertical(|ui| {
            // Ana Açma/Kapama
            gui.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🤖  PM AI Sistemi").size(13.0).strong().color(gui.t.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        draw_toggle(ui, &mut gui.pm_ai_aktif, &gui.t);
                    });
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new(
                    "Aktif olduğunda bot, oyun içindeki PM bölgesini düzenli tarayarak \
                     gelen mesajlara AI ile yanıt verir."
                ).size(10.0).color(gui.t.text_muted));
            });
            ui.add_space(8.0);

            // Backend Seçimi
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "🧠", "AI BACKEND");

                // Backend Seçici
                ui.horizontal(|ui| {
                    for (lbl, val) in [("🌐 Gemini", "gemini"), ("🔷 OpenAI", "openai"), ("💻 Ollama", "ollama")] {
                        let selected = gui.pm_ai_backend == val;
                        let col = if selected { gui.t.accent } else { gui.t.bg3 };
                        let tc  = if selected { egui::Color32::BLACK } else { gui.t.text };
                        if ui.add(egui::Button::new(egui::RichText::new(lbl).size(11.0).color(tc))
                            .fill(col).rounding(egui::Rounding::same(6.0))).clicked() {
                            gui.pm_ai_backend = val.to_string();
                        }
                    }
                });
                ui.add_space(4.0);

                // Backend açıklaması
                let desc = match gui.pm_ai_backend.as_str() {
                    "openai" => "GPT-4o-mini · ~$0.001/gün · En kaliteli yanıtlar",
                    "ollama" => "Yerel model · Tamamen ücretsiz · İnternet gerekmez",
                    _        => "Gemini 1.5 Flash · Ücretsiz 15 RPM · Hızlı başlangıç",
                };
                ui.label(egui::RichText::new(desc).size(9.0).color(gui.t.text_muted));
                ui.add_space(6.0);

                // API Key (Ollama için gizli)
                if gui.pm_ai_backend != "ollama" {
                    ui.label(egui::RichText::new("API Anahtarı:").size(10.0).color(gui.t.text_dim));
                    ui.add(egui::TextEdit::singleline(&mut gui.pm_ai_api_key)
                        .password(true)
                        .hint_text("sk-... veya AIza...")
                        .desired_width(ui.available_width()));
                } else {
                    ui.label(egui::RichText::new("ℹ️  Ollama için API anahtarı gerekmez").size(9.0).color(gui.t.text_muted));
                    ui.label(egui::RichText::new("localhost:11434 üzerinde çalışmalı").size(9.0).color(gui.t.text_muted));
                }
            });
            ui.add_space(8.0);

            // Sistem Promptu
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "📝", "SİSTEM PROMPTU");
                ui.label(egui::RichText::new("AI'a karakterin kim olduğunu anlat:").size(10.0).color(gui.t.text_dim));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::multiline(&mut gui.pm_system_prompt)
                    .desired_width(ui.available_width())
                    .desired_rows(4)
                    .hint_text("Sen Metin2 oynayan bir oyuncusun..."));
                ui.add_space(6.0);

                // Hazır promptlar
                ui.label(egui::RichText::new("Hazır Şablonlar:").size(9.0).color(gui.t.text_muted));
                ui.horizontal(|ui| {
                    if ui.small_button("🗡️ Savaşçı").clicked() {
                        gui.pm_system_prompt = "Sen agresif bir Metin2 savaşçısısın. PM'lere kısa, özlü ve bazen alaycı yanıt ver. Max 1 cümle.".to_string();
                    }
                    if ui.small_button("🤝 Dost").clicked() {
                        gui.pm_system_prompt = "Sen Metin2 oynayan samimi bir oyuncusun. Gelen mesajlara sıcak ve yardımsever yanıt ver. Max 2 cümle.".to_string();
                    }
                    if ui.small_button("🤐 Meşgul").clicked() {
                        gui.pm_system_prompt = "Sen farm yapan meşgul bir oyuncusun. PM'lere çok kısa yanıt ver: 'meşgulüm', 'sonra', 'afk' gibi.".to_string();
                    }
                });
            });
            ui.add_space(8.0);

            // PM Bölgesi & Cooldown
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "📍", "PM ALANI & ZAMANLAMA");

                // PM Bölgesi — durum göstergesi
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("PM Bölgesi:").size(10.0).color(gui.t.text_dim));
                    if gui.pm_region_x2 > gui.pm_region_x1 {
                        ui.label(egui::RichText::new(format!(
                            "({},{}) → ({},{})", gui.pm_region_x1, gui.pm_region_y1, gui.pm_region_x2, gui.pm_region_y2
                        )).size(10.0).color(gui.t.green));
                    } else {
                        ui.label(egui::RichText::new("Seçilmedi").size(10.0).color(gui.t.red));
                    }
                });

                // Mesaj göster
                if !gui.pm_region_mesaj.is_empty() {
                    ui.label(egui::RichText::new(&gui.pm_region_mesaj).size(9.0).color(gui.t.accent));
                }
                ui.add_space(6.0);

                // Seçim butonu — aktif client'ın ekranından sürükle-bırak ile seçim
                ui.horizontal(|ui| {
                    if gui.accent_btn(ui, "📸 Ekrandan Seç") {
                        // İlk aktif client'ın HWND'sini bul
                        let hwnd_val = gui.clients.iter()
                            .find(|c| c.active && !c.hwnd.is_empty() && c.hwnd != "0")
                            .and_then(|c| c.hwnd.parse::<usize>().ok())
                            .unwrap_or(0);

                        if hwnd_val != 0 {
                            let hwnd_ptr = hwnd_val as winapi::shared::windef::HWND;
                            gui.start_screenshot_selection(
                                hwnd_ptr, 0,
                                crate::gui::SelectionType::PmRegion
                            );
                            gui.screenshot_window_open = true;
                        } else {
                            gui.pm_region_mesaj = "Önce aktif bir client seçin (HWND gerekli)".to_string();
                        }
                    }
                    ui.add_space(4.0);
                    if gui.dim_btn(ui, "Sıfırla") {
                        gui.pm_region_x1 = 0; gui.pm_region_y1 = 0;
                        gui.pm_region_x2 = 0; gui.pm_region_y2 = 0;
                        gui.pm_region_mesaj = "PM bölgesi sıfırlandı".to_string();
                    }
                });
                ui.add_space(4.0);

                // Manuel koordinat girişi (alternatif)
                ui.label(egui::RichText::new("veya koordinat girin:").size(9.0).color(gui.t.text_muted));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X1:").size(9.0).color(gui.t.text_dim));
                    let mut x1 = gui.pm_region_x1;
                    if ui.add(egui::DragValue::new(&mut x1).speed(1)).changed() { gui.pm_region_x1 = x1; }
                    ui.label(egui::RichText::new("Y1:").size(9.0).color(gui.t.text_dim));
                    let mut y1 = gui.pm_region_y1;
                    if ui.add(egui::DragValue::new(&mut y1).speed(1)).changed() { gui.pm_region_y1 = y1; }
                    ui.label(egui::RichText::new("X2:").size(9.0).color(gui.t.text_dim));
                    let mut x2 = gui.pm_region_x2;
                    if ui.add(egui::DragValue::new(&mut x2).speed(1)).changed() { gui.pm_region_x2 = x2; }
                    ui.label(egui::RichText::new("Y2:").size(9.0).color(gui.t.text_dim));
                    let mut y2 = gui.pm_region_y2;
                    if ui.add(egui::DragValue::new(&mut y2).speed(1)).changed() { gui.pm_region_y2 = y2; }
                });
                ui.add_space(8.0);

                // Cooldown
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Yanıt Cooldown:").size(10.0).color(gui.t.text_dim));
                    let mut cd = gui.pm_cooldown_sn as i64;
                    if ui.add(egui::DragValue::new(&mut cd).speed(1).clamp_range(10..=600).suffix(" sn")).changed() {
                        gui.pm_cooldown_sn = cd as u64;
                    }
                });

                // Günlük limit
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Günlük Limit:").size(10.0).color(gui.t.text_dim));
                    let mut dl = gui.pm_daily_limit as i64;
                    if ui.add(egui::DragValue::new(&mut dl).speed(1).clamp_range(0..=1000).suffix(" PM")).changed() {
                        gui.pm_daily_limit = dl as u32;
                    }
                });
                ui.label(egui::RichText::new("0 = sınırsız").size(9.0).color(gui.t.text_muted));
            });
            ui.add_space(8.0);

                // PM Simge Bölgesi (pm_simge.png arama alanı)
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "", "PM SİMGE ARAMA BÖLGESİ");
                ui.label(egui::RichText::new("pm_simge.png'nin aranacağı alan (yanıp sönen bildirim ikonu)").size(9.0).color(gui.t.text_muted));
                ui.add_space(4.0);

                // PM Simge Bölgesi — durum göstergesi
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Simge Bölgesi:").size(10.0).color(gui.t.text_dim));
                    if gui.pm_simge_region_x2 > gui.pm_simge_region_x1 {
                        ui.label(egui::RichText::new(format!(
                            "({},{}) → ({},{})", gui.pm_simge_region_x1, gui.pm_simge_region_y1, gui.pm_simge_region_x2, gui.pm_simge_region_y2
                        )).size(10.0).color(gui.t.green));
                    } else {
                        ui.label(egui::RichText::new("Varsayılan (sağ taraf)").size(10.0).color(gui.t.text_muted));
                    }
                });
                ui.add_space(6.0);

                // Seçim butonu
                ui.horizontal(|ui| {
                    if gui.accent_btn(ui, "📸 Ekrandan Seç") {
                        // İlk aktif client'ın HWND'sini bul
                        let hwnd_val = gui.clients.iter()
                            .find(|c| c.active && !c.hwnd.is_empty() && c.hwnd != "0")
                            .and_then(|c| c.hwnd.parse::<usize>().ok())
                            .unwrap_or(0);

                        if hwnd_val != 0 {
                            let hwnd_ptr = hwnd_val as winapi::shared::windef::HWND;
                            gui.start_screenshot_selection(
                                hwnd_ptr, 0,
                                crate::gui::SelectionType::PmSimgeRegion
                            );
                            gui.screenshot_window_open = true;
                        } else {
                            gui.pm_region_mesaj = "Önce aktif bir client seçin (HWND gerekli)".to_string();
                        }
                    }
                    ui.add_space(4.0);
                    if gui.dim_btn(ui, "Sıfırla") {
                        gui.pm_simge_region_x1 = 0; gui.pm_simge_region_y1 = 0;
                        gui.pm_simge_region_x2 = 0; gui.pm_simge_region_y2 = 0;
                        gui.pm_region_mesaj = "PM simge bölgesi sıfırlandı (varsayılan kullanılacak)".to_string();
                    }
                });
                ui.add_space(4.0);

                // Manuel koordinat girişi
                ui.label(egui::RichText::new("veya koordinat girin:").size(9.0).color(gui.t.text_muted));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X1:").size(9.0).color(gui.t.text_dim));
                    let mut x1 = gui.pm_simge_region_x1;
                    if ui.add(egui::DragValue::new(&mut x1).speed(1)).changed() { gui.pm_simge_region_x1 = x1; }
                    ui.label(egui::RichText::new("Y1:").size(9.0).color(gui.t.text_dim));
                    let mut y1 = gui.pm_simge_region_y1;
                    if ui.add(egui::DragValue::new(&mut y1).speed(1)).changed() { gui.pm_simge_region_y1 = y1; }
                    ui.label(egui::RichText::new("X2:").size(9.0).color(gui.t.text_dim));
                    let mut x2 = gui.pm_simge_region_x2;
                    if ui.add(egui::DragValue::new(&mut x2).speed(1)).changed() { gui.pm_simge_region_x2 = x2; }
                    ui.label(egui::RichText::new("Y2:").size(9.0).color(gui.t.text_dim));
                    let mut y2 = gui.pm_simge_region_y2;
                    if ui.add(egui::DragValue::new(&mut y2).speed(1)).changed() { gui.pm_simge_region_y2 = y2; }
                });
            });

        });

        // ── Sağ Sütun: İstatistik & Kaydet ────────────────────────────────
        cols[1].vertical(|ui| {
            // İstatistikler
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "📊", "BUGÜNKÜ İSTATİSTİKLER");
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new(format!("{}", gui.pm_reply_count))
                            .size(28.0).strong().color(gui.t.accent));
                        ui.label(egui::RichText::new("Yanıtlanan PM").size(9.0).color(gui.t.text_dim));
                    });
                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new(format!("{} sn", gui.pm_cooldown_sn))
                            .size(28.0).strong().color(gui.t.text));
                        ui.label(egui::RichText::new("Cooldown").size(9.0).color(gui.t.text_dim));
                    });
                });
                ui.add_space(8.0);
                // Backend bilgisi
                let backend_name = match gui.pm_ai_backend.as_str() {
                    "openai" => "OpenAI GPT-4o-mini",
                    "ollama" => "Yerel Ollama",
                    _        => "Google Gemini Flash",
                };
                ui.label(egui::RichText::new(format!("🧠 Backend: {}", backend_name))
                    .size(10.0).color(gui.t.text_muted));
            });
            ui.add_space(8.0);

            // Nasıl Çalışır
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "ℹ️", "NASIL ÇALIŞIR?");
                let steps = [
                    ("1️⃣", "PM bölgesini seç (OCR tarayacak alan)"),
                    ("2️⃣", "AI backend ve API key gir"),
                    ("3️⃣", "Sistem promptunu ayarla"),
                    ("4️⃣", "PM AI'ı aktive et"),
                    ("5️⃣", "Bot, PM gelince AI'dan yanıt alır ve yazar"),
                ];
                for (num, step) in &steps {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(*num).size(11.0));
                        ui.label(egui::RichText::new(*step).size(10.0).color(gui.t.text_dim));
                    });
                    ui.add_space(2.0);
                }
            });
            ui.add_space(8.0);

            // Test & Kaydet
            gui.card_frame().show(ui, |ui| {
                gui.section_title(ui, "🔧", "TEST & KAYDET");

                if !gui.pm_test_sonucu.is_empty() {
                    let col = if gui.pm_test_sonucu.contains("✅") { gui.t.green }
                              else if gui.pm_test_sonucu.contains("❌") { gui.t.red }
                              else { gui.t.accent };
                    ui.label(egui::RichText::new(&gui.pm_test_sonucu).size(10.0).color(col));
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if gui.accent_btn(ui, "💾 Kaydet") {
                        let mut cfg = crate::config::AppConfig::load();
                        cfg.pm_ai_aktif   = gui.pm_ai_aktif;
                        cfg.pm_ai_backend = gui.pm_ai_backend.clone();
                        cfg.pm_ai_api_key = gui.pm_ai_api_key.clone();
                        cfg.pm_system_prompt = gui.pm_system_prompt.clone();
                        cfg.pm_region_x1  = gui.pm_region_x1;
                        cfg.pm_region_y1  = gui.pm_region_y1;
                        cfg.pm_region_x2  = gui.pm_region_x2;
                        cfg.pm_region_y2  = gui.pm_region_y2;
                        cfg.pm_cooldown_sn = gui.pm_cooldown_sn;
                        cfg.pm_daily_limit = gui.pm_daily_limit;
                        cfg.save();
                        gui.pm_test_sonucu = "✅ Ayarlar kaydedildi!".into();
                    }
                    ui.add_space(4.0);
                    if gui.dim_btn(ui, "🧪 API Test") {
                        if gui.pm_ai_api_key.is_empty() && gui.pm_ai_backend != "ollama" {
                            gui.pm_test_sonucu = "❌ API key boş!".into();
                        } else {
                            let backend = gui.pm_ai_backend.clone();
                            let api_key = gui.pm_ai_api_key.clone();
                            let prompt  = if gui.pm_system_prompt.is_empty() {
                                "Sen yardımcı bir asistansın.".to_string()
                            } else {
                                gui.pm_system_prompt.clone()
                            };
                            let (tx, rx) = std::sync::mpsc::channel::<String>();
                            std::thread::spawn(move || {
                                let msg = match backend.as_str() {
                                    "openai" => {
                                        let be = crate::pm_ai::AiBackend::from_config("openai", &api_key);
                                        let mut engine = crate::pm_ai::PmAiEngine::new(be, prompt, 0, 0);
                                        match engine.get_reply("TestBot", "Merhaba, bu bir test mesajıdır.", None) {
                                            Some(r) => format!("✅ OpenAI Yanıt: {}", r.chars().take(100).collect::<String>()),
                                            None    => "❌ OpenAI: Yanıt alınamadı".to_string(),
                                        }
                                    }
                                    "gemini" | _ => {
                                        // Gemini: detaylı hata mesajıyla test
                                        let test_prompt = format!("{}\n\nTest: Merhaba!", prompt);
                                        match crate::pm_ai::ask_gemini_detailed(&api_key, &test_prompt) {
                                            Ok(r)  => format!("✅ Gemini Yanıt: {}", r.chars().take(100).collect::<String>()),
                                            Err(e) => format!("❌ Hata: {}", e),
                                        }
                                    }
                                };
                                let _ = tx.send(msg);
                            });
                            gui.pm_test_rx = Some(rx);
                            gui.pm_test_sonucu = "🔄 Test gönderildi, yanıt bekleniyor...".into();
                        }
                    }
                });
            });
        });
    });
}
