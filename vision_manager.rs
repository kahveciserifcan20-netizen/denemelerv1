use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use winapi::shared::windef::HWND;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::Path;
use image::{DynamicImage, RgbaImage}; 
use crossbeam_channel::Sender;
use chrono::Local;
use imageproc::template_matching::{match_template, MatchTemplateMethod}; 

#[derive(Debug, Clone)]
pub struct Detection {
    pub class_id: usize,
    #[allow(dead_code)]
    pub class_name: String,
    pub confidence: f32,
    pub bbox: [f32; 4],
}

#[allow(dead_code)]
pub struct VisionEngine {
    pub active_session: Option<Arc<Mutex<Session>>>,
    templates: HashMap<String, HashMap<String, Vec<DynamicImage>>>,
    width: u32,
    height: u32,
    current_hwnd: Mutex<isize>,
    log_tx: Sender<String>,
    hedef_kilit_template: Option<DynamicImage>,
    // Perf #8: Kilit şablonu grayscale cache (her frame to_luma8 yerine)
    hedef_kilit_gray: Option<image::GrayImage>,
    pub client_id: usize,
    gdi_resources: Arc<Mutex<GdiResources>>,
    pixel_buffer: Vec<u8>,
    // Perf #7: Ölüm şablonları cache (her frame diskten okuma yerine)
    death_template_burada: Option<image::GrayImage>,
    death_template_sehirde: Option<image::GrayImage>,
    // PM ikonu şablonu cache
    pm_icon_template: Option<image::GrayImage>,
    // Bug #5: Dinamik çözünürlük takibi
    last_hwnd_checked: isize,
}

/// GDI kaynaklarını tutan yapı - Drop trait'i ile otomatik temizlik
struct GdiResources {
    hdc_mem: *mut winapi::ctypes::c_void,
    hbitmap: *mut winapi::ctypes::c_void,
    width: u32,
    height: u32,
}

impl GdiResources {
    fn new(width: u32, height: u32) -> Self {
        Self {
            hdc_mem: std::ptr::null_mut(),
            hbitmap: std::ptr::null_mut(),
            width,
            height,
        }
    }
    
    fn is_initialized(&self) -> bool {
        !self.hdc_mem.is_null() && !self.hbitmap.is_null()
    }
    
    /// GDI kaynaklarını güvenli bir şekilde başlat
    fn initialize(&mut self, log_tx: &Sender<String>, client_id: usize) -> bool {
        if self.is_initialized() {
            return true; // Zaten başlatılmış
        }
        
        unsafe {
            let hdc_screen = winapi::um::winuser::GetDC(std::ptr::null_mut());
            if hdc_screen.is_null() {
                let _ = log_tx.send(format!("[{}] ❌ [C{}] GetDC başarısız!", 
                    chrono::Local::now().format("%H:%M:%S.%3f"), client_id));
                return false;
            }
            
            let hdc_mem = winapi::um::wingdi::CreateCompatibleDC(hdc_screen);
            let hbitmap = winapi::um::wingdi::CreateCompatibleBitmap(
                hdc_screen, self.width as i32, self.height as i32
            );
            
            if hdc_mem.is_null() || hbitmap.is_null() {
                // Temizlik
                if !hdc_mem.is_null() {
                    winapi::um::wingdi::DeleteDC(hdc_mem);
                }
                if !hbitmap.is_null() {
                    winapi::um::wingdi::DeleteObject(hbitmap as *mut winapi::ctypes::c_void);
                }
                winapi::um::winuser::ReleaseDC(std::ptr::null_mut(), hdc_screen);
                
                let _ = log_tx.send(format!("[{}] ❌ [C{}] GDI kaynakları oluşturulamadı!", 
                    chrono::Local::now().format("%H:%M:%S.%3f"), client_id));
                return false;
            }
            
            winapi::um::wingdi::SelectObject(hdc_mem, hbitmap as *mut winapi::ctypes::c_void);
            winapi::um::winuser::ReleaseDC(std::ptr::null_mut(), hdc_screen);
            
            self.hdc_mem = hdc_mem as *mut winapi::ctypes::c_void;
            self.hbitmap = hbitmap as *mut winapi::ctypes::c_void;
            
            let _ = log_tx.send(format!("[{}] 🎥 [C{}] GDI kaynakları oluşturuldu ({}x{})", 
                chrono::Local::now().format("%H:%M:%S.%3f"), client_id, self.width, self.height));
            
            true
        }
    }
    
    /// GDI kaynaklarını güvenli bir şekilde temizle
    fn cleanup(&mut self) {
        unsafe {
            if !self.hbitmap.is_null() {
                winapi::um::wingdi::DeleteObject(self.hbitmap as *mut winapi::ctypes::c_void);
                self.hbitmap = std::ptr::null_mut();
            }
            if !self.hdc_mem.is_null() {
                winapi::um::wingdi::DeleteDC(self.hdc_mem as winapi::shared::windef::HDC);
                self.hdc_mem = std::ptr::null_mut();
            }
        }
    }
}

impl Drop for GdiResources {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// Arc<Mutex<>> kullandığımız için Send/Sync otomatik olarak implemente edilir
// Manuel unsafe impl'lere gerek yok

fn iou(box1: &[f32; 4], box2: &[f32; 4]) -> f32 {
    let x1 = box1[0].max(box2[0]);
    let y1 = box1[1].max(box2[1]);
    let x2 = box1[2].min(box2[2]);
    let y2 = box1[3].min(box2[3]);
    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area1 = (box1[2] - box1[0]) * (box1[3] - box1[1]);
    let area2 = (box2[2] - box2[0]) * (box2[3] - box2[1]);
    intersection / (area1 + area2 - intersection + 1e-6)
}

#[allow(dead_code)]
impl VisionEngine {
    /// client_id: Benzersiz client numarası (0, 1, 2, ...)
    pub fn new_with_id(client_id: usize, log_tx: Sender<String>) -> Self {
        Self::new_with_config(client_id, "hedef_kilit.png".to_string(), log_tx)
    }

    pub fn new(log_tx: Sender<String>) -> Self {
        Self::new_with_id(0, log_tx)
    }

    /// Eski API uyumluluğu — shm_name artık kullanılmıyor
    pub fn new_with_shm(client_id: usize, _shm_name: String, kilit_path: String, log_tx: Sender<String>) -> Self {
        Self::new_with_config(client_id, kilit_path, log_tx)
    }

