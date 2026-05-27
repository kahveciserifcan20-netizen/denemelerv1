// hw_simulator.rs — v111
// Donanım simülatörü: WinAPI (SendInput) veya Arduino (serial) üzerinden
// mouse / keyboard input. İnsansı hareket için kübik Bezier + jitter + ease-in-out.
//
// v111: Hibrit tıklama modu — PostMessageW (arka plan) + Focus-Swap (optimize)
//       Çoklu client desteği: PostMessageW modunda MOUSE_LOCK gereksiz
//
// rand 0.9+ API kullanır: rand::thread_rng() + Rng::random_range()

use std::thread;
use std::time::Duration;
use std::io::Write;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global fare kilidi - aynı anda sadece bir client fareyi kontrol edebilir
/// Daha verimli: Exponential backoff ile spin lock
static MOUSE_LOCK: AtomicBool = AtomicBool::new(false);

/// Fare kilidini al - Exponential backoff ile daha verimli bekleme
fn acquire_mouse_lock() {
    let mut attempts = 0;
    let max_backoff_ms = 100;
    
    loop {
        match MOUSE_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return, // Kilit alındı
            Err(_) => {
                // Exponential backoff
                attempts += 1;
                let backoff = (1 << attempts.min(6)).min(max_backoff_ms); // Max 64ms
                thread::sleep(Duration::from_millis(backoff as u64));
                
                // Çok uzun süre bekliyorsa log at (debug için)
                if attempts > 100 {
                    eprintln!("[HW] ⚠️ Fare kilidi uzun süre bekleniyor ({} deneme)", attempts);
                    attempts = 0; // Reset ama devam et
                }
            }
        }
    }
}

/// Fare kilidini bırak
fn release_mouse_lock() {
    MOUSE_LOCK.store(false, Ordering::Release);
}

/// Fare kilidini timeout ile al - başarısız olursa false döner
#[allow(dead_code)]
fn try_acquire_mouse_lock(timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    let mut attempts = 0;
    
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if MOUSE_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            return true;
        }
        
        // Exponential backoff
        attempts += 1;
        let backoff = (1 << attempts.min(4)).min(16); // Max 16ms
        thread::sleep(Duration::from_millis(backoff as u64));
    }
    
    false // Timeout
}

use winapi::um::winuser::{
    GetCursorPos, SendInput, INPUT, INPUT_MOUSE, INPUT_KEYBOARD,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    MapVirtualKeyA, MAPVK_VK_TO_VSC,
    PostMessageW, SetForegroundWindow, SetFocus, BringWindowToTop, ShowWindow,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_KEYDOWN, WM_KEYUP, MK_LBUTTON, SW_RESTORE,
    WM_MOUSEMOVE,
};
use winapi::shared::windef::{POINT, HWND};
use winapi::shared::minwindef::{WPARAM, LPARAM};
use std::mem::zeroed;

/// Tıklama modu — çoklu client optimizasyonu
/// PostMessageW: Tam arka plan, paralel çalışır, pencere değiştirme YOK
/// FocusSwap: Optimize pencere değiştirme (~20ms), fiziksel input
/// Hibrit: PostMessageW varsayılan, gerektiğinde FocusSwap (Captcha vb.)
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ClickMode {
    PostMessageW,   // Tam arka plan — 10+ client
    FocusSwap,      // Optimize pencere değiştirme — 3-4 client
    Hibrit,         // PostMessageW + gerekince FocusSwap
}

#[allow(dead_code)]
impl ClickMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postmessagew" | "postmessage" | "post" => ClickMode::PostMessageW,
            "focusswap" | "focus" | "swap" => ClickMode::FocusSwap,
            _ => ClickMode::Hibrit,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            ClickMode::PostMessageW => "PostMessageW",
            ClickMode::FocusSwap => "FocusSwap",
            ClickMode::Hibrit => "Hibrit",
        }
    }
}

