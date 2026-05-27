#![windows_subsystem = "windows"]

mod config;
mod gui;
mod captcha_solver;
mod vision_manager;
mod hw_simulator;
mod remote_server;
mod online_learner;
mod client_runner;
mod vk_codes;
mod client_params;
mod preset_manager;
mod stats_dashboard;
mod pm_ai;

use crossbeam_channel::{unbounded, Sender};
use std::thread;
use std::collections::HashMap;
use eframe::egui;
use std::time::Duration;
use chrono::Local;

// ── İstatistik yapısı (GUI'ye channel ile gönderilir) ─────────────────────
#[derive(Clone, Debug)]
pub struct BotStats {
    pub stones_mined: u32,
    pub stones_missed: u32,
    pub captcha_solved: u32,
    pub uptime_secs: f64,
    pub state_name: String,
    pub is_running: bool,
    pub pending_feedback: bool,
    pub last_detection_conf: f32,
    pub learning_samples: usize,
    pub learning_positive: usize,
    pub learning_negative: usize,
}

fn send_log_main(tx: &Sender<String>, msg: &str) {
    let ts = Local::now().format("%H:%M:%S.%3f");
    let _ = tx.send(format!("[{}] {}", ts, msg));
}

// ── Telegram Bildirim Fonksiyonu ─────────────────────────────────────────────
pub fn send_telegram_notification(webhook_url: &str, chat_id: &str, message: &str) -> bool {
    if webhook_url.is_empty() { return false; }
    let client = reqwest::blocking::Client::new();
    let url = if webhook_url.contains("/sendMessage") {
        webhook_url.to_string()
    } else {
        format!("{}/sendMessage", webhook_url.trim_end_matches('/'))
    };
    let body = if chat_id.is_empty() {
        serde_json::json!({ "text": message })
    } else {
        serde_json::json!({ "chat_id": chat_id, "text": message })
    };
    match client.post(&url).json(&body).send() {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false
    }
}

// ── Client Thread Yönetimi ──────────────────────────────────────────────────
struct ClientHandle {
    cmd_tx: Sender<String>,
    #[allow(dead_code)]
    thread: Option<thread::JoinHandle<()>>,
    // Crash Recovery: Son başlatma parametreleri
    last_params: Option<client_params::ClientStartParams>,
    restart_count: u32,
    last_heartbeat: std::time::Instant,
}

impl ClientHandle {
    fn new(cmd_tx: Sender<String>, thread: Option<thread::JoinHandle<()>>, params: client_params::ClientStartParams) -> Self {
        Self {
            cmd_tx,
            thread,
            last_params: Some(params),
            restart_count: 0,
            last_heartbeat: std::time::Instant::now(),
        }
    }
    
    fn update_heartbeat(&mut self) {
        self.last_heartbeat = std::time::Instant::now();
    }
    
    fn is_alive(&self) -> bool {
        self.last_heartbeat.elapsed().as_secs() < 10 // 10 saniye içinde heartbeat gelmeli
    }
    
    fn should_restart(&self) -> bool {
        !self.is_alive() && self.restart_count < 3 // Max 3 restart
    }
    
    fn increment_restart(&mut self) {
        self.restart_count += 1;
    }
}

