use eframe::egui;
use super::super::{MonolithGui, Screen};
use crate::gui::lang;

pub fn render_login(gui: &mut MonolithGui, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(gui.t.bg0))
        .show(ctx, |ui| {
            let panel_w = 310.0;
            let avail = ui.available_size();
            let x_off = (avail.x - panel_w) / 2.0;
            let y_off = avail.y * 0.05;

            ui.allocate_ui_at_rect(
                egui::Rect::from_min_size(egui::pos2(x_off, y_off), egui::vec2(panel_w, 450.0)),
                |ui| {
                    ui.vertical_centered(|ui| {
                        // Logo
                        ui.label(egui::RichText::new("⬡").size(48.0).color(gui.t.accent));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("K-BOT").size(28.0).strong().color(gui.t.accent));
                        ui.label(egui::RichText::new("OTOMASYON SİSTEMİ — V2.4.1").size(11.0).color(gui.t.text_dim));
                        ui.add_space(24.0);
                    });

                    // Login card
                    egui::Frame::none()
                        .fill(gui.t.bg1)
                        .rounding(egui::Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(28.0))
                        .stroke(egui::Stroke::new(1.0, gui.t.border))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⊙").size(14.0).color(gui.t.accent));
                                ui.label(egui::RichText::new(lang::t("HESABINIZA GİRİŞ YAPIN", gui.dil_idx)).size(12.0).strong().color(gui.t.text));
                            });
                            ui.add_space(16.0);

                            // E-Posta
                            ui.label(egui::RichText::new("E-POSTA").size(10.0).color(gui.t.text_dim));
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(gui.t.bg2)
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::same(10.0))
                                .stroke(egui::Stroke::new(1.0, gui.t.border))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("✉").size(14.0).color(gui.t.text_dim));
                                        ui.add(egui::TextEdit::singleline(&mut gui.login_email)
                                            .hint_text("ornek@mail.com")
                                            .desired_width(260.0)
                                            .frame(false));
                                    });
                                });
                            ui.add_space(12.0);

                            // Şifre
                            ui.label(egui::RichText::new(lang::t("ŞİFRE", gui.dil_idx)).size(10.0).color(gui.t.text_dim));
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(gui.t.bg2)
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::same(10.0))
                                .stroke(egui::Stroke::new(1.0, gui.t.border))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("🔒").size(14.0).color(gui.t.text_dim));
                                        let pe = egui::TextEdit::singleline(&mut gui.login_pass)
                                            .hint_text("••••••••")
                                            .password(!gui.login_show_pass)
                                            .desired_width(230.0)
                                            .frame(false);
                                        ui.add(pe);
                                        if ui.add(egui::Button::new(
                                            egui::RichText::new(if gui.login_show_pass {"👁"} else {"👁‍🗨"}).size(14.0).color(gui.t.text_dim))
                                            .frame(false)).clicked() {
                                            gui.login_show_pass = !gui.login_show_pass;
                                        }
                                    });
                                });
                            ui.add_space(8.0);
                            
                            // Beni Hatırla
                            ui.horizontal(|ui| {
                                crate::gui::draw_toggle(ui, &mut gui.remember_me, &gui.t.clone());
                                ui.label(egui::RichText::new(lang::t("Beni Hatırla", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
                            });
                            ui.add_space(16.0);

                            // Hata mesajı
                            if !gui.login_error.is_empty() {
                                ui.label(egui::RichText::new(&gui.login_error).size(11.0).color(gui.t.red));
                                ui.add_space(8.0);
                            }

                            // Giriş Yap butonu
                            let btn = ui.add(egui::Button::new(
                                egui::RichText::new(lang::t("⊙  GİRİŞ YAP", gui.dil_idx)).size(14.0).strong().color(egui::Color32::BLACK))
                                .fill(gui.t.accent)
                                .rounding(egui::Rounding::same(8.0))
                                .min_size(egui::vec2(ui.available_width(), 42.0)));
                                
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if btn.clicked() || enter_pressed {
                                if gui.login_email.is_empty() || gui.login_pass.is_empty() {
                                    gui.login_error = lang::t("Lütfen tüm alanları doldurun.", gui.dil_idx);
                                } else if gui.login_email == "admin" && gui.login_pass == "admin" {
                                    gui.login_error.clear();
                                    gui.screen = Screen::Main;
                                    let cfg = crate::config::AppConfig::load();
                                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(cfg.gui_width, cfg.gui_height)));
                                    
                                    // Save to config
                                    let mut cfg = crate::config::AppConfig::load();
                                    cfg.remember_me = gui.remember_me;
                                    if gui.remember_me {
                                        cfg.saved_email = gui.login_email.clone();
                                        cfg.saved_pass = gui.login_pass.clone();
                                    } else {
                                        cfg.saved_email.clear();
                                        cfg.saved_pass.clear();
                                    }
                                    cfg.save();
                                } else {
                                    gui.login_error = lang::t("Geçersiz kullanıcı adı veya şifre.", gui.dil_idx);
                                }
                            }
                        });

                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 170.0) / 2.0);
                            ui.label(egui::RichText::new(lang::t("Hesabınız yok mu?", gui.dil_idx)).size(11.0).color(gui.t.text_dim));
                            ui.label(egui::RichText::new(lang::t("Lisans satın al", gui.dil_idx)).size(11.0).color(gui.t.accent).underline());
                        });
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 170.0) / 2.0);
                            let tr_sel = gui.dil_idx == 0;
                            let en_sel = gui.dil_idx == 1;
                            let tr_bg = if tr_sel { gui.t.accent } else { gui.t.bg3 };
                            let en_bg = if en_sel { gui.t.accent } else { gui.t.bg3 };
                            let tr_tc = if tr_sel { egui::Color32::BLACK } else { gui.t.text };
                            let en_tc = if en_sel { egui::Color32::BLACK } else { gui.t.text };
                            if ui.add(egui::Button::new(egui::RichText::new("Türkçe").size(12.0).color(tr_tc)).fill(tr_bg).rounding(egui::Rounding::same(6.0)).min_size(egui::vec2(80.0, 28.0))).clicked() { 
                                gui.dil_idx = 0;
                                let mut cfg = crate::config::AppConfig::load();
                                cfg.dil = "tr".to_string();
                                cfg.save();
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("English").size(12.0).color(en_tc)).fill(en_bg).rounding(egui::Rounding::same(6.0)).min_size(egui::vec2(80.0, 28.0))).clicked() { 
                                gui.dil_idx = 1;
                                let mut cfg = crate::config::AppConfig::load();
                                cfg.dil = "en".to_string();
                                cfg.save();
                            }
                        });
                    });
                },
            );
        });
}