/// LPARAM paketleme: LOWORD=x, HIWORD=y (WM_LBUTTON* mesajları için)
fn make_lparam(x: i32, y: i32) -> LPARAM {
    ((x as u16 as u32) | ((y as u16 as u32) << 16)) as LPARAM
}

/// Input modu — WinAPI, Arduino serial veya Interception (kernel seviyesi)
/// `port_name` örn: "COM3", "Arduino_COM3", "Arduino_AUTO"
/// Interception: Anti-cheat atlatan kernel seviyesi sanal fare (driver gerektirir)
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum InputMode {
    WinAPI,
    Arduino { port_name: String },
    Interception, // Kernel seviyesi sanal fare - anti-cheat atlatır
}

/// Ana simülatör.
/// `serial_port` Arc<Mutex<...>> ile sarmalanır → &self ile yazma yapılabilir
/// ve thread-safe paylaşılabilir.
#[allow(dead_code)]
pub struct HwSimulator {
    mode: InputMode,
    screen_width: i32,
    screen_height: i32,
    serial_port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort + Send>>>,
    port_name_cache: String, // Reconnect için port adı cache
    last_reconnect_attempt: std::sync::Mutex<std::time::Instant>,
}

const ARDUINO_BAUD: u32 = 115200;

#[allow(dead_code)]
impl HwSimulator {
    pub fn new(mode: InputMode) -> Self {
        let (sw, sh) = unsafe {
            (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };

        let mut active_mode = mode.clone();
        let mut port_handle: Option<Box<dyn serialport::SerialPort + Send>> = None;

        if let InputMode::Arduino { ref port_name } = mode {
            // "Arduino_COM3" → "COM3", "Arduino_AUTO" → otomatik tara
            let resolved = Self::resolve_port_name(port_name);
            match resolved {
                Some(real_port) => {
                    match serialport::new(&real_port, ARDUINO_BAUD)
                        .timeout(Duration::from_millis(100))
                        .open()
                    {
                        Ok(p) => {
                            println!("[HW] Arduino: {} @ {}", real_port, ARDUINO_BAUD);
                            port_handle = Some(p);
                        }
                        Err(e) => {
                            eprintln!("[HW] Arduino acilamadi: {} - WinAPI mod", e);
                            active_mode = InputMode::WinAPI;
                        }
                    }
                }
                None => {
                    eprintln!("[HW] Arduino port yok - WinAPI mod");
                    active_mode = InputMode::WinAPI;
                }
            }
        }

        // Cache port name for reconnect
        let cached_port = if let InputMode::Arduino { port_name: _ } = mode {
            mode.clone()
        } else {
            active_mode.clone()
        };
        let port_name_cache = if let InputMode::Arduino { ref port_name } = cached_port {
            port_name.clone()
        } else {
            String::new()
        };

        Self {
            mode: active_mode,
            screen_width: sw,
            screen_height: sh,
            serial_port: std::sync::Mutex::new(port_handle),
            port_name_cache,
            last_reconnect_attempt: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        let mut active_mode = mode.clone();
        let mut port_handle: Option<Box<dyn serialport::SerialPort + Send>> = None;

        if let InputMode::Arduino { ref port_name } = mode {
            let resolved = Self::resolve_port_name(port_name);
            if let Some(real_port) = resolved {
                match serialport::new(&real_port, ARDUINO_BAUD)
                    .timeout(Duration::from_millis(100))
                    .open()
                {
                    Ok(p) => {
                        println!("[HW] Arduino mode changed: {} @ {}", real_port, ARDUINO_BAUD);
                        port_handle = Some(p);
                    }
                    Err(e) => {
                        eprintln!("[HW] Arduino couldn't open: {} - fallback to WinAPI", e);
                        active_mode = InputMode::WinAPI;
                    }
                }
            } else {
                eprintln!("[HW] No Arduino port found - fallback to WinAPI");
                active_mode = InputMode::WinAPI;
            }
        }

        let port_name_cache = if let InputMode::Arduino { ref port_name } = active_mode {
            port_name.clone()
        } else {
            String::new()
        };

        self.mode = active_mode;
        if let Ok(mut guard) = self.serial_port.lock() {
            *guard = port_handle;
        }
        self.port_name_cache = port_name_cache;
    }

    fn resolve_port_name(raw: &str) -> Option<String> {
        let upper = raw.to_uppercase();
        if upper.contains("AUTO") {
            // İlk uygun COM portu bul
            if let Ok(ports) = serialport::available_ports() {
                for p in ports {
                    if p.port_name.to_uppercase().starts_with("COM") {
                        return Some(p.port_name);
                    }
                }
            }
            return None;
        }
        // "Arduino_COM3" → "COM3"
        if let Some(idx) = upper.find("COM") {
            let tail: String = raw[idx..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            return Some(tail);
        }
        Some(raw.to_string())
    }

    pub fn get_cursor_pos(&self) -> (i32, i32) {
        unsafe {
            let mut pt: POINT = zeroed();
            GetCursorPos(&mut pt);
            (pt.x, pt.y)
        }
    }

    /// Anlık (insansı değil) mouse pozisyonlama - KİLİTLİ
    pub fn move_mouse(&self, x: i32, y: i32) {
        acquire_mouse_lock();
        match &self.mode {
            InputMode::WinAPI => self.winapi_move_absolute(x, y),
            InputMode::Arduino { .. } => {
                if !self.arduino_move_absolute(x, y) {
                    self.winapi_move_absolute(x, y);
                }
            }
            InputMode::Interception => {
                // Interception modu - şimdilik WinAPI'ye fallback
                // TODO: Interception driver API'si düzeltildiğinde gerçek implementasyon eklenecek
                self.winapi_move_absolute(x, y);
            }
        }
        release_mouse_lock();
    }

    /// İnsansı mouse: kübik Bezier + ease-in-out + mikro jitter + değişken hız - KİLİTLİ
    pub fn human_move(&self, target_x: i32, target_y: i32) {
        acquire_mouse_lock(); // Tüm hareket boyunca kilitli
        
        let (start_x, start_y) = self.get_cursor_pos();
        let mut rng = rand::thread_rng();

        let dx = (target_x - start_x) as f64;
        let dy = (target_y - start_y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < 1.5 {
            self.winapi_move_absolute(target_x, target_y);
            release_mouse_lock();
            return;
        }

        let base_steps = ((distance / 8.0) as i32).clamp(15, 80);
        let steps = (base_steps + rng.gen_range(-5..=5)).max(8);

        let deviation = (distance * 0.15).clamp(10.0, 120.0);
        let ctrl1_x = start_x as f64 + dx * 0.3 + rng.gen_range(-deviation..deviation);
        let ctrl1_y = start_y as f64 + dy * 0.3 + rng.gen_range(-deviation..deviation);
        let ctrl2_x = start_x as f64 + dx * 0.7 + rng.gen_range(-deviation..=deviation) * 0.5;
        let ctrl2_y = start_y as f64 + dy * 0.7 + rng.gen_range(-deviation..=deviation) * 0.5;

        let (sx, sy) = (start_x as f64, start_y as f64);
        let (ex, ey) = (target_x as f64, target_y as f64);

        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let te = Self::ease_in_out_cubic(t);

            let inv = 1.0 - te;
            let inv2 = inv * inv;
            let inv3 = inv2 * inv;
            let t2 = te * te;
            let t3 = t2 * te;

            let px = inv3 * sx + 3.0 * inv2 * te * ctrl1_x + 3.0 * inv * t2 * ctrl2_x + t3 * ex;
            let py = inv3 * sy + 3.0 * inv2 * te * ctrl1_y + 3.0 * inv * t2 * ctrl2_y + t3 * ey;

            let jx: f64 = rng.gen_range(-1.0..=1.0);
            let jy: f64 = rng.gen_range(-1.0..=1.0);

            // Direkt WinAPI kullan (move_mouse tekrar kilit almamalı)
            self.winapi_move_absolute((px + jx).round() as i32, (py + jy).round() as i32);

            let base_delay = if distance > 400.0 { 4.0 } else { 6.0 };
            let speed_factor = 1.0 + (1.0 - (2.0 * t - 1.0).abs()) * 0.5;
            let delay_ms: f64 = (base_delay / speed_factor) + rng.gen_range(0.0..2.0);
            thread::sleep(Duration::from_micros((delay_ms * 1000.0) as u64));
        }

        self.winapi_move_absolute(target_x, target_y);
        release_mouse_lock(); // Hareket tamamlandı, kilidi bırak
        thread::sleep(Duration::from_millis(rng.gen_range(5..25)));
    }

    pub fn click(&self, button: &str) {
        acquire_mouse_lock();
        let mut rng = rand::thread_rng();
        thread::sleep(Duration::from_millis(rng.gen_range(10..45)));

        match &self.mode {
            InputMode::WinAPI => self.winapi_click(button),
            InputMode::Arduino { .. } => {
                if !self.arduino_click(button) {
                    self.winapi_click(button);
                }
            }
            InputMode::Interception => {
                // Interception modu - şimdilik WinAPI'ye fallback
                // TODO: Interception driver API'si düzeltildiğinde gerçek implementasyon eklenecek
                self.winapi_click(button);
            }
        }

        thread::sleep(Duration::from_millis(rng.gen_range(30..80)));
        release_mouse_lock();
    }

    #[allow(dead_code)]
    pub fn key_press(&self, vk_code: u16) {
        let mut rng = rand::thread_rng();
        unsafe {
            let scan_code = MapVirtualKeyA(vk_code as u32, MAPVK_VK_TO_VSC) as u16;

            let mut input_down: INPUT = zeroed();
            input_down.type_ = INPUT_KEYBOARD;
            {
                let ki = input_down.u.ki_mut();
                ki.wVk = vk_code;
                ki.wScan = scan_code;
                ki.dwFlags = KEYEVENTF_SCANCODE;
            }
            SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);

            thread::sleep(Duration::from_millis(rng.gen_range(40..120)));

            let mut input_up: INPUT = zeroed();
            input_up.type_ = INPUT_KEYBOARD;
            {
                let ki_up = input_up.u.ki_mut();
                ki_up.wVk = vk_code;
                ki_up.wScan = scan_code;
                ki_up.dwFlags = KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP;
            }
            SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Tuşu belirli bir süre basılı tutar (arama/kamera çevirme için)
    /// duration_ms: milisaniye cinsinden basılı tutma süresi
    pub fn key_hold(&self, vk_code: u16, duration_ms: u64) {
        unsafe {
            let scan_code = MapVirtualKeyA(vk_code as u32, MAPVK_VK_TO_VSC) as u16;

            // Tuşa bas
            let mut input_down: INPUT = zeroed();
            input_down.type_ = INPUT_KEYBOARD;
            {
                let ki = input_down.u.ki_mut();
                ki.wVk = vk_code;
                ki.wScan = scan_code;
                ki.dwFlags = KEYEVENTF_SCANCODE;
            }
            SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);

            // Belirtilen süre kadar basılı tut
            thread::sleep(Duration::from_millis(duration_ms));

            // Tuşu bırak
            let mut input_up: INPUT = zeroed();
            input_up.type_ = INPUT_KEYBOARD;
            {
                let ki_up = input_up.u.ki_mut();
                ki_up.wVk = vk_code;
                ki_up.wScan = scan_code;
                ki_up.dwFlags = KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP;
            }
            SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[allow(dead_code)]
    pub fn key_press_char(&self, c: char) {
        self.key_press(c.to_ascii_uppercase() as u16);
    }

    // ─────────────────── WinAPI ───────────────────
    fn winapi_move_absolute(&self, x: i32, y: i32) {
        unsafe {
            let sw = self.screen_width.max(1) as f64;
            let sh = self.screen_height.max(1) as f64;
            let norm_x = ((x as f64) * 65535.0 / sw) as i32;
            let norm_y = ((y as f64) * 65535.0 / sh) as i32;

            let mut input: INPUT = zeroed();
            input.type_ = INPUT_MOUSE;
            {
                let mi = input.u.mi_mut();
                mi.dx = norm_x;
                mi.dy = norm_y;
                mi.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
            }
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn winapi_click(&self, button: &str) {
        unsafe {
            let (down_flag, up_flag) = match button {
                "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
                _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            };

            let mut input_down: INPUT = zeroed();
            input_down.type_ = INPUT_MOUSE;
            input_down.u.mi_mut().dwFlags = down_flag;
            SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32);

            let mut rng = rand::thread_rng();
            thread::sleep(Duration::from_millis(rng.gen_range(35..90)));

            let mut input_up: INPUT = zeroed();
            input_up.type_ = INPUT_MOUSE;
            input_up.u.mi_mut().dwFlags = up_flag;
            SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
        }
    }

    // ─────────────────── Arduino ───────────────────
    /// Arduino reconnect - bağlantı koptuğunda tekrar dener
    fn try_reconnect_arduino(&self) -> bool {
        // Reconnect denemeleri arası minimum 3 saniye bekle
        if let Ok(mut last) = self.last_reconnect_attempt.lock() {
            if last.elapsed() < Duration::from_secs(3) {
                return false;
            }
            *last = std::time::Instant::now();
        }

        if self.port_name_cache.is_empty() {
            return false;
        }

        let resolved = Self::resolve_port_name(&self.port_name_cache);
        if let Some(real_port) = resolved {
            match serialport::new(&real_port, ARDUINO_BAUD)
                .timeout(Duration::from_millis(100))
                .open()
            {
                Ok(p) => {
                    if let Ok(mut guard) = self.serial_port.lock() {
                        *guard = Some(p);
                        println!("[HW] Arduino yeniden bağlandı: {}", real_port);
                        return true;
                    }
                }
                Err(e) => {
                    eprintln!("[HW] Arduino reconnect başarısız: {}", e);
                }
            }
        }
        false
    }

    /// `true` döner → komut başarıyla gönderildi. `false` → fallback gerekli.
    fn arduino_move_absolute(&self, x: i32, y: i32) -> bool {
        if let Ok(mut guard) = self.serial_port.lock() {
            if let Some(port) = guard.as_mut() {
                let cmd = format!("M{},{}\n", x, y);
                match port.write_all(cmd.as_bytes()) {
                    Ok(_) => {
                        if let Err(e) = port.flush() {
                            eprintln!("[HW] Arduino flush hatası: {}", e);
                            // Bağlantı kopmuş olabilir, reconnect dene
                            drop(guard);
                            self.try_reconnect_arduino();
                            return false;
                        }
                        thread::sleep(Duration::from_millis(2));
                        return true;
                    }
                    Err(e) => {
                        eprintln!("[HW] Arduino yazma hatası: {}", e);
                        // Bağlantı kopmuş, reconnect dene
                        drop(guard);
                        self.try_reconnect_arduino();
                        return false;
                    }
                }
            }
        }
        // Port yoksa reconnect dene
        self.try_reconnect_arduino()
    }

    fn arduino_click(&self, button: &str) -> bool {
        if let Ok(mut guard) = self.serial_port.lock() {
            if let Some(port) = guard.as_mut() {
                let cmd: &[u8] = match button {
                    "right" => b"C2\n",
                    _ => b"C1\n",
                };
                match port.write_all(cmd) {
                    Ok(_) => {
                        thread::sleep(Duration::from_millis(50));
                        return true;
                    }
                    Err(_) => {
                        // Bağlantı kopmuş, reconnect dene
                        drop(guard);
                        self.try_reconnect_arduino();
                        return false;
                    }
                }
            }
        }
        // Port yoksa reconnect dene
        self.try_reconnect_arduino()
    }

    // ─────────────────── Helpers ───────────────────
    fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    // ═══════════════════ HİBRİT TIKLAMA SİSTEMİ ═══════════════════

    /// Ana tıklama fonksiyonu — ClickMode'a göre uygun yöntemi seçer
    /// Varsayılan: Hibrit mod (PostMessageW + gerekince FocusSwap)
    pub fn background_click(&self, hwnd: HWND, x: i32, y: i32) {
        self.background_click_mode(hwnd, x, y, &ClickMode::Hibrit);
    }

    /// Click mode seçimli tıklama
    /// Hibrit: FocusSwap tıklama + PostMessageW tuşlar (Metin2 PostMessageW fare tıklamasını işlemez)
    /// PostMessageW: Tam arka plan (sadece destekleyen sunucularda)
    /// FocusSwap: Fiziksel input (tüm sunucularda çalışır)
    pub fn background_click_mode(&self, hwnd: HWND, x: i32, y: i32, mode: &ClickMode) {
        if hwnd.is_null() { return; }
        match mode {
            ClickMode::PostMessageW => self.post_message_click(hwnd, x, y),
            ClickMode::FocusSwap | ClickMode::Hibrit => self.focus_swap_click(hwnd, x, y),
        }
    }

    /// PostMessageW ile tam arka plan tıklama — pencere değiştirme YOK
    /// Çoklu client için ideal: paralel çalışır, MOUSE_LOCK gereksiz
    /// 10+ client destekler, kullanıcı bilgisayarını normal kullanabilir
    fn post_message_click(&self, hwnd: HWND, x: i32, y: i32) {
        let lparam = make_lparam(x, y);
        let mut rng = rand::thread_rng();
        unsafe {
            // Önce WM_MOUSEMOVE gönder — oyunun fare pozisyonunu algılaması için
            PostMessageW(hwnd, WM_MOUSEMOVE, 0, lparam);
            thread::sleep(Duration::from_millis(rng.gen_range(8..20)));

            // Sol tuş bas
            PostMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON as WPARAM, lparam);
            thread::sleep(Duration::from_millis(rng.gen_range(30..80)));

            // Sol tuş bırak
            PostMessageW(hwnd, WM_LBUTTONUP, 0, lparam);
        }
    }

    /// Optimize Focus-Swap tıklama — hızlı pencere değiştirme (~20ms)
    /// Captcha, NPC etkileşim gibi PostMessageW'nin çalışmadığı durumlar için
    /// Eski ~100ms'den ~20ms'ye optimize edildi:
    ///   - ShowWindow+BringWindowToTop+SetFocus kaldırıldı (sadece SetForegroundWindow)
    ///   - 30ms+15ms bekleme → 5ms
    ///   - 20ms bekleme (eski 30ms'den kısa ama güvenli)
    ///   - Move → 10ms → Click (ayrı SendInput çağrıları)
    pub fn focus_swap_click(&self, hwnd: HWND, x: i32, y: i32) {
        if hwnd.is_null() { 
            eprintln!("[❌ FocusSwap] HWND null!");
            return; 
        }
        
        acquire_mouse_lock();
        
        let old_fg = unsafe { winapi::um::winuser::GetForegroundWindow() };
        let (old_x, old_y) = self.get_cursor_pos();
        
        // Client → ekran koordinatı
        let (sx, sy) = self.client_to_screen(hwnd, x, y);
        eprintln!("[🖱️ FocusSwap] Client: ({}, {}) → Screen: ({}, {})", x, y, sx, sy);
        
        // Pencereyi ön plana getir + input odağı ver
        unsafe {
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
        }
        thread::sleep(Duration::from_millis(50)); // Daha uzun bekle - pencere odaklanması için
        
        // 1. Fareyi hedefe taşı
        self.winapi_move_absolute(sx, sy);
        eprintln!("[🖱️ FocusSwap] Fare taşındı: ({}, {})", sx, sy);
        
        // 2. Oyunun fare pozisyonunu algılaması için bekle
        thread::sleep(Duration::from_millis(30)); // Daha uzun bekle
        
        // 3. Tıkla
        eprintln!("[🖱️ FocusSwap] Tıklama yapılıyor...");
        self.winapi_click("left");
        eprintln!("[🖱️ FocusSwap] Tıklama tamamlandı");
        
        // 4. Fareyi eski yerine döndür
        self.winapi_move_absolute(old_x, old_y);
        
        // 5. Eski pencereyi geri getir
        if old_fg != hwnd && !old_fg.is_null() {
            unsafe { SetForegroundWindow(old_fg); }
        }
        
        release_mouse_lock();
    }

    /// PostMessageW ile arka plan sağ tıklama
    pub fn background_right_click(&self, hwnd: HWND, x: i32, y: i32) {
        if hwnd.is_null() { return; }
        let lparam = make_lparam(x, y);
        unsafe {
            PostMessageW(hwnd, WM_RBUTTONDOWN, 1 as WPARAM, lparam);
            thread::sleep(Duration::from_millis(50));
            PostMessageW(hwnd, WM_RBUTTONUP, 0, lparam);
        }
    }

    /// PostMessageW ile arka plan tuş basma (tek basış)
    pub fn background_key_press(&self, hwnd: HWND, vk: u16) {
        if hwnd.is_null() { return; }
        let mut rng = rand::thread_rng();
        let wparam = vk as WPARAM;
        let lparam: LPARAM = 0x00000001; // repeat count = 1
        unsafe {
            PostMessageW(hwnd, WM_KEYDOWN, wparam, lparam);
            thread::sleep(Duration::from_millis(rng.gen_range(40..120)));
            PostMessageW(hwnd, WM_KEYUP, wparam, lparam | (1 << 30) | (1 << 31));
        }
    }

    /// PostMessageW ile arka plan tuş basılı tutma
    pub fn background_key_hold(&self, hwnd: HWND, vk: u16, duration_ms: u64) {
        if hwnd.is_null() { return; }
        let wparam = vk as WPARAM;
        let lparam: LPARAM = 0x00000001;
        unsafe {
            PostMessageW(hwnd, WM_KEYDOWN, wparam, lparam);
            thread::sleep(Duration::from_millis(duration_ms));
            PostMessageW(hwnd, WM_KEYUP, wparam, lparam | (1 << 30) | (1 << 31));
        }
    }

    /// PostMessageW ile arka plan tuş kombinasyonu (örn: Ctrl+G)
    /// modifier_vk: Modifikatör tuşu (örn: VK_CONTROL = 0x11)
    /// key_vk: Ana tuş (örn: G = 0x47)
    pub fn background_key_combo(&self, hwnd: HWND, modifier_vk: u16, key_vk: u16) {
        if hwnd.is_null() { return; }
        let mut rng = rand::thread_rng();
        let modifier_wparam = modifier_vk as WPARAM;
        let key_wparam = key_vk as WPARAM;
        let lparam_down: LPARAM = 0x00000001;
        let lparam_up: LPARAM = 0xC0000001; // repeat count + previous state + transition state
        
        unsafe {
            // 1. Modifikatör tuşu bas (Ctrl)
            PostMessageW(hwnd, WM_KEYDOWN, modifier_wparam, lparam_down);
            thread::sleep(Duration::from_millis(rng.gen_range(30..60)));
            
            // 2. Ana tuşu bas (G)
            PostMessageW(hwnd, WM_KEYDOWN, key_wparam, lparam_down);
            thread::sleep(Duration::from_millis(rng.gen_range(40..80)));
            
            // 3. Ana tuşu bırak
            PostMessageW(hwnd, WM_KEYUP, key_wparam, lparam_up);
            thread::sleep(Duration::from_millis(rng.gen_range(30..60)));
            
            // 4. Modifikatör tuşu bırak
            PostMessageW(hwnd, WM_KEYUP, modifier_wparam, lparam_up);
        }
    }

    /// PostMessageW ile arka plan tuş kombinasyonu (2 modifikatör + ana tuş)
    /// Örn: Ctrl+Shift+F1
    /// modifier1_vk: İlk modifikatör (örn: VK_CONTROL = 0x11)
    /// modifier2_vk: İkinci modifikatör (örn: VK_SHIFT = 0x10)
    /// key_vk: Ana tuş (örn: VK_F1 = 0x70)
    pub fn background_key_combo_with_two_modifiers(&self, hwnd: HWND, modifier1_vk: u16, modifier2_vk: u16, key_vk: u16) {
        if hwnd.is_null() { return; }
        let mut rng = rand::thread_rng();
        let mod1_wparam = modifier1_vk as WPARAM;
        let mod2_wparam = modifier2_vk as WPARAM;
        let key_wparam = key_vk as WPARAM;
        let lparam_down: LPARAM = 0x00000001;
        let lparam_up: LPARAM = 0xC0000001;
        
        unsafe {
            // 1. İlk modifikatörü bas (Ctrl)
            PostMessageW(hwnd, WM_KEYDOWN, mod1_wparam, lparam_down);
            thread::sleep(Duration::from_millis(rng.gen_range(20..40)));
            
            // 2. İkinci modifikatörü bas (Shift)
            PostMessageW(hwnd, WM_KEYDOWN, mod2_wparam, lparam_down);
            thread::sleep(Duration::from_millis(rng.gen_range(20..40)));
            
            // 3. Ana tuşu bas (F1)
            PostMessageW(hwnd, WM_KEYDOWN, key_wparam, lparam_down);
            thread::sleep(Duration::from_millis(rng.gen_range(40..80)));
            
            // 4. Ana tuşu bırak
            PostMessageW(hwnd, WM_KEYUP, key_wparam, lparam_up);
            thread::sleep(Duration::from_millis(rng.gen_range(20..40)));
            
            // 5. İkinci modifikatörü bırak (Shift)
            PostMessageW(hwnd, WM_KEYUP, mod2_wparam, lparam_up);
            thread::sleep(Duration::from_millis(rng.gen_range(20..40)));
            
            // 6. İlk modifikatörü bırak (Ctrl)
            PostMessageW(hwnd, WM_KEYUP, mod1_wparam, lparam_up);
        }
    }

    // ═══════════════════ PENCERE YÖNETİMİ ═══════════════════

    /// Pencereyi ön plana getir ve odakla
    pub fn bring_to_front(&self, hwnd: HWND) {
        if hwnd.is_null() { return; }
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
        }
    }

    /// Client (pencere içi) koordinatları ekran koordinatlarına çevir
    /// Fare hareketi için gerekli - human_move ekran koordinatları kullanır
    pub fn client_to_screen(&self, hwnd: HWND, client_x: i32, client_y: i32) -> (i32, i32) {
        if hwnd.is_null() { 
            return (client_x, client_y); 
        }
        unsafe {
            let mut pt = POINT { x: client_x, y: client_y };
            winapi::um::winuser::ClientToScreen(hwnd, &mut pt);
            (pt.x, pt.y)
        }
    }

    /// Ekran koordinatlarını client (pencere içi) koordinatlarına çevir
    pub fn screen_to_client(&self, hwnd: HWND, screen_x: i32, screen_y: i32) -> (i32, i32) {
        if hwnd.is_null() { 
            return (screen_x, screen_y); 
        }
        unsafe {
            let mut pt = POINT { x: screen_x, y: screen_y };
            winapi::um::winuser::ScreenToClient(hwnd, &mut pt);
            (pt.x, pt.y)
        }
    }
}

unsafe impl Send for HwSimulator {}
unsafe impl Sync for HwSimulator {}
