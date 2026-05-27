/// ═══════════════════════════════════════════════════════════════════════════════
/// ONLINE LEARNING SİSTEMİ - Gerçek zamanlı öğrenme
/// ═══════════════════════════════════════════════════════════════════════════════
/// 
/// Çalışma prensibi:
/// 1. Her tespit için feature vector çıkarılır (renk, şekil, texture)
/// 2. Kullanıcı feedback: Evet (+puan) / Hayır (-puan)
/// 3. Feature + label kaydedilir
/// 4. k-NN classifier ile tahmin yapılır
/// 5. Zamanla model "doğru" hedefleri öğrenir

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use image::DynamicImage;

/// Feature Vector - Görsel özellikler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    /// Ortalama renk değerleri
    pub avg_red: f32,
    pub avg_green: f32,
    pub avg_blue: f32,
    
    /// Renk varyansı
    pub var_red: f32,
    pub var_green: f32,
    pub var_blue: f32,
    
    /// Şekil özellikleri
    pub aspect_ratio: f32,      // genişlik / yükseklik
    pub area_ratio: f32,        // doluluk oranı
    
    /// Texture özellikleri
    pub edge_density: f32,      // kenar yoğunluğu
    pub color_diversity: f32,   // renk çeşitliliği
    
    /// Özel Metin2 özellikleri
    pub metin_blue_ratio: f32,  // metin taşı mavisi oranı
    pub glow_intensity: f32,    // parıltı yoğunluğu
    pub crystal_pattern: f32,    // kristal deseni skoru
}

/// Öğrenme örneği
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub id: u64,
    pub features: FeatureVector,
    pub label: bool,           // true = metin taşı, false = kesilemez taş
    pub score: i32,            // toplam puan
    pub feedback_count: u32,   // kaç kez feedback alındı
    pub timestamp: u64,
    pub screenshot_path: Option<String>,
}

/// Online Learner - Gerçek zamanlı öğrenme
pub struct OnlineLearner {
    /// Eğitim örnekleri
    samples: Vec<TrainingSample>,
    
    /// Feature -> Sample ID eşlemesi (hızlı arama için)
    feature_index: HashMap<String, u64>,
    
    /// Model dosyası yolu
    model_path: String,
    
    /// Sonraki sample ID
    next_id: u64,
    
    /// Minimum puan eşiği (bu üstü = metin taşı)
    #[allow(dead_code)]
    score_threshold: i32,
    
    /// k-NN için k değeri
    k_neighbors: usize,
    
    /// Log sender
    log_tx: crossbeam_channel::Sender<String>,
    
    /// Son dosya değişiklik zamanı (diğer client'ların değişikliklerini algılamak için)
    last_file_modified: Option<SystemTime>,
}

impl OnlineLearner {
    pub fn new(log_tx: crossbeam_channel::Sender<String>) -> Self {
        let model_path = "online_model.json".to_string();
        let mut learner = Self {
            samples: Vec::new(),
            feature_index: HashMap::new(),
            model_path,
            next_id: 1,
            score_threshold: 500,  // +500 = metin taşı
            k_neighbors: 5,
            log_tx,
            last_file_modified: None,
        };
        
        // Kaydedilmiş modeli yükle
        learner.load_model();
        learner
    }
    
    fn send_log(&self, msg: &str) {
        let ts = chrono::Local::now().format("%H:%M:%S.%3f");
        let _ = self.log_tx.send(format!("[{}] {}", ts, msg));
    }
    