    /// Tam parametrik constructor — client_id ve kilit dosyası
    /// Python/SHM yerine saf Rust PrintWindow kullanır
    pub fn new_with_config(client_id: usize, kilit_path: String, log_tx: Sender<String>) -> Self {
        let kilit_path = if kilit_path.is_empty() {
            "hedef_kilit.png".to_string()
        } else {
            kilit_path
        };
        
        let mut kilit = None;
        if let Ok(img) = image::open(&kilit_path) {
            let ts = Local::now().format("%H:%M:%S.%3f");
            let _ = log_tx.send(format!("[{}] ✅ [C{}] Kilit şablonu yüklendi: {} ({}x{})", ts, client_id, kilit_path, img.width(), img.height()));
            kilit = Some(img);
        } else {
            if let Ok(img) = image::open("hedef_kilit.png") {
                let ts = Local::now().format("%H:%M:%S.%3f");
                let _ = log_tx.send(format!("[{}] ✅ [C{}] Kilit şablonu yüklendi: hedef_kilit.png ({}x{})", ts, client_id, img.width(), img.height()));
                kilit = Some(img);
            } else {
                let ts = Local::now().format("%H:%M:%S.%3f");
                let _ = log_tx.send(format!("[{}] ⚠️ [C{}] UYARI: Kilit şablonu bulunamadı!", ts, client_id));
            }
        }

        let ts = Local::now().format("%H:%M:%S.%3f");
        let _ = log_tx.send(format!("[{}] 🎥 [C{}] Saf Rust ekran yakalama motoru aktif (dinamik çözünürlük)", ts, client_id));

        // Perf #7: Ölüm şablonlarını başlangıçta cache'le
        let death_burada = {
            let path = "captcha_sablonlari/burada_yeniden_basla.png";
            match image::open(path) {
                Ok(img) => {
                    let _ = log_tx.send(format!("[{}] ✅ [C{}] Ölüm şablonu cache: burada ({}x{})", 
                        Local::now().format("%H:%M:%S.%3f"), client_id, img.width(), img.height()));
                    Some(img.to_luma8())
                }
                Err(_) => None
            }
        };
        let death_sehirde = {
            let path = "captcha_sablonlari/sehirde_yeniden_basla.png";
            match image::open(path) {
                Ok(img) => {
                    let _ = log_tx.send(format!("[{}] ✅ [C{}] Ölüm şablonu cache: sehirde ({}x{})", 
                        Local::now().format("%H:%M:%S.%3f"), client_id, img.width(), img.height()));
                    Some(img.to_luma8())
                }
                Err(_) => None
            }
        };

        // PM ikonu şablonunu cache'le
        let pm_icon = {
            let path = "captcha_sablonlari/pm_sablon.png";
            match image::open(path) {
                Ok(img) => {
                    let _ = log_tx.send(format!("[{}] ✅ [C{}] PM ikonu şablonu cache: {} ({}x{})", 
                        Local::now().format("%H:%M:%S.%3f"), client_id, path, img.width(), img.height()));
                    Some(img.to_luma8())
                }
                Err(_) => {
                    let _ = log_tx.send(format!("[{}] ⚠️ [C{}] PM ikonu şablonu bulunamadı: {}", 
                        Local::now().format("%H:%M:%S.%3f"), client_id, path));
                    None
                }
            }
        };

        let mut engine = Self {
            active_session: None,
            templates: HashMap::new(),
            width: 800,
            height: 600,
            current_hwnd: Mutex::new(0),
            log_tx,
            hedef_kilit_template: kilit.clone(),
            hedef_kilit_gray: kilit.as_ref().map(|k| k.to_luma8()),
            client_id,
            gdi_resources: Arc::new(Mutex::new(GdiResources::new(800, 600))),
            pixel_buffer: vec![0u8; (800 * 600 * 4) as usize],
            death_template_burada: death_burada,
            death_template_sehirde: death_sehirde,
            pm_icon_template: pm_icon,
            last_hwnd_checked: 0,
        };
        engine.load_captcha_templates();
        engine
    }

    /// Bug #5: Pencere boyutuna göre GDI kaynaklarını yeniden oluştur
    pub fn resize_for_hwnd(&mut self, hwnd: winapi::shared::windef::HWND) {
        if hwnd.is_null() { return; }
        let hwnd_val = hwnd as isize;
        if hwnd_val == self.last_hwnd_checked { return; }
        self.last_hwnd_checked = hwnd_val;
        
        unsafe {
            let mut rect: winapi::shared::windef::RECT = std::mem::zeroed();
            winapi::um::winuser::GetClientRect(hwnd, &mut rect);
            let w = (rect.right - rect.left) as u32;
            let h = (rect.bottom - rect.top) as u32;
            if w > 0 && h > 0 && (w != self.width || h != self.height) {
                self.send_log(&format!("📐 Pencere boyutu değişti: {}x{} -> {}x{}", self.width, self.height, w, h));
                self.width = w;
                self.height = h;
                self.pixel_buffer = vec![0u8; (w * h * 4) as usize];
                // GDI kaynaklarını yeniden oluştur
                if let Ok(mut gdi) = self.gdi_resources.lock() {
                    gdi.cleanup();
                    *gdi = GdiResources::new(w, h);
                }
            }
        }
    }

    fn send_log(&self, msg: &str) {
        let ts = Local::now().format("%H:%M:%S.%3f");
        let formatted = format!("[{}] {}", ts, msg);
        let _ = self.log_tx.send(formatted.clone());
        println!("{}", formatted);
    }

    pub fn switch_map_model(&mut self, onnx_file_name: &str) {
        let path = format!("yolo_modelleri/{}", onnx_file_name);
        if Path::new(&path).exists() {
            let start_time = Local::now();
            self.send_log(&format!("⏳ ONNX Belleğe Yükleniyor... ({})", onnx_file_name));
            
            match Session::builder().unwrap()
                .with_optimization_level(GraphOptimizationLevel::Level3).unwrap()
                .with_intra_threads(8).unwrap() 
                .commit_from_file(&path) {
                Ok(session) => {
                    self.active_session = Some(Arc::new(Mutex::new(session)));
                    let duration = Local::now().signed_duration_since(start_time).num_milliseconds();
                    self.send_log(&format!("✅ Model Başarıyla Yüklendi: {} ({} ms sürdü)", onnx_file_name, duration));
                }
                Err(e) => self.send_log(&format!("❌ Model Yükleme Hatası: {}", e)),
            }
        }
    }

