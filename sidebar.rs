use eframe::egui;
use super::super::MonolithGui;
use crate::gui::lang;

pub fn render_sidebar(gui: &mut MonolithGui, ctx: &egui::Context) {
    egui::SidePanel::left("sidebar").exact_width(170.0)
        .frame(egui::Frame::none().fill(gui.t.bg1).inner_margin(egui::Margin::same(10.0)))
        .show(ctx, |ui| {
            // Logo
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⬡").size(24.0).color(gui.t.accent));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("K-BOT").size(16.0).strong().color(gui.t.accent));
                        ui.label(egui::RichText::new("METİN2 PANEL").size(8.0).color(gui.t.text_dim));
                    });
                });
                ui.add_space(8.0);
            });

            // Bot durumu + Başlat
            egui::Frame::none()
                .fill(gui.t.bg2)
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let on = gui.current_stats.is_running;
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(lang::t("BOT DURUMU", gui.dil_idx)).size(8.0).color(gui.t.text_dim));
                            let (dot_col, status) = if on { (gui.t.green, lang::t("Aktif", gui.dil_idx)) } else { (gui.t.red, lang::t("Durum: Durduruldu", gui.dil_idx).replace("Durum: ", "")) };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 3.0, dot_col);
                            ui.label(egui::RichText::new(status).size(8.0).color(dot_col));
                        });
                        ui.add_space(4.0);
                        let (col, txt) = if on { (gui.t.red, lang::t("⏹  DURDUR", gui.dil_idx)) } else { (gui.t.accent, lang::t("▶  BAŞLAT", gui.dil_idx)) };
                        let btn = ui.add(egui::Button::new(egui::RichText::new(txt).size(13.0).strong().color(if on { egui::Color32::WHITE } else { egui::Color32::BLACK }))
                            .fill(col).rounding(egui::Rounding::same(6.0)).min_size(egui::vec2(140.0, 32.0)));
                        if btn.clicked() {
                            if on {
                                let _ = gui.cmd_tx.send("STOP".into());
                            } else {
                                // Find the first active client with a valid HWND
                                if let Some(c) = gui.clients.iter().find(|c| c.active && !c.hwnd.is_empty() && c.hwnd != "0") {
                                    let _ = gui.cmd_tx.send(format!("START:{}", c.hwnd));
                                    // Make sure model and driver matches the client's setting
                                    let _ = gui.cmd_tx.send(format!("MODEL:{}", c.model));
                                    let _ = gui.cmd_tx.send(format!("DRIVER:{}", c.driver));
                                }
                            }
                        }
                    });
                });
            ui.add_space(8.0);

            // Menü
            let items = [
                ("\u{1F4CA}", "Genel Bakış"),
                ("\u{1F33E}", "Farming"),
                ("\u{26A1}", "Yetenekler"),
                ("\u{1F4E6}", "Eşyalar"),
                ("\u{1F510}", "Captcha"),
                ("\u{1F441}", "OCR"),
                ("\u{1F5A5}", "Çoklu Client"),
                ("\u{2699}", "Ayarlar"),
                ("\u{1F4CB}", "Kayıtlar"),
                ("\u{1F4AC}", "Auto-PM AI"),
            ];
            for (i, (ico, lbl)) in items.iter().enumerate() {
                let sel = gui.nav == i;
                let bg = if sel { gui.t.accent } else { egui::Color32::TRANSPARENT };
                let tc = if sel { egui::Color32::BLACK } else { gui.t.text_dim };
                if ui.add(egui::Button::new(egui::RichText::new(format!("{}  {}", ico, lang::t(lbl, gui.dil_idx))).size(12.0).color(tc))
                    .fill(bg).rounding(egui::Rounding::same(8.0)).min_size(egui::vec2(150.0, 30.0))).clicked() {
                    gui.nav = i;
                }
                ui.add_space(1.0);
            }

            // Alt kısım
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                if ui.add(egui::Button::new(egui::RichText::new(lang::t("↪ Çıkış Yap", gui.dil_idx)).size(11.0).color(gui.t.text_dim))
                    .frame(false)).clicked() {
                    gui.screen = super::super::Screen::Login;
                    gui.login_email.clear();
                    gui.login_pass.clear();
                }
                if ui.add(egui::Button::new(egui::RichText::new(lang::t("‹ Küçült", gui.dil_idx)).size(11.0).color(gui.t.text_dim))
                    .frame(false)).clicked() {
                    gui.screen = super::super::Screen::Mini;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(250.0, 120.0)));
                }
            });
        });
}
