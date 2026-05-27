# K-BOT v109 - Metin2 Otomasyon Botu

## 🎮 Özellikler

- 🪨 **Otomatik Taş Kesme** - YOLO AI ile taş tespiti ve otomatik kilit alma
- 🔐 **Captcha Çözümü** - OCRS + Tesseract ile otomatik captcha çözme
- 🖱️ **İnsansı Hareket** - Bezier eğrileri ile doğal fare hareketi
- 📦 **Eşya Toplama** - Taş kırıldıktan sonra otomatik eşya toplama
- 📱 **Telegram Bildirimleri** - Taş, captcha ve mola bildirimleri
- 🛡️ **Anti-Tespit** - Mikro hareket ve rastgele gecikme
- ☕ **Mola Sistemi** - Belirlenen aralıklarla otomatik mola
- 🎥 **OBS Bypass** - OBS yayını varsa arka planda çalışma
- 🖥️ **Çoklu Client** - Birden fazla pencere yönetimi
- 📡 **MQTT Remote** - APK ile uzaktan kontrol

---

## 📥 Kurulum

### Yöntem 1: Otomatik Kurulum (Önerilen)

1. `KURULUM.bat` dosyasına **sağ tıklayın**
2. **"Yönetici olarak çalıştır"** seçin
3. Kurulumun bitmesini bekleyin (5-10 dakika)
4. `the_absolute_monolith.exe` dosyasını çalıştırın

### Yöntem 2: Manuel Kurulum

Gerekli programlar:
- [Rust](https://rustup.rs) (Cargo ile birlikte)
- [Python 3.12](https://python.org)
- [Tesseract OCR](https://github.com/UB-Mannheim/tesseract/wiki)
- [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)

```bash
# Python kütüphaneleri
pip install opencv-python numpy pillow pyautogui mss

# Projeyi derle
cargo build --release
```

---

## 🚀 Kullanım

1. `the_absolute_monolith.exe` dosyasını çalıştır
2. Giriş yap (varsayılan: admin / admin)
3. **Genel** sayfasından hedef pencereyi seç (PID)
4. Ayarları yapılandır:
   - Model seç (yolo_modelleri klasöründen)
   - Fare sürücüsü seç (WinAPI veya Arduino)
   - Eşya toplama, mola, anti-tespit ayarları
5. **Başlat** butonuna tıkla

---

## ⚙️ Ayarlar

| Ayar | Açıklama |
|------|----------|
| `toplama_aktif` | Eşya toplama açık/kapalı |
| `toplama_tusu` | Toplama tuşu (Z, X, C, V, F, G, SPACE) |
| `anti_tespit_modu` | Mikro hareket aktif |
| `rastgele_gecikme` | Rastgele bekleme süresi |
| `obs_bypass` | OBS varsa arka planda çalış |
| `mola_sistemi_aktif` | Otomatik mola sistemi |
| `mola_aralik_dk` | Çalışma süresi (dakika) |
| `mola_sure_dk` | Mola süresi (dakika) |
| `telegram_bot` | Telegram bildirimleri |
| `telegram_webhook_url` | Telegram bot API URL |

---

## 📁 Proje Yapısı

```
the_absolute_monolith/
├── the_absolute_monolith.exe  # Ana program
├── KURULUM.bat                # Otomatik kurulum
├── config.json                # Ayarlar dosyası
├── hedef_kilit.png            # Taş kilit template'i
│
├── yolo_modelleri/            # YOLO AI modelleri
│   ├── vadi.onnx
│   ├── Doyum.onnx
│   └── ...
│
├── captcha_sablonlari/        # Captcha şablonları
│   ├── btn_onay.png
│   ├── hedef/
│   └── soru/
│
├── kbot_remote/               # APK remote kontrol
│
└── src/                       # Kaynak kodlar
    ├── main.rs
    ├── config.rs
    ├── captcha_solver.rs
    ├── vision_manager.rs
    ├── hw_simulator.rs
    ├── remote_server.rs
    └── gui/
```

---

## 🔧 Arduino Modu

Arduino kullanmak için:
1. Arduino'yu USB'ye tak
2. CH340 driver'ı kur (Arduino clone için)
3. Ayarlar'dan "Arduino" modunu seç
4. COM port otomatik algılanır

Arduino kodu için: `hw_simulator.rs` dosyasına bakın.

---

## 📱 MQTT Remote Control

APK ile uzaktan kontrol için:
1. `kbot_remote/` klasörünü Cordova ile derle
2. APK'yı telefona kur
3. MQTT broker: `broker.hivemq.com:1883`
4. Topic: `kbot/cmd`

---

## ⚠️ Uyarılar

- Bu bot eğitim amaçlıdır
- Oyun kurallarına uygun kullanın
- Hesap ban riski her zaman vardır
- Anti-tespit modu kullanmanızı öneririz

---

## 📞 Destek

Sorun yaşarsanız:
1. `cargo build --release` ile tekrar derleyin
2. `config.json` dosyasını kontrol edin
3. Tesseract'ın kurulu olduğundan emin olun

---

**K-BOT v109** - Metin2 Otomasyon Botu