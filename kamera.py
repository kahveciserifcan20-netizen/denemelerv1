import sys, mmap, time, ctypes, os, threading
import logging

logging.basicConfig(level=logging.DEBUG, 
                    format='%(asctime)s - %(message)s', 
                    handlers=[logging.FileHandler("kamera_log.txt", mode='w', encoding='utf-8'), 
                              logging.StreamHandler(sys.stdout)])

logging.info("\n=========================================")
logging.info("[PYTHON-PRO] PrintWindow API Ekran Yakalama Motoru Aktif!")

try:
    import numpy as np
except ImportError:
    logging.error("[PYTHON-FATAL] Eksik modul! CMD'ye yaz: pip install numpy")
    sys.exit(1)

if len(sys.argv) < 2:
    sys.exit(1)

HWND = int(sys.argv[1])

# v111: SHM ismi parametrik — çoklu client desteği
SHM_NAME = sys.argv[2] if len(sys.argv) >= 3 else "Local\\MonolithVision"
logging.info(f"[PYTHON-SHM] Shared Memory: {SHM_NAME} | HWND: {HWND}")

WIDTH, HEIGHT = 800, 600
BUF_SIZE = 5 + (WIDTH * HEIGHT * 4)

try:
    shm = mmap.mmap(-1, BUF_SIZE, SHM_NAME)
    shm.seek(0)
    shm.write(b'\x00' * BUF_SIZE)
    logging.info("[PYTHON-OK] RAM Havuzu Kilitlendi.")
except Exception as e:
    logging.error(f"Hata: {e}")
    sys.exit(1)

# Windows API tanımlamaları
user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

# PrintWindow flag'leri
PW_CLIENTONLY = 0x1
PW_RENDERFULLCONTENT = 0x2  # Windows 8.1+ için tam içerik

# BITMAPINFOHEADER yapısı
class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", ctypes.c_uint32),
        ("biWidth", ctypes.c_int32),
        ("biHeight", ctypes.c_int32),
        ("biPlanes", ctypes.c_uint16),
        ("biBitCount", ctypes.c_uint16),
        ("biCompression", ctypes.c_uint32),
        ("biSizeImage", ctypes.c_uint32),
        ("biXPelsPerMeter", ctypes.c_int32),
        ("biYPelsPerMeter", ctypes.c_int32),
        ("biClrUsed", ctypes.c_uint32),
        ("biClrImportant", ctypes.c_uint32),
    ]

class BITMAPINFO(ctypes.Structure):
    _fields_ = [
        ("bmiHeader", BITMAPINFOHEADER),
        ("bmiColors", ctypes.c_uint32 * 3),
    ]

def kill_on_orphan():
    try: sys.stdin.read()
    except: pass
    os._exit(0)

threading.Thread(target=kill_on_orphan, daemon=True).start()

# Bitmap oluştur
hdc = user32.GetDC(0)
hmemdc = gdi32.CreateCompatibleDC(hdc)
hbitmap = gdi32.CreateCompatibleBitmap(hdc, WIDTH, HEIGHT)
gdi32.SelectObject(hmemdc, hbitmap)
user32.ReleaseDC(0, hdc)

# BITMAPINFO hazırla
bmi = BITMAPINFO()
bmi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
bmi.bmiHeader.biWidth = WIDTH
bmi.bmiHeader.biHeight = -HEIGHT  # Negatif = top-down bitmap
bmi.bmiHeader.biPlanes = 1
bmi.bmiHeader.biBitCount = 32
bmi.bmiHeader.biCompression = 0  # BI_RGB

# Pixel buffer
pixel_buffer = ctypes.create_string_buffer(WIDTH * HEIGHT * 4)

heartbeat = 0
frame_count = 0

logging.info("[PYTHON-OK] PrintWindow döngüsü başlıyor...")

while True:
    if user32.IsWindow(HWND) == 0:
        logging.info("[PYTHON] Pencere kapandı, çıkılıyor...")
        break
    
    try:
        # PrintWindow ile arka plan penceresini yakala
        # PW_RENDERFULLCONTENT = 2 (Windows 8.1+)
        # PW_CLIENTONLY = 1 (sadece client area)
        result = user32.PrintWindow(HWND, hmemdc, PW_CLIENTONLY | PW_RENDERFULLCONTENT)
        
        if result == 0:
            # PrintWindow başarısız, eski yöntemi dene
            # Pencere koordinatlarını al
            rect = ctypes.wintypes.RECT()
            user32.GetWindowRect(HWND, ctypes.byref(rect))
            
            # Pencere boyutlarını kontrol et
            win_w = rect.right - rect.left
            win_h = rect.bottom - rect.top
            
            if win_w < WIDTH or win_h < HEIGHT:
                logging.warning(f"[PYTHON] Pencere çok küçük: {win_w}x{win_h}")
                time.sleep(0.1)
                continue
            
            # Client area offset hesapla
            pt = ctypes.wintypes.POINT(0, 0)
            user32.ClientToScreen(HWND, ctypes.byref(pt))
            client_x = pt.x
            client_y = pt.y
            
            # Ekran DC'sinden kopyala
            hdc_screen = user32.GetDC(0)
            gdi32.BitBlt(hmemdc, 0, 0, WIDTH, HEIGHT, hdc_screen, client_x, client_y, 0x00CC0020)  # SRCCOPY
            user32.ReleaseDC(0, hdc_screen)
        
        # Bitmap'ten pixel oku
        gdi32.GetDIBits(hmemdc, hbitmap, 0, HEIGHT, pixel_buffer, ctypes.byref(bmi), 0)  # DIB_RGB_COLORS
        
        # BGRA -> RGBA dönüşümü
        raw_bytes = pixel_buffer.raw
        frame = np.frombuffer(raw_bytes, dtype=np.uint8).reshape((HEIGHT, WIDTH, 4))
        
        # BGRA'dan RGBA'ya çevir
        rgba = np.empty_like(frame)
        rgba[:, :, 0] = frame[:, :, 2]  # R
        rgba[:, :, 1] = frame[:, :, 1]  # G
        rgba[:, :, 2] = frame[:, :, 0]  # B
        rgba[:, :, 3] = 255              # A
        
        pixel_bytes = rgba.tobytes()
        
        heartbeat = (heartbeat + 1) % 255
        shm.seek(0)
        shm.write(b'\x00') 
        shm.write(bytes([heartbeat])) 
        shm.write(b'\x00\x00\x00') 
        shm.write(pixel_bytes)
        shm.seek(0)
        shm.write(b'\x01')
        
        frame_count += 1
        if frame_count % 100 == 0:
            logging.info(f"[PYTHON] {frame_count} frame yakalandı")
        
    except Exception as e:
        logging.error(f"Frame yakalama hatasi: {e}")
    
    time.sleep(0.003)  # ~333 FPS

# Temizlik
gdi32.DeleteObject(hbitmap)
gdi32.DeleteDC(hmemdc)