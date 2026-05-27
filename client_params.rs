// client_params.rs — Yapılandırılmış client başlatma parametreleri
// Eski fragile string protocol yerine type-safe struct kullanır.

use serde::{Serialize, Deserialize};

/// Client başlatma parametreleri — string parsing yerine struct-based iletişim
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientStartParams {
    pub id: usize,
    pub hwnd: usize,
    pub model: String,
    pub driver: String,
    pub kilit_path: String,
    pub kilit_region: (i32, i32, i32, i32),
    pub ocr_region: (i32, i32, i32, i32),
    pub olum_skill_aktif: bool,
    pub olum_skill_tuslari: Vec<String>,
    pub olum_skill_bekleme: u64,
    pub olum_binek_aktif: bool,
    pub olum_binek_tusu: String,
    pub olum_binek_bekleme: u64,
    pub captcha_buton_x1: i32,
    pub captcha_buton_y1: i32,
    pub captcha_buton_x2: i32,
    pub captcha_buton_y2: i32,
    // Auto-PM AI parametreleri
    pub pm_ai_aktif: bool,
    pub pm_ai_backend: String,
    pub pm_ai_api_key: String,
    pub pm_system_prompt: String,
    pub pm_region: (i32, i32, i32, i32),
    pub pm_cooldown_sn: u64,
    pub pm_daily_limit: u32,
}

impl Default for ClientStartParams {
    fn default() -> Self {
        Self {
            id: 0,
            hwnd: 0,
            model: String::new(),
            driver: "Arduino_AUTO".to_string(),
            kilit_path: "hedef_kilit.png".to_string(),
            kilit_region: (200, 30, 600, 80),
            ocr_region: (313, 153, 457, 168),
            olum_skill_aktif: false,
            olum_skill_tuslari: vec![],
            olum_skill_bekleme: 3,
            olum_binek_aktif: false,
            olum_binek_tusu: String::new(),
            olum_binek_bekleme: 5,
            captcha_buton_x1: 0,
            captcha_buton_y1: 0,
            captcha_buton_x2: 0,
            captcha_buton_y2: 0,
            // Auto-PM AI varsayılan değerleri
            pm_ai_aktif: false,
            pm_ai_backend: "gemini".to_string(),
            pm_ai_api_key: String::new(),
            pm_system_prompt: "Sen Metin2 oynayan bir oyuncusun. Gelen özel mesajlara kısa, samimi ve Türkçe yanıt ver. Maksimum 2 cümle.".to_string(),
            pm_region: (0, 0, 0, 0),
            pm_cooldown_sn: 30,
            pm_daily_limit: 200,
        }
    }
}
