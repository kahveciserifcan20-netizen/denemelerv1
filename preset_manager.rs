// preset_manager.rs - Harita/Ayar şablonları yönetimi
#![allow(dead_code, unused_imports)]
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub model: String,
    pub kilit_region: (i32, i32, i32, i32),
    pub ocr_region: (i32, i32, i32, i32),
    pub pot_tusu: String,
    pub toplama_tusu: String,
    pub arama_q_sure: f32,
    pub arama_e_sure: f32,
    pub mola_aralik_dk: u32,
    pub mola_sure_dk: u32,
    pub created_at: String,
}

pub struct PresetManager {
    presets: HashMap<String, Preset>,
    presets_dir: String,
}

impl PresetManager {
    pub fn new() -> Self {
        let presets_dir = "presets".to_string();
        let _ = fs::create_dir_all(&presets_dir);
        
        let mut manager = Self {
            presets: HashMap::new(),
            presets_dir,
        };
        manager.load_all();
        manager
    }
    
    fn load_all(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.presets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(preset) = serde_json::from_str::<Preset>(&content) {
                            self.presets.insert(preset.name.clone(), preset);
                        }
                    }
                }
            }
        }
    }
    
    pub fn save(&self, preset: &Preset) -> bool {
        let filename = format!("{}/{}.json", self.presets_dir, preset.name);
        if let Ok(json) = serde_json::to_string_pretty(preset) {
            return fs::write(&filename, json).is_ok();
        }
        false
    }
    
    pub fn load(&self, name: &str) -> Option<Preset> {
        self.presets.get(name).cloned()
    }
    
    pub fn delete(&mut self, name: &str) -> bool {
        let filename = format!("{}/{}.json", self.presets_dir, name);
        if fs::remove_file(&filename).is_ok() {
            self.presets.remove(name);
            return true;
        }
        false
    }
    
    pub fn list(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }
    
    pub fn exists(&self, name: &str) -> bool {
        self.presets.contains_key(name)
    }
    
    /// Pencere başlığına göre preset öner
    pub fn suggest_for_window(&self, window_title: &str) -> Option<String> {
        let title_lower = window_title.to_lowercase();
        
        // Harita isimlerini tespit et
        let map_keywords: Vec<(&str, &str)> = vec![
            ("vadi", "Vadi"),
            ("doyum", "Doyum"),
            ("guatama", "Guatama"),
            ("kızıl", "Kızıl"),
            ("buyulu", "Buyulu"),
        ];
        
        for (keyword, preset_name) in map_keywords {
            if title_lower.contains(keyword) {
                if self.exists(preset_name) {
                    return Some(preset_name.to_string());
                }
            }
        }
        
        None
    }
}