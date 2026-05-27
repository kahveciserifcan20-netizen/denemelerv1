// pm_ai.rs — AI Destekli Otomatik PM Yanıt Sistemi
// OpenAI GPT, Google Gemini ve yerel Ollama desteği
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};
use winapi::shared::windef::HWND;

// ── AI Backend ──────────────────────────────────────────────────────────────
#[derive(Clone, Debug, PartialEq)]
pub enum AiBackend {
    OpenAI { api_key: String },
    Gemini { api_key: String },
    Ollama { model: String },
}

impl AiBackend {
    pub fn from_config(backend: &str, api_key: &str) -> Self {
        match backend {
            "openai" => AiBackend::OpenAI { api_key: api_key.to_string() },
            "ollama" => AiBackend::Ollama { model: "llama3.2:3b".to_string() },
            _ => AiBackend::Gemini { api_key: api_key.to_string() },
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiBackend::OpenAI { .. } => "OpenAI GPT-4o-mini",
            AiBackend::Gemini { .. } => "Google Gemini Flash",
            AiBackend::Ollama { .. } => "Yerel Ollama",
        }
    }
}

// ── PM Kaydı ─────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct PmRecord {
    pub sender: String,
    pub message: String,
    pub reply: String,
    pub timestamp: Instant,
    pub success: bool,
}

// ── PM AI Engine ─────────────────────────────────────────────────────────────
pub struct PmAiEngine {
    pub backend: AiBackend,
    pub system_prompt: String,
    pub cooldown: Duration,
    pub daily_limit: u32,
    sender_history: HashMap<String, Instant>,
    pub daily_count: u32,
    daily_reset: Instant,
    pub recent_pms: Vec<PmRecord>,
}

impl PmAiEngine {
    pub fn new(backend: AiBackend, system_prompt: String, cooldown_secs: u64, daily_limit: u32) -> Self {
        Self {
            backend,
            system_prompt,
            cooldown: Duration::from_secs(cooldown_secs),
            daily_limit,
            sender_history: HashMap::new(),
            daily_count: 0,
            daily_reset: Instant::now(),
            recent_pms: Vec::new(),
        }
    }

    /// Günlük sayacı sıfırla (24 saatte bir)
    fn maybe_reset_daily(&mut self) {
        if self.daily_reset.elapsed() >= Duration::from_secs(86400) {
            self.daily_count = 0;
            self.daily_reset = Instant::now();
        }
    }

    /// Bu göndericiye şu an yanıt verilebilir mi?
    pub fn can_reply(&mut self, sender: &str) -> Result<(), &'static str> {
        self.maybe_reset_daily();
        if self.daily_limit > 0 && self.daily_count >= self.daily_limit {
            return Err("Günlük PM limiti doldu");
        }
        if let Some(&last) = self.sender_history.get(sender) {
            if last.elapsed() < self.cooldown {
                return Err("Cooldown: Aynı kişiye çok sık yanıt");
            }
        }
        Ok(())
    }

    /// AI'dan yanıt al (blocking — ayrı thread'de çağrılmalı)
    /// raw_ocr_text: Ham OCR çıktısı (parse edilememiş birleşik metin olabilir)
    pub fn get_reply(&mut self, sender: &str, message: &str, raw_ocr_text: Option<&str>) -> Option<String> {
        if let Err(_) = self.can_reply(sender) {
            return None;
        }

        // Eğer ham OCR metni varsa ve parse edilmiş mesajdan farklıysa, AI'a çözümlemesi için gönder
        let prompt = if let Some(raw) = raw_ocr_text {
            format!(
                "{}\n\nGelen özel mesaj (Gönderen: {})\n\n\
                OCR'dan okunan HAM metin (birleşik/bozuk olabilir): '{}'\n\n\
                Parse edilmiş mesaj: '{}'\n\n\
                Lütfen HAM metni analiz ederek gönderenin ne demek istediğini anla ve buna göre cevap ver. \
                Eğer parse edilmiş mesaj anlamsızsa veya boşsa, HAM metinden anlam çıkarmaya çalış. \
                Türkçe karakterler bozuk olabilir (örn: 'i' yerine 'ı', 'g' yerine 'ğ'). \
                Kısa, samimi ve doğal bir cevap ver (max 2 cümle).",
                self.system_prompt, sender, raw, message
            )
        } else {
            format!(
                "{}\n\nGelen özel mesaj (Gönderen: {}): {}",
                self.system_prompt, sender, message
            )
        };

        let reply = match &self.backend.clone() {
            AiBackend::OpenAI { api_key } => ask_openai(api_key, &self.system_prompt, &prompt),
            AiBackend::Gemini { api_key }  => ask_gemini(api_key, &prompt),
            AiBackend::Ollama { model }    => ask_ollama(model, &prompt),
        };

        if let Some(ref r) = reply {
            self.sender_history.insert(sender.to_string(), Instant::now());
            self.daily_count += 1;
            // Son 50 PM kaydı tut
            if self.recent_pms.len() >= 50 { self.recent_pms.remove(0); }
            self.recent_pms.push(PmRecord {
                sender: sender.to_string(),
                message: message.to_string(),
                reply: r.clone(),
                timestamp: Instant::now(),
                success: true,
            });
        }

        reply
    }

    pub fn unique_senders(&self) -> usize { self.sender_history.len() }
}

