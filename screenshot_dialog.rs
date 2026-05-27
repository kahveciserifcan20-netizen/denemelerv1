//! Screenshot Selection Dialog - Oyun penceresinden anlık görüntü alıp seçim yapma

use eframe::egui;
use crate::gui::{MonolithGui, SelectionType};

/// Modal screenshot seçim penceresini çiz
pub fn render_screenshot_dialog(gui: &mut MonolithGui, ctx: &egui::Context) {
    if !gui.screenshot_window_open {
        return;
    }

    let type_name = match gui.screenshot_selection_type {
        SelectionType::SearchRegion   => "Arama Bolgesi",
        SelectionType::OcrRegion      => "OCR Bolgesi",
        SelectionType::CaptchaButton  => "Captcha Buton Bolgesi",
        SelectionType::PmRegion       => "PM Tarama Bolgesi",
        SelectionType::PmSimgeRegion  => "PM Simge Bolgesi",
    };

    let window_title = format!("{} - Surukle-birak ile secin", type_name);

    egui::Window::new(window_title)
        .resizable(true)
        .min_width(400.0)
        .min_height(300.0)
        .collapsible(false)
        .show(ctx, |ui| {
            // Görüntü yoksa uyarı göster
            let Some(ref image) = gui.screenshot_image else {
                ui.label(egui::RichText::new("Goruntu yuklenemedi!").color(egui::Color32::RED));
                if ui.button("Kapat").clicked() {
                    gui.screenshot_window_open = false;
                }
                return;
            };

            // Texture oluştur (ilk seferde)
            let texture_id = match &gui.screenshot_texture {
                Some(tex) => tex.id(),
                None => {
                    let tex = ctx.load_texture(
                        "screenshot_selection",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw()
                        ),
                        egui::TextureOptions::LINEAR
                    );
                    let id = tex.id();
                    gui.screenshot_texture = Some(tex);
                    id
                }
            };

            // Görüntü boyutları
            let img_w = image.width() as f32;
            let img_h = image.height() as f32;
            let aspect = img_h / img_w;

            // Görüntüyü göster ve etkileşimli yap
            let available_w = ui.available_width().min(800.0);
            let display_size = egui::vec2(available_w, available_w * aspect);

            let (rect, response) = ui.allocate_exact_size(display_size, egui::Sense::drag());

            // Görüntüyü çiz
            ui.painter().image(
                texture_id,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE
            );

            // Normalize koordinatları piksel koordinatlarına çevir
            let to_pixel = |pos: egui::Pos2| -> (i32, i32) {
                let x = ((pos.x - rect.left()) / rect.width() * img_w) as i32;
                let y = ((pos.y - rect.top()) / rect.height() * img_h) as i32;
                (x.max(0).min(image.width() as i32), y.max(0).min(image.height() as i32))
            };

            // Sürükleme işlemi
            if response.drag_started() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (x, y) = to_pixel(pos);
                    gui.screenshot_selection_start = Some((x as f32 / img_w, y as f32 / img_h));
                    gui.screenshot_selection_end = None;
                }
            }

            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (x, y) = to_pixel(pos);
                    gui.screenshot_selection_end = Some((x as f32 / img_w, y as f32 / img_h));
                }
            }

            if response.drag_stopped() {
                // Seçim tamamlandı - koordinatları kaydet
                if let (Some(start), Some(end)) = (gui.screenshot_selection_start, gui.screenshot_selection_end) {
                    let x1 = (start.0.min(end.0) * img_w) as i32;
                    let y1 = (start.1.min(end.1) * img_h) as i32;
                    let x2 = (start.0.max(end.0) * img_w) as i32;
                    let y2 = (start.1.max(end.1) * img_h) as i32;

                    let client_idx = gui.screenshot_selection_client_idx;

                    match gui.screenshot_selection_type {
                        SelectionType::PmRegion => {
                            // PM bölgesi global — client listesine bağlı değil
                            gui.pm_region_x1 = x1;
                            gui.pm_region_y1 = y1;
                            gui.pm_region_x2 = x2;
                            gui.pm_region_y2 = y2;
                            gui.pm_region_mesaj = format!(
                                "PM bolgesi kaydedildi: ({},{}) - ({},{})", x1, y1, x2, y2
                            );
                            let mut cfg = crate::config::AppConfig::load();
                            cfg.pm_region_x1 = x1; cfg.pm_region_y1 = y1;
                            cfg.pm_region_x2 = x2; cfg.pm_region_y2 = y2;
                            cfg.save();
                            let ts = chrono::Local::now().format("%H:%M:%S");
                            gui.logs.push_back(format!(
                                "[{}] PM bolge kaydedildi: ({},{}) - ({},{})", ts, x1, y1, x2, y2
                            ));
                        }
                        SelectionType::PmSimgeRegion => {
                            // PM simge bölgesi global — pm_simge.png arama alanı
                            gui.pm_simge_region_x1 = x1;
                            gui.pm_simge_region_y1 = y1;
                            gui.pm_simge_region_x2 = x2;
                            gui.pm_simge_region_y2 = y2;
                            gui.pm_region_mesaj = format!(
                                "PM simge bolgesi kaydedildi: ({},{}) - ({},{})", x1, y1, x2, y2
                            );
                            let mut cfg = crate::config::AppConfig::load();
                            cfg.pm_simge_x1 = x1; cfg.pm_simge_y1 = y1;
                            cfg.pm_simge_x2 = x2; cfg.pm_simge_y2 = y2;
                            cfg.save();
                            let ts = chrono::Local::now().format("%H:%M:%S");
                            gui.logs.push_back(format!(
                                "[{}] PM simge bolge kaydedildi: ({},{}) - ({},{})", ts, x1, y1, x2, y2
                            ));
                        }
                        _ => {
                            // Client-spesifik bölgeler
                            if client_idx < gui.clients.len() {
                                match gui.screenshot_selection_type {
                                    SelectionType::SearchRegion => {
                                        gui.clients[client_idx].search_x1 = x1;
                                        gui.clients[client_idx].search_y1 = y1;
                                        gui.clients[client_idx].search_x2 = x2;
                                        gui.clients[client_idx].search_y2 = y2;
                                    }
                                    SelectionType::OcrRegion => {
                                        gui.clients[client_idx].ocr_x1 = x1;
                                        gui.clients[client_idx].ocr_y1 = y1;
                                        gui.clients[client_idx].ocr_x2 = x2;
                                        gui.clients[client_idx].ocr_y2 = y2;
                                    }
                                    SelectionType::CaptchaButton => {
                                        gui.clients[client_idx].captcha_buton_x1 = x1;
                                        gui.clients[client_idx].captcha_buton_y1 = y1;
                                        gui.clients[client_idx].captcha_buton_x2 = x2;
                                        gui.clients[client_idx].captcha_buton_y2 = y2;
                                    }
                                    SelectionType::PmRegion => unreachable!(),
                                    SelectionType::PmSimgeRegion => unreachable!(),
                                }
                                let ts = chrono::Local::now().format("%H:%M:%S");
                                gui.logs.push_back(format!(
                                    "[{}] {} kaydedildi: ({},{}) - ({},{})",
                                    ts, type_name, x1, y1, x2, y2
                                ));
                            }
                        }
                    }

                    // Pencereyi kapat
                    gui.screenshot_window_open = false;
                    gui.screenshot_selection_active = false;
                    gui.screenshot_texture = None;
                }
            }

            // Seçim dikdörtgenini çiz (sürüklerken)
            if let (Some(start), Some(end)) = (gui.screenshot_selection_start, gui.screenshot_selection_end) {
                let x1 = rect.left() + start.0 * rect.width();
                let y1 = rect.top() + start.1 * rect.height();
                let x2 = rect.left() + end.0 * rect.width();
                let y2 = rect.top() + end.1 * rect.height();

                let sel_rect = egui::Rect::from_min_max(
                    egui::pos2(x1.min(x2), y1.min(y2)),
                    egui::pos2(x1.max(x2), y1.max(y2))
                );

                // Yarı saydam dolgu
                ui.painter().rect_filled(sel_rect, 0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 200, 100, 50));

                // Kenarlık
                ui.painter().rect_stroke(sel_rect, 0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100)));
            }

            // Bilgi metni
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Goruntu uzerinde surukle-birak ile bolge secin").size(12.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Iptal").clicked() {
                        gui.screenshot_window_open = false;
                        gui.screenshot_selection_active = false;
                        gui.screenshot_texture = None;
                        gui.screenshot_selection_start = None;
                        gui.screenshot_selection_end = None;
                    }
                });
            });

            // Koordinat gösterimi
            if let (Some(start), Some(end)) = (gui.screenshot_selection_start, gui.screenshot_selection_end) {
                let x1 = (start.0.min(end.0) * img_w) as i32;
                let y1 = (start.1.min(end.1) * img_h) as i32;
                let x2 = (start.0.max(end.0) * img_w) as i32;
                let y2 = (start.1.max(end.1) * img_h) as i32;

                ui.label(egui::RichText::new(
                    format!("Secim: ({},{}) - ({},{})", x1, y1, x2, y2)
                ).size(11.0).color(egui::Color32::YELLOW));
            }
        });
}