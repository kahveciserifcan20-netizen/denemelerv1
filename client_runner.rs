// client_runner.rs — v110
// Her client için bağımsız bot döngüsü thread'i
// PostMessageW tıklama, SendInput hareket, SetForegroundWindow odaklama

use crossbeam_channel::Sender;
use std::time::{Instant, Duration};
use std::thread;
use chrono::Local;
use rand::Rng;
use winapi::shared::windef::HWND;

use crate::vision_manager;
use crate::captcha_solver;
use crate::hw_simulator;
use crate::online_learner;
use crate::config;
use crate::BotStats;

// ── Radar Veri Yapıları ──────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct RadarPoint {
    pub x: f32,        // 0.0 - 1.0 (normalize)
    pub y: f32,        // 0.0 - 1.0 (normalize)
    pub point_type: RadarPointType,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum RadarPointType {
    Stone,
    BlacklistedStone,
    TargetStone,
}

#[derive(Clone, Debug)]
pub struct RadarData {
    pub client_id: usize,
    pub points: Vec<RadarPoint>,
}

// ── Bot Durumları ──────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
enum BotState {
    Arama,
    KilitBekleniyor { click_time: Instant, target_cx: i32, target_cy: i32 },
    Kesiyor { start_time: Instant },
    Mola { start_time: Instant, duration: Duration },
    HumanPause { start_time: Instant, duration: Duration },
}

// ── Mola Durumları ─────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
enum MolaState {
    Calisiyor,
    MolaBekleniyor, // Mola zamanı geldi, taş bitince başlayacak
    Molada,
}

fn send_log(tx: &Sender<String>, client_id: usize, msg: &str) {
    let ts = Local::now().format("%H:%M:%S.%3f");
    let _ = tx.send(format!("[{}] [C{}] {}", ts, client_id, msg));
}