    /// Görselden feature vector çıkar
    pub fn extract_features(&self, img: &DynamicImage, bbox: &[f32; 4]) -> FeatureVector {
        let x1 = bbox[0].max(0.0) as u32;
        let y1 = bbox[1].max(0.0) as u32;
        let x2 = bbox[2].min(img.width() as f32) as u32;
        let y2 = bbox[3].min(img.height() as f32) as u32;
        
        let w = x2.saturating_sub(x1);
        let h = y2.saturating_sub(y1);
        
        if w == 0 || h == 0 {
            return FeatureVector::default();
        }
        
        // Clone instead of crop to avoid mutable borrow
        let rgba = image::imageops::crop_imm(&img.to_rgba8(), x1, y1, w, h).to_image();
        
        // Renk istatistikleri
        let mut red_sum = 0.0f32;
        let mut green_sum = 0.0f32;
        let mut blue_sum = 0.0f32;
        let pixel_count = (w * h) as f32;
        
        for p in rgba.pixels() {
            red_sum += p[0] as f32;
            green_sum += p[1] as f32;
            blue_sum += p[2] as f32;
        }
        
        let avg_red = red_sum / pixel_count;
        let avg_green = green_sum / pixel_count;
        let avg_blue = blue_sum / pixel_count;
        
        // Varyans
        let mut var_red = 0.0f32;
        let mut var_green = 0.0f32;
        let mut var_blue = 0.0f32;
        
        for p in rgba.pixels() {
            var_red += (p[0] as f32 - avg_red).powi(2);
            var_green += (p[1] as f32 - avg_green).powi(2);
            var_blue += (p[2] as f32 - avg_blue).powi(2);
        }
        
        var_red /= pixel_count;
        var_green /= pixel_count;
        var_blue /= pixel_count;
        
        // Şekil özellikleri
        let aspect_ratio = w as f32 / h as f32;
        let area_ratio = pixel_count / (w.max(h) as f32).powi(2);
        
        // Kenar yoğunluğu (basit Sobel)
        let edge_density = self.calculate_edge_density(&rgba);
        
        // Renk çeşitliliği
        let color_diversity = self.calculate_color_diversity(&rgba);
        
        // Metin2 özel özellikler
        let metin_blue_ratio = self.calculate_metin_blue_ratio(&rgba);
        let glow_intensity = self.calculate_glow_intensity(&rgba);
        let crystal_pattern = self.calculate_crystal_pattern(&rgba);
        
        FeatureVector {
            avg_red,
            avg_green,
            avg_blue,
            var_red,
            var_green,
            var_blue,
            aspect_ratio,
            area_ratio,
            edge_density,
            color_diversity,
            metin_blue_ratio,
            glow_intensity,
            crystal_pattern,
        }
    }
    
    /// Kenar yoğunluğu hesapla
    fn calculate_edge_density(&self, img: &image::RgbaImage) -> f32 {
        let (w, h) = img.dimensions();
        if w < 3 || h < 3 { return 0.0; }
        
        let mut edge_count = 0;
        let total = ((w - 2) * (h - 2)) as f32;
        
        for y in 1..h-1 {
            for x in 1..w-1 {
                let p = img.get_pixel(x, y);
                let p_right = img.get_pixel(x + 1, y);
                let p_down = img.get_pixel(x, y + 1);
                
                // Basit gradient
                let gx = (p_right[0] as i32 - p[0] as i32).abs() +
                         (p_right[1] as i32 - p[1] as i32).abs() +
                         (p_right[2] as i32 - p[2] as i32).abs();
                
                let gy = (p_down[0] as i32 - p[0] as i32).abs() +
                         (p_down[1] as i32 - p[1] as i32).abs() +
                         (p_down[2] as i32 - p[2] as i32).abs();
                
                if gx + gy > 100 {
                    edge_count += 1;
                }
            }
        }
        
        edge_count as f32 / total
    }
    
    /// Renk çeşitliliği
    fn calculate_color_diversity(&self, img: &image::RgbaImage) -> f32 {
        let mut color_buckets: [u32; 64] = [0; 64];
        
        for p in img.pixels() {
            // 2-bit per channel = 6-bit index
            let r = (p[0] >> 6) & 0b11;
            let g = (p[1] >> 6) & 0b11;
            let b = (p[2] >> 6) & 0b11;
            let idx = (r << 4) | (g << 2) | b;
            color_buckets[idx as usize] += 1;
        }
        
        let _total = img.width() * img.height();
        let mut diversity = 0;
        
        for &count in &color_buckets {
            if count > 0 {
                diversity += 1;
            }
        }
        
        diversity as f32 / 64.0
    }
    
    /// Metin taşı mavisi oranı (özel renk)
    fn calculate_metin_blue_ratio(&self, img: &image::RgbaImage) -> f32 {
        let mut metin_pixels = 0;
        let total = (img.width() * img.height()) as f32;
        
        for p in img.pixels() {
            // Metin taşı mavisi: düşük kırmızı, orta yeşil, yüksek mavi
            if p[0] < 80 && p[1] > 60 && p[1] < 150 && p[2] > 120 {
                metin_pixels += 1;
            }
        }
        
        metin_pixels as f32 / total
    }
    
    /// Parıltı yoğunluğu
    fn calculate_glow_intensity(&self, img: &image::RgbaImage) -> f32 {
        let mut glow_sum = 0.0f32;
        let total = (img.width() * img.height()) as f32;
        
        for p in img.pixels() {
            // Yüksek parlaklık = parıltı
            let brightness = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
            if brightness > 200.0 {
                glow_sum += brightness - 200.0;
            }
        }
        
        glow_sum / (total * 55.0)
    }
    
