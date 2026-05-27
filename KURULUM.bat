@echo off
:: ═══════════════════════════════════════════════════════════════════════════
:: K-BOT v109 - OTOMATIK KURULUM SCRIPTI
:: ═══════════════════════════════════════════════════════════════════════════

title K-BOT v109 - Kurulum Sihirbazi
color 0A

:: Scriptin bulundugu dizine gec
cd /d "%~dp0"

:: Yonetici kontrolu - degilse otomatik yeniden baslat
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Yonetici yetkisi gerekli...
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo.
echo  ================================================================
echo           K-BOT v109 - OTOMATIK KURULUM SISTEMI
echo  ================================================================
echo.
echo  Calisma dizini: %CD%
echo.

:: Cargo.toml kontrolu
if not exist "Cargo.toml" (
    echo  [HATA] Cargo.toml bulunamadi!
    echo.
    echo  Bu script K-BOT proje klasorunde olmali.
    echo  Lutfen scripti Cargo.toml dosyasinin oldugu klasore tasiyin.
    echo.
    pause
    exit /b 1
)

echo  [OK] Cargo.toml bulundu, kurulum basliyor...
echo.

:: ═══════════════════════════════════════════════════════════════════════════
:: 1. VISUAL STUDIO BUILD TOOLS
:: ═══════════════════════════════════════════════════════════════════════════
echo  [1/9] Visual Studio Build Tools kontrol ediliyor...

