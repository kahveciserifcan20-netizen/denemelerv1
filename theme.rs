use eframe::egui;

/// Tema renk paletleri
pub const ACCENT_COLORS: &[(u8, u8, u8)] = &[
    (212, 160, 23),   // 0: Altın (varsayılan)
    (60, 130, 250),   // 1: Mavi
    (50, 205, 100),   // 2: Yeşil
    (220, 140, 40),   // 3: Turuncu
    (220, 80, 160),   // 4: Pembe
    (220, 60, 60),    // 5: Kırmızı
];

#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub bg0: egui::Color32,      // En koyu arka plan
    pub bg1: egui::Color32,      // Panel arka planı
    pub bg2: egui::Color32,      // Kart arka planı
    pub bg3: egui::Color32,      // Hover / aktif kart
    pub border: egui::Color32,   // Kenarlık
    pub border_light: egui::Color32,
    pub text: egui::Color32,     // Ana metin
    pub text_dim: egui::Color32, // Soluk metin
    pub text_muted: egui::Color32,
    pub accent: egui::Color32,   // Vurgu rengi
    pub accent_dim: egui::Color32,
    pub accent_bg: egui::Color32, // Vurgu arka planı (düşük opaklık)
    pub green: egui::Color32,
    pub red: egui::Color32,
    pub yellow: egui::Color32,
    pub purple: egui::Color32,
    pub cyan: egui::Color32,
    pub blue: egui::Color32,
}

impl Theme {
    pub fn new(accent_idx: usize) -> Self {
        let idx = accent_idx.min(ACCENT_COLORS.len() - 1);
        let (ar, ag, ab) = ACCENT_COLORS[idx];
        Self {
            bg0: egui::Color32::from_rgb(12, 12, 16),
            bg1: egui::Color32::from_rgb(18, 18, 24),
            bg2: egui::Color32::from_rgb(26, 26, 34),
            bg3: egui::Color32::from_rgb(35, 35, 45),
            border: egui::Color32::from_rgb(42, 42, 55),
            border_light: egui::Color32::from_rgb(55, 55, 70),
            text: egui::Color32::from_rgb(225, 225, 230),
            text_dim: egui::Color32::from_rgb(140, 140, 155),
            text_muted: egui::Color32::from_rgb(90, 90, 105),
            accent: egui::Color32::from_rgb(ar, ag, ab),
            accent_dim: egui::Color32::from_rgb(ar / 2, ag / 2, ab / 2),
            accent_bg: egui::Color32::from_rgba_premultiplied(ar / 4, ag / 4, ab / 4, 40),
            green: egui::Color32::from_rgb(50, 205, 100),
            red: egui::Color32::from_rgb(245, 70, 70),
            yellow: egui::Color32::from_rgb(220, 200, 40),
            purple: egui::Color32::from_rgb(160, 110, 245),
            cyan: egui::Color32::from_rgb(60, 200, 230),
            blue: egui::Color32::from_rgb(60, 130, 250),
        }
    }
}