    /// Kristal deseni skoru
    fn calculate_crystal_pattern(&self, img: &image::RgbaImage) -> f32 {
        let (w, h) = img.dimensions();
        if w < 10 || h < 10 { return 0.0; }
        
        // Merkez-kenar kontrastı
        let cx = w / 2;
        let cy = h / 2;
        let center = img.get_pixel(cx, cy);
        
        let mut edge_avg = 0.0f32;
        let edge_count = 8;
        
        // 8 yöndeki kenar pikselleri
        let positions = [
            (0, 0), (w-1, 0), (0, h-1), (w-1, h-1),
            (cx, 0), (cx, h-1), (0, cy), (w-1, cy)
        ];
        
        for (ex, ey) in positions {
            let ep = img.get_pixel(ex, ey);
            edge_avg += ((center[0] as i32 - ep[0] as i32).abs() +
                        (center[1] as i32 - ep[1] as i32).abs() +
                        (center[2] as i32 - ep[2] as i32).abs()) as f32;
        }
        
        edge_avg / (edge_count as f32 * 255.0)
    }
    
    /// Yeni tespit için tahmin yap
    pub fn predict(&self, features: &FeatureVector) -> (bool, f32) {
        // k-NN ile tahmin
        if self.samples.is_empty() {
            return (true, 0.5); // Veri yoksa varsayılan evet
        }
        
        // En yakın k komşuyu bul
        let mut distances: Vec<(f32, bool, i32)> = self.samples.iter()
            .map(|s| {
                let dist = self.feature_distance(features, &s.features);
                (dist, s.label, s.score)
            })
            .collect();
        
        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Ağırlıklı oylama
        let mut positive_weight = 0.0f32;
        let mut total_weight = 0.0f32;
        
        for (_i, (dist, label, score)) in distances.iter().take(self.k_neighbors).enumerate() {
            let weight = 1.0 / (dist + 0.001); // Uzaklık tersine orantılı
            let score_weight = (*score as f32).abs() / 1000.0; // Puan ağırlığı
            
            let final_weight = weight * (1.0 + score_weight);
            
            if *label {
                positive_weight += final_weight;
            }
            total_weight += final_weight;
        }
        
        let confidence = positive_weight / total_weight;
        let is_metin = confidence > 0.5;
        
        (is_metin, confidence)
    }
    
    /// Feature mesafesi (Euclidean)
    fn feature_distance(&self, f1: &FeatureVector, f2: &FeatureVector) -> f32 {
        let mut dist = 0.0f32;
        
        // Normalize edilmiş özellikler
        dist += (f1.avg_red / 255.0 - f2.avg_red / 255.0).powi(2);
        dist += (f1.avg_green / 255.0 - f2.avg_green / 255.0).powi(2);
        dist += (f1.avg_blue / 255.0 - f2.avg_blue / 255.0).powi(2);
        
        dist += (f1.var_red / 10000.0 - f2.var_red / 10000.0).powi(2);
        dist += (f1.var_green / 10000.0 - f2.var_green / 10000.0).powi(2);
        dist += (f1.var_blue / 10000.0 - f2.var_blue / 10000.0).powi(2);
        
        dist += (f1.aspect_ratio - f2.aspect_ratio).powi(2);
        dist += (f1.area_ratio - f2.area_ratio).powi(2);
        
        dist += (f1.edge_density - f2.edge_density).powi(2);
        dist += (f1.color_diversity - f2.color_diversity).powi(2);
        
        // Metin2 özel ağırlıklı
        dist += 2.0 * (f1.metin_blue_ratio - f2.metin_blue_ratio).powi(2);
        dist += 2.0 * (f1.glow_intensity - f2.glow_intensity).powi(2);
        dist += 1.5 * (f1.crystal_pattern - f2.crystal_pattern).powi(2);
        
        dist.sqrt()
    }
    
    /// Kullanıcı feedback'i işle
    pub fn process_feedback(&mut self, features: FeatureVector, is_positive: bool, screenshot_path: Option<String>) {
        // Benzer örnek var mı kontrol et
        let feature_key = self.feature_to_key(&features);
        
        // Log için değişkenleri hazırla
        let mut log_msg = String::new();
        
        if let Some(&existing_id) = self.feature_index.get(&feature_key) {
            // Mevcut örneği güncelle
            if let Some(sample) = self.samples.iter_mut().find(|s| s.id == existing_id) {
                let delta = if is_positive { 500 } else { -500 };
                sample.score = (sample.score + delta).clamp(-5000, 5000);
                sample.feedback_count += 1;
                sample.label = sample.score > 0;
                
                // Log mesajını hazırla (borrow sonrası)
                log_msg = format!(
                    "🧠 ÖĞRENME: Örnek #{} güncellendi → Puan: {} ({} feedback)",
                    sample.id, sample.score, sample.feedback_count
                );
            }
        } else {
            // Yeni örnek oluştur
            let sample = TrainingSample {
                id: self.next_id,
                features: features.clone(),
                label: is_positive,
                score: if is_positive { 500 } else { -500 },
                feedback_count: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                screenshot_path,
            };
            
            self.feature_index.insert(feature_key, self.next_id);
            self.next_id += 1;
            self.samples.push(sample);
            
            // Log mesajını hazırla
            log_msg = format!(
                "🧠 ÖĞRENME: Yeni örnek kaydedildi → {} (Toplam: {} örnek)",
                if is_positive { "METİN TAŞI ✓" } else { "KESİLEMEZ TAŞ ✗" },
                self.samples.len()
            );
        }
        
        // Log gönder (artık borrow yok)
        self.send_log(&log_msg);
        
        // Modeli kaydet
        self.save_model();
    }
    
