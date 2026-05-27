// vk_codes.rs — Paylaşılan Virtual Key Code haritası
// client_runner, hw_simulator ve gui modüllerinde tekrarlanan VK code mapping'i tek yerde toplar.

/// Tuş adından VK koduna dönüşüm.
/// Desteklenen formatlar: "A"-"Z", "0"-"9", "F1"-"F12", "SPACE", "ENTER", "CTRL", "SHIFT", "ALT", 
/// "TAB", "ESCAPE", "BACKSPACE", "DELETE", "INSERT", "HOME", "END", "PAGEUP", "PAGEDOWN",
/// "UP", "DOWN", "LEFT", "RIGHT", "NUM0"-"NUM9"
pub fn vk_from_name(name: &str) -> Option<u16> {
    let upper = name.trim().to_uppercase();
    match upper.as_str() {
        // Modifikatörler
        "CTRL" | "CONTROL" => Some(0x11),   // VK_CONTROL
        "SHIFT" => Some(0x10),              // VK_SHIFT
        "ALT" | "MENU" => Some(0x12),       // VK_MENU

        // Fonksiyon tuşları
        "F1"  => Some(0x70), "F2"  => Some(0x71), "F3"  => Some(0x72), "F4"  => Some(0x73),
        "F5"  => Some(0x74), "F6"  => Some(0x75), "F7"  => Some(0x76), "F8"  => Some(0x77),
        "F9"  => Some(0x78), "F10" => Some(0x79), "F11" => Some(0x7A), "F12" => Some(0x7B),

        // Sayı tuşları (üst sıra)
        "0" => Some(0x30), "1" => Some(0x31), "2" => Some(0x32), "3" => Some(0x33),
        "4" => Some(0x34), "5" => Some(0x35), "6" => Some(0x36), "7" => Some(0x37),
        "8" => Some(0x38), "9" => Some(0x39),

        // Harf tuşları
        "A" => Some(0x41), "B" => Some(0x42), "C" => Some(0x43), "D" => Some(0x44),
        "E" => Some(0x45), "F" => Some(0x46), "G" => Some(0x47), "H" => Some(0x48),
        "I" => Some(0x49), "J" => Some(0x4A), "K" => Some(0x4B), "L" => Some(0x4C),
        "M" => Some(0x4D), "N" => Some(0x4E), "O" => Some(0x4F), "P" => Some(0x50),
        "Q" => Some(0x51), "R" => Some(0x52), "S" => Some(0x53), "T" => Some(0x54),
        "U" => Some(0x55), "V" => Some(0x56), "W" => Some(0x57), "X" => Some(0x58),
        "Y" => Some(0x59), "Z" => Some(0x5A),

        // Özel tuşlar
        "SPACE" => Some(0x20),
        "ENTER" | "RETURN" => Some(0x0D),
        "TAB" => Some(0x09),
        "ESCAPE" | "ESC" => Some(0x1B),
        "BACKSPACE" | "BACK" => Some(0x08),
        "DELETE" | "DEL" => Some(0x2E),
        "INSERT" | "INS" => Some(0x2D),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" | "PGUP" => Some(0x21),
        "PAGEDOWN" | "PGDN" => Some(0x22),

        // Yön tuşları
        "UP" => Some(0x26),
        "DOWN" => Some(0x28),
        "LEFT" => Some(0x25),
        "RIGHT" => Some(0x27),

        // Numpad
        "NUM0" | "NUMPAD0" => Some(0x60), "NUM1" | "NUMPAD1" => Some(0x61),
        "NUM2" | "NUMPAD2" => Some(0x62), "NUM3" | "NUMPAD3" => Some(0x63),
        "NUM4" | "NUMPAD4" => Some(0x64), "NUM5" | "NUMPAD5" => Some(0x65),
        "NUM6" | "NUMPAD6" => Some(0x66), "NUM7" | "NUMPAD7" => Some(0x67),
        "NUM8" | "NUMPAD8" => Some(0x68), "NUM9" | "NUMPAD9" => Some(0x69),

        _ => None,
    }
}

/// Tuş kombinasyonunu parse eder (örn: "Ctrl+G", "Shift+F1", "Alt+1", "F2")
/// Dönen tuple: (modifikatörler, ana tuş VK kodu)
pub fn parse_key_combo(combo: &str) -> (Vec<u16>, Option<u16>) {
    let parts: Vec<&str> = combo.split('+').collect();
    let mut modifiers: Vec<u16> = Vec::new();
    let mut main_key: Option<u16> = None;

    for part in parts {
        let p = part.trim().to_uppercase();
        match p.as_str() {
            "CTRL" | "CONTROL" => modifiers.push(0x11),
            "SHIFT" => modifiers.push(0x10),
            "ALT" | "MENU" => modifiers.push(0x12),
            _ => {
                if let Some(vk) = vk_from_name(&p) {
                    main_key = Some(vk);
                }
            }
        }
    }

    (modifiers, main_key)
}

/// Binek tuş adından VK koduna dönüşüm (kısa alias'lar için fallback)
#[allow(dead_code)]
pub fn binek_vk_from_name(name: &str) -> u16 {
    vk_from_name(name).unwrap_or(0x47) // Varsayılan: G
}