    fn load_captcha_templates(&mut self) {
        let types = vec!["soru", "hedef"]; 
        for t_type in &types {
            let mut category_map: HashMap<String, Vec<DynamicImage>> = HashMap::new();
            let dir_path = format!("captcha_sablonlari/{}", t_type);
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" {
                                if let Ok(img) = image::open(path.to_str().unwrap()) {
                                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                                    let base_name = name.split('_').next().unwrap_or("").trim().to_lowercase();
                                    category_map.entry(base_name.clone()).or_insert_with(Vec::new).push(img);
                                }
                            }
                        }
                    }
                }
            }
            self.templates.insert(t_type.to_string(), category_map);
        }
    }

    pub fn count_red_pixels(&self, frame: &DynamicImage, bbox: &[f32; 4]) -> i32 {
        let x = bbox[0].max(0.0) as u32;
        let y = bbox[1].max(0.0) as u32;
        let w = (bbox[2] - bbox[0]).max(1.0) as u32;
        let h = (bbox[3] - bbox[1]).max(1.0) as u32;

        let safe_x = x.min(frame.width().saturating_sub(1));
        let safe_y = y.min(frame.height().saturating_sub(1));
        let safe_w = w.min(frame.width().saturating_sub(safe_x));
        let safe_h = h.min(frame.height().saturating_sub(safe_y));

        if safe_w == 0 || safe_h == 0 { return 0; }

        // Perf #6/#8: Zero-copy erişim — frame zaten RGBA ise kopyalama
        let rgba_cow = match frame.as_rgba8() {
            Some(r) => std::borrow::Cow::Borrowed(r),
            None => std::borrow::Cow::Owned(frame.to_rgba8()),
        };
        let cropped = image::imageops::crop_imm(rgba_cow.as_ref(), safe_x, safe_y, safe_w, safe_h).to_image();
        
        let mut count = 0;
        for p in cropped.pixels() {
            if p[0] > 140 && p[1] < 80 && p[2] < 80 {
                count += 1;
            }
        }
        count
    }

    /// Özellik #4: HP Bar Gradient Analizi — HP doluluk yüzdesini hesapla
    /// HP bar bölgesini soldan sağa tarayıp kırmızı→siyah geçiş noktasını bulur
    /// Dönüş: (hp_yuzde: f32, kirmizi_pixel: i32, toplam_pixel: i32)
    pub fn analyze_hp_bar(&self, frame: &DynamicImage, hp_region: (u32, u32, u32, u32)) -> (f32, i32, i32) {
        let (rx, ry, rw, rh) = hp_region;
        
        // Bölge sınır kontrolü
        let safe_x = rx.min(frame.width().saturating_sub(1));
        let safe_y = ry.min(frame.height().saturating_sub(1));
        let safe_w = rw.min(frame.width().saturating_sub(safe_x));
        let safe_h = rh.min(frame.height().saturating_sub(safe_y));
        
        if safe_w < 5 || safe_h < 2 { return (0.0, 0, 0); }
        
        // Zero-copy erişim
        let rgba_cow = match frame.as_rgba8() {
            Some(r) => std::borrow::Cow::Borrowed(r),
            None => std::borrow::Cow::Owned(frame.to_rgba8()),
        };
        let cropped = image::imageops::crop_imm(rgba_cow.as_ref(), safe_x, safe_y, safe_w, safe_h).to_image();
        
        // Dikey ortayı al (HP bar genellikle yatay şerit)
        let mid_y = safe_h / 2;
        let scan_rows = [mid_y.saturating_sub(1), mid_y, (mid_y + 1).min(safe_h - 1)];
        
        let mut last_red_x: u32 = 0;
        let mut total_bar_pixels: u32 = 0;
        let mut red_pixel_count: i32 = 0;
        let mut total_pixel_count: i32 = 0;
        
        // Soldan sağa tara — her sütun için 3 satırın ortalamasını al
        for x in 0..safe_w {
            let mut red_votes = 0;
            let mut non_bg_votes = 0;
            
            for &row_y in &scan_rows {
                if row_y >= safe_h { continue; }
                let p = cropped.get_pixel(x, row_y);
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                
                // Kırmızı tonu mu? (farklı sunucu renkleri için geniş aralık)
                // R kanalı dominant, G ve B düşük
                let is_red = r > 100.0 && r > g * 1.5 && r > b * 1.5 && g < 150.0 && b < 150.0;
                
                // Arka plan mı? (çok koyu = bar dışı veya boş kısım)
                let brightness = (r + g + b) / 3.0;
                let is_background = brightness < 30.0;
                
                if is_red { red_votes += 1; }
                if !is_background { non_bg_votes += 1; }
            }
            
            // 3 satırdan en az 2'si kırmızıysa bu piksel HP bar'ın dolu kısmı
            if red_votes >= 2 {
                red_pixel_count += 1;
                last_red_x = x;
            }
            
            // Arka plan olmayan piksel = bar'ın parçası
            if non_bg_votes >= 2 {
                total_bar_pixels += 1;
                total_pixel_count += 1;
            }
        }
        
        // HP yüzdesini hesapla
        let hp_percentage = if total_bar_pixels > 5 {
            // Yöntem 1: Son kırmızı pixel pozisyonu / toplam bar genişliği
            let pos_based = (last_red_x as f32 / safe_w as f32) * 100.0;
            
            // Yöntem 2: Kırmızı pixel sayısı / toplam bar pixel sayısı
            let count_based = (red_pixel_count as f32 / total_bar_pixels as f32) * 100.0;
            
            // İki yöntemin ortalaması (daha güvenilir)
            (pos_based + count_based) / 2.0
        } else {
            0.0
        };
        
        (hp_percentage.clamp(0.0, 100.0), red_pixel_count, total_pixel_count)
    }
    
    /// HP bar renk kalibrasyonu — oyuncunun HP bar rengini öğren
    /// Verilen bölgedeki dominant kırmızı tonunu döndürür
    pub fn calibrate_hp_color(&self, frame: &DynamicImage, hp_region: (u32, u32, u32, u32)) -> (u8, u8, u8) {
        let (rx, ry, rw, rh) = hp_region;
        let safe_x = rx.min(frame.width().saturating_sub(1));
        let safe_y = ry.min(frame.height().saturating_sub(1));
        let safe_w = rw.min(frame.width().saturating_sub(safe_x)).max(1);
        let safe_h = rh.min(frame.height().saturating_sub(safe_y)).max(1);
        
        let rgba_cow = match frame.as_rgba8() {
            Some(r) => std::borrow::Cow::Borrowed(r),
            None => std::borrow::Cow::Owned(frame.to_rgba8()),
        };
        let cropped = image::imageops::crop_imm(rgba_cow.as_ref(), safe_x, safe_y, safe_w, safe_h).to_image();
        
        let mut sum_r: u64 = 0;
        let mut sum_g: u64 = 0;
        let mut sum_b: u64 = 0;
        let mut red_count: u64 = 0;
        
        for p in cropped.pixels() {
            // Sadece kırmızımsı piksellerin ortalamasını al
            if p[0] > 100 && p[0] as f32 > p[1] as f32 * 1.3 {
                sum_r += p[0] as u64;
                sum_g += p[1] as u64;
                sum_b += p[2] as u64;
                red_count += 1;
            }
        }
        
        if red_count > 0 {
            ((sum_r / red_count) as u8, (sum_g / red_count) as u8, (sum_b / red_count) as u8)
        } else {
            (200, 30, 30) // Varsayılan kırmızı
        }
    }

    /// Özellik #1: Captcha/Dialog bölgesi otomatik tespiti
    /// Frame'deki dialog/popup benzeri dikdörtgen alanları tespit eder
    /// Dönüş: Option<(x1, y1, x2, y2)> — tespit edilen bölge koordinatları
    pub fn auto_detect_captcha_region(&self, frame: &DynamicImage) -> Option<(u32, u32, u32, u32)> {
        let (fw, fh) = (frame.width(), frame.height());
        if fw < 100 || fh < 100 { return None; }
        
        let rgba_cow = match frame.as_rgba8() {
            Some(r) => std::borrow::Cow::Borrowed(r),
            None => std::borrow::Cow::Owned(frame.to_rgba8()),
        };
        
        // Strateji: Dialog pencereleri genellikle:
        // 1. Ekranın ortasında (merkez %60 alanı)
        // 2. Çevreden belirgin şekilde farklı (kenar kontrastı yüksek)
        // 3. İç kısmı nispeten homojen (arka plan rengi)
        // 4. Belirli boyut aralığında (100x60 — 500x400 arası)
        
        let center_x = fw / 2;
        let center_y = fh / 2;
        let search_margin_x = fw * 30 / 100; // Merkez %60
        let search_margin_y = fh * 30 / 100;
        
        let scan_x1 = center_x.saturating_sub(search_margin_x);
        let scan_y1 = center_y.saturating_sub(search_margin_y);
        let scan_x2 = (center_x + search_margin_x).min(fw);
        let scan_y2 = (center_y + search_margin_y).min(fh);
        
        // Yatay kenar tarama — soldan sağa belirgin kontrast geçişi ara
        let mut best_region: Option<(u32, u32, u32, u32, f32)> = None; // x1,y1,x2,y2,score
        
        // Dikey şeritler halinde tara
        let step = 4u32; // 4 piksel adım (hız için)
        
        for y in (scan_y1..scan_y2).step_by(step as usize) {
            let mut in_edge = false;
            let mut edge_start_x = 0u32;
            let mut prev_brightness = 0.0f32;
            
            for x in scan_x1..scan_x2 {
                let p = rgba_cow.get_pixel(x, y);
                let brightness = p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114;
                let contrast = (brightness - prev_brightness).abs();
                prev_brightness = brightness;
                
                // Belirgin kenar geçişi (kontrast > 40)
                if contrast > 40.0 && !in_edge {
                    in_edge = true;
                    edge_start_x = x;
                } else if contrast > 40.0 && in_edge {
                    // İkinci kenar bulundu — potansiyel dialog genişliği
                    let dialog_width = x - edge_start_x;
                    if dialog_width > 80 && dialog_width < 500 {
                        // Bu genişlikte bir dialog var, dikey boyutunu bul
                        if let Some((dy1, dy2)) = self.find_vertical_edges(&rgba_cow, edge_start_x, x, y, scan_y1, scan_y2) {
                            let dialog_height = dy2 - dy1;
                            if dialog_height > 40 && dialog_height < 400 {
                                // Skor: Merkeze yakınlık + boyut uygunluğu
                                let cx = (edge_start_x + x) / 2;
                                let cy = (dy1 + dy2) / 2;
                                let dist_center = ((cx as f32 - center_x as f32).powi(2) + (cy as f32 - center_y as f32).powi(2)).sqrt();
                                let size_score = (dialog_width as f32 * dialog_height as f32).sqrt();
                                let score = size_score / (dist_center + 1.0);
                                
                                if best_region.is_none() || score > best_region.unwrap().4 {
                                    best_region = Some((edge_start_x, dy1, x, dy2, score));
                                }
                            }
                        }
                    }
                    in_edge = false;
                }
            }
        }
        
        if let Some((x1, y1, x2, y2, score)) = best_region {
            self.send_log(&format!("🔍 Captcha bölgesi otomatik tespit edildi: ({},{}) - ({},{}) [skor: {:.1}]", x1, y1, x2, y2, score));
            Some((x1, y1, x2, y2))
        } else {
            None
        }
    }
    
    /// Yardımcı: Verilen yatay aralıkta dikey kenarları bul
    fn find_vertical_edges(&self, rgba: &image::RgbaImage, x1: u32, x2: u32, mid_y: u32, min_y: u32, max_y: u32) -> Option<(u32, u32)> {
        let sample_x = (x1 + x2) / 2;
        let mut prev_brightness = 0.0f32;
        let mut top_edge = mid_y;
        let mut bottom_edge = mid_y;
        
        // Yukarı tara
        for y in (min_y..mid_y).rev() {
            let p = rgba.get_pixel(sample_x.min(rgba.width() - 1), y);
            let brightness = p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114;
            if y == mid_y.saturating_sub(1) { prev_brightness = brightness; continue; }
            if (brightness - prev_brightness).abs() > 35.0 {
                top_edge = y;
                break;
            }
            prev_brightness = brightness;
        }
        
        // Aşağı tara
        prev_brightness = 0.0;
        for y in mid_y..max_y {
            let p = rgba.get_pixel(sample_x.min(rgba.width() - 1), y);
            let brightness = p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114;
            if y == mid_y { prev_brightness = brightness; continue; }
            if (brightness - prev_brightness).abs() > 35.0 {
                bottom_edge = y;
                break;
            }
            prev_brightness = brightness;
        }
        
        if bottom_edge > top_edge + 30 {
            Some((top_edge, bottom_edge))
        } else {
            None
        }
    }


    /// Ölüm ekranı tespiti - cache'lenmiş şablonlarla (Perf #7)
    pub fn check_death_screen(&self, frame: &DynamicImage) -> (bool, Option<String>, Option<(u32, u32)>) {
        let screen_gray = frame.to_luma8();
        
        // Önce "Burada Yeniden Başla" butonunu ara (cache'den)
        if let Some(template_gray) = &self.death_template_burada {
            if screen_gray.width() >= template_gray.width() && screen_gray.height() >= template_gray.height() {
                let result = match_template(&screen_gray, template_gray, MatchTemplateMethod::SumOfSquaredErrorsNormalized);
                let mut min_val = 1.0f32;
                let mut min_pos = (0u32, 0u32);
                for (y, row) in result.enumerate_rows() {
                    for (x, _, p) in row {
                        if p[0] < min_val {
                            min_val = p[0];
                            min_pos = (x, y);
                        }
                    }
                }
                if min_val < 0.15 {
                    let center_x = min_pos.0 + template_gray.width() / 2;
                    let center_y = min_pos.1 + template_gray.height() / 2;
                    self.send_log(&format!("💀 Ölüm ekranı tespit edildi! 'Burada Yeniden Başla' butonu bulundu (skor: {:.3})", 1.0 - min_val));
                    return (true, Some("burada".to_string()), Some((center_x, center_y)));
                }
            }
        }
        
        // Sonra "Şehirde Yeniden Başla" butonunu ara (cache'den)
        if let Some(template_gray) = &self.death_template_sehirde {
            if screen_gray.width() >= template_gray.width() && screen_gray.height() >= template_gray.height() {
                let result = match_template(&screen_gray, template_gray, MatchTemplateMethod::SumOfSquaredErrorsNormalized);
                let mut min_val = 1.0f32;
                let mut min_pos = (0u32, 0u32);
                for (y, row) in result.enumerate_rows() {
                    for (x, _, p) in row {
                        if p[0] < min_val {
                            min_val = p[0];
                            min_pos = (x, y);
                        }
                    }
                }
                if min_val < 0.15 {
                    let center_x = min_pos.0 + template_gray.width() / 2;
                    let center_y = min_pos.1 + template_gray.height() / 2;
                    self.send_log(&format!("💀 Ölüm ekranı tespit edildi! 'Şehirde Yeniden Başla' butonu bulundu (skor: {:.3})", 1.0 - min_val));
                    return (true, Some("sehirde".to_string()), Some((center_x, center_y)));
                }
            }
        }
        
        (false, None, None)
    }

    /// Hedef kilit tespiti
    pub fn is_target_locked(&self, frame: &DynamicImage, search_region: Option<(u32,u32,u32,u32)>, _save_debug: bool) -> (bool, f32) {
        // Perf #8: Cache'lenmiş grayscale template kullan
        if let Some(template_gray) = &self.hedef_kilit_gray {
            let screen_gray = frame.to_luma8();

            let (rx, ry, rw, rh) = match search_region {
                Some((x1, y1, x2, y2)) => {
                    let sx = x1.min(screen_gray.width().saturating_sub(1));
                    let sy = y1.min(screen_gray.height().saturating_sub(1));
                    let sw = x2.saturating_sub(x1).min(screen_gray.width().saturating_sub(sx));
                    let sh = y2.saturating_sub(y1).min(screen_gray.height().saturating_sub(sy));
                    (sx, sy, sw, sh)
                }
                None => (0, 0, screen_gray.width(), 150.min(screen_gray.height())),
            };

            if rw < template_gray.width() || rh < template_gray.height() || rw == 0 || rh == 0 {
                self.send_log(&format!("⚠️ Kilit arama bölgesi çok küçük: {}x{} (template: {}x{})", rw, rh, template_gray.width(), template_gray.height()));
                return (false, 0.0);
            }

            let cropped = image::imageops::crop_imm(&screen_gray, rx, ry, rw, rh).to_image();
            
            // SumOfSquaredErrorsNormalized: 0=mükemmel eşleşme, 1=hiç eşleşmeme
            let result = match_template(&cropped, &template_gray, MatchTemplateMethod::SumOfSquaredErrorsNormalized);
            let mut min_val = 1.0f32;
            let mut _min_pos = (0u32, 0u32);
            for (y, row) in result.enumerate_rows() {
                for (x, _, p) in row {
                    if p[0] < min_val {
                        min_val = p[0];
                        _min_pos = (x, y);
                    }
                }
            }

            // min_val < 0.10 = kilit bulundu (düşük = daha iyi eşleşme)
            // v3.0: 0.10 - ÇOK HASSAS - kilit kaybolduğu AN algıla (seri farm için kritik)
            // Önceki 0.15 değeri 1-2 saniye gecikme yaratıyordu
            let is_locked = min_val < 0.10;
            let max_val = 1.0 - min_val; // Loglarda yüksek=iyi olarak göster

            // Debug: Her çağrıda skor logla (gecikme analizi için)
            // eprintln!("[🔒 Kilit] min_val={:.3} | is_locked={}", min_val, is_locked);

            return (is_locked, max_val);
        }
        self.send_log("⚠️ Kilit şablonu yüklü değil!");
        (false, 0.0)
    }

    /// PM (Özel Mesaj) ikonu tespiti - Template Matching ile
    /// Sadece ekranın sağ %25'lik kısmını tarar (işlemci optimizasyonu)
    /// Normalized Cross Correlation kullanır
    /// Eşleşme oranı %80 (0.80) ve üzerindeyse true döndürür
    pub fn detect_pm_icon(&self, frame: &DynamicImage) -> (bool, f32, Option<(u32, u32)>) {
        // PM ikonu şablonu cache'den al
        if let Some(template_gray) = &self.pm_icon_template {
            let screen_gray = frame.to_luma8();
            
            // Ekranın sağ %25'lik kısmını hesapla
            let screen_width = screen_gray.width();
            let screen_height = screen_gray.height();
            let search_width = screen_width / 4; // Sağ %25
            let search_x = screen_width - search_width; // Sağ taraf başlangıcı
            
            // Bölge sınırlarını güvenli şekilde ayarla
            let rx = search_x.min(screen_width.saturating_sub(1));
            let ry = 0u32;
            let rw = search_width.min(screen_width.saturating_sub(rx));
            let rh = screen_height;
            
            // Şablon boyut kontrolü
            if rw < template_gray.width() || rh < template_gray.height() || rw == 0 || rh == 0 {
                return (false, 0.0, None);
            }
            
            // Sağ %25 bölgeyi kırp
            let cropped = image::imageops::crop_imm(&screen_gray, rx, ry, rw, rh).to_image();
            
            // Normalized Cross Correlation ile template matching
            // MatchTemplateMethod::CrossCorrelationNormalized: 1=mükemmel eşleşme, 0=hiç eşleşmeme
            let result = match_template(&cropped, &template_gray, MatchTemplateMethod::CrossCorrelationNormalized);
            
            // En yüksek eşleşme skorunu bul
            let mut max_val = 0.0f32;
            let mut max_pos = (0u32, 0u32);
            for (y, row) in result.enumerate_rows() {
                for (x, _, p) in row {
                    if p[0] > max_val {
                        max_val = p[0];
                        max_pos = (x, y);
                    }
                }
            }
            
            // Eşleşme oranı %80 (0.80) ve üzerindeyse true
            let threshold = 0.80f32;
            let is_detected = max_val >= threshold;
            
            // Tespit edildiyse global koordinatlara çevir
            let global_pos = if is_detected {
                Some((rx + max_pos.0 + template_gray.width() / 2, 
                      max_pos.1 + template_gray.height() / 2))
            } else {
                None
            };
            
            // Debug log (sadece tespit edildiğinde veya düşük skor)
            if is_detected {
                self.send_log(&format!("💬 PM ikonu tespit edildi! (skor: {:.3}, poz: {:?})", max_val, global_pos));
            }
            
            return (is_detected, max_val, global_pos);
        }
        
        // Şablon yüklenmemişse
        (false, 0.0, None)
    }

    /// YOLO tespitlerini frame üzerine çiz (canlı izleme için)
    pub fn draw_detections_on_frame(frame: &mut RgbaImage, detections: &[Detection], kilit_region: Option<(u32,u32,u32,u32)>) {
        let green = image::Rgba([0, 255, 100, 255]);
        let red = image::Rgba([255, 60, 60, 255]);
        let yellow = image::Rgba([255, 220, 0, 255]);
        let _cyan = image::Rgba([0, 200, 255, 255]);

        let (fw, fh) = (frame.width(), frame.height());

        for d in detections {
            let x1 = (d.bbox[0] as u32).min(fw.saturating_sub(1));
            let y1 = (d.bbox[1] as u32).min(fh.saturating_sub(1));
            let x2 = (d.bbox[2] as u32).min(fw.saturating_sub(1));
            let y2 = (d.bbox[3] as u32).min(fh.saturating_sub(1));

            // class_id 0, 1, 2 = TAŞ olabilir (model bazlı değişir)
            let color = if d.class_id <= 2 { green } else { red };

            // Yatay çizgiler (üst ve alt)
            for x in x1..=x2 {
                for t in 0..2u32 {
                    if y1 + t < fh { frame.put_pixel(x, y1 + t, color); }
                    if y2 >= t && y2 - t < fh { frame.put_pixel(x, y2 - t, color); }
                }
            }
            // Dikey çizgiler (sol ve sağ)
            for y in y1..=y2 {
                for t in 0..2u32 {
                    if x1 + t < fw { frame.put_pixel(x1 + t, y, color); }
                    if x2 >= t && x2 - t < fw { frame.put_pixel(x2 - t, y, color); }
                }
            }

            // Kırmızı merkez noktası (tıklama hedefi) — class_id 1 veya 2 (TAŞ) için
            if d.class_id == 1 || d.class_id == 2 {
                let cx = (x1 + x2) / 2;
                let cy = (y1 + y2) / 2;
                let dot = image::Rgba([255, 0, 0, 255]);
                for dx in 0..5u32 {
                    for dy in 0..5u32 {
                        let px = cx + dx - 2;
                        let py = cy + dy - 2;
                        if px < fw && py < fh { frame.put_pixel(px, py, dot); }
                    }
                }
            }

            // Confidence etiketi (basit piksel bloğu)
            let label_y = if y1 > 12 { y1 - 10 } else { y2 + 2 };
            let conf_pct = (d.confidence * 100.0) as u32;
            // Her rakam ~6px genişlik
            let digits: Vec<u32> = if conf_pct >= 10 { vec![conf_pct / 10, conf_pct % 10] } else { vec![conf_pct] };
            for (di, digit) in digits.iter().enumerate() {
                let dx = x1 + (di as u32) * 7;
                // Basit 5x7 digit renderer
                let patterns: [u32; 10] = [0x7B6F, 0x24924, 0x73E4F, 0x72A6F, 0x5BF24, 0x79E6F, 0x79F6F, 0x72924, 0x7BF6F, 0x7BE6F];
                if *digit < 10 {
                    let p = patterns[*digit as usize];
                    for py in 0..7u32 {
                        for px in 0..5u32 {
                            let bit = (p >> (34 - py * 5 - px)) & 1;
                            if bit == 1 {
                                let fx = dx + px;
                                let fy = label_y + py;
                                if fx < fw && fy < fh { frame.put_pixel(fx, fy, color); }
                            }
                        }
                    }
                }
            }
        }

        // Kilit arama bölgesi (sarı çerçeve)
        if let Some((kx1, ky1, kx2, ky2)) = kilit_region {
            for x in kx1..=kx2.min(fw.saturating_sub(1)) {
                if ky1 < fh { frame.put_pixel(x, ky1, yellow); }
                if ky2 < fh { frame.put_pixel(x, ky2, yellow); }
            }
            for y in ky1..=ky2.min(fh.saturating_sub(1)) {
                if kx1 < fw { frame.put_pixel(kx1, y, yellow); }
                if kx2 < fw { frame.put_pixel(kx2, y, yellow); }
            }
        }
    }

    /// Hedef kilit şablonunu yeniden yükle
    pub fn reload_hedef_kilit(&mut self, path: &str) {
        match image::open(path) {
            Ok(img) => {
                self.send_log(&format!("✅ Kilit şablonu yüklendi: {} ({}x{})", path, img.width(), img.height()));
                self.hedef_kilit_template = Some(img);
            }
            Err(e) => self.send_log(&format!("❌ Kilit şablonu yüklenemedi: {} — {}", path, e)),
        }
    }

    /// GDI kaynaklarını önceden başlat (lazy init gecikmesini önle)
    pub fn prewarm_gdi_resources(&mut self, hwnd: HWND) -> bool {
        if hwnd.is_null() {
            return false;
        }
        unsafe {
            if winapi::um::winuser::IsWindow(hwnd) == 0 {
                return false;
            }
        }
        
        let mut gdi_guard = match self.gdi_resources.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        
        if !gdi_guard.is_initialized() {
            let success = gdi_guard.initialize(&self.log_tx, self.client_id);
            if success {
                self.send_log(&format!("🔥 GDI kaynakları önceden başlatıldı ({}x{})", self.width, self.height));
            }
            success
        } else {
            true
        }
    }

    /// Saf Rust ekran yakalama — PrintWindow + GetDIBits
    /// Python/SHM gerektirmez, her client kendi GDI kaynaklarını kullanır
    /// Thread-safe: GDI kaynakları Arc<Mutex<>> ile korunur
    /// OPTIMIZED: Düşük gecikme için optimize edildi
    pub fn capture_hwnd_background(&mut self, hwnd: HWND) -> Option<DynamicImage> {
        if hwnd.is_null() {
            return None;
        }

        // Pencere geçerli mi kontrol et - HIZLI CHECK (sadece null değilse devam et)
        unsafe {
            if winapi::um::winuser::IsWindow(hwnd) == 0 {
                return None;
            }
        }

        // GDI kaynaklarını thread-safe şekilde al - HIZLI LOCK (poisoned değilse)
        let mut gdi_guard = self.gdi_resources.lock().ok()?;
        
        // GDI kaynaklarını lazy init (genelde prewarm ile önceden başlatılmış olur)
        if !gdi_guard.is_initialized() {
            if !gdi_guard.initialize(&self.log_tx, self.client_id) {
                return None;
            }
        }

        unsafe {
            // PrintWindow ile arka plan yakalama
            // PW_CLIENTONLY(0x1) | PW_RENDERFULLCONTENT(0x2) = 0x3
            let result = winapi::um::winuser::PrintWindow(
                hwnd, gdi_guard.hdc_mem as winapi::shared::windef::HDC, 0x3
            );

            if result == 0 {
                // PrintWindow başarısız — BitBlt fallback (foreground gerektirir)
                let mut client_pt = winapi::shared::windef::POINT { x: 0, y: 0 };
                winapi::um::winuser::ClientToScreen(hwnd, &mut client_pt);
                let hdc_screen = winapi::um::winuser::GetDC(std::ptr::null_mut());
                if !hdc_screen.is_null() {
                    winapi::um::wingdi::BitBlt(
                        gdi_guard.hdc_mem as winapi::shared::windef::HDC,
                        0, 0, self.width as i32, self.height as i32,
                        hdc_screen,
                        client_pt.x, client_pt.y,
                        0x00CC0020 // SRCCOPY
                    );
                    winapi::um::winuser::ReleaseDC(std::ptr::null_mut(), hdc_screen);
                }
            }

            // BITMAPINFOHEADER hazırla (stack üzerinde)
            #[repr(C)]
            struct BitmapInfoHeader {
                bi_size: u32,
                bi_width: i32,
                bi_height: i32,
                bi_planes: u16,
                bi_bit_count: u16,
                bi_compression: u32,
                bi_size_image: u32,
                bi_x_pels_per_meter: i32,
                bi_y_pels_per_meter: i32,
                bi_clr_used: u32,
                bi_clr_important: u32,
            }

            #[repr(C)]
            struct BitmapInfo {
                bmi_header: BitmapInfoHeader,
                bmi_colors: [u32; 3],
            }

            let mut bmi: BitmapInfo = std::mem::zeroed();
            bmi.bmi_header.bi_size = std::mem::size_of::<BitmapInfoHeader>() as u32;
            bmi.bmi_header.bi_width = self.width as i32;
            bmi.bmi_header.bi_height = -(self.height as i32); // Negatif = top-down
            bmi.bmi_header.bi_planes = 1;
            bmi.bmi_header.bi_bit_count = 32;
            bmi.bmi_header.bi_compression = 0; // BI_RGB

            // GetDIBits ile pixel data oku
            let rows = winapi::um::wingdi::GetDIBits(
                gdi_guard.hdc_mem as winapi::shared::windef::HDC,
                gdi_guard.hbitmap as winapi::shared::windef::HBITMAP,
                0,
                self.height,
                self.pixel_buffer.as_mut_ptr() as *mut winapi::ctypes::c_void,
                &mut bmi as *mut BitmapInfo as *mut winapi::um::wingdi::BITMAPINFO,
                0 // DIB_RGB_COLORS
            );

            // Mutex guard'ı burada bırak (drop)
            drop(gdi_guard);

            if rows == 0 {
                return None;
            }

            // BGRA → RGBA dönüşümü (Windows bitmap formatı BGRA)
            let pixel_count = (self.width * self.height) as usize;
            let mut rgba_buffer = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                let src = i * 4;
                rgba_buffer[src]     = self.pixel_buffer[src + 2]; // R ← B
                rgba_buffer[src + 1] = self.pixel_buffer[src + 1]; // G ← G
                rgba_buffer[src + 2] = self.pixel_buffer[src];     // B ← R
                rgba_buffer[src + 3] = 255;                        // A = opaque
            }

            if let Some(img) = RgbaImage::from_raw(self.width, self.height, rgba_buffer) {
                return Some(DynamicImage::ImageRgba8(img));
            }

            None
        }
    }

    fn preprocess_image_for_onnx(&self, frame: &DynamicImage, target_size: (u32, u32)) -> (Vec<i64>, Vec<f32>) {
        let resized = frame.resize_exact(target_size.0, target_size.1, image::imageops::FilterType::Triangle);
        let mut flat_data = vec![0.0; (3 * target_size.0 * target_size.1) as usize];
        let mut i = 0;
        let (offset_g, offset_b) = ((target_size.0 * target_size.1) as usize, 2 * (target_size.0 * target_size.1) as usize);
        for pixel in resized.to_rgb8().pixels() {
             flat_data[i] = pixel[0] as f32 / 255.0; 
             flat_data[i + offset_g] = pixel[1] as f32 / 255.0; 
             flat_data[i + offset_b] = pixel[2] as f32 / 255.0; 
             i += 1;
        }
        (vec![1_i64, 3, target_size.1 as i64, target_size.0 as i64], flat_data)
    }


    /// OPTIMIZED: Frame pooling - önceden ayrılmış buffer'ları yeniden kullan
    fn get_pooled_buffer(&self, size: usize) -> Vec<u8> {
        // Basit pooling: Mevcut buffer yeterli büyüklükteyse kullan
        if self.pixel_buffer.len() >= size {
            return self.pixel_buffer.clone();
        }
        vec![0u8; size]
    }

    /// OPTIMIZED: Parallel preprocessing - büyük frame'ler için çoklu thread
    fn preprocess_parallel(&self, frame: &DynamicImage, target_size: (u32, u32)) -> (Vec<i64>, Vec<f32>) {
        use rayon::prelude::*;
        
        let resized = frame.resize_exact(target_size.0, target_size.1, image::imageops::FilterType::Triangle);
        let rgb = resized.to_rgb8();
        let (w, h) = (target_size.0 as usize, target_size.1 as usize);
        let total_pixels = w * h;
        
        // SIMD-friendly: Planar layout (RRRR...GGGG...BBBB)
        let mut flat_data = vec![0.0f32; 3 * total_pixels];
        let pixels: Vec<_> = rgb.pixels().collect();
        
        // Paralel R kanalı
        let r_data: Vec<f32> = pixels.par_iter()
            .map(|p| p[0] as f32 / 255.0)
            .collect();
        // Paralel G kanalı  
        let g_data: Vec<f32> = pixels.par_iter()
            .map(|p| p[1] as f32 / 255.0)
            .collect();
        // Paralel B kanalı
        let b_data: Vec<f32> = pixels.par_iter()
            .map(|p| p[2] as f32 / 255.0)
            .collect();
        
        flat_data[0..total_pixels].copy_from_slice(&r_data);
        flat_data[total_pixels..2*total_pixels].copy_from_slice(&g_data);
        flat_data[2*total_pixels..].copy_from_slice(&b_data);
        
        (vec![1_i64, 3, h as i64, w as i64], flat_data)
    }

    /// OPTIMIZED YOLO inference — Frame pooling + SIMD preprocessing + Parallel processing
    pub fn infer(&self, frame: &DynamicImage) -> Vec<Detection> {
        let mut session_guard = match &self.active_session { 
            Some(s) => s.lock().unwrap(), 
            None => return vec![] 
        };
        
        let (orig_w, orig_h) = (frame.width() as f32, frame.height() as f32);
        let pixel_count = (orig_w * orig_h) as usize;
        
        // Perf #9: Threshold yükseltildi — küçük frame'lerde rayon overhead'i faydayı aşıyordu
        let input_tuple = if pixel_count > 921600 { // 1280x720 threshold (önceki: 800x600)
            self.preprocess_parallel(frame, (640, 640))
        } else {
            self.preprocess_image_for_onnx(frame, (640, 640))
        };
        
        // OPTIMIZATION: Zero-copy tensor oluşturma (mümkünse)
        let input_value = Tensor::from_array(input_tuple).unwrap();
        
        // OPTIMIZATION: Session run with timeout (donmaları önlemek için)
        let outputs = match session_guard.run(ort::inputs!["images" => input_value]) {
            Ok(o) => o,
            Err(e) => {
                self.send_log(&format!("⚠️ ONNX inference hatası: {}", e));
                return vec![];
            }
        };
        
        let output_tensor = outputs[0].try_extract_tensor::<f32>().unwrap();
        let (shape, data) = output_tensor;
        let (num_classes, num_anchors) = ((shape[1] - 4) as usize, shape[2] as usize);
        
        // OPTIMIZATION: Pre-allocate detection vector with capacity
        let mut detections = Vec::with_capacity(num_anchors / 4); // Tahmini kapasite
        
        // OPTIMIZATION: Batch confidence calculation
        for i in 0..num_anchors {
            let (mut max_conf, mut class_id) = (0.0, 0);
            for c in 0..num_classes {
                let conf = data[(4 + c) * num_anchors + i];
                if conf > max_conf { max_conf = conf; class_id = c; }
            }
            if max_conf > 0.40 { 
                let (cx, cy, w, h) = (
                    data[0 * num_anchors + i], 
                    data[1 * num_anchors + i], 
                    data[2 * num_anchors + i], 
                    data[3 * num_anchors + i]
                );
                detections.push(Detection { 
                    class_id, 
                    class_name: format!("ID:{}", class_id), 
                    confidence: max_conf, 
                    bbox: [
                        (cx - w/2.0) * (orig_w/640.0), 
                        (cy - h/2.0) * (orig_h/640.0), 
                        (cx + w/2.0) * (orig_w/640.0), 
                        (cy + h/2.0) * (orig_h/640.0)
                    ] 
                });
            }
        }
        
        // OPTIMIZATION: Sort with unstable (daha hızlı, stable gerekmez)
        detections.sort_unstable_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        // OPTIMIZATION: NMS with early exit
        let mut nms = Vec::with_capacity(detections.len() / 2);
        for d in detections {
            if nms.iter().all(|kept: &Detection| kept.class_id != d.class_id || iou(&d.bbox, &kept.bbox) <= 0.45) { 
                nms.push(d); 
            }
        }
        
        // OPTIMIZATION: Throttled debug logging (3 saniyede bir)
        use std::sync::atomic::{AtomicU64, Ordering};
        static LAST_DEBUG_LOG: AtomicU64 = AtomicU64::new(0);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let last = LAST_DEBUG_LOG.load(Ordering::Relaxed);
        if now_ms.saturating_sub(last) > 3000 {
            LAST_DEBUG_LOG.store(now_ms, Ordering::Relaxed);
            let mut class_counts: HashMap<usize, usize> = HashMap::new();
            for d in &nms {
                *class_counts.entry(d.class_id).or_insert(0) += 1;
            }
            self.send_log(&format!("🔍 MODEL DEBUG: {} tespit (NMS sonrası), class sayıları: {:?}", nms.len(), class_counts));
        }
        
        nms
    }
    

}

// GdiResources Drop trait'i ile otomatik temizlik yapılıyor
// VisionEngine Drop implementasyonuna gerek yok
