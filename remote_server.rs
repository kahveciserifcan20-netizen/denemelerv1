// remote_server.rs — v109
// MQTT tabanlı uzaktan kontrol sunucusu
// APK'dan gelen komutları dinler ve durum yayını yapar

use rumqttc::{MqttOptions, Event, Incoming, QoS, Client};
use crossbeam_channel::Sender;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// ── Komut Yapısı ──────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteCommand {
    pub action: String,        // "start", "stop", "status", "ping"
    pub hwnd: Option<usize>,   // start için gerekli
    pub model: Option<String>, // opsiyonel model
    pub driver: Option<String>,// opsiyonel driver
}

// ── Durum Yapısı (APK'ya gönderilecek) ─────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RemoteStatus {
    pub is_running: bool,
    pub stones_mined: u32,
    pub stones_missed: u32,
    pub captcha_solved: u32,
    pub uptime_secs: f64,
    pub state_name: String,
    pub active_clients: Vec<ActiveClient>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveClient {
    pub hwnd: usize,
    pub is_running: bool,
    pub stones_mined: u32,
    pub state_name: String,
}

// ── MQTT Ayarları ──────────────────────────────────────────────────────────
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub client_id: String,
    pub topic_cmd: String,
    pub topic_status: String,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: "broker.hivemq.com".to_string(),
            port: 1883,
            client_id: "kbot_server".to_string(),
            topic_cmd: "kbot/+/cmd".to_string(),
            topic_status: "kbot/status".to_string(),
        }
    }
}

// ── Global Status (Thread-safe) ─────────────────────────────────────────────
lazy_static::lazy_static! {
    static ref GLOBAL_STATUS: Arc<Mutex<RemoteStatus>> = Arc::new(Mutex::new(RemoteStatus::default()));
    static ref MQTT_CLIENT: Arc<Mutex<Option<Client>>> = Arc::new(Mutex::new(None));
}

// ── MQTT Server ──────────────────────────────────────────────────────────
pub struct RemoteServer {
    config: MqttConfig,
    cmd_tx: Sender<String>,
    running: bool,
}

impl RemoteServer {
    pub fn new(cmd_tx: Sender<String>) -> Self {
        Self {
            config: MqttConfig::default(),
            cmd_tx,
            running: false,
        }
    }

    /// MQTT sunucusunu başlat (arka planda çalışır)
    pub fn start(&mut self) {
        if self.running {
            println!("[MQTT] Zaten çalışıyor!");
            return;
        }

        let broker = self.config.broker.clone();
        let port = self.config.port;
        let client_id = self.config.client_id.clone();
        let topic_cmd = self.config.topic_cmd.clone();
        let topic_status = self.config.topic_status.clone();
        let cmd_tx = self.cmd_tx.clone();

        thread::spawn(move || {
            println!("[MQTT] Bağlanıyor: {}:{}", broker, port);

            let mut mqttoptions = MqttOptions::new(client_id, broker, port);
            mqttoptions.set_keep_alive(Duration::from_secs(30));
            mqttoptions.set_clean_session(true);

            let (client, mut connection) = rumqttc::Client::new(mqttoptions, 10);
            
            // Global client'ı sakla (status publish için)
            *MQTT_CLIENT.lock().unwrap() = Some(client.clone());

            // Komut topic'ine abone ol
            match client.subscribe(&topic_cmd, QoS::AtLeastOnce) {
                Ok(_) => println!("[MQTT] Abone olundu: {}", topic_cmd),
                Err(e) => {
                    eprintln!("[MQTT] Abone hatası: {}", e);
                    return;
                }
            }

            // Event loop
            loop {
                match connection.recv() {
                    Ok(Ok(Event::Incoming(Incoming::Publish(publish)))) => {
                        if let Ok(payload) = String::from_utf8(publish.payload.to_vec()) {
                            println!("[MQTT] Gelen: {}", payload);
                            
                            if let Ok(cmd) = serde_json::from_str::<RemoteCommand>(&payload) {
                                Self::process_command(&cmd_tx, &cmd);
                            } else {
                                Self::process_text_command(&cmd_tx, &payload);
                            }
                        }
                    }
                    Ok(Ok(Event::Incoming(Incoming::ConnAck(_)))) => {
                        println!("[MQTT] Bağlantı onaylandı");
                        // Yeniden abone ol
                        let _ = client.subscribe(&topic_cmd, QoS::AtLeastOnce);
                    }
                    Ok(Ok(Event::Incoming(Incoming::Disconnect))) => {
                        println!("[MQTT] Bağlantı koptu, yeniden bağlanılacak...");
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        eprintln!("[MQTT] Bağlantı hatası: {:?}", e);
                        thread::sleep(Duration::from_secs(5));
                    }
                    Err(e) => {
                        eprintln!("[MQTT] Recv hatası: {:?}", e);
                        thread::sleep(Duration::from_secs(5));
                    }
                }
            }
        });

        // Status broadcast thread'i (her 2 saniyede bir)
        let topic_status_clone = topic_status.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(2));
                
                if let Some(client) = MQTT_CLIENT.lock().unwrap().as_ref() {
                    if let Ok(status) = GLOBAL_STATUS.lock() {
                        if let Ok(json) = serde_json::to_string(&*status) {
                            let _ = client.publish(&topic_status_clone, QoS::AtLeastOnce, false, json);
                        }
                    }
                }
            }
        });

        self.running = true;
        println!("[MQTT] Sunucu başlatıldı!");
    }

    /// Komut işle
    fn process_command(cmd_tx: &Sender<String>, cmd: &RemoteCommand) {
        match cmd.action.to_lowercase().as_str() {
            "start" => {
                // Tüm aktif clientleri başlat
                let _ = cmd_tx.send("START_ALL".to_string());
                println!("[MQTT] START_ALL komutu gönderildi");
            }
            "stop" => {
                // Tüm clientleri durdur
                let _ = cmd_tx.send("STOP".to_string());
                println!("[MQTT] STOP komutu gönderildi");
            }
            "ping" => {
                println!("[MQTT] PING alındı");
            }
            _ => {
                println!("[MQTT] Bilinmeyen komut: {}", cmd.action);
            }
        }
    }

    /// Basit text komut işle
    fn process_text_command(cmd_tx: &Sender<String>, text: &str) {
        let text = text.trim();
        
        if text == "START" || text == "start" {
            let _ = cmd_tx.send("START_ALL".to_string());
        } else if text == "STOP" || text == "stop" {
            let _ = cmd_tx.send("STOP".to_string());
        }
    }

    /// Durumu güncelle (main.rs'den çağrılır)
    pub fn update_status(status: RemoteStatus) {
        if let Ok(mut global) = GLOBAL_STATUS.lock() {
            *global = status;
        }
    }
}