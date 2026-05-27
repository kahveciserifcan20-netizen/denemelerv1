# 📖 K-BOT v109 - KULLANIM REHBERİ

Bu rehber, K-BOT'un tüm özelliklerini, ayarlarını ve kullanımını detaylı olarak açıklamaktadır.

---

## 📋 İÇİNDEKİLER

1. [Başlangıç](#1-başlangıç)
2. [Genel Sayfası](#2-genel-sayfası)
3. [Yetenek Sayfası](#3-yetenek-sayfası)
4. [Farming Sayfası](#4-farming-sayfası)
5. [Eşya Sayfası](#5-eşya-sayfası)
6. [Client Sayfası](#6-client-sayfası)
7. [Ayarlar Sayfası](#7-ayarlar-sayfası)
8. [Kayıtlar Sayfası](#8-kayıtlar-sayfası)
9. [OCR Sayfası](#9-ocr-sayfası)
10. [Captcha Sayfası](#10-captcha-sayfası)
11. [Gelişmiş Özellikler](#11-gelişmiş-özellikler)
12. [Sorun Giderme](#12-sorun-giderme)

---

## 1. BAŞLANGIÇ

### 1.1 Giriş Yapma

- **Varsayılan Kullanıcı:** `admin`
- **Varsayılan Şifre:** `admin`
- "Beni Hatırla" seçeneği ile sonraki girişlerde otomatik giriş yapılır

### 1.2 Arayüz Yapısı

```
┌─────────────────────────────────────────────────────────┐
│  [TOPBAR] - Durum, Başlat/Durdur, Sayaçlar              │
├──────────┬──────────────────────────────────────────────┤
│          │                                              │
│ [SIDEBAR]│              ANA İÇERİK                      │
│          │                                              │
│ • Genel  │         (Seçili sayfa içeriği)              │
│ • Yetenek│                                              │
│ • Farming│                                              │
│ • Eşya   │                                              │
│ • Client │                                              │
│ • Ayarlar│                                              │
│ • Kayıtlar│                                             │
│ • OCR    │                                              │
│ • Captcha│                                              │
│          │                                              │
└──────────┴──────────────────────────────────────────────┘
```

---

## 2. GENEL SAYFASI

### 2.1 Hedef Pencere Seçimi

**Ne İşe Yarar:** Botun hangi oyun penceresinde çalışacağını belirler.

**Nasıl Kullanılır:**
1. "Pencere Tara" butonuna tıklayın
2. Açılan listeden Metin2 penceresini seçin (PID ile)
3. Seçilen pencere "Aktif Hedef" olarak gösterilir

**İpucu:** Pencere adında "Metin2" veya oyun karakter adı görünür.

### 2.2 Model Seçimi

**Ne İşe Yarar:** YOLO AI modeli, taş tespiti için kullanılır.

**Mevcut Modeller:**
| Model | Harita | Açıklama |
|-------|--------|----------|
| `vadi.onnx` | Vadi | Genel kullanım, en stabil |
| `Doyum.onnx` | Doyum | Doyum haritası için optimize |
| `buyulu.onnx` | Büyülü | Büyülü harita için |
| `Kızıl.onnx` | Kızıl | Kızıl harita için |
| `Guatama.onnx` | Guatama | Guatama haritası için |

**Nasıl Seçilir:**
- Dropdown menüden uygun modeli seçin
- Farklı haritalarda farklı modeller deneyin

### 2.3 Fare Sürücüsü

**Ne İşe Yarar:** Fare hareketlerinin nasıl yapılacağını belirler.

**Seçenekler:**

| Mod | Açıklama | Avantaj | Dezavantaj |
|-----|----------|---------|------------|
| **WinAPI** | Windows API ile fare kontrolü | Hızlı, kurulum gerektirmez | Oyun algılayabilir |
| **Arduino** | Arduino ile donanımsal fare | Algılanamaz, en güvenli | Arduino gerektirir |

**Arduino Modu İçin:**
1. Arduino'yu USB'ye takın
2. CH340 driver kurulu olmalı
3. "Arduino_AUTO" seçin (otomatik COM tespiti)
4. Veya "Arduino_COM3" gibi belirli port girin

### 2.4 Başlatma ve Durdurma

- **Başlat:** Seçili pencerede botu başlatır (5 saniye geri sayım)
- **Durdur:** Botu durdurur ve durumu "Arama"ya sıfırlar
- **Klavye Kısayolu:** F9 (varsayılan)

---

## 3. YETENEK SAYFASI

### 3.1 Kamera Hareket Ayarları

**Ne İşe Yarar:** Taş bulunamadığında kameranın nasıl döndürüleceğini belirler.

**Ayarlanabilir Tuşlar:**

| Tuş | VK Kod | Varsayılan Süre | Açıklama |
|-----|--------|-----------------|----------|
| Q | 0x51 | 2.5 saniye | Sola döndür |
| E | 0x45 | 0.15 saniye | Sağa döndür |
| W | 0x57 | 0.3 saniye | İleri bak |
| A | 0x41 | 0.3 saniye | Sola bak |
| S | 0x53 | 0.3 saniye | Geri bak |
| D | 0x44 | 0.3 saniye | Sağa bak |

**Nasıl Ayarlanır:**
1. İlgili tuşun "Aktif" kutusunu işaretleyin
2. Süre (saniye) değerini ayarlayın
3. "Kaydet" butonuna tıklayın

**Öneri:** Genellikle Q ve E yeterlidir. Geniş alan taraması için Q'yu uzun (2-3 saniye), E'yi kısa (0.1-0.2 saniye) ayarlayın.

---

## 4. FARMING SAYFASI

### 4.1 Taş Kesme Mantığı

Bot şu döngüyle çalışır:

```
┌─────────┐    Taş bul    ┌──────────────┐    Kilit gör    ┌─────────┐
│ ARAMA   │ ────────────> │ KİLİT BEKLE  │ ─────────────> │ KESİYOR │
│         │               │ (500ms-3s)   │                │         │
└─────────┘               └──────────────┘                └─────────┘
     ↑                                                         │
     │                     Kilit kayboldu (taş kırıldı)        │
     └─────────────────────────────────────────────────────────┘
```

### 4.2 Kilit Arama Bölgesi

**Ne İşe Yarar:** Taşa tıklandıktan sonra "hedef_kilit.png" şablonunun aranacağı bölge.

**Varsayılan:** (300, 20) → (500, 90) - Ekranın üst kısmı

**Nasıl Ayarlanır:**
1. OCR sayfasından koordinatları görebilirsiniz
2. Config.json'dan `kilit_region_x1/y1/x2/y2` değerlerini değiştirebilirsiniz

### 4.3 Kara Liste Sistemi

**Ne İşe Yarar:** Tıklanıp kilit alınamayan taşları 15 saniye boyunca tekrar denememek.

**Otomatik Çalışır:** Taş ıskalandığında koordinatları kara listeye eklenir.

---

## 5. EŞYA SAYFASI

### 5.1 Otomatik Eşya Toplama

**Ne İşe Yarar:** Taş kırıldıktan sonra düşen eşyaları otomatik toplar.

**Ayarlar:**

| Ayar | Açıklama | Varsayılan |
|------|----------|------------|
| `toplama_aktif` | Eşya toplama açık/kapalı | false |
| `toplama_tusu` | Toplama tuşu | Z |

**Desteklenen Tuşlar:**
- Z, X, C, V, F, G, SPACE

**Nasıl Çalışır:**
1. Taş kırılır
2. 3-5 kez rastgele aralıklarla toplama tuşuna basılır
3. OBS Bypass aktifse PostMessageW ile arka planda basılır

**Öneri:** Yang ve değerli eşyalar için Z tuşu yaygın kullanılır.

---

## 6. CLIENT SAYFASI

### 6.1 Çoklu Pencere Yönetimi

**Ne İşe Yarar:** Birden fazla Metin2 penceresini yönetmek için.

**Özellikler:**
- Tüm pencereleri listeler
- Her pencere için ayrı durum takibi
- Toplu başlatma/durdurma

### 6.2 OBS Bypass Modu

**Ne İşe Yarar:** OBS Studio açıkken botun tespit edilmemesi için.

**Nasıl Çalışır:**
- OBS64.exe çalışıyorsa tespit eder
- Pencereyi öne getirmez
- PostMessageW ile arka planda tıklama yapar

**Ayar:**
```json
"obs_bypass": true
```

**Öneri:** Yayın yaparken bu modu aktif edin.

---

## 7. AYARLAR SAYFASI

### 7.1 Anti-Tespit Sistemi

**Ne İşe Yarar:** Botun algılanmasını zorlaştırmak için insansı davranış simülasyonu.

#### 7.1.1 Mikro Hareket

**Ayar:** `anti_tespit_modu: true`

**Nasıl Çalışır:**
- Her 3 saniyede fare pozisyonuna ±3 piksel offset ekler
- Fare sürekli küçük hareketler yapar

**Etkisi:** "Robotik" fare hareketlerini gizler.

#### 7.1.2 Rastgele Gecikme

**Ayar:** `rastgele_gecikme: true`

**Parametreler:**
| Ayar | Açıklama | Varsayılan |
|------|----------|------------|
| `gecikme_min` | Minimum gecikme (ms) | 180 |
| `gecikme_max` | Maksimum gecikme (ms) | 380 |

**Nasıl Çalışır:** Her işlem arasında rastgele 180-380 ms bekleme.

### 7.2 Mola Sistemi

**Ne İşe Yarar:** Uzun süreli çalışmada şüphe uyandırmamak için düzenli molalar.

**Ayarlar:**

| Ayar | Açıklama | Varsayılan |
|------|----------|------------|
| `mola_sistemi_aktif` | Mola sistemi açık | false |
| `mola_aralik_dk` | Çalışma süresi (dk) | 57 |
| `mola_sure_dk` | Mola süresi (dk) | 6 |

**Önerilen Değerler:**
- 57 dk çalış, 6 dk mola (gerçekçi)
- 45 dk çalış, 10 dk mola (daha güvenli)

### 7.3 Telegram Bildirimleri

**Ne İşe Yarar:** Botun durumu hakkında Telegram üzerinden bildirim almak.

**Ayarlar:**

| Ayar | Açıklama |
|------|----------|
| `telegram_bot` | Telegram bildirimleri aktif |
| `telegram_webhook_url` | Bot API URL |
| `telegram_chat_id` | Chat ID (opsiyonel) |
| `telegram_tas_bildirim` | Taş kırılma bildirimi |
| `telegram_captcha_bildirim` | Captcha bildirimi |
| `telegram_mola_bildirim` | Mola bildirimi |

**Webhook URL Nasıl Alınır:**
1. BotFather'da yeni bot oluşturun
2. API token'ı alın
3. URL: `https://api.telegram.org/bot<TOKEN>/sendMessage`

**Örnek:**
```
https://api.telegram.org/bot123456789:ABCDEF/sendMessage
```

### 7.4 Tema ve Dil

**Tema Renkleri:**
- 0: Koyu Tema (varsayılan)
- 1: Açık Tema
- 2: Mavi Tema

**Desteklenen Diller:**
- Türkçe (tr)
- English (en)

---

## 8. KAYITLAR SAYFASI

### 8.1 Log Görüntüleme

**Ne İşe Yarar:** Botun tüm işlemlerini gerçek zamanlı görüntüler.

**Log Türleri:**

| Emoji | Anlam |
|-------|-------|
| 🔍 | Arama/Kilit tespiti |
| ⚔️ | Taşa tıklama |
| 🔒 | Kilit alındı |
| 💎 | Taş kırıldı |
| ❌ | Iskalama/Hata |
| 🚨 | Captcha tespiti |
| ✅ | Başarılı işlem |
| 🛡️ | Anti-tespit |
| ☕ | Mola |
| 📱 | Telegram bildirimi |

### 8.2 İstatistikler

**Topbar'da Görünenler:**
- Taş: Toplam kırılan taş sayısı
- Iskala: Kaçırılan taş sayısı
- Captcha: Çözülen captcha sayısı
- Uptime: Çalışma süresi

---

## 9. OCR SAYFASI

### 9.1 Captcha OCR Bölgesi

**SABİT KOORDİNATLAR:**
```
x1: 313, y1: 153, x2: 457, y2: 168
```

**Önemli:** Bu koordinatlar değiştirilmemelidir. Captcha sorusu bu bölgede görünür.

### 9.2 OCR Modelleri

**Kullanılan Dosyalar:**
- `text-detection.rten` - Metin tespiti
- `text-recognition.rten` - Metin okuma

---

## 10. CAPTCHA SAYFASI

### 10.1 Captcha Çözüm Sistemi

**Nasıl Çalışır:**

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Captcha     │     │ OCR ile     │     │ Hedefleri   │
│ Tespiti     │ ──> │ Soru Okuma  │ ──> │ Tıklama     │
│             │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

**Adımlar:**
1. "btn_onay.png" şablonu aranır
2. OCR bölgesinden soru okunur (örn: "3 tıkla")
3. Hedef butonları tespit edilir
4. İnsansı hareketle 3 hedefe tıklanır
5. Onay butonuna tıklanır

### 10.2 Captcha Şablonları

**Klasör:** `captcha_sablonlari/`

| Dosya | Açıklama |
|-------|----------|
| `btn_onay.png` | Onay butonu şablonu |
| `hedef/` | Hedef buton şablonları |
| `soru/` | Soru şablonları |

**Önemli:** Şablonlar oyun güncellemelerinde değişebilir. Yeni şablonlar eklemek için:
1. Ekran görüntüsü alın
2. İlgili bölgeyi kesin
3. PNG olarak kaydedin

---

## 11. GELİŞMİŞ ÖZELLİKLER

### 11.1 MQTT Remote Control

**Ne İşe Yarar:** Android APK ile botu uzaktan kontrol etmek.

**Ayarlar:**

| Ayar | Değer |
|------|-------|
| Broker | `broker.hivemq.com` |
| Port | `1883` |
| Topic | `kbot/cmd` |

**Komutlar:**

| Komut | Açıklama |
|-------|----------|
| `START_ALL` | Tüm botları başlat |
| `STOP` | Botu durdur |
| `MODEL:vadi.onnx` | Model değiştir |

### 11.2 HP Bar Kontrolü

**Otomatik Çalışır:** Taş kesilirken HP bar'daki kırmızı pikseller takip edilir.

**Engel Takılma Tespiti:**
- 4 saniye boyunca HP azalmazsa
- Rastgele yön (W/A/S/D) ile kurtulma hareketi yapar

### 11.3 İnsansı Fare Hareketi

**Teknik:** Kübik Bezier Eğrisi + Ease-in-out + Mikro Jitter

**Parametreler:**
- Kontrol noktaları rastgele sapma (10-120 piksel)
- Adım sayısı: Mesafe / 8 (min 15, max 80)
- Hız değişimi: Orta noktada hızlanma

---

## 12. SORUN GİDERME

### 12.1 Taş Bulunamıyor

**Olası Nedenler:**
1. Yanlış model seçili → Doğru harita modelini seçin
2. Confidence eşiği çok yüksek → Config'den düşürün
3. Oyun penceresi küçük → Tam ekran yapın

### 12.2 Captcha Çözülemiyor

**Olası Nedenler:**
1. Tesseract kurulu değil → KURULUM.bat çalıştırın
2. OCR bölgesi yanlış → Sabit koordinatları kontrol edin
3. Şablonlar eski → Yeni şablonlar ekleyin

### 12.3 Arduino Bağlanmıyor

**Çözümler:**
1. CH340 driver kurulu mu kontrol edin
2. Farklı COM port deneyin
3. Arduino IDE'den port'u kontrol edin

### 12.4 Bot Durduruluyor

**Olası Nedenler:**
1. Oyun penceresi odağı kaybetti → OBS Bypass aktif edin
2. Mola sistemi aktif → Mola süresi doldu mu kontrol edin

### 12.5 Hata Logları

**Dosya:** `cargo_errors.log`

**Okuma:**
```
error[E0433]: failed to resolve: use of undeclared crate
  --> src/main.rs:10:5
   |
10 |     some_crate::function();
   |     ^^^^^^^^^^ use of undeclared crate
```

---

## 📋 HIZLI AYAR ÖNERİLERİ

### Güvenli Mod (Anti-Tespit Maksimum)
```json
{
  "anti_tespit_modu": true,
  "rastgele_gecikme": true,
  "gecikme_min": 200,
  "gecikme_max": 500,
  "mola_sistemi_aktif": true,
  "mola_aralik_dk": 45,
  "mola_sure_dk": 10,
  "mouse_mode": "Arduino_AUTO"
}
```

### Hızlı Mod (Riskli)
```json
{
  "anti_tespit_modu": false,
  "rastgele_gecikme": false,
  "mola_sistemi_aktif": false,
  "mouse_mode": "WinAPI"
}
```

### Yayın Modu (OBS ile)
```json
{
  "obs_bypass": true,
  "anti_tespit_modu": true,
  "rastgele_gecikme": true
}
```

---

## 📞 DESTEK

Sorun yaşarsanız:
1. Logları kontrol edin
2. Config.json'ı doğrulayın
3. `cargo build --release` ile tekrar derleyin
4. KURULUM.bat ile bağımlılıkları kontrol edin

---

**K-BOT v109** - Metin2 Otomasyon Botu
**Versiyon:** 109.0.0
**Son Güncelleme:** 2026