// ── OpenAI GPT-4o-mini ───────────────────────────────────────────────────────
fn ask_openai(api_key: &str, system_prompt: &str, user_message: &str) -> Option<String> {
    if api_key.is_empty() { return None; }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build().ok()?;

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user",   "content": user_message}
        ],
        "max_tokens": 120,
        "temperature": 0.7
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send().ok()?;

    let json: serde_json::Value = resp.json().ok()?;
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
}

// ── Google Gemini Flash (Ücretsiz tier: 15 RPM) ──────────────────────────────
fn ask_gemini(api_key: &str, prompt: &str) -> Option<String> {
    match ask_gemini_detailed(api_key, prompt) {
        Ok(r)  => Some(r),
        Err(_) => None,
    }
}

/// Detaylı hata mesajıyla Gemini çağrısı (test için)
pub fn ask_gemini_detailed(api_key: &str, prompt: &str) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API key boş".to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP client hatası: {}", e))?;

    // Bu API key'inde mevcut modeller (v1beta) — sırayla dene
    // gemini-1.5-flash bu key'de mevcut DEĞİL
    let models: &[(&str, &str)] = &[
        ("gemini-2.5-flash",      "v1beta"),  // En güncel, ücretsiz tier
        ("gemini-2.0-flash-lite", "v1beta"),  // Hafif, ücretsiz
        ("gemini-flash-latest",   "v1beta"),  // Genel alias
        ("gemini-2.0-flash",      "v1beta"),  // Yedek (ücretli olabilir)
    ];

    let mut errors: Vec<String> = Vec::new();

    for (model, api_ver) in models {
        let url = format!(
            "https://generativelanguage.googleapis.com/{}/models/{}:generateContent?key={}",
            api_ver, model, api_key
        );
        let body = serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {"maxOutputTokens": 150, "temperature": 0.7}
        });

        let resp = match client.post(&url).json(&body).send() {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{}: ağ hatası: {}", model, e));
                continue;
            }
        };

        let status = resp.status();
        let text = resp.text().map_err(|e| format!("Yanıt okunamadı: {}", e))?;

        if !status.is_success() {
            let api_err = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|j| j["error"]["message"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| text.chars().take(150).collect());
            errors.push(format!("{}: HTTP {} — {}", model, status.as_u16(),
                api_err.chars().take(120).collect::<String>()));
            continue;
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON parse hatası: {}", e))?;

        if let Some(s) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            return Ok(s.trim().to_string());
        }
        errors.push(format!("{}: yanıt yapısı beklenenden farklı", model));
    }

    let err_summary = if errors.is_empty() {
        "Bilinmeyen hata".to_string()
    } else {
        errors.join(" | ")
    };
    Err(err_summary)
}

// ── Yerel Ollama (Tamamen ücretsiz, offline) ─────────────────────────────────
fn ask_ollama(model: &str, prompt: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build().ok()?;

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    let resp = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send().ok()?;

    let json: serde_json::Value = resp.json().ok()?;
    json["response"].as_str().map(|s| s.trim().to_string())
}

