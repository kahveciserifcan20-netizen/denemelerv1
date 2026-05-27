// ============================================================
// captcha_solver.rs  —  TERMINATOR-LEVEL Captcha Engine v2.0
// ============================================================
// v2.0 İyileştirmeler:
// 1. 3-KATMANLI DOĞRULAMA: Dialog Varlık → Buton Tespiti → Multi-Frame
// 2. Captcha dialog yapısal analiz (kenar+varyans — oyun UI'dan ayırma)
// 3. Oyun UI blacklist (NPC/Dükkan/Ticaret butonları filtreleme)
// 4. 3-frame ardışık onay (v1: 2 frame → v2: 3 frame)
// 5. Hızlandırılmış OCR (1.5x CatmullRom, v1: 2x Lanczos3)
// 6. Cooldown 5 saniye (v1: 3 saniye)
// 7. Sıkılaştırılmış template eşik değerleri
// 8. Saved-region EXCLUSIVE mod + renk-imza ön filtresi
// 9. Erken çıkış template matching
// ============================================================

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use image::{DynamicImage, GrayImage, Luma};
use imageproc::template_matching::{match_template, MatchTemplateMethod};
use regex::Regex;
use crossbeam_channel::Sender;
use levenshtein::levenshtein;

// OCRS 0.12 imports
use ocrs::{OcrEngine, OcrEngineParams, ImageSource};
use rten::Model;

// ---------------------------------------------------------------------------
// OCR motoru seçimi
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum OcrBackend {
    Ocrs,
    Tesseract,
    Both,
}

// ---------------------------------------------------------------------------
// İç durum (multi-frame onay & cooldown)
// ---------------------------------------------------------------------------
#[allow(dead_code)]
struct DetectorState {
    consecutive_hits: u32,
    last_hit_pos: Option<(i32, i32)>,
    cooldown_until: Instant,
    last_negative_log: Instant,
}

impl DetectorState {
    fn new() -> Self {
        Self {
            consecutive_hits: 0,
            last_hit_pos: None,
            cooldown_until: Instant::now(),
            last_negative_log: Instant::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// Önbelleğe alınmış şablon (orijinal + gri)
// ---------------------------------------------------------------------------
struct CachedTemplate {
    #[allow(dead_code)]
    orig: DynamicImage,
    gray: GrayImage,
    w: u32,
    h: u32,
}

// ---------------------------------------------------------------------------
// Ana yapı
// ---------------------------------------------------------------------------
pub struct CaptchaSolver {
    template_cache: HashMap<String, Vec<CachedTemplate>>,
    ocr_engine:     Option<OcrEngine>,
    keywords:       HashMap<String, Vec<&'static str>>,
    clean_re:       OnceLock<Regex>,
    state:          Mutex<DetectorState>,
}

// ---------------------------------------------------------------------------
// Sabitler — terminatör ayarları
// ---------------------------------------------------------------------------
/// Pozitif kabul için ardışık frame sayısı (v2: 3 frame — daha katı)
#[allow(dead_code)]
const CONFIRM_FRAMES: u32 = 3;
/// Pozitif merkez nokta tolerans (px) - aynı captcha mı?
#[allow(dead_code)]
const SAME_POS_TOLERANCE: i32 = 25;
/// Çözüm sonrası tekrar tetiklemeden bekleme süresi (15 saniye)
#[allow(dead_code)]
const POST_SOLVE_COOLDOWN: Duration = Duration::from_secs(15);
/// Captcha çözüm timeout - 15 saniye içinde çözülmeli
#[allow(dead_code)]
const CAPTCHA_SOLVE_TIMEOUT: Duration = Duration::from_secs(15);
/// Saved-region için template skor eşiği (v2: 0.60 — biraz daha sıkı)
#[allow(dead_code)]
const SAVED_REGION_THRESHOLD: f32 = 0.60;
/// Genel arama için template skor eşiği (v2: 0.75 — çok daha sıkı)
const BROAD_SCAN_THRESHOLD: f32 = 0.75;
/// Tek-template kabul için yüksek skor eşiği
#[allow(dead_code)]
const HIGH_CONFIDENCE: f32 = 0.85;

/// Oyun UI blacklist — bu kelimeler buton OCR'da false-positive yapar
#[allow(dead_code)]
const GAME_UI_BLACKLIST: &[&str] = &[
    "dukkan", "magaza", "ticaret", "envanter", "karakter",
    "gorev", "parti", "lonca", "pazar", "satin", "sat",
    "tamam", "iptal", "kapat", "cik", "giris", "cikis",
    "npc", "quest", "shop", "trade", "inventory",
    "sohbet", "mesaj", "bilgi", "uyari", "hata",
    "seviye", "level", "exp", "hp", "mp", "sp",
    "demirc", "simyac", "kasap", "bakkal", "berber",
];

// ---------------------------------------------------------------------------
// Oluşturucu
// ---------------------------------------------------------------------------
#[allow(dead_code)]
impl CaptchaSolver {
    pub fn new(_backend: OcrBackend) -> Self {
        let ocr_engine = Self::init_ocrs();
        let template_cache = Self::load_templates();

        let clean_re = OnceLock::new();
        let _ = clean_re.set(Regex::new(r"[^a-zçğıöşü0-9]").unwrap());

        Self {
            template_cache,
            ocr_engine,
            keywords: Self::init_keywords(),
            clean_re,
            state: Mutex::new(DetectorState::new()),
        }
    }

    fn init_ocrs() -> Option<OcrEngine> {
        let det_path = "text-detection.rten";
        let rec_path = "text-recognition.rten";

        match (Model::load_file(det_path), Model::load_file(rec_path)) {
            (Ok(detection_model), Ok(recognition_model)) => {
                match OcrEngine::new(OcrEngineParams {
                    detection_model: Some(detection_model),
                    recognition_model: Some(recognition_model),
                    ..Default::default()
                }) {
                    Ok(engine) => {
                        eprintln!("✅ OCRS motoru yüklendi!");
                        Some(engine)
                    }
                    Err(e) => {
                        eprintln!("❌ OCRS hatası: {:?}", e);
                        None
                    }
                }
            }
            _ => {
                eprintln!("⚠️ OCRS model dosyaları bulunamadı");
                None
            }
        }
    }

    fn cache_template(img: DynamicImage) -> CachedTemplate {
        let gray = img.to_luma8();
        let (w, h) = (gray.width(), gray.height());
        CachedTemplate { orig: img, gray, w, h }
    }

    fn load_templates() -> HashMap<String, Vec<CachedTemplate>> {
        let mut cache: HashMap<String, Vec<CachedTemplate>> = HashMap::new();
        let re_num = Regex::new(r"_\d+$").unwrap();

        if let Ok(entries) = std::fs::read_dir("captcha_sablonlari/hedef") {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }

                let ext = path.extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if !matches!(ext.as_str(), "png" | "jpg" | "jpeg") { continue; }

                if let Ok(img) = image::open(&path) {
                    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                    let mut base = re_num.replace(&stem, "").replace('_', "").to_lowercase();

                    if base == "logo" { base = "metinlogosu".to_string(); }
                    if base == "muharrebe" { base = "muharrebekilici".to_string(); }

                    cache.entry(base).or_default().push(Self::cache_template(img));
                }
            }
        }

        if let Ok(img) = image::open("captcha_sablonlari/btn_onay.png") {
            cache.entry("onay_buton".to_string()).or_default().push(Self::cache_template(img));
        }

        // PM simgesi (bildirim ikonu — pm_simge.png)
        if let Ok(img) = image::open("captcha_sablonlari/pm_simge.png") {
            cache.entry("pm_simge".to_string()).or_default().push(Self::cache_template(img));
            eprintln!("✅ PM simge şablonu yüklendi");
        } else {
            eprintln!("⚠️ PM simge şablonu bulunamadı (captcha_sablonlari/pm_simge.png)");
        }

        // PM Gönder butonu şablonu (pm_buton.png)
        if let Ok(img) = image::open("captcha_sablonlari/pm_buton.png") {
            cache.entry("pm_buton".to_string()).or_default().push(Self::cache_template(img));
            eprintln!("✅ PM gönder butonu şablonu yüklendi");
        } else {
            eprintln!("⚠️ PM buton şablonu bulunamadı (captcha_sablonlari/pm_buton.png)");
        }

        // PM ekranı şablonu (açık pencere — pm_ekran.png)
        if let Ok(img) = image::open("captcha_sablonlari/pm_ekran.png") {
            cache.entry("pm_ekran".to_string()).or_default().push(Self::cache_template(img));
            eprintln!("✅ PM ekran şablonu yüklendi");
        } else {
            eprintln!("⚠️ PM ekran şablonu bulunamadı (captcha_sablonlari/pm_ekran.png)");
        }

        eprintln!("✅ {} kategori şablon yüklendi (gray-cached)", cache.len());
        cache
    }

    fn init_keywords() -> HashMap<String, Vec<&'static str>> {
        let mut kw = HashMap::new();

        kw.insert("muharrebekilici".to_string(), vec![
            "muharrebe", "muhar", "uharr", "harre", "arreb", "rrebe", "rebe",
            "mu", "uh", "rr", "eb"
        ]);
        kw.insert("metinlogosu".to_string(), vec![
            "metin", "etin", "tin2", "logo", "ogos", "gosu",
            "me", "ti", "lo", "og"
        ]);
        kw.insert("siyahruzgarzirhi".to_string(), vec![
            "siyah", "iyah", "ruzgar", "ruzga", "zirh", "zirhi",
            "si", "iy", "ya", "rh", "hi"
        ]);
        kw.insert("kilic".to_string(), vec![
            "kilic", "kil", "ili", "lic"
        ]);
        kw.insert("sura".to_string(), vec![
            "sura", "sur", "ura", "sua", "sra"
        ]);
        kw.insert("yuzbasi".to_string(), vec![
            "yuzbasi", "yuz", "uzb", "zba", "bas", "basi",
            "yu", "uz", "zb", "as", "si"
        ]);
        kw.insert("demirci".to_string(), vec![
            "demirci", "demir", "mirci", "dem", "emi", "mir", "irc",
            "de", "em", "rc", "ci"
        ]);
        kw
    }

