// stats_dashboard.rs - Gelişmiş istatistik ve analiz paneli
#![allow(dead_code)]
use std::collections::HashMap;
use std::time::{Instant, Duration};

#[derive(Clone, Debug)]
pub struct ClientStats {
    pub client_id: usize,
    pub stones_mined: u32,
    pub stones_missed: u32,
    pub captcha_solved: u32,
    pub captcha_failed: u32,
    pub uptime_secs: f64,
    pub stones_per_hour: f32,
    pub captcha_success_rate: f32,
    pub efficiency: f32, // Taş/Toplam deneme oranı
    pub last_update: Instant,
    pub hourly_history: Vec<(u32, u32)>, // (saat, taş sayısı)
    pub state_history: Vec<(Instant, String)>, // Durum geçmişi
}

impl Default for ClientStats {
    fn default() -> Self {
        Self {
            client_id: 0,
            stones_mined: 0,
            stones_missed: 0,
            captcha_solved: 0,
            captcha_failed: 0,
            uptime_secs: 0.0,
            stones_per_hour: 0.0,
            captcha_success_rate: 0.0,
            efficiency: 0.0,
            last_update: Instant::now(),
            hourly_history: Vec::new(),
            state_history: Vec::new(),
        }
    }
}

pub struct StatsDashboard {
    clients: HashMap<usize, ClientStats>,
    global_start_time: Instant,
    total_stones_all_time: u32,
    total_captcha_all_time: u32,
}

impl StatsDashboard {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            global_start_time: Instant::now(),
            total_stones_all_time: 0,
            total_captcha_all_time: 0,
        }
    }
    
    pub fn update_client(&mut self, client_id: usize, stones: u32, missed: u32, captcha: u32, uptime: f64) {
        let entry = self.clients.entry(client_id).or_insert_with(|| ClientStats {
            client_id,
            last_update: Instant::now(),
            ..Default::default()
        });
        
        // Değişimleri hesapla
        let stone_diff = stones.saturating_sub(entry.stones_mined);
        let captcha_diff = captcha.saturating_sub(entry.captcha_solved);
        
        entry.stones_mined = stones;
        entry.stones_missed = missed;
        entry.captcha_solved = captcha;
        entry.uptime_secs = uptime;
        entry.last_update = Instant::now();
        
        // Saatlik verimlilik hesapla
        if uptime > 0.0 {
            entry.stones_per_hour = (stones as f64 / uptime * 3600.0) as f32;
        }
        
        // Toplam deneme = başarılı + başarısız
        let total_attempts = stones + missed;
        if total_attempts > 0 {
            entry.efficiency = stones as f32 / total_attempts as f32;
        }
        
        // Global istatistikleri güncelle
        self.total_stones_all_time += stone_diff;
        self.total_captcha_all_time += captcha_diff;
    }
    
    pub fn get_client_stats(&self, client_id: usize) -> Option<&ClientStats> {
        self.clients.get(&client_id)
    }
    
    pub fn get_all_clients(&self) -> Vec<&ClientStats> {
        self.clients.values().collect()
    }
    
    pub fn get_global_summary(&self) -> GlobalStats {
        let total_clients = self.clients.len();
        let active_clients = self.clients.values()
            .filter(|c| c.last_update.elapsed() < Duration::from_secs(30))
            .count();
        
        let total_stones: u32 = self.clients.values().map(|c| c.stones_mined).sum();
        let total_missed: u32 = self.clients.values().map(|c| c.stones_missed).sum();
        let total_captcha: u32 = self.clients.values().map(|c| c.captcha_solved).sum();
        
        let avg_efficiency = if total_clients > 0 {
            self.clients.values().map(|c| c.efficiency).sum::<f32>() / total_clients as f32
        } else {
            0.0
        };
        
        let total_uptime: f64 = self.clients.values().map(|c| c.uptime_secs).sum();
        let global_stones_per_hour = if total_uptime > 0.0 {
            (total_stones as f64 / total_uptime * 3600.0) as f32
        } else {
            0.0
        };
        
        GlobalStats {
            total_clients,
            active_clients,
            total_stones,
            total_missed,
            total_captcha,
            avg_efficiency,
            global_stones_per_hour,
            global_uptime_hours: total_uptime / 3600.0,
        }
    }
    
    pub fn remove_client(&mut self, client_id: usize) {
        self.clients.remove(&client_id);
    }
    
    /// En verimli client'ı bul
    pub fn get_top_performer(&self) -> Option<&ClientStats> {
        self.clients.values()
            .max_by(|a, b| a.stones_per_hour.partial_cmp(&b.stones_per_hour).unwrap_or(std::cmp::Ordering::Equal))
    }
    
    /// En düşük verimli client'ı bul (optimizasyon önerisi için)
    pub fn get_needs_attention(&self) -> Option<&ClientStats> {
        self.clients.values()
            .filter(|c| c.last_update.elapsed() < Duration::from_secs(60))
            .min_by(|a, b| a.efficiency.partial_cmp(&b.efficiency).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[derive(Clone, Debug)]
pub struct GlobalStats {
    pub total_clients: usize,
    pub active_clients: usize,
    pub total_stones: u32,
    pub total_missed: u32,
    pub total_captcha: u32,
    pub avg_efficiency: f32,
    pub global_stones_per_hour: f32,
    pub global_uptime_hours: f64,
}

impl GlobalStats {
    pub fn format_summary(&self) -> String {
        format!(
            "📊 GENEL İSTATİSTİKLER\n\
             ━━━━━━━━━━━━━━━━━━━━━\n\
             🖥️  Aktif Client: {}/{}\n\
             💎 Toplam Taş: {} ({}% verim)\n\
             📝 Captcha: {} çözüldü\n\
             ⚡ Saatlik Ort: {:.1} taş/saat\n\
             ⏱️  Toplam Çalışma: {:.1} saat",
            self.active_clients,
            self.total_clients,
            self.total_stones,
            (self.avg_efficiency * 100.0) as i32,
            self.total_captcha,
            self.global_stones_per_hour,
            self.global_uptime_hours
        )
    }
}