// ── PM Metin Çıkarımı (OCR sonucundan PM parse) ──────────────────────────────
/// OCR ile okunan metinden PM gönderen ve içeriğini çıkarır.
/// Metin2 PM formatı: "[Kullanıcı]: mesaj" veya "Kullanıcı > mesaj"
/// 
/// v3: OCR HATA TESPITI + Fuzzy Parsing
/// "steuijetarett" gibi anlamsız metinleri tespit et ve reddet
pub fn extract_pm_from_ocr(ocr_text: &str) -> Option<(String, String)> {
    let text = ocr_text.trim();
    if text.is_empty() { return None; }
    
    // v3: OCR HATA TESPITI - Anlamsız uzun tek kelime mi?
    // "steuijetarett" gibi - boşluk yok, çok uzun, anlamsız
    let has_whitespace = text.chars().any(|c| c.is_whitespace());
    let has_separator = text.contains(':') || text.contains('>') || text.contains('-') || text.contains(']');
    
    // Eğer boşluk yok VE ayırıcı yok VE uzunsa = OCR hatası
    if !has_whitespace && !has_separator && text.len() > 10 {
        // Fuzzy parse dene - belki ayırıcı karakter OCR'da kaybolmuştur
        // Örn: "Steuijetarett" -> "Steuije" + "t" + "arett" (t = :)
        return try_fuzzy_pm_parse(text);
    }
    
    // v3: Çok kısa metinleri reddet
    if text.len() < 5 {
        return None;
    }
    
    // v3: Sadece küçük harflerden oluşan basit metinleri reddet
    let lowercase_ratio = text.chars().filter(|c| c.is_lowercase()).count() as f32 / text.len() as f32;
    if lowercase_ratio > 0.95 && text.len() < 15 {
        return None;
    }

    // Format 1: [OyuncuAdı]: mesaj (EN GÜVENİLİR)
    if text.starts_with('[') {
        if let Some(close) = text.find(']') {
            let sender = text[1..close].trim().to_string();
            let rest = text[close + 1..].trim_start_matches(':').trim().to_string();
            
            if sender.len() >= 2 && sender.len() <= 20 &&
               !rest.is_empty() && rest.len() >= 2 {
                return Some((sender, rest));
            }
        }
    }

    // Format 2: "Kullanıcı: mesaj" (iki nokta üst üste ile)
    let separators = [":", " :", ">", " -", "- ", "»", "›"];
    for sep in &separators {
        if let Some(idx) = text.find(sep) {
            let sender = text[..idx].trim().to_string();
            let after = text[idx + sep.len()..].trim().to_string();
            
            if sender.len() >= 2 && sender.len() <= 20 &&
               after.len() >= 2 && after.len() <= 200 {
                // Gönderici adı anlamlı mı? (en az 2 harf/rakam)
                let sender_alnum = sender.chars().filter(|c| c.is_alphanumeric()).count();
                if sender_alnum >= 2 {
                    return Some((sender, after));
                }
            }
        }
    }

    // Format 3: PM: veya özel mesaj: içeren satırlar
    let pm_keywords = ["PM:", "pm:", "Özel:", "özel:", "Private:", "private:"];
    for kw in &pm_keywords {
        if let Some(idx) = text.find(kw) {
            let after = text[idx + kw.len()..].trim().to_string();
            if after.len() >= 3 {
                return Some(("Bilinmeyen".to_string(), after));
            }
        }
    }

    // v3: Fuzzy parse son çare
    try_fuzzy_pm_parse(text)
}

/// Fuzzy PM Parse - OCR hatalarını düzeltmeye çalış
/// "steuijetarett" -> "Steuije" + "t" (ayırıcı) + "arett" (naber kanka?)
fn try_fuzzy_pm_parse(text: &str) -> Option<(String, String)> {
    // Eğer metin çok uzun ve tek kelimeyse, belki ayırıcı kaybolmuştur
    // "Steuijetarett" -> "Steuije" (6 harf) + "t" + "arett"
    // veya "Steuije" + "tarett" -> "Steuije" + "naber kanka" (OCR hatası)
    
    if text.len() < 8 || text.len() > 40 {
        return None;
    }
    
    // Büyük harfle başlayan kısmı gönderici adı olarak al
    // "Steuijetarett" -> "Steuije" kısmı büyük harfle başlar
    let chars: Vec<char> = text.chars().collect();
    
    // İlk büyük harf konumunu bul
    let mut sender_end = 0;
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && c.is_uppercase() {
            // Büyük harf bulundu, bu gönderici adının sonu olabilir
            // Ama "Steuije" -> "S" büyük, gerisi küçük
            sender_end = i;
            break;
        }
    }
    
    // Eğer büyük harf bulunamadıysa, ilk 6-12 karakteri dene
    if sender_end == 0 {
        // İsim genelde 4-12 karakter
        for len in (4..=12).rev() {
            if len < chars.len() {
                let sender: String = chars[..len].iter().collect();
                let msg: String = chars[len..].iter().collect();
                
                // Mesaj anlamlı mı? (en az 3 karakter, harf içermeli)
                if msg.len() >= 3 && msg.chars().any(|c| c.is_alphabetic()) {
                    return Some((sender, msg));
                }
            }
        }
    } else {
        // Büyük harf bulundu, gönderici adı burada bitiyor olabilir
        let sender: String = chars[..sender_end].iter().collect();
        let msg: String = chars[sender_end..].iter().collect();
        
        if sender.len() >= 2 && sender.len() <= 20 && msg.len() >= 3 {
            return Some((sender, msg));
        }
    }
    
    None
}