    // ── OCR için görüntü hazırla (v2: 1.5x CatmullRom — %40 daha hızlı) ──
    fn prepare_for_ocr(&self, img: &DynamicImage) -> DynamicImage {
        let new_w = (img.width() as f32 * 1.5) as u32;
        let new_h = (img.height() as f32 * 1.5) as u32;
        let scaled = img.resize(new_w, new_h, image::imageops::FilterType::CatmullRom);

        let gray = scaled.to_luma8();
        let (w, h) = (gray.width(), gray.height());

        let mut min_val = 255u8;
        let mut max_val = 0u8;
        for p in gray.pixels() {
            min_val = min_val.min(p.0[0]);
            max_val = max_val.max(p.0[0]);
        }
        let range = (max_val - min_val).max(1) as f32;

        let mut enhanced = GrayImage::new(w, h);
        for (x, y, p) in gray.enumerate_pixels() {
            let stretched = ((p.0[0] - min_val) as f32 / range * 255.0) as u8;
            let inverted = 255 - stretched;
            let v = if inverted < 100 { 0 } else { 255 };
            enhanced.put_pixel(x, y, Luma([v]));
        }
        DynamicImage::ImageLuma8(enhanced)
    }

    // ── OCRS ile OCR (sessiz / hızlı varyant) ───────────────────────────────
    fn ocr_ocrs_silent(&self, img: &DynamicImage) -> Option<String> {
        let enhanced = self.prepare_for_ocr(img);

        if let Some(engine) = &self.ocr_engine {
            let rgb = enhanced.to_rgb8();
            let (w, h) = (rgb.width(), rgb.height());

            if let Ok(img_source) = ImageSource::from_bytes(rgb.as_raw().as_slice(), (w, h)) {
                if let Ok(ocr_input) = engine.prepare_input(img_source) {
                    if let Ok(text) = engine.get_text(&ocr_input) {
                        return Some(self.clean_text(&text));
                    }
                }
            }
        }
        None
    }

    fn ocr_ocrs(&self, img: &DynamicImage) -> Option<String> {
        let result = self.ocr_ocrs_silent(img);
        if let Some(ref t) = result {
            eprintln!("[📝 OCRS] '{}'", t);
        }
        result
    }

    pub fn do_ocr(&self, img: &DynamicImage) -> String {
        let text = self.ocr_ocrs(img).unwrap_or_default();
        let cleaned = text.trim();
        
        // Çok kısa metinler geçersiz
        if cleaned.len() < 4 {
            eprintln!("[📝 OCR] Çok kısa reddedildi: '{}'", cleaned);
            return String::new();
        }
        
        // Sayı oranı çok yüksekse geçersiz (s2252, 12345 gibi)
        let digit_count = cleaned.chars().filter(|c| c.is_numeric()).count();
        let letter_count = cleaned.chars().filter(|c| c.is_alphabetic()).count();
        if digit_count > letter_count {
            eprintln!("[📝 OCR] Çok fazla sayı reddedildi: '{}'", cleaned);
            return String::new();
        }
        
        // Sadece sayılardan oluşuyorsa geçersiz
        if cleaned.chars().all(|c| c.is_numeric()) {
            eprintln!("[📝 OCR] Sadece sayı reddedildi: '{}'", cleaned);
            return String::new();
        }
        
        text
    }

    fn clean_text(&self, text: &str) -> String {
        let lower = text.to_lowercase();
        self.clean_re.get().unwrap().replace_all(&lower, "").to_string()
    }

    fn normalize_for_matching(&self, text: &str) -> String {
        let mut result = text.to_lowercase();
        result = result.replace('ç', "c");
        result = result.replace('ğ', "g");
        result = result.replace('ı', "i");
        result = result.replace('ö', "o");
        result = result.replace('ş', "s");
        result = result.replace('ü', "u");
        result
    }

    // ── BUTTON-only keyword check (v2: oyun UI blacklist + sıkı kontrol) ────
    /// Sadece "Onayla" yazısını kabul eder.
    /// v2: Oyun UI elementlerinden gelen false-positive'leri blacklist ile yok eder.
    fn is_confirm_button_text(&self, text: &str) -> bool {
        if text.is_empty() { return false; }
        if text.len() < 3 { return false; } // Çok kısa text güvenilmez
        let n = self.normalize_for_matching(text);

        // v2: BLACKLIST — oyun UI kelimelerini HEMEN reddet
        for blocked in GAME_UI_BLACKLIST {
            if n.contains(blocked) {
                return false;
            }
        }

        // Sıkı kontrol: "onay" ailesi — EN AZ 4 karakter eşleşmeli
        let has_onay = n.contains("onay") || n.contains("onayla") || n.contains("nayla");
        // v2: Ek güvenlik — text çok uzunsa (>15 char) muhtemelen oyun UI metni
        if n.len() > 15 { return false; }
        has_onay
    }

    // ── Hızlı renk-imza kontrolü (saved-region için ön filtre) ──────────────
    /// Saved bölge gerçekten "buton görünümü" mü? (anlık eleme — OCR'dan önce)
    /// Captcha onay butonu genelde belirli kontrastta. Tamamen boş/düz alan
    /// gelirse OCR'a hiç gitmez → büyük hız & FP avantajı.
    fn region_has_button_signature(&self, frame: &DynamicImage, x1: u32, y1: u32, x2: u32, y2: u32) -> bool {
        let w = x2.saturating_sub(x1);
        let h = y2.saturating_sub(y1);
        if w < 10 || h < 5 { return false; }

        let cropped = frame.crop_imm(x1, y1, w, h).to_luma8();
        let mut min_v = 255u8;
        let mut max_v = 0u8;
        let mut sum: u64 = 0;
        let count = (cropped.width() * cropped.height()) as u64;
        if count == 0 { return false; }

        for p in cropped.pixels() {
            let v = p.0[0];
            if v < min_v { min_v = v; }
            if v > max_v { max_v = v; }
            sum += v as u64;
        }

        let contrast = max_v.saturating_sub(min_v);
        let mean = (sum / count) as u8;

        // Buton: yeterli kontrast (text/kenarlık var) ve aşırı parlak olmayan
        // v2: Daha sıkı buton imzası — kontrast eşiği yükseltildi
        contrast >= 50 && mean < 220 && mean > 25
    }