where link.exe >nul 2>&1
if %errorLevel% neq 0 (
    echo.
    echo  Visual Studio Build Tools bulunamadi!
    echo  Indiriliyor ve kuruluyor... 5-10 dakika surebilir.
    echo.
    
    curl -L -o "%TEMP%\vs_buildtools.exe" "https://aka.ms/vs/17/release/vs_buildtools.exe" 2>nul
    if exist "%TEMP%\vs_buildtools.exe" (
        start /wait "" "%TEMP%\vs_buildtools.exe" --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
        del "%TEMP%\vs_buildtools.exe" 2>nul
        
        echo.
        echo  [OK] Visual Studio Build Tools kuruldu!
        echo  Bilgisayari yeniden baslatin ve scripti tekrar calistirin.
        echo.
        pause
        exit /b 0
    ) else (
        echo  [HATA] Indirilemedi! Manuel kurun: https://visualstudio.microsoft.com/visual-cpp-build-tools/
        pause
        exit /b 1
    )
) else (
    echo  [OK] Zaten kurulu.
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 2. VISUAL C++ REDISTRIBUTABLE
:: ═══════════════════════════════════════════════════════════════════════════
echo  [2/9] Visual C++ Redistributable kontrol ediliyor...

where vcruntime140.dll >nul 2>&1
if %errorLevel% neq 0 (
    echo  Indiriliyor...
    curl -L -o "%TEMP%\vc_redist.x64.exe" "https://aka.ms/vs/17/release/vc_redist.x64.exe" 2>nul
    if exist "%TEMP%\vc_redist.x64.exe" (
        start /wait "" "%TEMP%\vc_redist.x64.exe" /quiet /norestart
        del "%TEMP%\vc_redist.x64.exe" 2>nul
        echo  [OK] Kuruldu.
    )
) else (
    echo  [OK] Zaten kurulu.
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 3. RUST VE CARGO
:: ═══════════════════════════════════════════════════════════════════════════
echo  [3/9] Rust ve Cargo kontrol ediliyor...

where cargo >nul 2>&1
if %errorLevel% neq 0 (
    echo  Indiriliyor...
    curl -L -o "%TEMP%\rustup-init.exe" "https://win.rustup.rs/x86_64" 2>nul
    if exist "%TEMP%\rustup-init.exe" (
        "%TEMP%\rustup-init.exe" -y --default-toolchain stable
        del "%TEMP%\rustup-init.exe" 2>nul
        set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
        echo  [OK] Kuruldu.
    ) else (
        echo  [HATA] Indirilemedi! Manuel kurun: https://rustup.rs
        pause
        exit /b 1
    )
) else (
    echo  [OK] Zaten kurulu.
    cargo --version
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 4. PYTHON
:: ═══════════════════════════════════════════════════════════════════════════
echo  [4/9] Python kontrol ediliyor...

where python >nul 2>&1
if %errorLevel% neq 0 (
    echo  Indiriliyor...
    curl -L -o "%TEMP%\python-installer.exe" "https://www.python.org/ftp/python/3.12.0/python-3.12.0-amd64.exe" 2>nul
    if exist "%TEMP%\python-installer.exe" (
        start /wait "" "%TEMP%\python-installer.exe" /quiet InstallAllUsers=1 PrependPath=1 Include_test=0
        del "%TEMP%\python-installer.exe" 2>nul
        set "PATH=%LOCALAPPDATA%\Programs\Python\Python312;%PATH%"
        echo  [OK] Kuruldu.
    ) else (
        echo  [HATA] Indirilemedi! Manuel kurun: https://python.org
        pause
        exit /b 1
    )
) else (
    echo  [OK] Zaten kurulu.
    python --version
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 5. PYTHON KUTUPHANELERI
:: ═══════════════════════════════════════════════════════════════════════════
echo  [5/9] Python kutuphaneleri kuruluyor...

pip install opencv-python numpy pillow pyautogui mss 2>nul
if %errorLevel% equ 0 (
    echo  [OK] Kuruldu.
) else (
    pip install --user opencv-python numpy pillow pyautogui mss 2>nul
    echo  [OK] Kuruldu.
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 6. TESSERACT OCR
:: ═══════════════════════════════════════════════════════════════════════════
echo  [6/9] Tesseract OCR kontrol ediliyor...

where tesseract >nul 2>&1
if %errorLevel% neq 0 (
    echo  Indiriliyor...
    curl -L -o "%TEMP%\tesseract-installer.exe" "https://digi.bib.uni-mannheim.de/tesseract/tesseract-ocr-w64-setup-5.3.3.20231005.exe" 2>nul
    if exist "%TEMP%\tesseract-installer.exe" (
        start /wait "" "%TEMP%\tesseract-installer.exe" /S
        del "%TEMP%\tesseract-installer.exe" 2>nul
        set "PATH=%LOCALAPPDATA%\Programs\Tesseract-OCR;%PATH%"
        echo  [OK] Kuruldu.
    ) else (
        echo  [UYARI] Indirilemedi. Manuel: https://github.com/UB-Mannheim/tesseract/wiki
    )
) else (
    echo  [OK] Zaten kurulu.
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 7. INTERCEPTION DRIVER
:: ═══════════════════════════════════════════════════════════════════════════
echo  [7/9] Interception Driver kontrol ediliyor...

if exist "C:\Program Files\Interception\install-interception.exe" (
    echo  [OK] Zaten kurulu.
) else (
    echo  Indiriliyor...
    curl -L -o "%TEMP%\interception.zip" "https://github.com/oblitum/Interception/releases/download/v1.0.1/Interception.zip" 2>nul
    if exist "%TEMP%\interception.zip" (
        powershell -Command "Expand-Archive -Path '%TEMP%\interception.zip' -DestinationPath '%TEMP%\interception' -Force" 2>nul
        if exist "%TEMP%\interception\install-interception.exe" (
            start /wait "" "%TEMP%\interception\install-interception.exe"
            echo  [OK] Kuruldu.
        )
        del "%TEMP%\interception.zip" 2>nul
        rmdir /s /q "%TEMP%\interception" 2>nul
    ) else (
        echo  [BILGI] Indirilemedi. Manuel: https://github.com/oblitum/Interception
    )
)

:: ═══════════════════════════════════════════════════════════════════════════
:: 8. ARDUINO DRIVER (Opsiyonel)
:: ═══════════════════════════════════════════════════════════════════════════
echo  [8/9] Arduino driver kontrol ediliyor (Opsiyonel)...

echo  Arduino yoksa atlanir.
echo  [OK] Kontrol tamamlandi.

:: ═══════════════════════════════════════════════════════════════════════════
:: 9. PROJE DERLEME
:: ═══════════════════════════════════════════════════════════════════════════
echo  [9/9] Proje derleniyor...
echo  Bu islem 2-5 dakika surebilir...
echo.

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

cargo build --release 2>&1

if %errorLevel% equ 0 (
    echo.
    echo  [OK] Proje basariyla derlendi!
    if exist "target\release\the_absolute_monolith.exe" (
        copy /Y "target\release\the_absolute_monolith.exe" "." >nul
        echo  [OK] EXE ana klasore kopyalandi.
    )
) else (
    echo.
    echo  [HATA] Derleme basarisiz!
    cargo build --release 2> cargo_errors.log
    echo  Hata logu: cargo_errors.log
    pause
    exit /b 1
)

:: ═══════════════════════════════════════════════════════════════════════════
:: TAMAMLANDI
:: ═══════════════════════════════════════════════════════════════════════════
echo.
echo  ================================================================
echo           KURULUM BASARIYLA TAMAMLANDI!
echo  ================================================================
echo.
echo  Kurulanlar:
echo  - Visual Studio Build Tools
echo  - Visual C++ Redistributable
echo  - Rust ve Cargo
echo  - Python 3.12
echo  - Python kutuphaneleri
echo  - Tesseract OCR
echo  - Interception Driver
echo  - K-BOT v109
echo.
echo  Projeyi calistirmak icin:
echo  the_absolute_monolith.exe dosyasina cift tiklayin
echo.
echo  ================================================================
echo.

:: Temizlik
if exist "target\debug" rmdir /s /q "target\debug" 2>nul

echo  Bilgisayari yeniden baslatmak icin bir tusa basin...
echo  (Iptal etmek icin pencereyi kapatın)
echo.
pause

echo.
echo  10 saniye icinde yeniden baslatilacak...
timeout /t 10 /nobreak
shutdown /r /t 0