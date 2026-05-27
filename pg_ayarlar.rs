use eframe::egui;
use super::super::MonolithGui;
use crate::gui::theme::ACCENT_COLORS;

use crate::gui::lang;

pub fn page_ayarlar(gui: &mut MonolithGui, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("⚙").size(20.0).color(gui.t.accent));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(lang::t("Ayarlar", gui.dil_idx)).size(18.0).strong().color(gui.t.text));
            ui.label(egui::RichText::new(lang::t("Bot konfigürasyonu ve genel tercihler", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
        });
    });
    ui.add_space(16.0);

    // Satır 1: Güvenlik | Mola | Bildirimler
    ui.columns(3, |cols| {
        // Güvenlik
        gui.card_frame().show(&mut cols[0], |ui| {
            gui.section_title(ui, "🛡", &lang::t("GÜVENLİK & ANTİ-TESPİT", gui.dil_idx));
            ui.label(egui::RichText::new(lang::t("ⓘ Bu ayarlar bot tespitini zorlaştırmak için geliştirilmiştir.", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("Anti-Tespit Modu", gui.dil_idx)).size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.anti_tespit, &gui.t.clone()); });
            });
            ui.label(egui::RichText::new(lang::t("İnsan davranışını taklit eder", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("Rastgele Gecikme", gui.dil_idx)).size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.rastgele_gecikme, &gui.t.clone()); });
            });
            ui.label(egui::RichText::new(lang::t("Tıklamalar arasına rastgele ms ekler", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("İnsan Modu", gui.dil_idx)).size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.insan_modu, &gui.t.clone()); });
            });
            ui.label(egui::RichText::new(lang::t("Fare hareketi simülasyonu", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(lang::t("OBS Bypass", gui.dil_idx)).size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.obs_bypass, &gui.t.clone()); });
            });
            ui.label(egui::RichText::new(lang::t("Yayın programlarında botu gizler", gui.dil_idx)).size(9.0).color(gui.t.text_muted));
            ui.add_space(6.0);
            
            // Tıklama Modu seçici
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Tıklama Modu").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_source("tikla_modu_combo")
                        .width(120.0)
                        .selected_text(&gui.tikla_modu)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut gui.tikla_modu, "Hibrit".to_string(), "Hibrit");
                            ui.selectable_value(&mut gui.tikla_modu, "PostMessageW".to_string(), "PostMessageW");
                            ui.selectable_value(&mut gui.tikla_modu, "FocusSwap".to_string(), "FocusSwap");
                        });
                });
            });
            let modu_aciklama = match gui.tikla_modu.as_str() {
                "PostMessageW" => "Tam arka plan — 10+ client, paralel",
                "FocusSwap" => "Fiziksel input — 3-4 client, sıralı",
                _ => "PostMessageW + gerekince FocusSwap",
            };
            ui.label(egui::RichText::new(modu_aciklama).size(9.0).color(gui.t.text_muted));
            ui.add_space(6.0);
            
            ui.label(egui::RichText::new(lang::t("Gecikme (ms)", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Min").size(10.0).color(gui.t.text_muted));
                ui.add(egui::DragValue::new(&mut gui.gecikme_min).speed(1).clamp_range(50..=1000));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Max").size(10.0).color(gui.t.text_muted));
                ui.add(egui::DragValue::new(&mut gui.gecikme_max).speed(1).clamp_range(100..=2000));
            });
        });

        // Mola
        gui.card_frame().show(&mut cols[1], |ui| {
            gui.section_title(ui, "☕", "MOLA SİSTEMİ");
            ui.label(egui::RichText::new("Belirlenen çalışma süresi dolduğunda bot otomatik olarak duraklar ve mola süresi bitince tekrar başlar.").size(9.0).color(gui.t.text_muted));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Çalışılacak Süre").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{} dk", gui.mola_aralik)).size(11.0).color(gui.t.accent));
                });
            });
            ui.add(egui::Slider::new(&mut gui.mola_aralik, 10..=120).show_value(false));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Mola Süresi").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{} dk", gui.mola_sure)).size(11.0).color(gui.t.accent));
                });
            });
            ui.add(egui::Slider::new(&mut gui.mola_sure, 1..=30).show_value(false));
            ui.add_space(8.0);
            egui::Frame::none().fill(gui.t.bg3).rounding(egui::Rounding::same(6.0)).inner_margin(egui::Margin::same(10.0)).show(ui, |ui| {
                ui.label(egui::RichText::new("Sonraki Mola").size(10.0).color(gui.t.text_muted));
                ui.label(egui::RichText::new("--:--").size(20.0).strong().color(gui.t.accent).family(egui::FontFamily::Monospace));
            });
        });

        // Bildirimler
        gui.card_frame().show(&mut cols[2], |ui| {
            gui.section_title(ui, "🔔", "BİLDİRİMLER");
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Uygulama Bildirimleri").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.bildirim_uygulama, &gui.t.clone()); });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Ses Uyarıları").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.bildirim_ses, &gui.t.clone()); });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Telegram Bot").size(11.0).color(gui.t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { crate::gui::draw_toggle(ui, &mut gui.telegram_bot, &gui.t.clone()); });
            });
            ui.label(egui::RichText::new("Bot etkinliklerini Telegram'a gönderir").size(9.0).color(gui.t.text_muted));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Bot Token / Chat ID (Webhook)").size(10.0).color(gui.t.text_dim));
            ui.add(egui::TextEdit::singleline(&mut gui.telegram_webhook).hint_text("https://api.telegram.org/...").desired_width(ui.available_width()));
            ui.add_space(4.0);
            if ui.add(egui::Button::new(egui::RichText::new("Bağlantı Testi").size(11.0).color(egui::Color32::BLACK)).fill(gui.t.accent)).clicked() {
                if gui.telegram_webhook.is_empty() {
                    gui.test_sonucu = "URL boş olamaz!".into();
                } else if !gui.telegram_webhook.contains("api.telegram.org") {
                    gui.test_sonucu = "\u{274C} URL geçersiz! 'api.telegram.org' içermeli".into();
                } else {
                    // Fix #6: Gerçek test - thread'de gönder
                    let url = gui.telegram_webhook.clone();
                    std::thread::spawn(move || {
                        crate::send_telegram_notification(&url, "", "\u{1F916} K-BOT bağlantı testi \u{2705} ");
                    });
                    gui.test_sonucu = "Test gönderildi! Telegram'dan kontrol edin \u{1F4F1}".into();
                }
            }
            if !gui.test_sonucu.is_empty() {
                let col = if gui.test_sonucu.contains("başarılı") { gui.t.green } else { gui.t.red };
                ui.label(egui::RichText::new(&gui.test_sonucu).size(10.0).color(col));
            }
        });
    });
    ui.add_space(12.0);

    // Satır 2: Kısayollar | Görünüm | Lisans
    ui.columns(3, |cols| {
        // Kısayol Tuşları
        gui.card_frame().show(&mut cols[0], |ui| {
            gui.section_title(ui, "⌨", &lang::t("KISAYOL TUŞLARI", gui.dil_idx));
            for (label, val) in [
                (lang::t("Bot Başlat/Durdur", gui.dil_idx), &mut gui.kisayol_baslat),
                (lang::t("Log Temizle", gui.dil_idx), &mut gui.kisayol_log),
                (lang::t("Ekran Görüntüsü", gui.dil_idx), &mut gui.kisayol_ekran),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(label).size(11.0).color(gui.t.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::TextEdit::singleline(val).desired_width(60.0).font(egui::TextStyle::Monospace));
                    });
                });
                ui.add_space(4.0);
            }
        });

        // Görünüm
        gui.card_frame().show(&mut cols[1], |ui| {
            gui.section_title(ui, "🎨", &lang::t("GÖRÜNÜM AYARI", gui.dil_idx));
            ui.label(egui::RichText::new(lang::t("Tema Rengi", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
            ui.horizontal(|ui| {
                for (i, (r, g, b)) in ACCENT_COLORS.iter().enumerate() {
                    let col = egui::Color32::from_rgb(*r, *g, *b);
                    let size = egui::vec2(28.0, 28.0);
                    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                    let painter = ui.painter();
                    painter.circle_filled(rect.center(), 12.0, col);
                    if gui.tema_renk_idx == i {
                        painter.circle_stroke(rect.center(), 14.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
                        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "✓", egui::FontId::proportional(10.0), egui::Color32::WHITE);
                    }
                    if resp.clicked() {
                        gui.tema_renk_idx = i;
                        gui.t = crate::gui::theme::Theme::new(i);
                    }
                }
            });
        });

        // GUI Pencere Boyutu
        gui.card_frame().show(&mut cols[2], |ui| {
            gui.section_title(ui, "📏", &lang::t("GUI PENCERE BOYUTU", gui.dil_idx));
            ui.label(egui::RichText::new(lang::t("Hazır Şablonlar:", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
            ui.horizontal(|ui| {
                if ui.button("900x600").clicked() { gui.custom_w_str = "900".into(); gui.custom_h_str = "600".into(); }
                if ui.button("1024x768").clicked() { gui.custom_w_str = "1024".into(); gui.custom_h_str = "768".into(); }
                if ui.button("1200x800").clicked() { gui.custom_w_str = "1200".into(); gui.custom_h_str = "800".into(); }
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(lang::t("Özel Boyut:", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
            ui.horizontal(|ui| {
                ui.label(lang::t("Genişlik:", gui.dil_idx));
                ui.add(egui::TextEdit::singleline(&mut gui.custom_w_str).desired_width(50.0));
                ui.label(lang::t("Yükseklik:", gui.dil_idx));
                ui.add(egui::TextEdit::singleline(&mut gui.custom_h_str).desired_width(50.0));
            });
            ui.add_space(6.0);
            if gui.dim_btn(ui, &lang::t("Uygula", gui.dil_idx)) {
                if let (Ok(w), Ok(h)) = (gui.custom_w_str.parse::<f32>(), gui.custom_h_str.parse::<f32>()) {
                    let mut cfg = crate::config::AppConfig::load();
                    cfg.gui_width = w;
                    cfg.gui_height = h;
                    cfg.save();
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
                }
            }
        });
    });
    ui.add_space(12.0);

    // Kaydet
    if gui.accent_btn(ui, &lang::t("💾 Ayarları Kaydet", gui.dil_idx)) {
        let mut cfg = crate::config::AppConfig::load();
        cfg.anti_tespit_modu = gui.anti_tespit;
        cfg.rastgele_gecikme = gui.rastgele_gecikme;
        cfg.insan_modu = gui.insan_modu;
        cfg.obs_bypass = gui.obs_bypass;
        cfg.gecikme_min = gui.gecikme_min;
        cfg.gecikme_max = gui.gecikme_max;
        cfg.mola_aralik_dk = gui.mola_aralik;
        cfg.mola_sure_dk = gui.mola_sure;
        cfg.bildirim_uygulama = gui.bildirim_uygulama;
        cfg.bildirim_ses = gui.bildirim_ses;
        cfg.telegram_bot = gui.telegram_bot;
        cfg.telegram_webhook_url = gui.telegram_webhook.clone();
        cfg.kisayol_baslat_durdur = gui.kisayol_baslat.clone();
        cfg.kisayol_log_temizle = gui.kisayol_log.clone();
        cfg.kisayol_ekran_goruntusu = gui.kisayol_ekran.clone();
        cfg.tema_renk_idx = gui.tema_renk_idx;
        cfg.dil = if gui.dil_idx == 1 { "en".into() } else { "tr".into() };
        cfg.toplama_aktif = gui.toplama_aktif;
        cfg.toplama_tusu = gui.toplama_tusu.clone();
        cfg.tikla_modu = gui.tikla_modu.clone();
        cfg.save();
    }
}