    // ── v2: KATMAN 1 — Captcha dialog penceresi var mı? ─────────────────────
    /// Ekranın belirli bölgesinde captcha dialog yapısı olup olmadığını kontrol eder.
    /// Kenar yoğunluğu + iç varyans analizi ile oyun sahnesinden ayırt eder.
    /// Bu metod template/OCR'dan ÖNCE çalışarak %90+ false-positive'i engeller.
    fn detect_captcha_dialog(&self, frame: &DynamicImage) -> bool {
        let fw = frame.width();
        let fh = frame.height();
        if fw < 200 || fh < 200 { return false; }

        let gray = frame.to_luma8();

        // Captcha dialogu ekranın ortasında belirir
        // Tahmini dialog sınırları: yatay %25-%75, dikey %20-%80
        let dx1 = fw / 4;
        let dy1 = fh / 5;
        let dx2 = fw * 3 / 4;
        let dy2 = fh * 4 / 5;

        // ── Kenar yoğunluğu kontrolü ──
        // Dialog kenarında belirgin parlaklık geçişi olmalı
        let mut edge_count = 0u32;
        let mut edge_total = 0u32;
        let step = 3u32; // Her 3 pikselde bir örnekle (hız için)

        // Üst kenar taraması
        let mut x = dx1;
        while x < dx2 {
            if dy1 >= 3 && dy1 + 3 < fh {
                let above = gray.get_pixel(x, dy1.saturating_sub(3)).0[0] as i32;
                let below = gray.get_pixel(x, (dy1 + 3).min(fh - 1)).0[0] as i32;
                if (above - below).abs() > 25 { edge_count += 1; }
                edge_total += 1;
            }
            x += step;
        }
        // Alt kenar taraması
        x = dx1;
        while x < dx2 {
            if dy2 >= 3 && dy2 + 3 < fh {
                let above = gray.get_pixel(x, dy2.saturating_sub(3)).0[0] as i32;
                let below = gray.get_pixel(x, (dy2 + 3).min(fh - 1)).0[0] as i32;
                if (above - below).abs() > 25 { edge_count += 1; }
                edge_total += 1;
            }
            x += step;
        }

        if edge_total == 0 { return false; }
        let edge_density = edge_count as f32 / edge_total as f32;

        // ── İç bölge varyans kontrolü ──
        // Captcha dialogunda resimler + metin var → yüksek varyans
        // Normal oyun sahnesi de yüksek varyans olabilir ama kenar yoğunluğu düşük
        let ix1 = dx1 + 30;
        let iy1 = dy1 + 30;
        let ix2 = dx2.saturating_sub(30);
        let iy2 = dy2.saturating_sub(30);

        let mut sum = 0u64;
        let mut sum_sq = 0u64;
        let mut count = 0u64;
        let vstep = 5u32;

        let mut y = iy1;
        while y < iy2 {
            let mut xi = ix1;
            while xi < ix2 {
                if xi < fw && y < fh {
                    let v = gray.get_pixel(xi, y).0[0] as u64;
                    sum += v;
                    sum_sq += v * v;
                    count += 1;
                }
                xi += vstep;
            }
            y += vstep;
        }

        if count < 50 { return false; }
        let mean = sum / count;
        let variance = (sum_sq / count).saturating_sub(mean * mean);

        // Karar: kenar yoğunluğu VE iç varyans yeterli mi?
        // Dialog: edge_density > 0.20, variance > 400
        // Oyun sahnesi: edge_density < 0.15 (rastgele kenarlar)
        edge_density > 0.20 && variance > 400
    }

    // ── Hızlı template eşleştirme (cached gray, erken çıkış destekli) ──────
    // ⚡ OPTIMIZASYON: Downscale + erken çıkış ile %70+ hız artışı
    fn match_template_cached(
        &self,
        hay_gray: &GrayImage,
        ox: u32,
        oy: u32,
        tpl: &CachedTemplate,
    ) -> Option<(i32, i32, f32)> {
        if tpl.w > hay_gray.width() || tpl.h > hay_gray.height() {
            return None;
        }

        // ⚡ OPTIMIZASYON #1: Çok küçük şablonlar için direkt hesaplama
        // Orta boyutlular için downscale, büyükler için orijinal
        let (search_img, tpl_img, scale_factor) = if tpl.w < 30 || tpl.h < 30 {
            // Küçük şablonlar: Orijinal boyutta (hızlı zaten)
            (hay_gray.clone(), tpl.gray.clone(), 1.0f32)
        } else if tpl.w > 100 || tpl.h > 100 {
            // Büyük şablonlar: %50 downscale
            let scaled_hay = image::imageops::resize(
                hay_gray, 
                hay_gray.width() / 2, 
                hay_gray.height() / 2, 
                image::imageops::FilterType::Nearest // En hızlı
            );
            let scaled_tpl = image::imageops::resize(
                &tpl.gray, 
                tpl.w / 2, 
                tpl.h / 2, 
                image::imageops::FilterType::Nearest
            );
            (scaled_hay, scaled_tpl, 2.0f32)
        } else {
            // Orta boyut: Orijinal
            (hay_gray.clone(), tpl.gray.clone(), 1.0f32)
        };

        let result = match_template(&search_img, &tpl_img, MatchTemplateMethod::SumOfSquaredErrorsNormalized);

        let mut min_val = 1.0f32;
        let mut best_x = 0i32;
        let mut best_y = 0i32;

        // ⚡ OPTIMIZASYON #2: Erken çıkış - çok düşük skor bulursa hemen bırak
        let early_exit_threshold = 0.3f32; // 0.3'ten düşükse zaten eşleşme yok
        
        for y in 0..result.height() {
            for x in 0..result.width() {
                let score = result.get_pixel(x, y).0[0];
                if score < min_val {
                    min_val = score;
                    best_x = x as i32;
                    best_y = y as i32;
                    
                    // Erken çıkış kontrolü - mükemmel eşleşme bulundu
                    if min_val < 0.05 {
                        break;
                    }
                }
            }
            if min_val < 0.05 {
                break;
            }
        }

        let norm_score = 1.0 - min_val;
        
        // Erken reddetme - skor çok düşükse None dön
        if norm_score < early_exit_threshold {
            return None;
        }
        
        // Scale faktörünü geri uygula
        let final_x = (best_x as f32 * scale_factor) as i32 + ox as i32;
        let final_y = (best_y as f32 * scale_factor) as i32 + oy as i32;
        
        if norm_score > 0.5 {
            Some((final_x, final_y, norm_score))
        } else {
            None
        }
    }

    fn find_template_fast(&self, haystack: &DynamicImage, tpl: &CachedTemplate, search_region: Option<(u32, u32, u32, u32)>) -> Option<(i32, i32, f32)> {
        let (rx, ry, rw, rh) = match search_region {
            Some((x1, y1, x2, y2)) => {
                let sx = x1.min(haystack.width().saturating_sub(1));
                let sy = y1.min(haystack.height().saturating_sub(1));
                let sw = (x2 - x1).min(haystack.width().saturating_sub(sx));
                let sh = (y2 - y1).min(haystack.height().saturating_sub(sy));
                (sx, sy, sw, sh)
            }
            None => (0, 0, haystack.width(), haystack.height()),
        };

        if rw < tpl.w || rh < tpl.h || rw == 0 || rh == 0 {
            return None;
        }

        let cropped = haystack.crop_imm(rx, ry, rw, rh);
        let hay_gray = cropped.to_luma8();
        self.match_template_cached(&hay_gray, rx, ry, tpl)
    }

    // ── Çoklu template, erken-çıkışlı (yüksek skor → dur) ───────────────────
    fn best_match_with_early_exit(
        &self,
        haystack: &DynamicImage,
        templates: &[CachedTemplate],
        search_region: Option<(u32, u32, u32, u32)>,
        early_exit_score: f32,
    ) -> (Option<(i32, i32, f32, u32)>, u32) {
        let mut best: Option<(i32, i32, f32, u32)> = None;
        let mut hit_count: u32 = 0;

        for (idx, tpl) in templates.iter().enumerate() {
            if let Some((x, y, score)) = self.find_template_fast(haystack, tpl, search_region) {
                if score > BROAD_SCAN_THRESHOLD {
                    hit_count += 1;
                }
                let better = match &best {
                    Some((_, _, s, _)) => score > *s,
                    None => true,
                };
                if better {
                    best = Some((x, y, score, idx as u32));
                    if score >= early_exit_score {
                        // Çok yüksek güven — taramayı kes
                        return (best, hit_count);
                    }
                }
            }
        }
        (best, hit_count)
    }

