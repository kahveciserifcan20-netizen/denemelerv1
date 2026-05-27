use serde::{Deserialize, Serialize};
use std::fs;

// ── Per-Client Config ─────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientConfig {
    pub name: String,
    pub hwnd: usize,
    pub active: bool,
    pub model: String,
    pub driver: String,
    pub kilit_path: String,
    // Kilit arama bölgesi
    pub search_x1: i32,
    pub search_y1: i32,
    pub search_x2: i32,
    pub search_y2: i32,
    // Şablon bölgesi
    pub template_x1: i32,
    pub template_y1: i32,
    pub template_x2: i32,
    pub template_y2: i32,
    // OCR (Captcha) bölgesi - canlı kamera üzerinden seçilebilir
    #[serde(default = "default_ocr_x1")]
    pub ocr_x1: i32,
    #[serde(default = "default_ocr_y1")]
    pub ocr_y1: i32,
    #[serde(default = "default_ocr_x2")]
    pub ocr_x2: i32,
    #[serde(default = "default_ocr_y2")]
    pub ocr_y2: i32,
    // Radar
    #[serde(default)]
    pub radar_aktif: bool,
    // Captcha buton bölgesi (per-client)
    #[serde(default)]
    pub captcha_buton_x1: i32,
    #[serde(default)]
    pub captcha_buton_y1: i32,
    #[serde(default)]
    pub captcha_buton_x2: i32,
    #[serde(default)]
    pub captcha_buton_y2: i32,
}

