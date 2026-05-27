pub fn t(key: &str, dil_idx: usize) -> String {
    let is_en = dil_idx == 1;
    if !is_en {
        return key.to_string();
    }
    
    // Turkish -> English mapping
    let en = match key {
        // Genel (Sidebar / Menü)
        "Genel Bakış" => "Overview",
        "Farming" => "Farming",
        "Yetenekler" => "Skills",
        "Eşyalar" => "Items",
        "Captcha" => "Captcha",
        "OCR" => "OCR",
        "Çoklu Client" => "Multi-Client",
        "Ayarlar" => "Settings",
        "Kayıtlar" => "Logs",
        "↪ Çıkış Yap" => "↪ Logout",
        "‹ Küçült" => "‹ Minimize",
        "Dashboard" => "Dashboard",
        "Bağlı" => "Connected",

        // Login Ekranı
        "⊙  GİRİŞ YAP" => "⊙  LOGIN",
        "Lütfen tüm alanları doldurun." => "Please fill all fields.",
        "Geçersiz kullanıcı adı veya şifre." => "Invalid username or password.",
        "Beni Hatırla" => "Remember Me",
        "Hesabınız yok mu?" => "Don't have an account?",
        "Lisans satın al" => "Buy license",

        // Mini Ekran
        "K-BOT MINI" => "K-BOT MINI",
        "Durum: Aktif" => "Status: Active",
        "Durum: Durduruldu" => "Status: Stopped",
        "Büyüt" => "Maximize",

        // Butonlar / Ortak
        "▶  BAŞLAT" => "▶  START",
        "⏹  DURDUR" => "⏹  STOP",
        "💾 Ayarları Kaydet" => "💾 Save Settings",
        "🎯 Test Tıklama" => "🎯 Test Click",
        "🗑 Temizle" => "🗑 Clear",
        "Aktif" => "Active",
        "Pasif" => "Inactive",

        // Genel Bakış Sayfası
        "BOT DURUMU" => "BOT STATUS",
        "Sistem boşta" => "System idle",
        "Çalışıyor" => "Running",
        "İSTATİSTİKLER" => "STATISTICS",
        "Kırılan Taş" => "Mined Stones",
        "Iskalanan" => "Missed",
        "Çözülen Captcha" => "Solved Captchas",
        "Çalışma Süresi" => "Uptime",
        "SON SAATTEKİ PERFORMANS" => "PERFORMANCE (LAST HOUR)",
        "Kırılan Taş Grafiği" => "Mined Stones Graph",
        "MEVCUT HARİTA" => "CURRENT MAP",
        "Gerçek zamanlı bot durumu ve istatistikler" => "Real-time bot status and statistics",
        "son saatte" => "last hour",
        "Canlı" => "Live",
        "Veri toplanıyor..." => "Collecting data...",
        "Son dakika" => "Last minute",
        "Max:" => "Max:",

        // Farming Sayfası
        "Farming Ayarları" => "Farming Settings",
        "Savaş, hedef ve bölge konfigürasyonu" => "Combat, target, and zone configuration",
        "HEDEF METİN TAŞLARI" => "TARGET METIN STONES",
        "EŞYA TOPLAMA" => "ITEM PICKUP",
        "Topla" => "Pickup",
        "Tuş" => "Key",

        // Yetenekler Sayfası
        "YETENEKLER & BUFFLAR" => "SKILLS & BUFFS",
        "Tuş ve Bekleme Süresi (sn)" => "Key and Cooldown (sec)",
        "İksir Otomasyonu" => "Potion Automation",
        
        // Eşyalar Sayfası
        "Eşya Filtresi" => "Item Filter",
        "Düşen eşya toplama kuralları" => "Looting rules",
        "Bu bölüm ileride eklenecektir." => "This section will be added later.",
        "KULLANILACAK EŞYALAR" => "ITEMS TO USE",
        
        // Captcha Sayfası
        "Captcha İstatistikleri" => "Captcha Statistics",
        "Captcha çözüm başarı oranları" => "Captcha solve success rates",
        "BAŞARILI" => "SUCCESS",
        "BAŞARISIZ" => "FAILED",
        "TOPLAM" => "TOTAL",
        "Toplam captcha" => "Total captchas",
        "BAŞARI ORANI" => "SUCCESS RATE",
        "Başarı:" => "Success:",
        "Başarısız:" => "Failed:",
        "Captcha Loglarını Sıfırla" => "Reset Captcha Logs",
        "CAPTCHA ÇÖZÜCÜ" => "CAPTCHA SOLVER",
        "Sürücü (Motor)" => "Driver (Engine)",
        "Durdur (Alarm Çal)" => "Stop (Play Alarm)",
        "İstatistikler" => "Statistics",
        "Oran:" => "Rate:",

        // OCR Sayfası
        "OCR & Kilit Ayarları" => "OCR & Lock Settings",
        "Captcha OCR alanı ve hedef kilit konfigürasyonu" => "Captcha OCR region and target lock configuration",
        "CAPTCHA OCR ALANI (SABİT)" => "CAPTCHA OCR REGION (FIXED)",
        "(Sabit)" => "(Fixed)",
        "OCR alanı bu koordinatlarda çalışır." => "OCR region works in these coordinates.",
        "HEDEF KİLİT AYARLARI" => "TARGET LOCK SETTINGS",
        "📂 Şablon Seç" => "📂 Select Template",
        "⏳ 3:Sol üst  4:Sağ alt" => "⏳ 3:Top-Left  4:Bot-Right",
        "🔒 Bölge Seç (3/4)" => "🔒 Select Region (3/4)",

        // Çoklu Client Sayfası
        "AÇIK METİN2 PENCERELERİ" => "OPEN METIN2 WINDOWS",
        "Mevcut açık ve bot tarafından yönetilen clientler." => "Currently open clients managed by the bot.",
        "Çoklu Client Yönetimi" => "Multi-Client Management",
        "Birden fazla oyun penceresini yönetin" => "Manage multiple game windows",
        "CLIENT LİSTESİ" => "CLIENT LIST",
        "Henüz client eklenmedi." => "No clients added yet.",
        "⏳ Shift ile seç..." => "⏳ Select with Shift...",
        "🎯 PID Seç" => "🎯 Select PID",
        "📺 Kapat" => "📺 Close",
        "📺 İzle" => "📺 Watch",
        "Fare:" => "Mouse:",
        "Yeni Client Ekle" => "Add New Client",
        "CANLI EKRAN —" => "LIVE SCREEN —",
        "Kamera verisi bekleniyor..." => "Waiting for camera data...",
        "Kaldır" => "Remove",

        // Ayarlar Sayfası
        "GÜVENLİK & ANTİ-TESPİT" => "SECURITY & ANTI-DETECT",
        "ⓘ Bu ayarlar bot tespitini zorlaştırmak için geliştirilmiştir." => "ⓘ These settings are developed to prevent bot detection.",
        "Anti-Tespit Modu" => "Anti-Detect Mode",
        "İnsan davranışını taklit eder" => "Simulates human behavior",
        "Rastgele Gecikme" => "Random Delay",
        "Tıklamalar arasına rastgele ms ekler" => "Adds random ms between clicks",
        "OBS Bypass" => "OBS Bypass",
        "Yayın programlarında botu gizler" => "Hides bot in streaming software",
        "Donanım (Mouse) Sürücüsü" => "Hardware (Mouse) Driver",
        "MOLA SİSTEMİ" => "BREAK SYSTEM",
        "Her" => "Every",
        "dakikada bir" => "minutes",
        "dakika mola ver." => "minutes break.",
        "BİLDİRİMLER" => "NOTIFICATIONS",
        "Uygulama İçi Uyarılar" => "In-App Alerts",
        "Sesli Bildirim" => "Sound Notification",
        "Telegram'a Bildir" => "Notify Telegram",
        "Webhook URL:" => "Webhook URL:",
        "Test Mesajı Gönder" => "Send Test Message",
        "KISAYOL TUŞLARI" => "HOTKEYS",
        "Bot Başlat/Durdur" => "Start/Stop Bot",
        "Log Temizle" => "Clear Logs",
        "Ekran Görüntüsü" => "Screenshot",
        "GÖRÜNÜM AYARI" => "APPEARANCE SETTINGS",
        "Tema Rengi" => "Theme Color",
        "GUI PENCERE BOYUTU" => "GUI WINDOW SIZE",
        "Hazır Şablonlar:" => "Presets:",
        "Özel Boyut:" => "Custom Size:",
        "Genişlik:" => "Width:",
        "Yükseklik:" => "Height:",
        "Uygula" => "Apply",

        // Kayıtlar Sayfası
        "SİSTEM KAYITLARI" => "SYSTEM LOGS",
        "Kayıtlar (Logs)" => "Logs",
        "Gerçek zamanlı bot etkinlik akışı" => "Real-time bot event stream",
        "🔍 Log ara..." => "🔍 Search logs...",
        "Tümü" => "All",
        "Temizle" => "Clear",
        "kayıt" => "records",
        "Log kaydı bulunmadı." => "No logs found.",

        _ => key, // Eğer çeviri yoksa orjinalini döndür
    };
    
    en.to_string()
}