// ── PM Cevabını Oyuna Yazma ─────────────────────────────────────────────────
/// PM cevabını Metin2'ye insansı bir şekilde yazar
/// 1. PM kutusuna tıklar (varsa)
/// 2. Cevabı yazar (insansı gecikmelerle)
/// 3. Enter'a basar
pub fn send_pm_reply(
    hwnd: HWND,
    hw_sim: &crate::hw_simulator::HwSimulator,
    reply_text: &str,
    pm_region: (i32, i32, i32, i32),
) -> bool {
    use std::thread;
    use std::time::Duration;
    use rand::Rng;
    
    if hwnd.is_null() || reply_text.is_empty() {
        return false;
    }
    
    let (pm_x1, pm_y1, pm_x2, pm_y2) = pm_region;
    if pm_x2 <= pm_x1 || pm_y2 <= pm_y1 {
        return false; // Geçersiz bölge
    }
    
    // PM kutusunun ortasına tıkla (PM bölgesinin ortası)
    let pm_center_x = pm_x1 + (pm_x2 - pm_x1) / 2;
    let pm_center_y = pm_y1 + (pm_y2 - pm_y1) / 2;
    
    // 1. PM kutusuna insansı hareketle tıkla
    let (screen_x, screen_y) = hw_sim.client_to_screen(hwnd, pm_center_x, pm_center_y);
    hw_sim.human_move(screen_x, screen_y);
    thread::sleep(Duration::from_millis(200 + rand::thread_rng().gen_range(50..150)));
    hw_sim.background_click_mode(hwnd, pm_center_x, pm_center_y, &crate::hw_simulator::ClickMode::FocusSwap);
    thread::sleep(Duration::from_millis(300 + rand::thread_rng().gen_range(100..200)));
    
    // 2. Cevap yazmadan önce kısa bekle (insansı)
    thread::sleep(Duration::from_millis(500 + rand::thread_rng().gen_range(200..500)));
    
    // 3. Cevabı karakter karakter yaz (insansı gecikmelerle)
    let chars: Vec<char> = reply_text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        // Her karakter için rastgele gecikme (tipik insan yazma hızı: 80-150 WPM)
        let delay_ms = if *c == ' ' {
            // Boşluk tuşu için daha kısa gecikme
            rand::thread_rng().gen_range(30..80)
        } else {
            // Normal karakterler için 50-150ms
            rand::thread_rng().gen_range(50..150)
        };
        
        // Özel karakterler için farklı işleme
        let vk = char_to_vk(*c);
        if vk > 0 {
            hw_sim.background_key_press(hwnd, vk);
        }
        
        thread::sleep(Duration::from_millis(delay_ms));
        
        // Her 5 karakterde bir daha uzun duraklama (insansı)
        if i > 0 && i % 5 == 0 && rand::thread_rng().gen_bool(0.3) {
            thread::sleep(Duration::from_millis(100 + rand::thread_rng().gen_range(50..200)));
        }
    }
    
    // 4. Göndermeden önce kısa bekle
    thread::sleep(Duration::from_millis(300 + rand::thread_rng().gen_range(100..300)));
    
    // 5. Enter'a bas (gönder)
    hw_sim.background_key_press(hwnd, 0x0D); // VK_RETURN = 0x0D
    
    true
}