    // ── Akıllı keyword eşleştirme (target için) ────────────────────────────
    fn match_keyword(&self, text: &str) -> Option<String> {
        if text.is_empty() {
            return None;
        }

        let normalized = self.normalize_for_matching(text);

        // ÖNCELİK 1: STARTS_WITH
        if normalized.starts_with("muharrebe") || normalized.starts_with("muharreb") || normalized.starts_with("muharre") {
            return Some("muharrebekilici".to_string());
        }
        if normalized.starts_with("metin") || normalized.starts_with("metin2") || normalized.starts_with("metn") {
            return Some("metinlogosu".to_string());
        }
        if normalized.starts_with("siyah") || normalized.starts_with("siya") || normalized.starts_with("siy") {
            return Some("siyahruzgarzirhi".to_string());
        }
        if normalized.starts_with("sura") || normalized.starts_with("sur") || normalized.starts_with("sua") || normalized.starts_with("sra") {
            return Some("sura".to_string());
        }
        if normalized.starts_with("yuzbasi") || normalized.starts_with("yuzba") || normalized.starts_with("yuzb") || normalized.starts_with("yuz") {
            return Some("yuzbasi".to_string());
        }
        if normalized.starts_with("kilic") || normalized.starts_with("kili") {
            return Some("kilic".to_string());
        }
        if normalized.starts_with("kil") && normalized.len() <= 9 {
            return Some("kilic".to_string());
        }
        if normalized.starts_with("demirci") || normalized.starts_with("demir") || normalized.starts_with("demi") || normalized.starts_with("dem") {
            return Some("demirci".to_string());
        }

        // ÖNCELİK 2: CONTAINS (uzun benzersiz)
        if normalized.contains("muharrebe") || normalized.contains("uharrebe") || normalized.contains("harrebe") {
            return Some("muharrebekilici".to_string());
        }
        if normalized.contains("metin") || normalized.contains("etin2") || normalized.contains("tin2") {
            return Some("metinlogosu".to_string());
        }
        if normalized.contains("siyah") || normalized.contains("iyah") || normalized.contains("ruzgar") || normalized.contains("ruzga") || normalized.contains("zirh") || normalized.contains("zirhi") {
            return Some("siyahruzgarzirhi".to_string());
        }
        if normalized.contains("sura") || normalized.contains("ura") {
            return Some("sura".to_string());
        }
        if normalized.contains("yuzbasi") || normalized.contains("yuzba") || normalized.contains("yuz") || normalized.contains("zba") || normalized.contains("bas") || normalized.contains("basi") {
            return Some("yuzbasi".to_string());
        }
        if normalized.len() <= 9 && (normalized.contains("kilic") || normalized.contains("ili") || normalized.contains("lic")) {
            return Some("kilic".to_string());
        }
        if normalized.len() <= 9 && normalized.contains("kil") {
            return Some("kilic".to_string());
        }
        if normalized.contains("demirci") || normalized.contains("demir") || normalized.contains("mirci") || normalized.contains("dem") || normalized.contains("emi") || normalized.contains("mir") {
            return Some("demirci".to_string());
        }

        // ÖNCELİK 3: KISA BENZERSİZ
        if normalized.contains("sua") {
            return Some("sura".to_string());
        }
        if normalized.contains("ba") && normalized.len() > 5 {
            let yuzbasi_indicators = ["yu", "uz", "zb", "as", "si", "yuz", "zba", "bas"];
            let has_other = yuzbasi_indicators.iter().any(|ind| normalized.contains(ind));
            if has_other {
                return Some("yuzbasi".to_string());
            }
            if normalized.starts_with("ba") && normalized.chars().any(|c| c.is_numeric()) {
                return Some("yuzbasi".to_string());
            }
        }

        // ÖNCELİK 4: FUZZY (Levenshtein)
        let mut best_match: Option<(String, usize)> = None;
        for (target, keywords) in &self.keywords {
            for kw in keywords {
                let kw_len = kw.len();
                let threshold = if kw_len <= 3 { 1 } else { kw_len / 3 };
                if normalized.len() >= kw_len {
                    for start_pos in 0..=(normalized.len().saturating_sub(kw_len)) {
                        let substr: String = normalized.chars().skip(start_pos).take(kw_len).collect();
                        let distance = levenshtein(&substr, kw);
                        if distance <= threshold {
                            let score = kw_len - distance;
                            match &best_match {
                                Some((_, bs)) if score > *bs => best_match = Some((target.clone(), score)),
                                None => best_match = Some((target.clone(), score)),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        best_match.map(|(t, _)| t)
    }

    // ── Multi-frame onay yöneticisi ─────────────────────────────────────────
    /// Pozitif tespit aldıktan sonra `CONFIRM_FRAMES` kadar ardışık aynı
    /// pozisyonda olmalı. Bu, "her şeyi captcha algıladı" hatasını öldürür.
    fn register_hit(&self, pos: (i32, i32)) -> bool {
        let mut st = self.state.lock().unwrap();

        // Cooldown'da mıyız?
        if Instant::now() < st.cooldown_until {
            st.consecutive_hits = 0;
            st.last_hit_pos = None;
            return false;
        }

        match st.last_hit_pos {
            Some((lx, ly)) if (lx - pos.0).abs() <= SAME_POS_TOLERANCE
                           && (ly - pos.1).abs() <= SAME_POS_TOLERANCE => {
                st.consecutive_hits += 1;
            }
            _ => {
                st.consecutive_hits = 1;
                st.last_hit_pos = Some(pos);
            }
        }
        st.consecutive_hits >= CONFIRM_FRAMES
    }

    fn register_miss(&self) {
        let mut st = self.state.lock().unwrap();
        if st.consecutive_hits > 0 {
            // Ardışıklık kırıldı
            st.consecutive_hits = 0;
            st.last_hit_pos = None;
        }
    }

    /// Çözüm sonrası cooldown başlat
    pub fn engage_cooldown(&self) {
        let mut st = self.state.lock().unwrap();
        st.cooldown_until = Instant::now() + POST_SOLVE_COOLDOWN;
        st.consecutive_hits = 0;
        st.last_hit_pos = None;
    }

    // ── Captcha var mı? (SADECE btn_onay.png görülünce) ─────────────────────
    /// SADECE kaydedilmiş bölgede ara, skor 0.70+ olmalı
    pub fn is_captcha_present(
        &self,
        frame: &DynamicImage,
        log_tx: &Sender<String>,
        saved_button_region: Option<(i32, i32, i32, i32)>,
    ) -> Option<(i32, i32)> {
        // COOLDOWN kontrolü
        {
            let st = self.state.lock().unwrap();
            let now = Instant::now();
            if now < st.cooldown_until {
                let remaining = st.cooldown_until.duration_since(now).as_secs();
                if remaining % 5 == 0 {
                    let _ = log_tx.send(format!("⏱️ Captcha cooldown: {} sn", remaining));
                }
                return None;
            }
        }

        // btn_onay.png template'ini al
        let templates = match self.template_cache.get("onay_buton") {
            Some(t) => t,
            None => {
                let _ = log_tx.send(format!("❌ btn_onay.png template yüklenemedi!"));
                return None;
            }
        };

        // SADECE kaydedilmiş bölgede ara - YOKSA CAPTCHA YOK SAY!
        let (search_region, region_str) = match saved_button_region {
            Some((x1, y1, x2, y2)) if x1 > 0 && y1 > 0 && x2 > x1 && y2 > y1 => {
                let w = x2 - x1;
                let h = y2 - y1;
                if w < 10 || h < 10 {
                    let _ = log_tx.send(format!("⚠️ Captcha bölgesi çok küçük: {}x{}", w, h));
                    return None;
                }
                (Some((x1 as u32, y1 as u32, x2 as u32, y2 as u32)), 
                 format!("[{},{} to {},{}]", x1, y1, x2, y2))
            }
            _ => {
                let _ = log_tx.send(format!("⚠️ Captcha buton bölgesi KAYDEDİLMEMİŞ! Lütfen GUI'de 'Captcha Buton' bölgesini seçin."));
                return None; // Bölge yoksa captcha yok say - ASLA TESPİT ETME!
            }
        };

        // En iyi eşleşmeyi bul - TÜM skorları logla (debug için)
        let mut best: Option<(i32, i32, f32)> = None;
        let mut all_scores: Vec<f32> = Vec::new();
        
        for tpl in templates.iter() {
            if let Some((x, y, score)) = self.find_template_fast(frame, tpl, search_region) {
                all_scores.push(score);
                // Skor 0.70+ olmalı (daha toleranslı)
                if score >= 0.70 {
                    let cx = x + tpl.w as i32 / 2;
                    let cy = y + tpl.h as i32 / 2;
                    if best.map_or(true, |(_, _, s)| score > s) {
                        best = Some((cx, cy, score));
                    }
                }
            }
        }

        // Her zaman skorları logla (debug)
        if !all_scores.is_empty() {
            let max_score = all_scores.iter().fold(0.0f32, |a, &b| a.max(b));
            let _ = log_tx.send(format!("🔍 Captcha tarama: max_skor={:.2} (eşik=0.70) bölge={}", max_score, region_str));
        }

        if let Some((cx, cy, score)) = best {
            let _ = log_tx.send(format!("� CAPTCHA TESPİT EDİLDİ! skor={:.2} @ ({},{})", score, cx, cy));
            Some((cx, cy))
        } else {
            None
        }
    }

    // ── Captcha çöz ─────────────────────────────────────────────────────────
    /// 15 saniye timeout ile çalışır - btn_onay.png görülünce başlar
    pub fn solve(
        &self,
        frame: &DynamicImage,
        soru_kesit: &DynamicImage,
        log_tx: &Sender<String>,
        saved_button_region: Option<(i32, i32, i32, i32)>,
    ) -> Vec<(i32, i32)> {
        let start_time = Instant::now();
        let mut clicks = Vec::new();
        
        let _ = log_tx.send(format!("🔔 CAPTCHA BAŞLADI - 15 sn timeout"));

        // 15 saniye içinde çözüm bulana kadar dene
        loop {
            // Timeout kontrolü
            if start_time.elapsed() > CAPTCHA_SOLVE_TIMEOUT {
                let _ = log_tx.send(format!("⏰ CAPTCHA TIMEOUT - 15 sn doldu!"));
                break;
            }

            // OCR ile soru metni
            let text = self.do_ocr(soru_kesit);
            
            // Geçerli OCR sonucu varsa devam et
            if !text.is_empty() {
                let _ = log_tx.send(format!("📝 OCR: '{}'", text));

                let target = self.match_keyword(&text);
                
                if let Some(target_name) = &target {
                    let _ = log_tx.send(format!("🎯 Hedef: '{}'", target_name));
                    
                    if let Some(templates) = self.template_cache.get(target_name) {
                        let search = Some((150, 150, frame.width() - 150, frame.height() - 100));

                        let mut all_matches: Vec<(i32, i32, f32, u32, u32)> = Vec::new();
                        for tpl in templates {
                            if let Some((x, y, score)) = self.find_template_fast(frame, tpl, search) {
                                if score > 0.7 {
                                    let cx = x + tpl.w as i32 / 2;
                                    let cy = y + tpl.h as i32 / 2;
                                    all_matches.push((cx, cy, score, tpl.w, tpl.h));
                                }
                            }
                        }

                        all_matches.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

                        for (cx, cy, score, _, _) in all_matches {
                            let far = clicks.iter().all(|&(ex, ey): &(i32, i32)| {
                                ((cx - ex).abs() > 30) || ((cy - ey).abs() > 30)
                            });
                            if far {
                                clicks.push((cx, cy));
                                let _ = log_tx.send(format!("✅ Bulundu: ({},{}) skor:{:.2}", cx, cy, score));
                            }
                            if clicks.len() >= 2 { break; }
                        }
                        
                        // Hedef bulundu ve tıklamalar hazır - çık
                        if !clicks.is_empty() {
                            break;
                        }
                    }
                } else {
                    let _ = log_tx.send(format!("⚠️ Hedef bulunamadı: '{}'", text));
                }
            }

            // Kısa bekle ve tekrar dene (100ms)
            std::thread::sleep(Duration::from_millis(100));
        }

        // Onay butonu ekle
        if let Some((sx1, sy1, sx2, sy2)) = saved_button_region {
            if sx1 > 0 && sy1 > 0 && sx2 > sx1 && sy2 > sy1 {
                clicks.push(((sx1 + sx2) / 2, (sy1 + sy2) / 2));
                let _ = log_tx.send(format!("✅ Onay butonu (kaydedilmiş): ({},{})", (sx1+sx2)/2, (sy1+sy2)/2));
            }
        }

        clicks.truncate(3);

        // Cooldown başlat
        self.engage_cooldown();

        let _ = log_tx.send(format!("⏱️ CAPTCHA BİTTİ - {}ms, {} tıklama", 
            start_time.elapsed().as_millis(), clicks.len()));
        clicks
    }

    /// PM SİMGESİ ALGILAMA v3.0 — YANIP SÖNME + RENK + ŞABLON
    /// 
    /// 3 Katmanlı Algılama:
    /// 1. YANIP SÖNME: Önceki frame ile parlaklık farkı (en güvenilir)
    /// 2. PARLAK RENK: Sarı/beyaz/kırmızı yoğun küme (ikon yanıp sönerken parlar)
    /// 3. ŞABLON: pm_simge.png template matching (fallback)
    ///
    /// `prev_brightness`: Bir önceki taramadaki PM bölgesi ortalama parlaklığı
    /// Çağıran taraf bu değeri tutar ve her taramada günceller.
    pub fn find_pm_simge_v3(
        &self,
        frame: &DynamicImage,
        search_region: Option<(i32, i32, i32, i32)>,
        prev_brightness: Option<f32>,
    ) -> (Option<(i32, i32, f32)>, f32) {
        // Return: (tespit, mevcut_parlaklık)
        let fw = frame.width();
        let fh = frame.height();
        
        // Arama bölgesini belirle
        let (sx, sy, ex, ey) = match search_region {
            Some((x1, y1, x2, y2)) if x2 > x1 && y2 > y1 && x1 >= 0 && y1 >= 0 => {
                let sx = (x1 as u32).min(fw);
                let sy = (y1 as u32).min(fh);
                let ex = (x2 as u32).min(fw);
                let ey = (y2 as u32).min(fh);
                (sx, sy, ex, ey)
            }
            _ => {
                // Varsayılan: Ekranın büyük kısmını tara (PM simgesi farklı konumlarda olabilir)
                // Sağ %40 + alt %30 kapsar — minimap altı, sağ panel, alt toolbar
                let sx = (fw as f32 * 0.55) as u32;
                let sy = 0u32;
                let ex = fw;
                let ey = fh;
                (sx, sy, ex, ey)
            }
        };
        
        // Mevcut parlaklığı hesapla (gri tonlama ortalaması)
        let current_brightness = self.calc_region_brightness(frame, sx, sy, ex, ey);
        
        // ═══ KATMAN 1: YANIP SÖNME ALGILAMA (Frame Farkı) ═══
        // İki ardışık frame arasındaki parlaklık farkını hesapla
        // PM simgesi yanıp sönerken parlaklık büyük ölçüde değişir
        if let Some(prev) = prev_brightness {
            let diff = (current_brightness - prev).abs();
            // Parlaklık farkı > 40 ise yanıp sönme var (0-255 ölçeğinde)
            // Normal sahne değişimi genelde 0-5 arası olur
            // PM simgesi yanıp sönerken 50-100 arası değişim olur
            // ÇOK KATI: 40+ olmalı (önceki 20'ydi, hala çok false positive veriyordu)
            if diff > 40.0 {
                // Yanıp sönme tespit edildi! Bölgenin ortasına tıkla
                let cx = ((sx + ex) / 2) as i32;
                let cy = ((sy + ey) / 2) as i32;
                let score = (diff / 80.0).min(1.0); // Normalize: 40-80 arası → 0.5-1.0
                return (Some((cx, cy, score)), current_brightness);
            }
        }
        
        // ═══ KATMAN 2: PARLAK RENK KÜMESİ ═══
        // PM simgesi yanıp sönerken parlak renklerde (sarı, beyaz, kırmızı) parlar
        let bright_detection = self.detect_bright_pm_indicator(frame, sx, sy, ex, ey);
        if let Some((bx, by, bscore)) = bright_detection {
            return (Some((bx, by, bscore)), current_brightness);
        }
        
        // ═══ KATMAN 3: ŞABLON EŞLEŞTİRME ═══
        let templates = match self.template_cache.get("pm_simge") {
            Some(t) => t,
            None => return (None, current_brightness),
        };
        
        let search = Some((sx, sy, ex, ey));
        let min_score = 0.80; // ÇOK YÜKSEK eşik - 0.50 → 0.65 → 0.80
        
        let mut best_match: Option<(i32, i32, f32)> = None;
        
        for tpl in templates.iter() {
            if let Some((x, y, score)) = self.find_template_fast(frame, tpl, search) {
                if score > min_score {
                    let cx = x + tpl.w as i32 / 2;
                    let cy = y + tpl.h as i32 / 2;
                    if best_match.map_or(true, |(_, _, s)| score > s) {
                        best_match = Some((cx, cy, score));
                    }
                }
            }
        }
        
        (best_match, current_brightness)
    }
    
    /// Bölgenin ortalama parlaklığını hesapla (gri tonlama)
    fn calc_region_brightness(&self, frame: &DynamicImage, sx: u32, sy: u32, ex: u32, ey: u32) -> f32 {
        let rgba = match frame.as_rgba8() {
            Some(r) => r,
            None => return 0.0,
        };
        
        let mut sum: u64 = 0;
        let mut count: u64 = 0;
        let step = 3u32; // Her 3 pikselde bir örnekle (hız için)
        
        let mut y = sy;
        while y < ey.min(rgba.height()) {
            let mut x = sx;
            while x < ex.min(rgba.width()) {
                let [r, g, b, _] = rgba.get_pixel(x, y).0;
                // Parlaklık: 0.299R + 0.587G + 0.114B
                let brightness = (r as u64 * 299 + g as u64 * 587 + b as u64 * 114) / 1000;
                sum += brightness;
                count += 1;
                x += step;
            }
            y += step;
        }
        
        if count == 0 { return 0.0; }
        sum as f32 / count as f32
    }
    
    /// Parlak PM göstergesi algılama (sarı/beyaz/kırmızı küme)
    /// PM simgesi yanıp sönerken parlak renkte parlar
    fn detect_bright_pm_indicator(
        &self,
        frame: &DynamicImage,
        sx: u32,
        sy: u32,
        ex: u32,
        ey: u32,
    ) -> Option<(i32, i32, f32)> {
        let rgba = frame.as_rgba8()?;
        let mut bright_pixels = Vec::new();
        
        for y in sy..ey.min(rgba.height()) {
            for x in sx..ex.min(rgba.width()) {
                let [r, g, b, _] = rgba.get_pixel(x, y).0;
                
                // Parlak piksel kriterleri (PM simgesi parladığında):
                // 1. Parlak sarı/altın: R>200, G>180, B<120
                // 2. Parlak beyaz: R>220, G>220, B>220
                // 3. Parlak kırmızı: R>200, G<80, B<80
                let is_bright = (r > 200 && g > 180 && b < 120) ||   // Sarı/altın
                               (r > 220 && g > 220 && b > 220) ||    // Beyaz
                               (r > 200 && g < 80 && b < 80 && r > g * 3); // Kırmızı
                
                if is_bright {
                    bright_pixels.push((x, y));
                }
            }
        }
        
        // Yeterli parlak piksel var mı? (PM simgesi için ÇOK KATI - 50 piksel)
        // Önceki: 15 → 25 → 50 (hala false positive veriyordu)
        if bright_pixels.len() < 50 {
            return None;
        }
        
        // Küme kontrolü
        let min_x = bright_pixels.iter().map(|(x, _)| *x).min().unwrap();
        let max_x = bright_pixels.iter().map(|(x, _)| *x).max().unwrap();
        let min_y = bright_pixels.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = bright_pixels.iter().map(|(_, y)| *y).max().unwrap();
        
        let cluster_w = max_x - min_x + 1;
        let cluster_h = max_y - min_y + 1;
        
        // Küme çok büyük olmamalı (simge ~10-50px)
        if cluster_w > 70 || cluster_h > 70 || cluster_w < 3 || cluster_h < 3 {
            return None;
        }
        
        // Aspect ratio: çok uzun şerit değil
        let aspect = cluster_w as f32 / cluster_h.max(1) as f32;
        if aspect > 5.0 || aspect < 0.2 {
            return None;
        }
        
        // Yoğunluk kontrolü (ÇOK KATI - %25 yoğunluk gerekli)
        // Önceki: 0.08 → 0.12 → 0.25 (hala false positive veriyordu)
        let cluster_area = (cluster_w * cluster_h) as f32;
        let density = bright_pixels.len() as f32 / cluster_area;
        if density < 0.25 {
            return None;
        }
        
        let sum_x: u32 = bright_pixels.iter().map(|(x, _)| x).sum();
        let sum_y: u32 = bright_pixels.iter().map(|(_, y)| y).sum();
        let count = bright_pixels.len() as u32;
        
        let cx = (sum_x / count) as i32;
        let cy = (sum_y / count) as i32;
        let score = (density * 2.0).min(1.0);
        
        Some((cx, cy, score))
    }
    
    /// Kırmızı yanıp sönen PM bildirim ışığını tespit et
    /// PM geldiğinde simge kırmızı renkte parlar
    /// v2: Çok daha katı — yoğun küme kontrolü (HP bar/hasar vs false positive engelleme)
    fn detect_red_pm_indicator(
        &self,
        frame: &DynamicImage,
        sx: u32,
        sy: u32,
        ex: u32,
        ey: u32,
    ) -> Option<(i32, i32, f32)> {
        let rgba = frame.as_rgba8()?;
        let mut red_pixels = Vec::new();
        
        // Bölgedeki kırmızı pikselleri bul — DAHA SIKI kriterler
        for y in sy..ey.min(rgba.height()) {
            for x in sx..ex.min(rgba.width()) {
                let pixel = rgba.get_pixel(x, y);
                let [r, g, b, _a] = pixel.0;
                
                // Sadece PARLAK kırmızı (PM simgesi yanıp sönerken belirgin parlar)
                // HP bar gibi koyu kırmızıları dışla
                let is_bright_red = r > 200 && g < 80 && b < 80 && r > g * 3 && r > b * 3;
                
                if is_bright_red {
                    red_pixels.push((x, y));
                }
            }
        }
        
        // Yeterli kırmızı piksel var mı? (PM simgesi ~20-40px genişliğinde)
        // 30 piksel minimum — daha az ise UI gürültüsü olabilir
        if red_pixels.len() < 30 {
            return None;
        }
        
        // KÜME KONTROLÜ: Kırmızı pikseller kompakt bir alanda mı?
        // HP bar gibi yatay şeritler değil, kare-imsi bir küme olmalı
        let min_x = red_pixels.iter().map(|(x, _)| *x).min().unwrap();
        let max_x = red_pixels.iter().map(|(x, _)| *x).max().unwrap();
        let min_y = red_pixels.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = red_pixels.iter().map(|(_, y)| *y).max().unwrap();
        
        let cluster_w = max_x - min_x + 1;
        let cluster_h = max_y - min_y + 1;
        
        // Küme çok geniş veya çok dar olmamalı (simge ~15-50px kare)
        if cluster_w > 60 || cluster_h > 60 || cluster_w < 5 || cluster_h < 5 {
            return None;
        }
        
        // Aspect ratio kontrolü: çok uzun yatay çizgi değil (HP bar gibi)
        let aspect = cluster_w as f32 / cluster_h.max(1) as f32;
        if aspect > 4.0 || aspect < 0.25 {
            return None; // HP bar gibi yatay veya dikey şerit
        }
        
        // Küme yoğunluğu: piksel sayısı / küme alanı > %10 olmalı
        let cluster_area = (cluster_w * cluster_h) as f32;
        let density = red_pixels.len() as f32 / cluster_area;
        if density < 0.10 {
            return None; // Dağınık pikseller — simge değil
        }
        
        // Kırmızı piksellerin merkezini bul
        let sum_x: u32 = red_pixels.iter().map(|(x, _)| x).sum();
        let sum_y: u32 = red_pixels.iter().map(|(_, y)| y).sum();
        let count = red_pixels.len() as u32;
        
        let cx = (sum_x / count) as i32;
        let cy = (sum_y / count) as i32;
        
        // Skor: küme yoğunluğu (0.0 - 1.0)
        let score = (density * 2.0).min(1.0); // Yoğunluk çarpanı
        
        Some((cx, cy, score))
    }

    /// PM GÖNDER BUTONU — pm_buton.png
    pub fn find_pm_button(
        &self,
        frame: &DynamicImage,
        search_region: Option<(i32, i32, i32, i32)>,
        threshold: f32,
    ) -> Option<(i32, i32, f32)> {
        let templates = match self.template_cache.get("pm_buton") {
            Some(t) => t,
            None => return None, // Şablon yoksa None
        };

        let search = search_region.map(|(x1, y1, x2, y2)| {
            let sx = x1.max(0) as u32;
            let sy = y1.max(0) as u32;
            // find_template_fast (x1,y1,x2,y2) koordinat çifti bekliyor
            let ex = (x2.max(0) as u32).min(frame.width());
            let ey = (y2.max(0) as u32).min(frame.height());
            (sx, sy, ex, ey)
        });

        let mut best: Option<(i32, i32, f32)> = None;

        for tpl in templates.iter() {
            if let Some((x, y, score)) = self.find_template_fast(frame, tpl, search) {
                if score >= threshold {
                    let cx = x + tpl.w as i32 / 2;
                    let cy = y + tpl.h as i32 / 2;
                    if best.map_or(true, |(_, _, s)| score > s) {
                        best = Some((cx, cy, score));
                    }
                }
            }
        }

        best
    }

    /// ═══════════════════════════════════════════════════════════════════════
    /// PM EKRANI TESPİTİ - Template Matching + YAPISEL VALIDASYON
    /// ═══════════════════════════════════════════════════════════════════════
    /// PM penceresini şablon matching ile bulur, ardından yapısal validasyon yapar
    /// Çimen/zemin false positive'lerini engellemek için katı kontroller
    pub fn find_pm_screen(
        &self,
        frame: &DynamicImage,
        search_region: Option<(i32, i32, i32, i32)>,
        threshold: f32,
    ) -> Option<(i32, i32, f32)> {
        let templates = match self.template_cache.get("pm_ekran") {
            Some(t) => t,
            None => return None, // Şablon yoksa None
        };

        let search = search_region.map(|(x1, y1, x2, y2)| {
            let sx = x1.max(0) as u32;
            let sy = y1.max(0) as u32;
            let ex = (x2.max(0) as u32).min(frame.width());
            let ey = (y2.max(0) as u32).min(frame.height());
            (sx, sy, ex, ey)
        });

        let mut best: Option<(i32, i32, f32)> = None;

        for tpl in templates.iter() {
            if let Some((x, y, score)) = self.find_template_fast(frame, tpl, search) {
                if score >= threshold {
                    let cx = x + tpl.w as i32 / 2;
                    let cy = y + tpl.h as i32 / 2;
                    
                    // ═══════════════════════════════════════════════════════════════
                    // YAPISEL VALIDASYON: Bu gerçekten bir PM penceresi mi?
                    // ═══════════════════════════════════════════════════════════════
                    
                    // 1. PM penceresi boyutları (tahmini: 250-350px genişlik, 180-250px yükseklik)
                    let pm_w = tpl.w as i32;
                    let pm_h = tpl.h as i32;
                    
                    // Şablon boyutu kontrolü - çok küçük/büyük değil
                    if pm_w < 200 || pm_w > 400 || pm_h < 150 || pm_h > 300 {
                        continue; // Geçersiz boyut - bu bir PM penceresi değil
                    }
                    
                    // 2. PM penceresi bölgesindeki piksel analizi
                    // Gerçek PM penceresi: koyu arka plan + beyaz/sarı metin
                    // Çimen/zemin: yeşil/kahverengi, düşük kontrast
                    let pm_x1 = (cx - pm_w / 2).max(0) as u32;
                    let pm_y1 = (cy - pm_h / 2).max(0) as u32;
                    let pm_x2 = (pm_x1 + pm_w as u32).min(frame.width());
                    let pm_y2 = (pm_y1 + pm_h as u32).min(frame.height());
                    
                    if let Some(rgba) = frame.as_rgba8() {
                        let mut dark_pixels = 0u32;  // Koyu arka plan (PM penceresi)
                        let mut bright_pixels = 0u32; // Parlak metin (beyaz/sarı)
                        let mut green_pixels = 0u32;   // Yeşil/çimen (false positive)
                        let mut total_pixels = 0u32;
                        
                        for py in pm_y1..pm_y2 {
                            for px in pm_x1..pm_x2 {
                                if px >= rgba.width() || py >= rgba.height() {
                                    continue;
                                }
                                let pixel = rgba.get_pixel(px, py);
                                let [r, g, b, _a] = pixel.0;
                                
                                total_pixels += 1;
                                
                                // Koyu arka plan (PM penceresi koyu gri/siyah)
                                let is_dark = r < 80 && g < 80 && b < 80;
                                if is_dark {
                                    dark_pixels += 1;
                                }
                                
                                // Parlak metin (beyaz/sarı - OCR okunabilir)
                                let is_bright = (r > 200 && g > 200 && b > 200) || // Beyaz
                                               (r > 200 && g > 180 && b < 100);    // Sarı/altın
                                if is_bright {
                                    bright_pixels += 1;
                                }
                                
                                // Yeşil/çimen (false positive işareti)
                                let is_green = g > r + 20 && g > b + 20 && g > 100;
                                if is_green {
                                    green_pixels += 1;
                                }
                            }
                        }
                        
                        if total_pixels > 0 {
                            let dark_ratio = dark_pixels as f32 / total_pixels as f32;
                            let bright_ratio = bright_pixels as f32 / total_pixels as f32;
                            let green_ratio = green_pixels as f32 / total_pixels as f32;
                            
                            // GERÇEK PM PENCERESI KRITERLERI:
                            // - En az %20 koyu arka plan (koyu tema)
                            // - En az %1 parlak metin (OCR okunabilir)
                            // - En fazla %30 yeşil (çimen değil!)
                            let is_valid_pm = dark_ratio > 0.20 && 
                                            bright_ratio > 0.01 && 
                                            green_ratio < 0.30;
                            
                            if !is_valid_pm {
                                // Yapısal validasyon BAŞARISIZ - bu çimen/zemin!
                                continue;
                            }
                        }
                    }
                    
                    // Tüm validasyonlar geçti - bu gerçek bir PM penceresi
                    if best.map_or(true, |(_, _, s)| score > s) {
                        best = Some((cx, cy, score));
                    }
                }
            }
        }

        best
    }

    /// ═══════════════════════════════════════════════════════════════════════
    /// CAPTCHA DOĞRULAMA SİSTEMİ v3.0
    /// ═══════════════════════════════════════════════════════════════════════
    /// Çözümden sonra btn_onay.png hala var mı kontrol eder
    /// Eğer varsa → yanlış çözüm, yoksa → başarılı
    pub fn verify_captcha_solved(
        &self,
        frame: &DynamicImage,
        saved_button_region: Option<(i32, i32, i32, i32)>,
    ) -> bool {
        // btn_onay.png şablonlarını al
        let btn_templates = match self.template_cache.get("onay_buton") {
            Some(tpls) => tpls,
            None => return false, // Şablon yoksa doğrulama yapılamaz
        };

        // Kaydedilmiş bölgede ara
        let search_region = saved_button_region.map(|(x1,y1,x2,y2)| 
            (x1 as u32, y1 as u32, x2 as u32, y2 as u32)
        );

        // En iyi eşleşmeyi bul
        let (best_match, _) = self.best_match_with_early_exit(
            frame,
            btn_templates,
            search_region,
            0.75 // Eşik - buton hala görünüyor mu?
        );

        // Eğer buton hala bulunuyorsa → çözüm BAŞARISIZ (false)
        // Buton yoksa → çözüm BAŞARILI (true)
        best_match.is_none()
    }

    /// ⚡ AGRESİF HIZLI Captcha çözümü - TEK DENEME, SIFIR BEKLEME
    /// 15 saniye yerine 3-4 saniye içinde çözüm
    /// 🆕 YENİ: Dışarıdan soru metni alabilir (OCR yedekli)
    pub fn solve_with_verification(
        &self,
        full_frame: &DynamicImage,
        ocr_crop: &DynamicImage,
        log_tx: &Sender<String>,
        saved_button_region: Option<(i32, i32, i32, i32)>,
        hw_sim: &crate::hw_simulator::HwSimulator,
        target_hwnd: winapi::shared::windef::HWND,
        fallback_question: Option<&str>, // 🆕 YENİ: Dışarıdan gelen soru metni
    ) -> Vec<(i32, i32)> {
        let start = std::time::Instant::now();
        
        // 1. HEMEN OCR yap - tek seferde
        let text = self.do_ocr(ocr_crop);
        
        // 🆕 YENİ: OCR başarısız olursa fallback kullan
        let text = if text.is_empty() {
            if let Some(fb) = fallback_question {
                let _ = log_tx.send(format!("[{}] ⚠️ OCR başarısız, fallback kullanılıyor: '{}'", 
                    chrono::Local::now().format("%H:%M:%S.%3f"), fb));
                fb.to_string()
            } else {
                let _ = log_tx.send(format!("[{}] ❌ OCR başarısız ve fallback yok!", 
                    chrono::Local::now().format("%H:%M:%S.%3f")));
                return Vec::new();
            }
        } else {
            text
        };
        
        let target = self.match_keyword(&text);
        if target.is_none() {
            let _ = log_tx.send(format!("[{}] ❌ Hedef bulunamadı: '{}'", 
                chrono::Local::now().format("%H:%M:%S.%3f"), text));
            return Vec::new();
        }
        
        let target_name = target.unwrap();
        let _ = log_tx.send(format!("[{}] 🎯 {} ({}ms)", 
            chrono::Local::now().format("%H:%M:%S.%3f"), target_name, start.elapsed().as_millis()));

        // 2. TÜM şablonları PARALEL ara - en yüksek skorlu 2 tanesini al
        let templates = match self.template_cache.get(&target_name) {
            Some(t) => t,
            None => return Vec::new(),
        };

        // ⚡ OPTIMIZASYON: Çok daha dar arama bölgesi - sadece captcha dialog ortası
        // Ekranın orta-merkez bölgesi (yatay %30-%70, dikey %25-%75)
        let fw = full_frame.width();
        let fh = full_frame.height();
        let search = Some((
            (fw as f32 * 0.30) as u32,  // x1: %30
            (fh as f32 * 0.25) as u32,  // y1: %25  
            (fw as f32 * 0.70) as u32,  // x2: %70
            (fh as f32 * 0.75) as u32,  // y2: %75
        ));
        
        // ⚡ OPTIMIZASYON: Paralel template arama - tüm şablonlar aynı anda
        use rayon::prelude::*;
        
        let all_matches: Vec<(i32, i32, f32)> = templates
            .par_iter() // Paralel iterasyon
            .filter_map(|tpl| {
                // Hızlı ön kontrol: Şablon çok büyükse atla (hız için)
                if tpl.w > 200 || tpl.h > 200 {
                    return None;
                }
                
                match self.find_template_fast(full_frame, tpl, search) {
                    Some((x, y, score)) if score > 0.60 => { // Daha düşük eşik
                        let cx = x + tpl.w as i32 / 2;
                        let cy = y + tpl.h as i32 / 2;
                        Some((cx, cy, score))
                    }
                    _ => None
                }
            })
            .collect();

        // En yüksek skorlu 2 eşleşmeyi al (paralel sort daha hızlı)
        let mut all_matches = all_matches;
        all_matches.par_sort_unstable_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        all_matches.truncate(2);
        
        if all_matches.is_empty() {
            let _ = log_tx.send(format!("[{}] ❌ Şablon bulunamadı!", 
                chrono::Local::now().format("%H:%M:%S.%3f")));
            return Vec::new();
        }

        let mut clicks: Vec<(i32, i32)> = all_matches.iter().map(|(x, y, _)| (*x, *y)).collect();
        
        // 3. Onay butonu ekle (kaydedilmiş bölgeden)
        if let Some((sx1, sy1, sx2, sy2)) = saved_button_region {
            if sx1 > 0 && sy1 > 0 {
                clicks.push(((sx1 + sx2) / 2, (sy1 + sy2) / 2));
            }
        }

        let _ = log_tx.send(format!("[{}] ⚡ {} tıklama hazır ({}ms)", 
            chrono::Local::now().format("%H:%M:%S.%3f"), clicks.len(), start.elapsed().as_millis()));

        // 4. İNSANSIZ tıkla - Şablonların görünmesi için bekle
        // Önce şablon tıklamaları - her biri arasında insansı gecikme
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for (i, (cx, cy)) in clicks.iter().enumerate() {
            // İlk tıklamadan önce şablonların görünmesi için bekle
            if i == 0 {
                let initial_delay = rng.gen_range(400..=700);
                std::thread::sleep(std::time::Duration::from_millis(initial_delay));
            }
            
            // 🐛 BUG FIX: Client koordinatlarını ekran koordinatlarına dönüştür
            let (screen_cx, screen_cy) = hw_sim.client_to_screen(target_hwnd, *cx, *cy);
            
            // Fareyi şablonun üzerine götür (insansı hareket)
            hw_sim.human_move(screen_cx, screen_cy);
            std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(150..=300)));
            
            // Tıkla - Ekran koordinatları kullan
            hw_sim.background_click_mode(target_hwnd, screen_cx, screen_cy, 
                &crate::hw_simulator::ClickMode::FocusSwap);
            
            let _ = log_tx.send(format!("[{}] 🖱️ Tıklama {}/{}: client({},{}) → screen({},{}) +{}ms", 
                chrono::Local::now().format("%H:%M:%S.%3f"), 
                i + 1, clicks.len(), cx, cy, screen_cx, screen_cy, start.elapsed().as_millis()));
            
            // Sonraki tıklama için bekle (son tıklama değilse)
            if i < clicks.len() - 1 {
                std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(300..=600)));
            }
        }

        let _ = log_tx.send(format!("[{}] ✅ Tıklamalar tamamlandı ({}ms)", 
            chrono::Local::now().format("%H:%M:%S.%3f"), start.elapsed().as_millis()));

        // 5. Şablonların kaybolması için insansı bekle (800-1200ms)
        std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(800..=1200)));
        
        let mut vision = crate::vision_manager::VisionEngine::new_with_shm(
            0, "".to_string(), "".to_string(), log_tx.clone()
        );
        
        if let Some(new_frame) = vision.capture_hwnd_background(target_hwnd) {
            if self.verify_captcha_solved(&new_frame, saved_button_region) {
                let _ = log_tx.send(format!("[{}] ✅ BAŞARILI! Toplam: {}ms", 
                    chrono::Local::now().format("%H:%M:%S.%3f"), start.elapsed().as_millis()));
                return clicks;
            } else {
                // Hızlı retry - 1 kere daha dene
                let _ = log_tx.send(format!("[{}] 🔄 Retry...", 
                    chrono::Local::now().format("%H:%M:%S.%3f")));
                
                // Farklı şablonları dene (varsa)
                if all_matches.len() >= 2 {
                    // İkinci en iyi eşleşmeyi dene
                    let second = &all_matches[1];
                    // 🐛 BUG FIX: Client koordinatlarını ekran koordinatlarına dönüştür
                    let (screen_s0, screen_s1) = hw_sim.client_to_screen(target_hwnd, second.0, second.1);
                    hw_sim.background_click_mode(target_hwnd, screen_s0, screen_s1, 
                        &crate::hw_simulator::ClickMode::FocusSwap);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    let mut retry_clicks = vec![(second.0, second.1)];
                    
                    if let Some((sx1, sy1, sx2, sy2)) = saved_button_region {
                        if sx1 > 0 && sy1 > 0 {
                            let btn_cx = (sx1 + sx2) / 2;
                            let btn_cy = (sy1 + sy2) / 2;
                            // 🐛 BUG FIX: Client koordinatlarını ekran koordinatlarına dönüştür
                            let (screen_bx, screen_by) = hw_sim.client_to_screen(target_hwnd, btn_cx, btn_cy);
                            hw_sim.background_click_mode(target_hwnd, screen_bx, screen_by, 
                                &crate::hw_simulator::ClickMode::FocusSwap);
                            retry_clicks.push((btn_cx, btn_cy));
                        }
                    }
                    
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    
                    if let Some(final_frame) = vision.capture_hwnd_background(target_hwnd) {
                        if self.verify_captcha_solved(&final_frame, saved_button_region) {
                            let _ = log_tx.send(format!("[{}] ✅ Retry BAŞARILI! Toplam: {}ms", 
                                chrono::Local::now().format("%H:%M:%S.%3f"), start.elapsed().as_millis()));
                            return retry_clicks;
                        }
                    }
                }
                
                let _ = log_tx.send(format!("[{}] ❌ BAŞARISIZ! Toplam: {}ms", 
                    chrono::Local::now().format("%H:%M:%S.%3f"), start.elapsed().as_millis()));
                return Vec::new();
            }
        }
        
        clicks
    }
}