    /// Feature -> Hash key
    fn feature_to_key(&self, f: &FeatureVector) -> String {
        // Yuvarlatılmış değerler ile key oluştur
        format!(
            "{:.0}_{:.0}_{:.0}_{:.1}_{:.1}",
            f.avg_red, f.avg_green, f.avg_blue,
            f.metin_blue_ratio * 10.0,
            f.glow_intensity * 10.0
        )
    }
    
    /// Modeli kaydet — atomic write ile (temp dosyaya yaz, sonra rename)
    /// Çoklu client'ta aynı dosyaya yazıyor, corruption önlenir
    fn save_model(&mut self) {
        let json = serde_json::to_string_pretty(&self.samples).unwrap_or_default();
        let temp_path = format!("{}.tmp", self.model_path);
        if let Ok(mut file) = File::create(&temp_path) {
            if file.write_all(json.as_bytes()).is_ok() {
                let _ = std::fs::rename(&temp_path, &self.model_path);
                // Son değişiklik zamanını güncelle
                if let Ok(meta) = std::fs::metadata(&self.model_path) {
                    self.last_file_modified = meta.modified().ok();
                }
            }
        }
    }
    
    /// Modeli yükle
    fn load_model(&mut self) {
        if !Path::new(&self.model_path).exists() {
            return;
        }
        
        if let Ok(file) = File::open(&self.model_path) {
            let reader = BufReader::new(file);
            if let Ok(samples) = serde_json::from_reader::<_, Vec<TrainingSample>>(reader) {
                self.samples = samples;
                
                // Index'i yeniden oluştur
                for sample in &self.samples {
                    let key = self.feature_to_key(&sample.features);
                    self.feature_index.insert(key, sample.id);
                    if sample.id >= self.next_id {
                        self.next_id = sample.id + 1;
                    }
                }
                
                self.send_log(&format!("🧠 Online Model yüklendi: {} örnek", self.samples.len()));
                
                // Son değişiklik zamanını kaydet
                if let Ok(meta) = std::fs::metadata(&self.model_path) {
                    self.last_file_modified = meta.modified().ok();
                }
            }
        }
    }
    
    /// Diğer client'ların değişikliklerini kontrol et ve yükle
    /// Çoklu client paylaşımlı model için: periyodik olarak çağrılır
    pub fn reload_if_changed(&mut self) {
        if !Path::new(&self.model_path).exists() { return; }
        
        if let Ok(meta) = std::fs::metadata(&self.model_path) {
            if let Ok(modified) = meta.modified() {
                let should_reload = match self.last_file_modified {
                    Some(last) => modified > last,
                    None => true,
                };
                
                if should_reload {
                    let old_count = self.samples.len();
                    self.load_model();
                    if self.samples.len() != old_count {
                        self.send_log(&format!("🧠 Model güncellendi (diğer client): {} → {} örnek", 
                            old_count, self.samples.len()));
                    }
                }
            }
        }
    }
    
    /// İstatistikler
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let positive = self.samples.iter().filter(|s| s.label).count();
        let negative = self.samples.len() - positive;
        (self.samples.len(), positive, negative)
    }
    
    /// Modeli sıfırla
    #[allow(dead_code)]
    pub fn reset_model(&mut self) {
        self.samples.clear();
        self.feature_index.clear();
        self.next_id = 1;
        
        if Path::new(&self.model_path).exists() {
            let _ = std::fs::remove_file(&self.model_path);
        }
        
        self.send_log("🧠 Online Model sıfırlandı");
    }
}

impl Default for FeatureVector {
    fn default() -> Self {
        Self {
            avg_red: 0.0,
            avg_green: 0.0,
            avg_blue: 0.0,
            var_red: 0.0,
            var_green: 0.0,
            var_blue: 0.0,
            aspect_ratio: 1.0,
            area_ratio: 1.0,
            edge_density: 0.0,
            color_diversity: 0.0,
            metin_blue_ratio: 0.0,
            glow_intensity: 0.0,
            crystal_pattern: 0.0,
        }
    }
}