/// Karakteri Windows Virtual Key Code'a çevir
fn char_to_vk(c: char) -> u16 {
    match c {
        'a'..='z' => (c as u16 - 'a' as u16) + 0x41, // A-Z = 0x41-0x5A
        'A'..='Z' => (c as u16 - 'A' as u16) + 0x41,
        '0'..='9' => (c as u16 - '0' as u16) + 0x30, // 0-9 = 0x30-0x39
        ' ' => 0x20, // VK_SPACE
        '.' => 0xBE, // VK_OEM_PERIOD
        ',' => 0xBC, // VK_OEM_COMMA
        ';' => 0xBA, // VK_OEM_1
        '/' => 0xBF, // VK_OEM_2
        '`' => 0xC0, // VK_OEM_3
        '[' => 0xDB, // VK_OEM_4
        '\\' => 0xDC, // VK_OEM_5
        ']' => 0xDD, // VK_OEM_6
        '\'' => 0xDE, // VK_OEM_7
        '-' => 0xBD, // VK_OEM_MINUS
        '=' => 0xBB, // VK_OEM_PLUS
        _ => {
            // Türkçe karakterler ve diğerleri için basit mapping
            match c {
                'ç' | 'Ç' => 0x43, // C olarak gönder
                'ğ' | 'Ğ' => 0x47, // G olarak gönder
                'ı' | 'İ' => 0x49, // I olarak gönder
                'ö' | 'Ö' => 0x4F, // O olarak gönder
                'ş' | 'Ş' => 0x53, // S olarak gönder
                'ü' | 'Ü' => 0x55, // U olarak gönder
                _ => 0x00, // Bilinmeyen karakter
            }
        }
    }
}

// ── PM İşleyici (Client Runner'da kullanılır) ───────────────────────────────
/// PM AI işleyici - OCR sonucunu alır, AI'dan cevap alır ve oyuna yazar
/// 
/// YENİ: Parse başarısız olsa bile ham OCR metnini AI'a gönderir.
/// AI hem çözümleyip hem cevap verir.
pub fn process_pm_with_ai(
    hwnd: HWND,
    hw_sim: &crate::hw_simulator::HwSimulator,
    ocr_text: &str,
    pm_engine: &mut PmAiEngine,
    pm_region: (i32, i32, i32, i32),
    log_tx: &crossbeam_channel::Sender<String>,
    client_id: usize,
) -> bool {
    use chrono::Local;
    
    // Önce parse etmeyi dene
    let parse_result = extract_pm_from_ocr(ocr_text);
    
    let (sender, message) = if let Some((s, m)) = parse_result {
        // Başarılı parse
        let _ = log_tx.send(format!(
            "[{}] 💬 [C{}] PM Parse edildi - {}: {}",
            Local::now().format("%H:%M:%S.%3f"),
            client_id,
            s,
            m.chars().take(40).collect::<String>()
        ));
        (s, m)
    } else {
        // Parse başarısız - AI ham metni çözümleyecek
        let _ = log_tx.send(format!(
            "[{}] 🔍 [C{}] PM Parse edilemedi, AI çözümleyecek: '{}'",
            Local::now().format("%H:%M:%S.%3f"),
            client_id,
            ocr_text.chars().take(50).collect::<String>()
        ));
        ("Oyuncu".to_string(), ocr_text.to_string())
    };
    
    // AI'dan cevap al - HER ZAMAN ham OCR metnini de gönder
    if let Some(reply) = pm_engine.get_reply(&sender, &message, Some(ocr_text)) {
        let _ = log_tx.send(format!(
            "[{}] 🤖 [C{}] AI Cevabı: {}",
            Local::now().format("%H:%M:%S.%3f"),
            client_id,
            reply.chars().take(50).collect::<String>()
        ));
        
        // Cevabı oyuna yaz
        if send_pm_reply(hwnd, hw_sim, &reply, pm_region) {
            let _ = log_tx.send(format!(
                "[{}] ✅ [C{}] PM Cevabı gönderildi: {}",
                Local::now().format("%H:%M:%S.%3f"),
                client_id,
                sender
            ));
            true
        } else {
            let _ = log_tx.send(format!(
                "[{}] ❌ [C{}] PM Cevabı gönderilemedi!",
                Local::now().format("%H:%M:%S.%3f"),
                client_id
            ));
            false
        }
    } else {
        let _ = log_tx.send(format!(
            "[{}] ⏸️ [C{}] PM yanıtlanmadı (cooldown/limit): {}",
            Local::now().format("%H:%M:%S.%3f"),
            client_id,
            sender
        ));
        false
    }
}