fn main() -> Result<(), eframe::Error> {
    let (log_tx, log_rx) = unbounded::<String>();
    let (hb_tx, hb_rx) = unbounded::<(usize, std::time::Instant)>(); // Crash Recovery: Heartbeat kanalı
    let (cmd_tx, cmd_rx) = unbounded::<String>();
    // Çoklu client: frame ve stats tagged with client_id
    let (frame_tx, frame_rx) = unbounded::<(usize, image::RgbaImage)>();
    let (stats_tx, stats_rx) = unbounded::<(usize, BotStats)>();
    let (radar_tx, radar_rx) = unbounded::<client_runner::RadarData>();
    let (ctx_tx, ctx_rx) = unbounded::<egui::Context>();

    let log_tx_backend = log_tx.clone();

    // ── MQTT Remote Server Başlat ──────────────────────────────────────────────
    let mut mqtt_server = remote_server::RemoteServer::new(cmd_tx.clone());
    mqtt_server.start();
    println!("[MAIN] MQTT Remote Server başlatıldı!");

    // ── Client Manager Thread ──────────────────────────────────────────────────
    thread::spawn(move || {
        // Context'i bekle - GUI hazır olana kadar
        let ctx = match ctx_rx.recv() {
            Ok(c) => c,
            Err(_) => {
                send_log_main(&log_tx_backend, "❌ GUI Context alınamadı! Thread sonlandırılıyor.");
                return;
            }
        };
        send_log_main(&log_tx_backend, "✅ GUI Context alındı, Client Manager hazır.");
        
        let mut client_handles: HashMap<usize, ClientHandle> = HashMap::new();
        let mut preview_hwnd: usize = 0;

        // Eski tek-client uyumluluk: Canlı preview için VisionEngine
        let mut preview_vision = vision_manager::VisionEngine::new(log_tx_backend.clone());
        let mut preview_active_hwnd: usize = 0;



        send_log_main(&log_tx_backend, "🎯 Client Manager başlatıldı. Sınırsız client desteği aktif!");

        loop {
            // ── Komut işleme ──
            while let Ok(cmd) = cmd_rx.try_recv() {
                if cmd.starts_with("START_CLIENT_JSON:") {
                    // ── YENİ: Struct-based protocol ──
                    let json_str = cmd.replace("START_CLIENT_JSON:", "");
                    match serde_json::from_str::<client_params::ClientStartParams>(&json_str) {
                        Ok(params) => {
                            send_log_main(&log_tx_backend, &format!(
                                "🔧 START_CLIENT (JSON) id={}, HWND={}, Model={}, Driver={}",
                                params.id, params.hwnd, params.model, params.driver
                            ));

                            if params.hwnd == 0 {
                                send_log_main(&log_tx_backend, &format!("❌ Client {} HWND=0, başlatılamadı!", params.id));
                                continue;
                            }
                            if params.kilit_path.is_empty() || params.kilit_path == "0" {
                                send_log_main(&log_tx_backend, &format!("❌ Client {} kilit_path boş, başlatılamadı!", params.id));
                                continue;
                            }

                            // Mevcut client varsa durdur
                            if let Some(old) = client_handles.remove(&params.id) {
                                let _ = old.cmd_tx.send("STOP".into());
                            }

                            // Yeni client thread spawn
                            let (client_cmd_tx, client_cmd_rx) = unbounded::<String>();
                            let log_tx_c = log_tx_backend.clone();
                            let stats_tx_c = stats_tx.clone();
                            let frame_tx_c = frame_tx.clone();
                            let radar_tx_c = radar_tx.clone();
                            let ctx_c = ctx.clone();
                            let id = params.id;

                            send_log_main(&log_tx_backend, &format!(
                                "🚀 Client {} başlatılıyor... HWND:{} Model:{} Driver:{}",
                                id, params.hwnd, params.model, params.driver
                            ));

                            let hb_tx_c = hb_tx.clone();
                            let params_for_thread = params.clone();
                            let handle = thread::spawn(move || {
                                client_runner::run_client(
                                    params_for_thread.id, params_for_thread.hwnd, params_for_thread.model, params_for_thread.driver,
                                    params_for_thread.kilit_path, params_for_thread.kilit_region, params_for_thread.ocr_region,
                                    log_tx_c, client_cmd_rx, stats_tx_c, frame_tx_c, radar_tx_c, ctx_c,
                                    params_for_thread.olum_skill_aktif, params_for_thread.olum_skill_tuslari, params_for_thread.olum_skill_bekleme,
                                    params_for_thread.olum_binek_aktif, params_for_thread.olum_binek_tusu, params_for_thread.olum_binek_bekleme,
                                    params_for_thread.captcha_buton_x1, params_for_thread.captcha_buton_y1,
                                    params_for_thread.captcha_buton_x2, params_for_thread.captcha_buton_y2,
                                    hb_tx_c, // Heartbeat kanalı
                                    // Auto-PM AI parametreleri
                                    params_for_thread.pm_ai_aktif,
                                    params_for_thread.pm_ai_backend.clone(),
                                    params_for_thread.pm_ai_api_key.clone(),
                                    params_for_thread.pm_system_prompt.clone(),
                                    params_for_thread.pm_region,
                                    params_for_thread.pm_cooldown_sn,
                                    params_for_thread.pm_daily_limit,
                                );
                            });

                            client_handles.insert(id, ClientHandle::new(client_cmd_tx, Some(handle), params));
                        }
                        Err(e) => {
                            send_log_main(&log_tx_backend, &format!("❌ START_CLIENT JSON parse hatası: {}", e));
                        }
                    }
                } else if cmd.starts_with("START_CLIENT:") {
                    // ── ESKİ: String-based protocol (geriye dönük uyumluluk) ──
                    let cleaned = cmd.replace("START_CLIENT:", "");
                    let parts: Vec<&str> = cleaned.split(':').collect();
                    
                    send_log_main(&log_tx_backend, &format!("🔧 START_CLIENT komutu alındı (eski format), parts: {}", parts.len()));
                    
                    if parts.len() >= 5 {
                        let id: usize = parts[0].parse().unwrap_or(0);
                        let hwnd: usize = parts[1].parse().unwrap_or(0);
                        let model = parts[2].to_string();
                        let driver = parts[3].to_string();
                        let kilit_path = parts[4].to_string();
                        let kilit_region = if parts.len() >= 6 {
                            let coords: Vec<&str> = parts[5].split(',').collect();
                            if coords.len() == 4 {
                                (coords[0].parse().unwrap_or(200), coords[1].parse().unwrap_or(30),
                                 coords[2].parse().unwrap_or(600), coords[3].parse().unwrap_or(80))
                            } else { (200, 30, 600, 80) }
                        } else { (200, 30, 600, 80) };
                        let ocr_region = if parts.len() >= 7 {
                            let coords: Vec<&str> = parts[6].split(',').collect();
                            if coords.len() == 4 {
                                (coords[0].parse().unwrap_or(313), coords[1].parse().unwrap_or(153),
                                 coords[2].parse().unwrap_or(457), coords[3].parse().unwrap_or(168))
                            } else { (313, 153, 457, 168) }
                        } else { (313, 153, 457, 168) };
                        
                        let olum_skill_aktif: bool = parts.get(7).map(|v| *v == "1" || v.to_lowercase() == "true").unwrap_or(false);
                        let olum_skill_tuslari: Vec<String> = parts.get(8).map(|v| {
                            if v.is_empty() || *v == "EMPTY" {
                                vec![]
                            } else {
                                v.split(',').map(|s| s.to_string()).collect()
                            }
                        }).unwrap_or_default();
                        let olum_skill_bekleme: u64 = parts.get(9).and_then(|v| v.parse().ok()).unwrap_or(3);
                        let olum_binek_aktif: bool = parts.get(10).map(|v| *v == "1" || v.to_lowercase() == "true").unwrap_or(false);
                        let olum_binek_tusu: String = parts.get(11).unwrap_or(&"").to_string();
                        let olum_binek_bekleme: u64 = parts.get(12).and_then(|v| v.parse().ok()).unwrap_or(5);
                        let (captcha_buton_x1, captcha_buton_y1, captcha_buton_x2, captcha_buton_y2) = if parts.len() >= 14 {
                            let coords: Vec<&str> = parts[13].split(',').collect();
                            if coords.len() == 4 {
                                (coords[0].parse().unwrap_or(0), coords[1].parse().unwrap_or(0),
                                 coords[2].parse().unwrap_or(0), coords[3].parse().unwrap_or(0))
                            } else { (0, 0, 0, 0) }
                        } else { (0, 0, 0, 0) };

                        if hwnd == 0 {
                            send_log_main(&log_tx_backend, &format!("❌ Client {} HWND=0, başlatılamadı!", id));
                            continue;
                        }
                        if kilit_path.is_empty() || kilit_path == "0" {
                            send_log_main(&log_tx_backend, &format!("❌ Client {} kilit_path boş, başlatılamadı!", id));
                            continue;
                        }

                        if let Some(old) = client_handles.remove(&id) {
                            let _ = old.cmd_tx.send("STOP".into());
                        }

                        let (client_cmd_tx, client_cmd_rx) = unbounded::<String>();
                        let log_tx_c = log_tx_backend.clone();
                        let stats_tx_c = stats_tx.clone();
                        let frame_tx_c = frame_tx.clone();
                        let radar_tx_c = radar_tx.clone();
                        let ctx_c = ctx.clone();

                        send_log_main(&log_tx_backend, &format!(
                            "🚀 Client {} başlatılıyor... HWND:{} Model:{} Driver:{}",
                            id, hwnd, model, driver
                        ));

                        // Clone değerleri thread için
                        let model_c = model.clone();
                        let driver_c = driver.clone();
                        let kilit_path_c = kilit_path.clone();
                        let olum_skill_tuslari_c = olum_skill_tuslari.clone();
                        let olum_binek_tusu_c = olum_binek_tusu.clone();
                        let hb_tx_c = hb_tx.clone();
                        
                        // Eski format için params oluştur (PM AI varsayılan değerlerle)
                        let params = client_params::ClientStartParams {
                            id, hwnd, model: model_c.clone(), driver: driver_c.clone(),
                            kilit_path: kilit_path_c.clone(), kilit_region, ocr_region,
                            olum_skill_aktif, olum_skill_tuslari: olum_skill_tuslari_c.clone(),
                            olum_skill_bekleme, olum_binek_aktif, olum_binek_tusu: olum_binek_tusu_c.clone(),
                            olum_binek_bekleme, captcha_buton_x1, captcha_buton_y1,
                            captcha_buton_x2, captcha_buton_y2,
                            // Auto-PM AI varsayılan değerler (eski format desteği)
                            pm_ai_aktif: false,
                            pm_ai_backend: "gemini".to_string(),
                            pm_ai_api_key: String::new(),
                            pm_system_prompt: "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string(),
                            pm_region: (0, 0, 0, 0),
                            pm_cooldown_sn: 30,
                            pm_daily_limit: 200,
                        };
                        
                        let handle = thread::spawn(move || {
                            client_runner::run_client(
                                id, hwnd, model_c, driver_c, kilit_path_c, kilit_region, ocr_region,
                                log_tx_c, client_cmd_rx, stats_tx_c, frame_tx_c, radar_tx_c, ctx_c,
                                olum_skill_aktif, olum_skill_tuslari_c, olum_skill_bekleme,
                                olum_binek_aktif, olum_binek_tusu_c, olum_binek_bekleme,
                                captcha_buton_x1, captcha_buton_y1, captcha_buton_x2, captcha_buton_y2,
                                hb_tx_c, // Heartbeat kanalı
                                // Auto-PM AI parametreleri (varsayılan - eski format)
                                false, // pm_ai_aktif
                                "gemini".to_string(), // pm_ai_backend
                                String::new(), // pm_ai_api_key
                                "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string(), // pm_system_prompt
                                (0, 0, 0, 0), // pm_region
                                30, // pm_cooldown_sn
                                200, // pm_daily_limit
                            );
                        });

                        client_handles.insert(id, ClientHandle::new(client_cmd_tx, Some(handle), params));
                    }
                } else if cmd.starts_with("STOP_CLIENT:") {
                    let id: usize = cmd.replace("STOP_CLIENT:", "").parse().unwrap_or(999);
                    // Fix #2: remove() ile hem kaldır hem al
                    if let Some(handle) = client_handles.remove(&id) {
                        let _ = handle.cmd_tx.send("STOP".into());
                        send_log_main(&log_tx_backend, &format!("\u{1F6D1} Client {} durduruldu ve kaldırıldı.", id));
                    } else {
                        send_log_main(&log_tx_backend, &format!("\u{26A0}\u{FE0F} STOP_CLIENT: Client {} zaten durmuş veya bulunamadı.", id));
                    }
                } else if cmd == "STOP" || cmd == "STOP_ALL" {
                    for (id, handle) in &client_handles {
                        let _ = handle.cmd_tx.send("STOP".into());
                        send_log_main(&log_tx_backend, &format!("🛑 Client {} durduruluyor...", id));
                    }
                } else if cmd == "START_ALL" {
                    send_log_main(&log_tx_backend, "⚠️ START_ALL: GUI'den client bazlı başlatın.");
                } else if cmd.starts_with("PREVIEW:") {
                    let hwnd_val: usize = cmd.replace("PREVIEW:", "").parse().unwrap_or(0);
                    preview_hwnd = hwnd_val;
                    preview_active_hwnd = 0;
                    if hwnd_val > 0 {
                        send_log_main(&log_tx_backend, &format!("📺 Canlı önizleme: HWND {}", hwnd_val));
                    }
                } else if cmd.starts_with("MODEL:") || cmd.starts_with("DRIVER:")
                    || cmd.starts_with("LOAD_KILIT:") || cmd.starts_with("UPDATE_KILIT_REGION:")
                {
                    for handle in client_handles.values() {
                        let _ = handle.cmd_tx.send(cmd.clone());
                    }
                }
            }

            // ── Stats channel'dan aggregate istatistikleri oku ──
            // Bug #3: MQTT istatistikleri GUI tarafında güncelleniyor (stats_rx orada okunuyor)

            // ═══════════════════════════════════════════════════════════════════════
            // CRASH RECOVERY: Heartbeat dinle ve ölü client'ları restart et
            // ═══════════════════════════════════════════════════════════════════════
            // Heartbeat kanalından mesajları işle
            while let Ok((client_id, _)) = hb_rx.try_recv() {
                if let Some(handle) = client_handles.get_mut(&client_id) {
                    handle.update_heartbeat();
                }
            }
            
            // Ölü client'ları tespit et ve restart et (max 3 deneme)
            let dead_clients: Vec<usize> = client_handles.iter()
                .filter(|(_, handle)| handle.should_restart())
                .map(|(id, _)| *id)
                .collect();
            
            for id in dead_clients {
                if let Some(mut handle) = client_handles.remove(&id) {
                    handle.increment_restart();
                    let restart_count = handle.restart_count;
                    
                    send_log_main(&log_tx_backend, &format!(
                        "💀 CRASH RECOVERY: Client {} yanıt vermiyor! Restart #{}/3 deneniyor...", 
                        id, restart_count
                    ));
                    
                    // Eski thread'i durdur
                    let _ = handle.cmd_tx.send("STOP".into());
                    thread::sleep(Duration::from_millis(500));
                    
                    // Parametreleri al ve yeniden başlat
                    if let Some(params) = handle.last_params.clone() {
                        let (client_cmd_tx, client_cmd_rx) = unbounded::<String>();
                        let log_tx_c = log_tx_backend.clone();
                        let stats_tx_c = stats_tx.clone();
                        let frame_tx_c = frame_tx.clone();
                        let radar_tx_c = radar_tx.clone();
                        let ctx_c = ctx.clone();
                        
                        let params_for_thread = params.clone();
                        let hb_tx_c = hb_tx.clone();
                        let new_handle = thread::spawn(move || {
                            client_runner::run_client(
                                params_for_thread.id, params_for_thread.hwnd, params_for_thread.model, params_for_thread.driver,
                                params_for_thread.kilit_path, params_for_thread.kilit_region, params_for_thread.ocr_region,
                                log_tx_c, client_cmd_rx, stats_tx_c, frame_tx_c, radar_tx_c, ctx_c,
                                params_for_thread.olum_skill_aktif, params_for_thread.olum_skill_tuslari, params_for_thread.olum_skill_bekleme,
                                params_for_thread.olum_binek_aktif, params_for_thread.olum_binek_tusu, params_for_thread.olum_binek_bekleme,
                                params_for_thread.captcha_buton_x1, params_for_thread.captcha_buton_y1,
                                params_for_thread.captcha_buton_x2, params_for_thread.captcha_buton_y2,
                                hb_tx_c, // Heartbeat kanalı
                                // Auto-PM AI parametreleri (crash recovery ile korunur)
                                params_for_thread.pm_ai_aktif,
                                params_for_thread.pm_ai_backend.clone(),
                                params_for_thread.pm_ai_api_key.clone(),
                                params_for_thread.pm_system_prompt.clone(),
                                params_for_thread.pm_region,
                                params_for_thread.pm_cooldown_sn,
                                params_for_thread.pm_daily_limit,
                            );
                        });
                        
                        client_handles.insert(id, ClientHandle::new(client_cmd_tx, Some(new_handle), params));
                        send_log_main(&log_tx_backend, &format!(
                            "🔄 Client {} restart edildi! ({}. deneme)", id, restart_count
                        ));
                    }
                }
            }

            // ── Preview: bot çalışmayan client'ların kamerasını çalıştır ──
            if preview_hwnd > 0 {
                let has_running_client = !client_handles.is_empty();
                if !has_running_client {
                    let hwnd_ptr = preview_hwnd as winapi::shared::windef::HWND;
                    if preview_active_hwnd != preview_hwnd {
                        preview_active_hwnd = preview_hwnd;
                    }
                    if let Some(frame) = preview_vision.capture_hwnd_background(hwnd_ptr) {
                        if frame_tx.len() < 4 {
                            let rgba = frame.to_rgba8();
                            let _ = frame_tx.send((999, rgba)); // 999 = preview-only
                            ctx.request_repaint();
                        }
                    }
                }
            }

            // Bug #3: MQTT status güncellemesi GUI tarafında yapılıyor (per_client_stats)

            thread::sleep(Duration::from_millis(10));
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 480.0])
            .with_resizable(false),
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };

    eframe::run_native(
        "K-BOT — METİN2 PANEL",
        options,
        Box::new(move |cc| {
            let _ = ctx_tx.send(cc.egui_ctx.clone());
            Box::new(gui::MonolithGui::new(cc, log_rx, cmd_tx, frame_rx, stats_rx, radar_rx))
        }),
    )
}