/// Tek bir client için bot döngüsü — ayrı thread'de çalışır
pub fn run_client(
    client_id: usize,
    hwnd: usize,
    model_name: String,
    driver_mode: String,
    kilit_path: String,
    kilit_region: (i32, i32, i32, i32),
    ocr_region: (i32, i32, i32, i32), // OCR (Captcha) bölgesi
    log_tx: Sender<String>,
    cmd_rx: crossbeam_channel::Receiver<String>,
    stats_tx: Sender<(usize, BotStats)>,
    frame_tx: Sender<(usize, image::RgbaImage)>,
    radar_tx: Sender<RadarData>,
    ctx: eframe::egui::Context,
    // Ölüm sonrası skill ve binek ayarları (3 skill tuşu desteği)
    olum_skill_aktif: bool,
    olum_skill_tuslari: Vec<String>,
    olum_skill_bekleme: u64,
    olum_binek_aktif: bool,
    olum_binek_tusu: String,
    olum_binek_bekleme: u64,
    // Captcha buton bölgesi (kullanıcı tarafından kaydedilmiş - dikdörtgen)
    captcha_buton_x1: i32,
    captcha_buton_y1: i32,
    captcha_buton_x2: i32,
    captcha_buton_y2: i32,
    // Crash Recovery: Heartbeat kanalı
    hb_tx: crossbeam_channel::Sender<(usize, std::time::Instant)>,
    // Auto-PM AI parametreleri (artık config'den okunuyor — geriye uyumluluk için tutuldu)
    _pm_ai_aktif: bool,
    _pm_ai_backend: String,
    _pm_ai_api_key: String,
    _pm_system_prompt: String,
    _pm_region: (i32, i32, i32, i32),
    _pm_cooldown_sn: u64,
    _pm_daily_limit: u32,
) {
    send_log(&log_tx, client_id, &format!("🚀 Client thread başlatıldı. HWND: {}", hwnd));

    // GDI kaynaklarını önceden başlat (gecikmeyi önle)
    let mut vision = vision_manager::VisionEngine::new_with_shm(
        client_id, "".to_string(), kilit_path.clone(), log_tx.clone()
    );
    let captcha_engine = captcha_solver::CaptchaSolver::new(captcha_solver::OcrBackend::Ocrs);
    let mut online_learner = online_learner::OnlineLearner::new(log_tx.clone());
    let mut pending_feedback: Option<(online_learner::FeatureVector, String)> = None;
    let mut last_detection_for_feedback: Option<(vision_manager::Detection, [f32; 4])> = None;

    // HW Simulator — her client kendi instance'ına sahip
    let hw_mode = if driver_mode.contains("Arduino") {
        hw_simulator::InputMode::Arduino { port_name: driver_mode.clone() }
    } else {
        hw_simulator::InputMode::WinAPI
    };
    let hw_sim = hw_simulator::HwSimulator::new(hw_mode);

    // Auto-PM AI Engine başlat (config'den — lazy init, loop'ta da kontrol edilir)
    let mut pm_engine: Option<crate::pm_ai::PmAiEngine> = None;
    {
        let init_cfg = config::AppConfig::load();
        if init_cfg.pm_ai_aktif && init_cfg.pm_region_x2 > init_cfg.pm_region_x1 && init_cfg.pm_region_y2 > init_cfg.pm_region_y1 {
            let backend = crate::pm_ai::AiBackend::from_config(&init_cfg.pm_ai_backend, &init_cfg.pm_ai_api_key);
            pm_engine = Some(crate::pm_ai::PmAiEngine::new(
                backend,
                init_cfg.pm_system_prompt.clone(),
                init_cfg.pm_cooldown_sn,
                init_cfg.pm_daily_limit
            ));
            send_log(&log_tx, client_id, &format!("🤖 PM AI aktif! Backend: {}, Cooldown: {}sn, Limit: {} PM/gün", 
                init_cfg.pm_ai_backend, init_cfg.pm_cooldown_sn, init_cfg.pm_daily_limit));
        } else if init_cfg.pm_ai_aktif {
            send_log(&log_tx, client_id, "⚠️ PM AI aktif ama bölge seçilmemiş! PM AI devre dışı.");
        }
    }

    // PM kontrol zamanlayıcıları
    let mut last_pm_check = Instant::now();
    let pm_check_interval = Duration::from_millis(400); // 400ms — yanıp sönen simgeyi yakalamak için HIZLI
    let pm_captcha_engine = captcha_solver::CaptchaSolver::new(captcha_solver::OcrBackend::Ocrs);
    let mut pm_prev_brightness: Option<f32> = None; // Blink detection: önceki frame parlaklığı
    let mut pm_blink_count: u32 = 0; // Ardışık blink tespit sayısı (false positive azaltma)
    
    // Model yükle
    if !model_name.is_empty() && model_name != "MODEL_BULUNAMADI" {
        vision.switch_map_model(&model_name);
    }

    let target_hwnd_ptr = hwnd as HWND;
    
    // 🔥 GDI kaynaklarını HEMEN başlat - ilk frame gecikmesini önle
    send_log(&log_tx, client_id, "🔥 GDI kaynakları önceden başlatılıyor...");
    let _ = vision.prewarm_gdi_resources(target_hwnd_ptr);
    // Bug #5: Pencere boyutuna göre GDI kaynaklarını dinamik ayarla
    vision.resize_for_hwnd(target_hwnd_ptr);
    let mut bot_state = BotState::Arama;
    let mut blacklist: Vec<((i32, i32), Instant)> = Vec::new();

    // attack_cooldown aşağıda cached_cfg yüklendikten sonra başlatılır
    let mut last_attack_time = Instant::now();
    let mut last_camera_move = Instant::now();
    let camera_move_cooldown = Duration::from_millis(500);
            let mut last_captcha_check = Instant::now();
            let captcha_check_interval = Duration::from_millis(200); // 200ms - ÇOK HIZLI captcha tepkisi (15 sn sınırı için)
    let mut last_log_time = Instant::now();
    #[allow(unused_assignments)]
    let mut captcha_solving = false;
    let mut captcha_solve_start = Instant::now();
    let mut last_stats_send = Instant::now();

    let mut kilit_region_opt: Option<(u32,u32,u32,u32)> = if kilit_region.2 > kilit_region.0 && kilit_region.3 > kilit_region.1 {
        Some((kilit_region.0.max(0) as u32, kilit_region.1.max(0) as u32,
              kilit_region.2.max(0) as u32, kilit_region.3.max(0) as u32))
    } else {
        // Metin2 HP bar bölgesi - ekranın üst orta kısmı
        Some((200, 30, 600, 80))
    };

    // İstatistikler
    let mut stats_stones_mined = 0u32;
    let mut stats_stones_missed = 0u32;
    let mut stats_captcha_solved = 0u32;
    let mut stats_uptime_total = 0.0f64;
    let mut stats_uptime_last = Instant::now();
    let mut last_seen_stone_pos: Option<(i32, i32)> = None;

    // HP bar takip
    let mut last_hp_red_count: i32 = 0;
    let mut hp_check_start: Option<Instant> = None;
    let hp_stuck_threshold = Duration::from_secs(4);

    // Otomatik Pot Sistemi
    let mut last_pot_time = Instant::now();
    let pot_cooldown = Duration::from_secs(3); // 3 saniye cooldown

    // Eşya toplama
    let mut last_item_pickup = Instant::now();
    let item_pickup_cooldown = Duration::from_millis(500);

    // Anti-tespit
    let mut last_micro_move = Instant::now();
    let micro_move_interval = Duration::from_secs(3);

    // Frame sayacı (debug log için)
    let mut frame_count: u64 = 0;

    // OBS Bypass - pencere görünürlük kontrolü
    let mut obs_check_interval = Duration::from_secs(5);
    let mut last_obs_check = Instant::now();
    #[allow(unused_assignments)]
    let mut _is_window_visible = true;

    // Mola sistemi
    let mut mola_state = MolaState::Calisiyor;
    let mut mola_start_time: Option<Instant> = None;
    let mut last_mola_check = Instant::now();
    let mut total_work_time = Duration::from_secs(0);
    let mut mola_count = 0u32;

    // İnsancıl çalışma - rastgele duraklamalar
    let mut next_human_pause = Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(300..600));
    #[allow(unused_assignments)]
    let mut human_pause_duration = Duration::from_secs(0);
    let mut is_human_pausing = false;

    // Ölüm sonrası yeniden başlatma
    let mut death_detected = false;
    let mut death_detect_time: Option<Instant> = None;
    let mut death_button_clicked = false;

    let mut is_running = true;

    // Bug #2 fix: Config cache — 2 saniyede bir yenile (her frame'de disk I/O yerine)
    let mut cached_cfg = config::AppConfig::load();
    let mut last_cfg_reload = Instant::now();
    let cfg_reload_interval = Duration::from_secs(2);
    // Fix #11: attack_cooldown ilk değeri cached_cfg'den al
    // Loop'un ilk iterasyonunda cfg'den yeniden güncellenir (unused_assignments kasıtlı)
    #[allow(unused_assignments)]
    let mut attack_cooldown = Duration::from_millis(cached_cfg.attack_cooldown_ms);


    loop {
        // Komut kontrolü
        while let Ok(cmd) = cmd_rx.try_recv() {
            if cmd == "STOP" {
                send_log(&log_tx, client_id, "🛑 Client durduruldu.");
                is_running = false;
            } else if cmd.starts_with("MODEL:") {
                let m = cmd.replace("MODEL:", "");
                if m != "MODEL_BULUNAMADI" { vision.switch_map_model(&m); }
            } else if cmd.starts_with("LOAD_KILIT:") {
                let p = cmd.replace("LOAD_KILIT:", "");
                vision.reload_hedef_kilit(&p);
            } else if cmd.starts_with("UPDATE_KILIT_REGION:") {
                let clean = cmd.replace("UPDATE_KILIT_REGION:", "");
                let p: Vec<&str> = clean.split(',').collect();
                if p.len() == 4 {
                    let x1: u32 = p[0].parse().unwrap_or(0);
                    let y1: u32 = p[1].parse().unwrap_or(0);
                    let x2: u32 = p[2].parse().unwrap_or(0);
                    let y2: u32 = p[3].parse().unwrap_or(0);
                    if x2 > x1 && y2 > y1 {
                        kilit_region_opt = Some((x1, y1, x2, y2));
                    }
                }
            }
        }

        if !is_running {
            // İstatistik gönder ve çık
            let _ = stats_tx.send((client_id, BotStats {
                stones_mined: stats_stones_mined, stones_missed: stats_stones_missed,
                captcha_solved: stats_captcha_solved, uptime_secs: stats_uptime_total,
                state_name: "Durduruldu".to_string(), is_running: false,
                pending_feedback: false, last_detection_conf: 0.0,
                learning_samples: 0, learning_positive: 0, learning_negative: 0,
            }));
            break;
        }

        blacklist.retain(|&(_, time)| time.elapsed().as_secs() < 15);

        // 🧠 Çoklu client: diğer client'ların öğrenme verilerini al (30 saniyede bir)
        if last_stats_send.elapsed() >= Duration::from_secs(30) {
            online_learner.reload_if_changed();
        }

            // İstatistik gönder (200ms aralıkla)
            if last_stats_send.elapsed() >= Duration::from_millis(200) {
                stats_uptime_total += stats_uptime_last.elapsed().as_secs_f64();
                stats_uptime_last = Instant::now();
                let state_name = match &bot_state {
                    BotState::Arama => "Taş Arıyor".to_string(),
                    BotState::KilitBekleniyor { .. } => "Kilit Bekleniyor".to_string(),
                    BotState::Kesiyor { .. } => "Taş Kesiliyor".to_string(),
                    BotState::Mola { .. } => "Mola".to_string(),
                    BotState::HumanPause { .. } => "İnsancıl Duraklama".to_string(),
                };
                let (l_total, l_pos, l_neg) = online_learner.get_stats();
                let _ = stats_tx.send((client_id, BotStats {
                    stones_mined: stats_stones_mined, stones_missed: stats_stones_missed,
                    captcha_solved: stats_captcha_solved, uptime_secs: stats_uptime_total,
                    state_name, is_running: true,
                    pending_feedback: pending_feedback.is_some(),
                    last_detection_conf: last_detection_for_feedback.as_ref().map(|d| d.0.confidence).unwrap_or(0.0),
                    learning_samples: l_total, learning_positive: l_pos, learning_negative: l_neg,
                }));
                last_stats_send = Instant::now();
                
                // ═══════════════════════════════════════════════════════════════════════
                // CRASH RECOVERY: Heartbeat gönder (5 saniyede bir)
                // ═══════════════════════════════════════════════════════════════════════
                let _ = hb_tx.send((client_id, std::time::Instant::now()));
            }

        let loop_start = Instant::now();
        // Bug #2: Config cache — diskten sadece 2 saniyede bir oku
        if last_cfg_reload.elapsed() >= cfg_reload_interval {
            cached_cfg = config::AppConfig::load();
            last_cfg_reload = Instant::now();
        }
        let cfg = &cached_cfg;
        // Fix #11: attack_cooldown config'den yükle (cfg.attack_cooldown_ms)
        attack_cooldown = Duration::from_millis(cfg.attack_cooldown_ms);

        // ═══════════════════════════════════════════════════════════════════════
        // MODERN & HUMAN-LIKE WORK SYSTEM - OBS Bypass + Mola + Human Pause
        // ═══════════════════════════════════════════════════════════════════════
        
        // 1. OBS BYPASS - Pencere görünürlük kontrolü (5 saniyede bir)
        if cfg.obs_bypass && last_obs_check.elapsed() >= obs_check_interval {
            last_obs_check = Instant::now();
            unsafe {
                // IsWindowVisible yerine daha güvenli: IsIconic (minimize) veya GetForegroundWindow kontrolü
                let is_minimized = winapi::um::winuser::IsIconic(target_hwnd_ptr) != 0;
                let fg_window = winapi::um::winuser::GetForegroundWindow();
                let _is_fg = fg_window == target_hwnd_ptr;
                
                // OBS Bypass mantığı: Eğer pencere minimize veya arka plandaysa, daha agresif çalış
                // Çünkü kullanıcı ekranda değil, yayın yapıyor
                let _ = !is_minimized; // Window visibility tracking for future use
                
                if is_minimized {
                    // OBS modunda: Daha hızlı çalış (kullanıcı ekranda değil)
                    obs_check_interval = Duration::from_secs(2); // Daha sık kontrol
                } else {
                    // Normal mod: Standart kontrol
                    obs_check_interval = Duration::from_secs(5);
                }
            }
        }

        // 2. MOLA SİSTEMİ - Zaman tabanlı otomatik mola
        if cfg.mola_sistemi_aktif {
            // Çalışma süresini güncelle (sadece aktif çalışırken)
            let mola_elapsed = last_mola_check.elapsed(); // Fix #3: bir kez ölç, iki kez kullan
            if mola_elapsed >= Duration::from_secs(1) {
                total_work_time += mola_elapsed;
                last_mola_check = Instant::now();
            }
            
            let mola_aralik = Duration::from_secs((cfg.mola_aralik_dk * 60) as u64);
            let mola_sure = Duration::from_secs((cfg.mola_sure_dk * 60) as u64);
            
            match mola_state {
                MolaState::Calisiyor => {
                    if total_work_time >= mola_aralik {
                        mola_state = MolaState::MolaBekleniyor;
                        send_log(&log_tx, client_id, &format!("☕ Mola zamanı geldi! ({} dk çalışıldı). Mevcut taş bitince mola başlayacak...", 
                            cfg.mola_aralik_dk));
                    }
                }
                MolaState::MolaBekleniyor => {
                    // Taş kesme durumunda değilsek veya arama durumundaysak molaya geç
                    if matches!(bot_state, BotState::Arama) {
                        mola_state = MolaState::Molada;
                        mola_start_time = Some(Instant::now());
                        mola_count += 1;
                        send_log(&log_tx, client_id, &format!("☕ MOLA BAŞLADI! (#{}). Süre: {} dk", 
                            mola_count, cfg.mola_sure_dk));
                        
                        // Fix #12: Telegram çağrısını ayrı thread'de yap (GUI'yi bloklamaz)
                        if cfg.telegram_bot && !cfg.telegram_webhook_url.is_empty() {
                            let msg = format!("\u{1F916} K-BOT Client {}: Mola başladı! (#{} - {} dk çalışıldı)",
                                client_id, mola_count, cfg.mola_aralik_dk);
                            let url = cfg.telegram_webhook_url.clone();
                            std::thread::spawn(move || {
                                crate::send_telegram_notification(&url, "", &msg);
                            });
                            send_log(&log_tx, client_id, "\u{1F4F1} Telegram mola bildirimi gönderildi");
                        }
                    }
                }
                MolaState::Molada => {
                    if let Some(start) = mola_start_time {
                        if start.elapsed() >= mola_sure {
                            // Mola bitti, çalışmaya devam
                            mola_state = MolaState::Calisiyor;
                            total_work_time = Duration::from_secs(0);
                            mola_start_time = None;
                            send_log(&log_tx, client_id, &format!("☕ Mola bitti! Çalışmaya devam ediliyor... (Toplam {} mola yapıldı)", mola_count));
                            
                            // Moladan sonra kamera çevir (yeni taşlar bulmak için)
                            let kamera_tuslari: [(u16, u64); 2] = [(0x51, 500), (0x45, 500)];
                            let (vk, ms) = kamera_tuslari[rand::thread_rng().gen_range(0..2)];
                            hw_sim.background_key_hold(target_hwnd_ptr, vk, ms);
                        } else {
                            // Moladayız - hiçbir şey yapma, sadece bekle
                            // Frame'i atla, captcha kontrolü yapma
                            thread::sleep(Duration::from_millis(100));
                            continue; // Ana döngüye devam et (frame işleme atlanır)
                        }
                    }
                }
            }
        }

        // 3. İNSANCIL ÇALIŞMA - Rastgele duraklamalar (5-10 dakikada bir)
        if cfg.insan_modu && !is_human_pausing && Instant::now() >= next_human_pause {
            // Rastgele duraklama süresi: 10-30 saniye
            let pause_secs = rand::thread_rng().gen_range(10..=30);
            human_pause_duration = Duration::from_secs(pause_secs);
            is_human_pausing = true;
            send_log(&log_tx, client_id, &format!("🧍 İnsancıl duraklama: {} saniye bekleniyor (gerçek oyuncu gibi)...", pause_secs));
            
            // Bot state'i HumanPause'a geçir
            bot_state = BotState::HumanPause { 
                start_time: Instant::now(), 
                duration: human_pause_duration 
            };
        }
        
        // İnsancıl duraklama state kontrolü
        if let BotState::HumanPause { start_time, duration } = &bot_state {
            if start_time.elapsed() >= *duration {
                // Duraklama bitti
                is_human_pausing = false;
                next_human_pause = Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(300..600));
                bot_state = BotState::Arama;
                send_log(&log_tx, client_id, "🧍 İnsancıl duraklama bitti, çalışmaya devam...");
            } else {
                // Hala duraklama devam ediyor - sadece bekle
                let remaining = duration.as_secs() - start_time.elapsed().as_secs();
                if remaining % 5 == 0 && remaining > 0 {
                    send_log(&log_tx, client_id, &format!("🧍 İnsancıl duraklama: {} saniye kaldı...", remaining));
                }
                thread::sleep(Duration::from_millis(100));
                continue; // Frame işleme atla
            }
        }

        // 4. ANA FRAME İŞLEME
        if let Some(frame) = vision.capture_hwnd_background(target_hwnd_ptr) {
            // ── Ölüm sonrası yeniden başlatma kontrolü ──
            if !death_detected && !death_button_clicked {
                let (is_death_screen, button_type, _button_pos) = vision.check_death_screen(&frame);
                if is_death_screen {
                    death_detected = true;
                    death_detect_time = Some(Instant::now());
                    let btn_name = button_type.as_deref().unwrap_or("bilinmiyor");
                    send_log(&log_tx, client_id, &format!("💀 Ölüm ekranı tespit edildi! Buton: {}, {} saniye bekleniyor...", btn_name, 10));
                }
            } else if death_detected && !death_button_clicked {
                if let Some(detect_time) = death_detect_time {
                    let elapsed = detect_time.elapsed().as_secs();
                    let wait_secs = 10u64; // 10 saniye bekle
                    
                    if elapsed >= wait_secs {
                        // 10 saniye doldu, butona tıkla
                        let (_, button_type, button_pos) = vision.check_death_screen(&frame);
                        
                        if let Some((bx, by)) = button_pos {
                            // Buton bulundu, tıkla
                            let (screen_bx, screen_by) = hw_sim.client_to_screen(target_hwnd_ptr, bx as i32, by as i32);
                            hw_sim.human_move(screen_bx, screen_by);
                            thread::sleep(Duration::from_millis(300));
                            hw_sim.background_click_mode(target_hwnd_ptr, bx as i32, by as i32, &hw_simulator::ClickMode::FocusSwap);
                            
                            let btn_name = button_type.as_deref().unwrap_or("bilinmiyor");
                            send_log(&log_tx, client_id, &format!("💀 '{}' butonuna tıklandı! Yeniden başlatılıyor...", btn_name));
                            death_button_clicked = true;
                        } else {
                            // Buton bulunamadı, tekrar dene
                            send_log(&log_tx, client_id, "⚠️ Ölüm butonu bulunamadı, tekrar aranıyor...");
                        }
                    } else {
                        // Henüz bekleme süresi dolmadı, her saniye log at
                        if elapsed > 0 && elapsed % 1 == 0 {
                            let remaining = wait_secs - elapsed;
                            if remaining % 2 == 0 { // Her 2 saniyede bir log
                                send_log(&log_tx, client_id, &format!("⏱️ Ölüm sonrası bekleme: {} saniye kaldı...", remaining));
                            }
                        }
                    }
                }
            } else if death_button_clicked {
                // Butona tıklandıktan sonra skill ve binek kullanımı
                thread::sleep(Duration::from_secs(3)); // Yükleme için bekle
                
                // ── SKILL KULLANIMI (3 Tuş Kombinasyonu) ──
                if olum_skill_aktif && !olum_skill_tuslari.is_empty() {
                    for (i, skill_tusu) in olum_skill_tuslari.iter().enumerate() {
                        if skill_tusu.is_empty() { continue; }
                        
                        send_log(&log_tx, client_id, &format!("⚔️ Skill {} kullanılıyor: {} ({} sn bekleme)", 
                            i + 1, skill_tusu, olum_skill_bekleme));
                        
                        // Arch #12: Paylaşılan vk_codes modülü kullanılıyor
                        let (modifiers, main_key) = crate::vk_codes::parse_key_combo(skill_tusu);
                        
                        // Tuş kombinasyonunu uygula
                        if let Some(key_vk) = main_key {
                            if modifiers.len() == 2 {
                                // İki modifikatör + ana tuş (örn: Ctrl+Shift+F1)
                                hw_sim.background_key_combo_with_two_modifiers(
                                    target_hwnd_ptr, 
                                    modifiers[0], 
                                    modifiers[1], 
                                    key_vk
                                );
                            } else if modifiers.len() == 1 {
                                // Tek modifikatör + ana tuş (örn: Ctrl+G)
                                hw_sim.background_key_combo(target_hwnd_ptr, modifiers[0], key_vk);
                            } else {
                                // Sadece ana tuş (örn: F1)
                                hw_sim.background_key_press(target_hwnd_ptr, key_vk);
                            }
                            
                            thread::sleep(Duration::from_millis(500)); // Skill animasyonu için kısa bekle
                            
                            // Skill bekleme süresi
                            send_log(&log_tx, client_id, &format!("⏱️ Skill {} bekleme: {} saniye...", 
                                i + 1, olum_skill_bekleme));
                            thread::sleep(Duration::from_secs(olum_skill_bekleme));
                        }
                    }
                }
                
                // ── BİNEK KULLANIMI ──
                if olum_binek_aktif && !olum_binek_tusu.is_empty() {
                    send_log(&log_tx, client_id, &format!("🐴 Bineğe biniliyor: {} ({} sn bekleme)", olum_binek_tusu, olum_binek_bekleme));
                    
                    // Arch #12: Binek tuşunu paylaşılan modülle parse et
                    let (binek_mods, binek_main) = crate::vk_codes::parse_key_combo(&olum_binek_tusu);
                    if let Some(key_vk) = binek_main {
                        if !binek_mods.is_empty() {
                            hw_sim.background_key_combo(target_hwnd_ptr, binek_mods[0], key_vk);
                        } else {
                            hw_sim.background_key_press(target_hwnd_ptr, key_vk);
                        }
                    } else {
                        // Fallback: G tuşu
                        hw_sim.background_key_press(target_hwnd_ptr, 0x47);
                    }
                    
                    thread::sleep(Duration::from_millis(500)); // Binek animasyonu için kısa bekle
                    
                    // Binek bekleme süresi
                    send_log(&log_tx, client_id, &format!("⏱️ Binek bekleme: {} saniye...", olum_binek_bekleme));
                    thread::sleep(Duration::from_secs(olum_binek_bekleme));
                }
                
                // Resetle ve devam et
                death_detected = false;
                death_detect_time = None;
                death_button_clicked = false;
                bot_state = BotState::Arama;
                blacklist.clear();
                send_log(&log_tx, client_id, "🔄 Ölüm sonrası yeniden başlatma tamamlandı, bot devam ediyor...");
            }

            // ── Captcha kontrolü ──
            // ÖNCE OCR bölgesini kontrol et - soru metni var mı?
            let (ocr_x1, ocr_y1, ocr_x2, ocr_y2) = (
                ocr_region.0.max(0) as u32, 
                ocr_region.1.max(0) as u32, 
                ocr_region.2.max(0) as u32, 
                ocr_region.3.max(0) as u32
            );
            
            let has_ocr_region = ocr_x2 > ocr_x1 && ocr_y2 > ocr_y1 && 
                                ocr_x2 <= frame.width() && ocr_y2 <= frame.height();
            
            // Kaydedilmiş buton bölgesi
            let saved_button_region = if captcha_buton_x1 > 0 && captcha_buton_y1 > 0 && 
                                        captcha_buton_x2 > captcha_buton_x1 && captcha_buton_y2 > captcha_buton_y1 {
                Some((captcha_buton_x1, captcha_buton_y1, captcha_buton_x2, captcha_buton_y2))
            } else {
                None
            };
            
            // CAPTCHA TESPIT - SADECE btn_onay.png görülünce (metin taşı kilit gibi)
            if last_captcha_check.elapsed() >= captcha_check_interval && !captcha_solving && !death_detected && has_ocr_region {
                last_captcha_check = Instant::now();
                
                // 1. ÖNCE butonu kontrol et - SADECE kaydedilmiş bölgede, skor 0.85+
                if let Some((btn_x, btn_y)) = captcha_engine.is_captcha_present(&frame, &log_tx, saved_button_region) {
                    // 2. Buton bulundu! Şimdi OCR yap
                    let ocr_w = ocr_x2.saturating_sub(ocr_x1);
                    let ocr_h = ocr_y2.saturating_sub(ocr_y1);
                    
                    // AGRESIF DEBUG: OCR bölgesi bilgisi
                    send_log(&log_tx, client_id, &format!(
                        "🔍 OCR BÖLGESI: x1={}, y1={}, w={}, h={}, frame={}x{}", 
                        ocr_x1, ocr_y1, ocr_w, ocr_h,
                        frame.width(), frame.height()
                    ));
                    
                    // OCR bölgesi geçerli mi?
                    if ocr_w < 30 || ocr_h < 10 {
                        send_log(&log_tx, client_id, &format!(
                            "❌ OCR bölgesi ÇOK KÜÇÜK! w={}, h={}. Ayarlar > Captcha > 'Soru Metni' bölgesini kontrol et!",
                            ocr_w, ocr_h
                        ));
                        continue;
                    }
                    
                    let soru_kesit = image::DynamicImage::ImageRgba8(
                        image::imageops::crop_imm(&frame, ocr_x1, ocr_y1, ocr_w, ocr_h).to_image()
                    );
                    
                    // AGRESIF DEBUG: Her captcha'da OCR görüntüsünü kaydet
                    let debug_filename = format!("captcha_ocr_debug_{}.png", Local::now().format("%H%M%S_%3f"));
                    let _ = soru_kesit.save(&debug_filename);
                    send_log(&log_tx, client_id, &format!("💾 OCR görüntüsü kaydedildi: {}", debug_filename));
                    
                    let ocr_text = captcha_engine.do_ocr(&soru_kesit);
                    
                    // AGRESIF DEBUG: OCR sonucu
                    send_log(&log_tx, client_id, &format!("📝 OCR SONUCU: '{}' (len={})", ocr_text, ocr_text.len()));
                    
                    // 3. Geçerli OCR varsa çöz - YOKSA DA DENE (fallback)
                    let final_text = if ocr_text.is_empty() {
                        send_log(&log_tx, client_id, "⚠️ OCR boş! Fallback: 'metin' kullanılıyor...");
                        "metin".to_string() // En yaygın captcha tipi
                    } else {
                        ocr_text.clone()
                    };
                    
                    // Her durumda çözümü dene
                    captcha_solving = true; // Fix #1: asıl bayrağı set et
                    captcha_solve_start = Instant::now();
                    send_log(&log_tx, client_id, &format!("🚨 CAPTCHA TESPIT! Buton@({},{}) Soru: '{}'", btn_x, btn_y, final_text));

                    // 4. Çözümü yap - AGRESİF HIZLI
                    // 🆕 YENİ: final_text'i fallback olarak geç (OCR başarısız olursa kullanılır)
                    let clicks = captcha_engine.solve_with_verification(
                        &frame, 
                        &soru_kesit, 
                        &log_tx, 
                        saved_button_region,
                        &hw_sim,
                        target_hwnd_ptr,
                        Some(&final_text)  // 🆕 Fallback soru metni
                    );
                    
                    if !clicks.is_empty() {
                        stats_captcha_solved += 1;
                        send_log(&log_tx, client_id, &format!("✅ Captcha BAŞARIYLA çözüldü! {} tıklama ({}ms)", clicks.len(), 
                            captcha_solve_start.elapsed().as_millis()));
                    } else {
                        send_log(&log_tx, client_id, "🚨 Captcha çözümü BAŞARISIZ! 3 deneme de failed.");
                    }
                    
                    // captcha_solving sonraki frame'de zaman aşımı kontrolü tarafından sıfırlanır (line 632+)
                    // Burada false yapmayız ki !captcha_solving guard sonraki 10sn boyunca gelinmesin
                    bot_state = BotState::Arama;
                    blacklist.clear();
                }
            }
            if captcha_solving && captcha_solve_start.elapsed().as_secs() > 10 {
                captcha_solving = false;
            }

            // ── PM Tespiti v3.0 ──
            // Güvenilir PM algılama: blink detection + renk + şablon + OCR
            // ÖNEMLİ: pm_region'ı CONFIG'den oku (GUI'den güncel değer)
            let (pm_x1_raw, pm_y1_raw, pm_x2_raw, pm_y2_raw) = (cfg.pm_region_x1, cfg.pm_region_y1, cfg.pm_region_x2, cfg.pm_region_y2);
            let pm_bölge_geçerli = pm_x2_raw > pm_x1_raw && pm_y2_raw > pm_y1_raw;
            if pm_bölge_geçerli && last_pm_check.elapsed() >= pm_check_interval {
                last_pm_check = Instant::now();
                
                // PM bölgesini frame sınırlarına clamp et (frame dışı koordinatları düzelt)
                let fw_i = frame.width() as i32;
                let fh_i = frame.height() as i32;
                let pm_x1 = pm_x1_raw.max(0).min(fw_i - 1);
                let pm_y1 = pm_y1_raw.max(0).min(fh_i - 1);
                let pm_x2 = pm_x2_raw.max(0).min(fw_i);
                let pm_y2 = pm_y2_raw.max(0).min(fh_i);
                
                let pm_w = (pm_x2 - pm_x1) as u32;
                let pm_h = (pm_y2 - pm_y1) as u32;
                
                // Clamp sonrası bölge çok küçükse uyar
                if pm_w < 20 || pm_h < 10 {
                    if frame_count % 100 == 1 {
                        send_log(&log_tx, client_id, &format!(
                            "🚨 PM bölgesi frame dışında! Config=({},{})→({},{}) Frame={}x{} — PM bölgesini oyun penceresine göre yeniden ayarlayın!", 
                            pm_x1_raw, pm_y1_raw, pm_x2_raw, pm_y2_raw, fw_i, fh_i
                        ));
                    }
                }
                
                // PM bölgesi yeterince büyük mi? (clamp sonrası)
                if pm_w >= 20 && pm_h >= 10 {
                        
                        // Lazy PM AI Engine init — config'den aktifse ve engine yoksa oluştur
                        if pm_engine.is_none() && cfg.pm_ai_aktif && !cfg.pm_ai_api_key.is_empty() {
                            let backend = crate::pm_ai::AiBackend::from_config(&cfg.pm_ai_backend, &cfg.pm_ai_api_key);
                            pm_engine = Some(crate::pm_ai::PmAiEngine::new(
                                backend, cfg.pm_system_prompt.clone(), cfg.pm_cooldown_sn, cfg.pm_daily_limit
                            ));
                            send_log(&log_tx, client_id, &format!("🤖 PM AI Engine lazy başlatıldı! Backend: {}", cfg.pm_ai_backend));
                        }

                        // ════════════════════════════════════════════════════
                        // PM TEKİL AKIŞ (3 ADIM):
                        // 1. pm_simge.png görünce → tıkla (PM penceresi açılır)
                        // 2. Yeni frame yakala → pm_ekran.png + OCR → cevap yaz
                        // 3. PM'i kapat (Esc + X butonu)
                        // ════════════════════════════════════════════════════
                        let pm_search = Some((pm_x1, pm_y1, pm_x2, pm_y2));

                        // DEBUG: PM tarama bilgisi (her 10 kontrolde bir)
                        if frame_count % 10 == 1 {
                            send_log(&log_tx, client_id, &format!(
                                "🔎 PM Tarama #{}: simge_bölge=[{},{} → {},{}] frame={}x{} prev_bright={:.1} blink_cnt={}", 
                                frame_count, cfg.pm_simge_x1, cfg.pm_simge_y1, cfg.pm_simge_x2, cfg.pm_simge_y2,
                                frame.width(), frame.height(),
                                pm_prev_brightness.unwrap_or(-1.0), pm_blink_count
                            ));
                            
                            // KRİTİK UYARI: PM simge bölgesi frame dışında mı?
                            if cfg.pm_simge_x2 > 0 && (cfg.pm_simge_x2 as u32 > frame.width() || cfg.pm_simge_y2 as u32 > frame.height()) {
                                send_log(&log_tx, client_id, &format!(
                                    "🚨 UYARI: PM simge bölgesi ({},{}) frame sınırları ({}x{}) DIŞINDA! Bölgeyi yeniden ayarlayın!", 
                                    cfg.pm_simge_x2, cfg.pm_simge_y2, frame.width(), frame.height()
                                ));
                            }
                        }

                        // ─── ADIM 1: PM Simgesi — BLINK DETECTION v3.0 ────────
                        // PM Simge arama bölgesi — frame sınırlarına clamp et!
                        let pm_simge_region: Option<(i32, i32, i32, i32)> = if cfg.pm_simge_x2 > cfg.pm_simge_x1 && cfg.pm_simge_y2 > cfg.pm_simge_y1 {
                            // Frame sınırlarına clamp
                            let fw = frame.width() as i32;
                            let fh = frame.height() as i32;
                            let sx = cfg.pm_simge_x1.max(0).min(fw - 1);
                            let sy = cfg.pm_simge_y1.max(0).min(fh - 1);
                            let ex = cfg.pm_simge_x2.max(0).min(fw);
                            let ey = cfg.pm_simge_y2.max(0).min(fh);
                            
                            if ex > sx + 5 && ey > sy + 5 {
                                Some((sx, sy, ex, ey))
                            } else {
                                // Clamp sonrası bölge çok küçük — varsayılanı kullan
                                send_log(&log_tx, client_id, &format!(
                                    "⚠️ PM simge bölgesi frame dışında! Config=({},{})→({},{}) Frame={}x{} — TÜM SAĞ TARAF taranıyor", 
                                    cfg.pm_simge_x1, cfg.pm_simge_y1, cfg.pm_simge_x2, cfg.pm_simge_y2, fw, fh
                                ));
                                None
                            }
                        } else {
                            None // Bölge ayarlanmamış — varsayılan kullan
                        };
                        
                        // 3 Katmanlı algılama: yanıp sönme + renk + şablon
                        let (simge_detection, current_brightness) = pm_captcha_engine.find_pm_simge_v3(
                            &frame, pm_simge_region, pm_prev_brightness
                        );
                        
                        // Parlaklık farkını logla (her 10 taramada bir)
                        if frame_count % 10 == 0 {
                            if let Some(prev) = pm_prev_brightness {
                                let diff = (current_brightness - prev).abs();
                                if diff > 2.0 {
                                    send_log(&log_tx, client_id, &format!(
                                        "💡 PM Blink: prev={:.1} → curr={:.1} (diff={:.1})", 
                                        prev, current_brightness, diff
                                    ));
                                }
                            }
                        }
                        pm_prev_brightness = Some(current_brightness);
                        
                        // Simge bulunduysa → tıkla ve HEMEN ADIM 2'ye geç
                        let mut pm_pencere_acildi = false;
                        
                        if let Some((sig_cx, sig_cy, sig_score)) = simge_detection {
                            // Ardışık blink sayacı — 4 kez ardışık tespit ederse tıkla
                            // (tek seferlik sahne değişimi false positive engeli)
                            pm_blink_count += 1;
                            
                            // ÇOK KATI: 4 kez ardışık blink gerekli (önceki 2'ydi, hala false positive veriyordu)
                            if pm_blink_count >= 4 {
                                send_log(&log_tx, client_id, &format!(
                                    "📬 PM SİMGESİ BULUNDU! ({},{}) skor={:.2} blink={}— TIKLANIYOR...", 
                                    sig_cx, sig_cy, sig_score, pm_blink_count
                                ));
                                
                                // Simgeye tıkla → PM penceresi açılsın
                                let (sx, sy) = hw_sim.client_to_screen(target_hwnd_ptr, sig_cx, sig_cy);
                                hw_sim.human_move(sx, sy);
                                thread::sleep(Duration::from_millis(150));
                                hw_sim.background_click_mode(target_hwnd_ptr, sig_cx, sig_cy, &hw_simulator::ClickMode::FocusSwap);
                                
                                // Pencere açılsın diye bekle
                                thread::sleep(Duration::from_millis(800));
                                send_log(&log_tx, client_id, "⏳ PM penceresi açılması bekleniyor...");
                                pm_pencere_acildi = true;
                                pm_blink_count = 0;
                            } else {
                                send_log(&log_tx, client_id, &format!(
                                    "🔸 PM blink tespit #{} ({},{}) skor={:.2} — onay bekleniyor...", 
                                    pm_blink_count, sig_cx, sig_cy, sig_score
                                ));
                            }
                        } else {
                            // Tespit yok — blink sayacını sıfırla
                            if pm_blink_count > 0 {
                                pm_blink_count = 0;
                            }
                        }
                        
                        // ─── ADIM 2: PM Penceresi → OCR → AI Cevap ──────────────────
                        // Simge tıklandıysa yeni frame yakala ve PM penceresini BUL
                        let pm_pencere_konumu = if pm_pencere_acildi {
                            // Simgeye tıkladık — yeni frame'de PM penceresini şablonla bul
                            thread::sleep(Duration::from_millis(600)); // Pencerenin açılması için bekle
                            if let Some(new_frame) = vision.capture_hwnd_background(target_hwnd_ptr) {
                                // TÜM ekranda PM penceresi şablonunu ara (sabit bölge değil!)
                                let ekran_found = pm_captcha_engine.find_pm_screen(&new_frame, None, 0.55);
                                if let Some((scr_cx, scr_cy, scr_score)) = ekran_found {
                                    send_log(&log_tx, client_id, &format!(
                                        "📨 PM PENCERESİ BULUNDU! ({},{}) skor={:.2}", scr_cx, scr_cy, scr_score
                                    ));
                                    // PM penceresi boyutları (pm_ekran.png şablonundan)
                                    let pm_pw = 280i32; // PM pencere genişliği (tahmini)
                                    let pm_ph = 200i32; // PM pencere yüksekliği (tahmini)
                                    let px1 = scr_cx - pm_pw / 2;
                                    let py1 = scr_cy - pm_ph / 2;
                                    let px2 = scr_cx + pm_pw / 2;
                                    let py2 = scr_cy + pm_ph / 2;
                                    Some((px1, py1, px2, py2, new_frame))
                                } else {
                                    send_log(&log_tx, client_id, "⚠️ PM penceresi bulunamadı! Şablon eşleşmedi.");
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            // Pasif kontrol: mevcut frame'de pm_ekran.png ara
                            let ekran_found = pm_captcha_engine.find_pm_screen(&frame, None, 0.45);
                            if let Some((scr_cx, scr_cy, scr_score)) = ekran_found {
                                let pm_pw = 280i32;
                                let pm_ph = 200i32;
                                let px1 = scr_cx - pm_pw / 2;
                                let py1 = scr_cy - pm_ph / 2;
                                let px2 = scr_cx + pm_pw / 2;
                                let py2 = scr_cy + pm_ph / 2;
                                Some((px1, py1, px2, py2, frame.clone()))
                            } else {
                                None
                            }
                        };
                        
                        if let Some((pm_px1, pm_py1, pm_px2, pm_py2, ref current_frame)) = pm_pencere_konumu {
                            // — 2a. OCR ile mesajı oku —
                            // v4: DINAMIK OCR bölgesi - PM penceresinin içindeki mesaj alanı
                            // PM penceresi yapısı: başlık (25px) + gönderici (20px) + mesaj alanı
                            let msg_x1 = (pm_px1 + 15).max(0) as u32; // Sol kenardan 15px içeride
                            let msg_y1 = (pm_py1 + 50).max(0) as u32; // Üstten 50px aşağıda (başlık+gönderici)
                            let msg_w  = ((pm_px2 - pm_px1 - 30).max(100) as u32).min(current_frame.width().saturating_sub(msg_x1));
                            let msg_h  = ((pm_py2 - pm_py1 - 70).max(50) as u32).min(current_frame.height().saturating_sub(msg_y1));

                            if msg_w > 50 && msg_h > 20 {
                                let msg_crop = image::DynamicImage::ImageRgba8(
                                    image::imageops::crop_imm(current_frame, msg_x1, msg_y1, msg_w, msg_h).to_image()
                                );
                                
                                // DEBUG: OCR görüntüsünü kaydet (pencere konumlu)
                                let debug_ocr_file = format!("pm_ocr_debug_{}_{}_{}x{}.png", 
                                    client_id, Local::now().format("%H%M%S_%3f"), msg_x1, msg_y1);
                                let _ = msg_crop.save(&debug_ocr_file);
                                
                                let pm_text = pm_captcha_engine.do_ocr(&msg_crop);
                                
                                // v3: Parse dene - başarısız olursa AI'a ham metni gönder (OCR hatasını AI düzeltsin)
                                let pm_parse_result = crate::pm_ai::extract_pm_from_ocr(&pm_text);
                                
                                let (sender, message, use_ai_parse) = if let Some((s, m)) = pm_parse_result {
                                    // ✅ Başarılı parse
                                    send_log(&log_tx, client_id, &format!("💬 PM Parse: [{}] → '{}'", s, m.chars().take(40).collect::<String>()));
                                    (s, m, false)
                                } else {
                                    // ⚠️ Parse başarısız - AI'a ham OCR metnini gönder, AI çözümlesin
                                    send_log(&log_tx, client_id, &format!(
                                        "⚠️ PM Parse başarısız! OCR: '{}' — AI çözümleyecek...", 
                                        pm_text.chars().take(40).collect::<String>()
                                    ));
                                    ("Oyuncu".to_string(), pm_text.clone(), true)
                                };

                                // — 2b. Giriş kutusuna tıkla (DINAMIK PM penceresi konumuna göre) —
                                // PM penceresinin alt orta kısmı (giriş kutusu)
                                let input_x = pm_px1 + (pm_px2 - pm_px1) / 2;
                                let input_y = pm_py2 - 30; // Alt kenardan 30px yukarıda
                                let (ix, iy) = hw_sim.client_to_screen(target_hwnd_ptr, input_x, input_y);
                                hw_sim.human_move(ix, iy);
                                thread::sleep(Duration::from_millis(150));
                                hw_sim.background_click_mode(target_hwnd_ptr, input_x, input_y, &hw_simulator::ClickMode::FocusSwap);
                                thread::sleep(Duration::from_millis(250));

                                // — 2c. AI ile cevap üret ve yaz —
                                let reply_text = if let Some(ref mut engine) = pm_engine {
                                    if use_ai_parse {
                                        // AI'a ham OCR metnini gönder - AI hem parse etsin hem cevap versin
                                        send_log(&log_tx, client_id, &format!("🤖 AI çözümleme + cevap: '{}'", message.chars().take(30).collect::<String>()));
                                        engine.get_reply(&sender, &message, Some(&message))
                                    } else {
                                        send_log(&log_tx, client_id, &format!("🤖 AI cevabı: {} → {}", sender, message));
                                        engine.get_reply(&sender, &message, Some(&pm_text))
                                    }
                                } else {
                                    send_log(&log_tx, client_id, "⚠️ PM AI aktif değil — mesaj okundu ama cevap yok");
                                    None
                                };

                                if let Some(ref cevap) = reply_text {
                                    send_log(&log_tx, client_id, &format!("✍️ Cevap yazılıyor: '{}'", cevap.chars().take(40).collect::<String>()));
                                    crate::pm_ai::send_pm_reply(target_hwnd_ptr, &hw_sim, cevap, (pm_px1, pm_py1, pm_px2, pm_py2));
                                    thread::sleep(Duration::from_millis(300));

                                    // — 2d. Gönder: pm_buton.png veya Enter (DINAMIK arama) —
                                    let send_frame = vision.capture_hwnd_background(target_hwnd_ptr);
                                    let btn_frame = send_frame.as_ref().unwrap_or(current_frame);
                                    // Gönder butonunu PM penceresi bölgesinde ara
                                    let btn_search = Some((pm_px1, pm_py1, pm_px2, pm_py2));
                                    if let Some((btn_cx, btn_cy, _)) = pm_captcha_engine.find_pm_button(btn_frame, btn_search, 0.55) {
                                        send_log(&log_tx, client_id, &format!("🔘 Gönder butonuna tıklanıyor ({},{})...", btn_cx, btn_cy));
                                        let (sbx, sby) = hw_sim.client_to_screen(target_hwnd_ptr, btn_cx, btn_cy);
                                        hw_sim.human_move(sbx, sby);
                                        thread::sleep(Duration::from_millis(100));
                                        hw_sim.background_click_mode(target_hwnd_ptr, btn_cx, btn_cy, &hw_simulator::ClickMode::FocusSwap);
                                    } else {
                                        send_log(&log_tx, client_id, "↩️ Gönder butonu bulunamadı, Enter ile gönderiliyor...");
                                        hw_sim.background_key_press(target_hwnd_ptr, 0x0D); // VK_RETURN
                                    }
                                    thread::sleep(Duration::from_millis(400));
                                    send_log(&log_tx, client_id, "✅ PM cevabı gönderildi!");

                                    // — 2e. PM penceresini kapat (Esc + X butonu DINAMIK) —
                                    thread::sleep(Duration::from_millis(300));
                                    hw_sim.background_key_press(target_hwnd_ptr, 0x1B); // VK_ESCAPE
                                    thread::sleep(Duration::from_millis(200));
                                    // X butonu sağ üst köşede (dinamik PM konumuna göre)
                                    let close_x = pm_px2 - 15;
                                    let close_y = pm_py1 + 15;
                                    let (cx2, cy2) = hw_sim.client_to_screen(target_hwnd_ptr, close_x, close_y);
                                    hw_sim.human_move(cx2, cy2);
                                    thread::sleep(Duration::from_millis(150));
                                    hw_sim.background_click_mode(target_hwnd_ptr, close_x, close_y, &hw_simulator::ClickMode::FocusSwap);
                                    send_log(&log_tx, client_id, "❎ PM penceresi kapatıldı.");
                                } else {
                                    // AI cevap üretemedi - pencereyi kapat
                                    send_log(&log_tx, client_id, "⚠️ AI cevap üretemedi - PM penceresi kapatılıyor");
                                    thread::sleep(Duration::from_millis(200));
                                    hw_sim.background_key_press(target_hwnd_ptr, 0x1B);
                                }
                            } else {
                                send_log(&log_tx, client_id, &format!(
                                    "⚠️ PM OCR bölgesi çok küçük: {}x{} — PM bölgesini yeniden ayarlayın", msg_w, msg_h
                                ));
                            }
                        } else if !pm_pencere_acildi {
                            // v2: OCR fallback KALDIRILDI - sadece simge tespitiyle çalış
                            // Bu çok fazla false positive üretiyordu
                            // PM simgesi görülmeden ASLA AI'a mesaj gönderme!
                        }
                    }
                }

            // ── YOLO Tespit ──
            let detections = vision.infer(&frame);
            let detections_for_draw = detections.clone();

            // ── Radar verisi gönder ──
            let mut radar_points = Vec::new();
            let (fw, fh) = (frame.width() as f32, frame.height() as f32);
            for d in &detections {
                if d.class_id <= 2 && d.confidence >= 0.50 {
                    let cx = ((d.bbox[0] + d.bbox[2]) / 2.0) / fw;
                    let cy = ((d.bbox[1] + d.bbox[3]) / 2.0) / fh;
                    let dcx = ((d.bbox[0] + d.bbox[2]) / 2.0) as i32;
                    let dcy = ((d.bbox[1] + d.bbox[3]) / 2.0) as i32;
                    let is_bl = blacklist.iter().any(|&((bx, by), _)| (bx - dcx).abs() < 50 && (by - dcy).abs() < 50);
                    let pt = if is_bl { RadarPointType::BlacklistedStone } else { RadarPointType::Stone };
                    radar_points.push(RadarPoint { x: cx, y: cy, point_type: pt, confidence: d.confidence });
                }
            }
            let _ = radar_tx.send(RadarData { client_id, points: radar_points });

            // ── Gözlem logu ──
            if bot_state == BotState::Arama && last_log_time.elapsed() >= Duration::from_secs(2) {
                let stones: Vec<_> = detections.iter().filter(|d| d.class_id == 2 && d.confidence >= 0.65).collect();
                if !stones.is_empty() {
                    let (l_total, l_pos, l_neg) = online_learner.get_stats();
                    send_log(&log_tx, client_id, &format!("👁️ {} Taş | Kara: {} | 🧠 {}(+{}/−{})", 
                        stones.len(), blacklist.len(), l_total, l_pos, l_neg));
                }
                last_log_time = Instant::now();
            }

            // ── Ana bot mantığı ──
            match bot_state.clone() {
                BotState::Mola { .. } | BotState::HumanPause { .. } => {
                    // Bu state'ler yukarıda continue ile atlanıyor, buraya gelmemeli
                    // Ama Rust exhaustive match istiyor
                }
                BotState::Arama => {
                    if last_camera_move.elapsed() < camera_move_cooldown { /* bekle */ }
                    else {
                        let all_stones: Vec<_> = detections.iter()
                            .filter(|d| {
                                if d.class_id > 2 || d.confidence < 0.65 { return false; }
                                let w = d.bbox[2] - d.bbox[0]; let h = d.bbox[3] - d.bbox[1];
                                let area = w * h;
                                if w > 400.0 || h > 400.0 || area > 100000.0 { return false; }
                                if w < 20.0 || h < 20.0 || area < 400.0 { return false; }
                                true
                            })
                            .map(|d| {
                                let cx = ((d.bbox[0] + d.bbox[2]) / 2.0) as i32;
                                let cy = ((d.bbox[1] + d.bbox[3]) / 2.0) as i32;
                                (d.clone(), cx, cy)
                            }).collect();

                        let mut target_stone = None;
                        let mut stones_found = false;
                        for (stone, cx, cy) in &all_stones {
                            stones_found = true;
                            let is_bl = blacklist.iter().any(|&((bx, by), _)| (bx - cx).abs() < 50 && (by - cy).abs() < 50);
                            if is_bl { continue; }
                            
                            // 🧠 Online Learning filtresi — yeterli veri varsa tahmini kullan
                            let (l_total, _, _) = online_learner.get_stats();
                            if l_total >= 20 {
                                let features = online_learner.extract_features(&frame, &stone.bbox);
                                let (is_good, confidence) = online_learner.predict(&features);
                                if !is_good && confidence < 0.25 {
                                    send_log(&log_tx, client_id, &format!(
                                        "🧠 Taş atlandı ({},{}) — öğrenme: %{:.0} kötü", cx, cy, (1.0 - confidence) * 100.0
                                    ));
                                    continue;
                                }
                            }
                            target_stone = Some(stone.clone());
                            break;
                        }

                        if !stones_found {
                            // 🏃 KARAKTER HAREKETİ - Taş bulunamadığında ileri/geri git
                            // Bu, ekrana yeni metin taşlarının düşmesini sağlar (seri farm için kritik)
                            let hareket_tuslari: Vec<(char, u16, u64)> = vec![
                                ('W', 0x57, 1000), // İleri - 1 saniye
                                ('S', 0x53, 800),  // Geri - 0.8 saniye
                            ];
                            
                            // Rastgele ileri veya geri
                            let idx = rand::thread_rng().gen_range(0..hareket_tuslari.len());
                            let (tus_char, tus_vk, tus_sure_ms) = hareket_tuslari[idx];
                            
                            send_log(&log_tx, client_id, &format!(
                                "🏃 Taş bulunamadı! Karakter {} tuşuna {}ms basılı tutuyor (yeni taşlar için)...", 
                                tus_char, tus_sure_ms
                            ));
                            
                            // Karakteri hareket ettir - bu yeni taşların ekrana düşmesini sağlar
                            hw_sim.background_key_hold(target_hwnd_ptr, tus_vk, tus_sure_ms);
                            
                            // Kısa bekle ve tekrar dene
                            thread::sleep(Duration::from_millis(200));
                            
                            // Ayrıca kamera da çevir (opsiyonel)
                            let mut aktif_tuslar: Vec<(char, u16, f32)> = Vec::new();
                            if cfg.arama_q_aktif { aktif_tuslar.push(('Q', 0x51, cfg.arama_q_sure)); }
                            if cfg.arama_e_aktif { aktif_tuslar.push(('E', 0x45, cfg.arama_e_sure)); }
                            
                            if !aktif_tuslar.is_empty() {
                                let cam_idx = rand::thread_rng().gen_range(0..aktif_tuslar.len());
                                let (_, cam_vk, cam_sure) = aktif_tuslar[cam_idx];
                                let cam_ms = (cam_sure * 1000.0) as u64;
                                hw_sim.background_key_hold(target_hwnd_ptr, cam_vk, cam_ms);
                            }
                            
                            last_camera_move = Instant::now();
                            thread::sleep(Duration::from_millis(100));
                        }

                        if let Some(stone) = target_stone {
                            if last_attack_time.elapsed() >= attack_cooldown {
                                let cx = ((stone.bbox[0] + stone.bbox[2]) / 2.0) as i32;
                                let cy = ((stone.bbox[1] + stone.bbox[3]) / 2.0) as i32;
                                last_seen_stone_pos = Some((cx, cy));

                                // İNSANSIZ HAREKET - Önce fareyi taşın üzerine götür
                                let (screen_cx, screen_cy) = hw_sim.client_to_screen(target_hwnd_ptr, cx, cy);
                                send_log(&log_tx, client_id, &format!("🖱️ Fare hareketi: ekran ({}, {}) -> client ({}, {})", screen_cx, screen_cy, cx, cy));
                                hw_sim.human_move(screen_cx, screen_cy);
                                thread::sleep(Duration::from_millis(300)); // Hareketin tamamlanması için bekle

                                // Şimdi tıkla - FocusSwap ile (daha güvenilir)
                                hw_sim.background_click_mode(target_hwnd_ptr, cx, cy, &hw_simulator::ClickMode::FocusSwap);
                                send_log(&log_tx, client_id, &format!("⚔️ Taşa tıklandı ({},{}) - İnsansı hareket + tıklama", cx, cy));
                                
                                last_attack_time = Instant::now();

                                let features = online_learner.extract_features(&frame, &stone.bbox);
                                // Debug kaydetme KALDIRILDI - hız için
                                // pending_feedback = Some((features, format!("stone_{}.png", stats_stones_mined)));
                                pending_feedback = Some((features, String::new()));
                                last_detection_for_feedback = Some((stone.clone(), stone.bbox));

                                bot_state = BotState::KilitBekleniyor {
                                    click_time: Instant::now(), target_cx: cx, target_cy: cy,
                                };
                            }
                        }
                    }
                }

                BotState::KilitBekleniyor { click_time, target_cx, target_cy } => {
                    let elapsed_ms = click_time.elapsed().as_millis();
                    if elapsed_ms >= 1500 { // 1.5 saniye bekle - kilitlenme animasyonu için
                        let (is_locked, score) = vision.is_target_locked(&frame, kilit_region_opt, true);
                        if is_locked {
                            send_log(&log_tx, client_id, &format!("🔒 Kilit tespit! (skor: {:.3})", score));
                            if let Some((features, _)) = pending_feedback.as_ref() {
                                online_learner.process_feedback(features.clone(), true, None);
                            }
                            bot_state = BotState::Kesiyor { start_time: Instant::now() };
                        } else if elapsed_ms >= 3000 {
                            stats_stones_missed += 1;
                            send_log(&log_tx, client_id, &format!("❌ Iskala! Toplam: {}", stats_stones_missed));
                            if let Some((features, _)) = pending_feedback.take() {
                                online_learner.process_feedback(features, false, None);
                            }
                            blacklist.push(((target_cx, target_cy), Instant::now()));
                            bot_state = BotState::Arama;
                            thread::sleep(Duration::from_millis(200));
                        }
                    }
                }

                BotState::Kesiyor { start_time } => {
                    // 🚀 ANLIK KİLİT KONTROLÜ - Her frame'de, SIFIR gecikme ile!
                    let (is_locked, lock_score) = vision.is_target_locked(&frame, kilit_region_opt, false);
                    let elapsed = start_time.elapsed();

                    // Kilit kayboldu mu? ANLIK algıla - SIFIR gecikme!
                    if !is_locked {
                        // Kilit kayboldu - taş kırıldı veya öldü!
                        stats_stones_mined += 1;
                        send_log(&log_tx, client_id, &format!(
                            "💎 Taş bitti! (Kilit kayboldu @ {:.1}s, skor: {:.2}) | Toplam: {} | Iskala: {}",
                            elapsed.as_secs_f64(), lock_score, stats_stones_mined, stats_stones_missed
                        ));

                        // Eşya toplama — PostMessageW ile
                        if cfg.toplama_aktif && last_item_pickup.elapsed() >= item_pickup_cooldown {
                            let toplama_vk: u16 = match cfg.toplama_tusu.to_uppercase().as_str() {
                                "Z" => 0x5A, "X" => 0x58, "C" => 0x43, "V" => 0x56,
                                "F" => 0x46, "G" => 0x47, "SPACE" => 0x20, _ => 0x5A,
                            };
                            let press_count = rand::thread_rng().gen_range(3..=5);
                            for _ in 0..press_count {
                                hw_sim.background_key_press(target_hwnd_ptr, toplama_vk);
                                let wait = rand::thread_rng().gen_range(150..=300);
                                thread::sleep(Duration::from_millis(wait));
                            }
                            last_item_pickup = Instant::now();
                        }

                        hp_check_start = None;
                        last_hp_red_count = 0;
                        bot_state = BotState::Arama;
                        continue; // HEMEN yeni taş ara, bekleme yok!
                    }

                    // Anti-tespit mikro hareket
                    if cfg.anti_tespit_modu && last_micro_move.elapsed() >= micro_move_interval {
                        last_micro_move = Instant::now();
                    }
                    if cfg.rastgele_gecikme {
                        let delay = rand::thread_rng().gen_range(cfg.gecikme_min as u64..=cfg.gecikme_max as u64);
                        thread::sleep(Duration::from_millis(delay));
                    }

                    // 🚀 AGRESİF HP BAR STAGNASYON KONTROLÜ - Her frame'de!
                    let mut current_hp_red = 0i32;
                    for d in &detections {
                        // Sadece taş sınıfları için HP bar kontrolü (class_id 0 ve 1)
                        if (d.class_id == 0 || d.class_id == 1) && d.confidence >= 0.30 {
                            let red_count = vision.count_red_pixels(&frame, &d.bbox);
                            current_hp_red += red_count;
                        }
                    }
                    
                    // DEBUG: Her 2 saniyede bir HP bar durumunu logla (daha sık)
                    if frame_count % 60 == 0 {
                        let stagnasyon_suresi = hp_check_start.map(|s| s.elapsed().as_secs()).unwrap_or(0);
                        send_log(&log_tx, client_id, &format!(
                            "🔴 HP: {} piksel | Stagnasyon: {}sn/4sn | locked={}", 
                            current_hp_red, stagnasyon_suresi, is_locked
                        ));
                    }
                    
                    // AGRESİF STAGNASYON ALGILAMA - 4 saniye yeterli!
                    if current_hp_red > 1 { // Eşik: 1 piksel bile yeterli (çok düşük = daha hassas)
                        match hp_check_start {
                            None => {
                                // İlk tespit - zamanlayıcıyı başlat ve logla
                                hp_check_start = Some(Instant::now());
                                last_hp_red_count = current_hp_red;
                                send_log(&log_tx, client_id, &format!("🔴 HP takip BAŞLADI: {} piksel", current_hp_red));
                            }
                            Some(hp_start) => {
                                let stagnasyon_suresi = hp_start.elapsed();
                                
                                // 4 saniye doldu mu?
                                if stagnasyon_suresi >= hp_stuck_threshold {
                                    // Değişim miktarı (artma veya azalma) - SADECE 1 piksel tolerans!
                                    let change = (last_hp_red_count - current_hp_red).abs();
                                    
                                    if change <= 1 { // Sadece 1 piksel değişim bile yeterli!
                                        // ⚠️ STAGNASYON - HP bar değişmiyor! HEMEN farklı taş geç
                                        send_log(&log_tx, client_id, &format!(
                                            "🚨 STAGNASYON ALGILANDI! 4sn boyunca HP değişmedi ({} -> {} piksel). YENİ TAŞ ARANIYOR...", 
                                            last_hp_red_count, current_hp_red
                                        ));
                                        
                                        // Mevcut taşı blacklist'e ekle (eğer pozisyon biliniyorsa)
                                        if let Some((last_cx, last_cy)) = last_seen_stone_pos {
                                            blacklist.push(((last_cx, last_cy), Instant::now()));
                                            send_log(&log_tx, client_id, &format!(
                                                "⛔ Taş blacklist'e eklendi: ({}, {})", last_cx, last_cy
                                            ));
                                        }
                                        
                                        // HEMEN kamera çevir ve yeni taş ara
                                        let kamera_tuslari: [(u16, u64); 2] = [(0x51, 300), (0x45, 300)];
                                        let (vk, ms) = kamera_tuslari[rand::thread_rng().gen_range(0..2)];
                                        hw_sim.background_key_hold(target_hwnd_ptr, vk, ms);
                                        
                                        // Reset ve yeni taş ara
                                        hp_check_start = None;
                                        last_hp_red_count = 0;
                                        bot_state = BotState::Arama;
                                        continue; // Döngüyü hemen bitir
                                    } else {
                                        // HP değişiyor - normal, zamanlayıcıyı resetle
                                        if change > 1 {
                                            send_log(&log_tx, client_id, &format!(
                                                "✅ HP değişimi algılandı: {} -> {} piksel ({}sn içinde)", 
                                                last_hp_red_count, current_hp_red, stagnasyon_suresi.as_secs()
                                            ));
                                        }
                                        hp_check_start = Some(Instant::now());
                                        last_hp_red_count = current_hp_red;
                                    }
                                } else {
                                    // 4 saniye dolmadı - değişimi kontrol et (erken çıkış için)
                                    let change = (last_hp_red_count - current_hp_red).abs();
                                    if change > 1 {
                                        // HP değişiyor, zamanlayıcıyı resetle
                                        hp_check_start = Some(Instant::now());
                                        last_hp_red_count = current_hp_red;
                                    }
                                }
                            }
                        }
                    }

                    // ═══════════════════════════════════════════════════════════════════════
                    // OTOMATİK POT SİSTEMİ - HP düşükse pot bas
                    // ═══════════════════════════════════════════════════════════════════════
                    if cfg.otomatik_pot_aktif && last_pot_time.elapsed() >= pot_cooldown {
                        // HP bar bölgesini analiz et
                        // Fix #8: HP bar için ayrı bölge - config'den al, yoksa kilit_region fallback
                    let hp_region_opt = if cfg.hp_bar_x1 > 0 && cfg.hp_bar_x2 > cfg.hp_bar_x1 && cfg.hp_bar_y2 > cfg.hp_bar_y1 {
                        Some((cfg.hp_bar_x1 as u32, cfg.hp_bar_y1 as u32, cfg.hp_bar_x2 as u32, cfg.hp_bar_y2 as u32))
                    } else {
                        kilit_region_opt // fallback: kilit bölgesi
                    };
                    if let Some(hp_region) = hp_region_opt {
                            let (hp_percentage, _, _) = vision.analyze_hp_bar(&frame, hp_region);
                            
                            // HP % pot_esik'in altındaysa pot bas
                            if hp_percentage < cfg.pot_esik as f32 {
                                // Pot tuşunu parse et
                                let pot_vk = crate::vk_codes::vk_from_name(&cfg.pot_tusu)
                                    .unwrap_or(0x31); // Varsayılan: "1" tuşu (0x31)
                                
                                send_log(&log_tx, client_id, &format!(
                                    "🧪 OTOMATİK POT! HP: %{:.0} < Eşik: {}% | Tuş: {} basılıyor...", 
                                    hp_percentage, cfg.pot_esik, cfg.pot_tusu
                                ));
                                
                                // Pot bas
                                hw_sim.background_key_press(target_hwnd_ptr, pot_vk);
                                last_pot_time = Instant::now();
                                
                                // Fix #12: Telegram pot bildirimini thread'de gönder
                                if cfg.telegram_bot && cfg.telegram_tas_bildirim && !cfg.telegram_webhook_url.is_empty() {
                                    let msg = format!("\u{1F916} K-BOT Client {}: Pot kullanıldı! (HP: %{:.0})",
                                        client_id, hp_percentage);
                                    let url = cfg.telegram_webhook_url.clone();
                                    std::thread::spawn(move || {
                                        crate::send_telegram_notification(&url, "", &msg);
                                    });
                                }
                            }
                        }
                    }
                    // ═══════════════════════════════════════════════════════════════════════
                     else {
                        // Yeterli kırmızı piksel yok - taş ölmüş olabilir veya HP bar yok
                        // Sessizce resetle, log atma - zaten kilit kontrolü taşın bittiğini söyleyecek
                        if hp_check_start.is_some() {
                            // Sadece debug modunda log at (şimdilik sessiz)
                            // send_log(&log_tx, client_id, &format!(
                            //     "🔇 HP bar yok ({} piksel) - muhtemelen taş yeni başladı veya bitti", current_hp_red
                            // ));
                            hp_check_start = None;
                            last_hp_red_count = 0;
                        }
                    }

                    // Not: Kilit kontrolü satır 871'de zaten yapılıyor.
                    // İkinci kontrol mükerrer taş sayımına neden oluyordu — kaldırıldı (Bug #1).
                }
            }

            // ⚡ FRAME GÖNDERME - Video gibi akıcı, düşük gecikme
            // Her 3 frame'de 1 gönder, buffer doluysa atla (drop frame stratejisi)
            if frame_count % 3 == 0 {
                // Buffer doluysa frame atla - lag oluşmasın!
                if frame_tx.len() < 2 {
                    let mut draw_frame = frame.to_rgba8();
                    vision_manager::VisionEngine::draw_detections_on_frame(&mut draw_frame, &detections_for_draw, kilit_region_opt);
                    // Try_send kullan - bloklama yok, doluysa atla
                    let _ = frame_tx.try_send((client_id, draw_frame));
                    ctx.request_repaint();
                }
                // Buffer doluysa sessizce atla, log spam yapma
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }

        // Frame sayacını artır
        frame_count = frame_count.wrapping_add(1);

        let elapsed = loop_start.elapsed().as_millis() as u64;
        if elapsed < 8 { thread::sleep(Duration::from_millis(8 - elapsed)); }
    }

    send_log(&log_tx, client_id, "🏁 Client thread sonlandı.");
}