fn default_ocr_x1() -> i32 { 313 }
fn default_ocr_y1() -> i32 { 153 }
fn default_ocr_x2() -> i32 { 457 }
fn default_ocr_y2() -> i32 { 168 }

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "Client 01".to_string(),
            hwnd: 0,
            active: true,
            model: String::new(),
            driver: "Arduino_AUTO".to_string(),
            kilit_path: "hedef_kilit.png".to_string(),
            search_x1: 300, search_y1: 20,
            search_x2: 500, search_y2: 90,
            template_x1: 354, template_y1: 54,
            template_x2: 398, template_y2: 71,
            ocr_x1: 313, ocr_y1: 153,
            ocr_x2: 457, ocr_y2: 168,
            radar_aktif: false,
            captcha_buton_x1: 0, captcha_buton_y1: 0,
            captcha_buton_x2: 0, captcha_buton_y2: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub selected_model: String,
    pub mouse_mode: String,
    pub ocr_x1: i32,
    pub ocr_y1: i32,
    pub ocr_x2: i32,
    pub ocr_y2: i32,
    pub live_view_enabled: bool,
    pub kilit_path: String,
    pub kilit_x1: i32,
    pub kilit_y1: i32,
    pub kilit_x2: i32,
    pub kilit_y2: i32,
    // Arama tuşları (taş bulunamadığında kamera çevirme)
    pub arama_q_aktif: bool,
    pub arama_e_aktif: bool,
    pub arama_w_aktif: bool,
    pub arama_a_aktif: bool,
    pub arama_s_aktif: bool,
    pub arama_d_aktif: bool,
    pub arama_q_sure: f32,
    pub arama_e_sure: f32,
    pub arama_w_sure: f32,
    pub arama_a_sure: f32,
    pub arama_s_sure: f32,
    pub arama_d_sure: f32,
    // Kilit arama bölgesi (hedef_kilit.png aranacak bölge)
    pub kilit_region_x1: i32,
    pub kilit_region_y1: i32,
    pub kilit_region_x2: i32,
    pub kilit_region_y2: i32,

    // ── Yeni GUI Ayarları ──
    #[serde(default = "default_false")]
    pub toplama_aktif: bool,
    #[serde(default = "default_toplama_tusu")]
    pub toplama_tusu: String,
    #[serde(default)]
    pub tema_renk_idx: usize,
    #[serde(default = "default_dil")]
    pub dil: String,
    #[serde(default = "default_gui_width")]
    pub gui_width: f32,
    #[serde(default = "default_gui_height")]
    pub gui_height: f32,
    #[serde(default = "default_kisayol_baslat")]
    pub kisayol_baslat_durdur: String,
    #[serde(default = "default_kisayol_log")]
    pub kisayol_log_temizle: String,
    #[serde(default = "default_kisayol_ekran")]
    pub kisayol_ekran_goruntusu: String,

    // Güvenlik & Anti-Tespit
    #[serde(default = "default_false")]
    pub anti_tespit_modu: bool,
    #[serde(default = "default_false")]
    pub rastgele_gecikme: bool,
    #[serde(default = "default_false")]
    pub insan_modu: bool,
    #[serde(default = "default_false")]
    pub obs_bypass: bool,
    #[serde(default = "default_gecikme_min")]
    pub gecikme_min: u32,
    #[serde(default = "default_gecikme_max")]
    pub gecikme_max: u32,

    // Mola sistemi
    #[serde(default = "default_mola_aralik")]
    pub mola_aralik_dk: u32,
    #[serde(default = "default_mola_sure")]
    pub mola_sure_dk: u32,

    // Bildirimler
    #[serde(default = "default_true")]
    pub bildirim_uygulama: bool,
    #[serde(default = "default_false")]
    pub bildirim_ses: bool,
    #[serde(default = "default_false")]
    pub telegram_bot: bool,
    #[serde(default)]
    pub telegram_webhook_url: String,
    
    // Login
    #[serde(default = "default_false")]
    pub remember_me: bool,
    #[serde(default)]
    pub saved_email: String,
    #[serde(default)]
    pub saved_pass: String,

    // MQTT Remote Control (APK Bağlantısı)
    #[serde(default = "default_mqtt_aktif")]
    pub mqtt_aktif: bool,
    #[serde(default)]
    pub selected_hwnd: usize,
    #[serde(default = "default_mqtt_broker")]
    pub mqtt_broker: String,
    #[serde(default = "default_mqtt_port")]
    pub mqtt_port: u16,
    #[serde(default = "default_mqtt_topic")]
    pub mqtt_topic: String,

    // Otomatik Pot Sistemi
    #[serde(default = "default_false")]
    pub otomatik_pot_aktif: bool,
    #[serde(default = "default_pot_tusu")]
    pub pot_tusu: String,
    #[serde(default = "default_pot_esik")]
    pub pot_esik: u32, // HP % kaçınca pot bassın

    // Telegram Bildirim Ayarları
    #[serde(default)]
    pub telegram_chat_id: String,
    #[serde(default = "default_false")]
    pub telegram_captcha_bildirim: bool,
    #[serde(default = "default_false")]
    pub telegram_tas_bildirim: bool,
    #[serde(default = "default_false")]
    pub telegram_mola_bildirim: bool,

    // Mola Sistemi Aktif
    #[serde(default = "default_false")]
    pub mola_sistemi_aktif: bool,

    // Çoklu Client Tıklama Modu
    #[serde(default = "default_tikla_modu")]
    pub tikla_modu: String, // "Hibrit", "PostMessageW", "FocusSwap"
    // Captcha buton bölgesi (dikdörtgen - OCR/Arama gibi)
    #[serde(default = "default_zero")]
    pub captcha_buton_x1: i32,
    #[serde(default = "default_zero")]
    pub captcha_buton_y1: i32,
    #[serde(default = "default_zero")]
    pub captcha_buton_x2: i32,
    #[serde(default = "default_zero")]
    pub captcha_buton_y2: i32,

    // ── Çoklu Client Konfigürasyonu ──
    #[serde(default)]
    pub clients: Vec<ClientConfig>,

    // HP Bar Bölgesi (Otomatik Pot için - kilit bölgesinden ayrı)
    #[serde(default = "default_zero")]
    pub hp_bar_x1: i32,
    #[serde(default = "default_zero")]
    pub hp_bar_y1: i32,
    #[serde(default = "default_zero")]
    pub hp_bar_x2: i32,
    #[serde(default = "default_zero")]
    pub hp_bar_y2: i32,

    // Saldırı Cooldown (ms) - taşa tıklama aralığı
    #[serde(default = "default_attack_cooldown")]
    pub attack_cooldown_ms: u64,

    // ── Auto-PM AI Sistemi ──
    #[serde(default = "default_false")]
    pub pm_ai_aktif: bool,
    #[serde(default = "default_pm_backend")]
    pub pm_ai_backend: String,   // "openai", "gemini", "ollama"
    #[serde(default)]
    pub pm_ai_api_key: String,
    #[serde(default = "default_pm_prompt")]
    pub pm_system_prompt: String,
    #[serde(default = "default_zero")]
    pub pm_region_x1: i32,
    #[serde(default = "default_zero")]
    pub pm_region_y1: i32,
    #[serde(default = "default_zero")]
    pub pm_region_x2: i32,
    #[serde(default = "default_zero")]
    pub pm_region_y2: i32,
    #[serde(default = "default_pm_cooldown")]
    pub pm_cooldown_sn: u64,
    #[serde(default = "default_pm_daily_limit")]
    pub pm_daily_limit: u32,
    
    // ── PM Simge Arama Bölgesi (pm_simge.png için) ──
    #[serde(default = "default_zero")]
    pub pm_simge_x1: i32,
    #[serde(default = "default_zero")]
    pub pm_simge_y1: i32,
    #[serde(default = "default_zero")]
    pub pm_simge_x2: i32,
    #[serde(default = "default_zero")]
    pub pm_simge_y2: i32,
}

fn default_zero() -> i32 { 0 }

fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_toplama_tusu() -> String { "Z".to_string() }
fn default_dil() -> String { "tr".to_string() }
fn default_kisayol_baslat() -> String { "F9".to_string() }
fn default_kisayol_log() -> String { "Ctrl+L".to_string() }
fn default_kisayol_ekran() -> String { "Ctrl+P".to_string() }
fn default_gecikme_min() -> u32 { 180 }
fn default_gecikme_max() -> u32 { 380 }
fn default_mola_aralik() -> u32 { 60 }
fn default_mola_sure() -> u32 { 5 }
fn default_mqtt_aktif() -> bool { true }
fn default_mqtt_broker() -> String { "broker.hivemq.com".to_string() }
fn default_mqtt_port() -> u16 { 1883 }
fn default_mqtt_topic() -> String { "kbot/cmd".to_string() }
fn default_pot_tusu() -> String { "1".to_string() }
fn default_pot_esik() -> u32 { 50 }
fn default_tikla_modu() -> String { "Hibrit".to_string() }
fn default_attack_cooldown() -> u64 { 1500 }
fn default_pm_backend() -> String { "gemini".to_string() }
fn default_pm_prompt() -> String { "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string() }
fn default_pm_cooldown() -> u64 { 30 }
fn default_pm_daily_limit() -> u32 { 200 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            selected_model: String::new(),
            mouse_mode: "Arduino_AUTO".to_string(),
            ocr_x1: 313, ocr_y1: 152, ocr_x2: 460, ocr_y2: 168,
            live_view_enabled: false,
            kilit_path: "hedef_kilit.png".to_string(),
            kilit_x1: 354, kilit_y1: 54, kilit_x2: 398, kilit_y2: 71,
            arama_q_aktif: true, arama_e_aktif: true,
            arama_w_aktif: false, arama_a_aktif: false,
            arama_s_aktif: false, arama_d_aktif: false,
            arama_q_sure: 0.15, arama_e_sure: 0.15,
            arama_w_sure: 0.3, arama_a_sure: 0.3,
            arama_s_sure: 0.3, arama_d_sure: 0.3,
            kilit_region_x1: 300,
            kilit_region_y1: 20,
            kilit_region_x2: 500,
            kilit_region_y2: 90,
            // Yeni GUI Ayarları
            toplama_aktif: false,
            toplama_tusu: "Z".to_string(),
            tema_renk_idx: 0,
            dil: "tr".to_string(),
            gui_width: default_gui_width(),
            gui_height: default_gui_height(),
            kisayol_baslat_durdur: "F9".to_string(),
            kisayol_log_temizle: "Ctrl+L".to_string(),
            kisayol_ekran_goruntusu: "Ctrl+P".to_string(),
            anti_tespit_modu: false,
            rastgele_gecikme: false,
            insan_modu: false,
            obs_bypass: false,
            gecikme_min: 180,
            gecikme_max: 380,
            mola_aralik_dk: 60,
            mola_sure_dk: 5,
            bildirim_uygulama: true,
            bildirim_ses: false,
            telegram_bot: false,
            telegram_webhook_url: String::new(),
            remember_me: false,
            saved_email: String::new(),
            saved_pass: String::new(),
            // MQTT Remote Control
            mqtt_aktif: true,
            selected_hwnd: 0,
            mqtt_broker: "broker.hivemq.com".to_string(),
            mqtt_port: 1883,
            mqtt_topic: "kbot/cmd".to_string(),
            // Otomatik Pot
            otomatik_pot_aktif: false,
            pot_tusu: "1".to_string(),
            pot_esik: 50,
            // Telegram Bildirimler
            telegram_chat_id: String::new(),
            telegram_captcha_bildirim: false,
            telegram_tas_bildirim: false,
            telegram_mola_bildirim: false,
            // Mola Sistemi Aktif
            mola_sistemi_aktif: false,
            // Tıklama Modu
            tikla_modu: "Hibrit".to_string(),
            // Captcha buton bölgesi - Varsayılan: (354,420)-(445,449)
            captcha_buton_x1: 354,
            captcha_buton_y1: 420,
            captcha_buton_x2: 445,
            captcha_buton_y2: 449,
            // Çoklu Client
            clients: Vec::new(),
            // HP Bar bölgesi
            hp_bar_x1: 0, hp_bar_y1: 0, hp_bar_x2: 0, hp_bar_y2: 0,
            // Saldırı cooldown
            attack_cooldown_ms: 1500,
            // Auto-PM AI
            pm_ai_aktif: false,
            pm_ai_backend: "gemini".to_string(),
            pm_ai_api_key: String::new(),
            pm_system_prompt: "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string(),
            pm_region_x1: 0, pm_region_y1: 0, pm_region_x2: 0, pm_region_y2: 0,
            pm_cooldown_sn: 30,
            pm_daily_limit: 200,
            // PM Simge bölgesi
            pm_simge_x1: 0,
            pm_simge_y1: 0,
            pm_simge_x2: 0,
            pm_simge_y2: 0,
        }
    }
}

fn default_gui_width() -> f32 { 900.0 }
fn default_gui_height() -> f32 { 600.0 }

impl AppConfig {
    pub fn load() -> Self {
        if let Ok(data) = fs::read_to_string("config.json") {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write("config.json", data);
        }
    }
}
