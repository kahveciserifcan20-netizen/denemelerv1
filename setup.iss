; ═══════════════════════════════════════════════════════════════════════════════
; K-BOT v109 - Inno Setup Kurulum Scripti
; ═══════════════════════════════════════════════════════════════════════════════

[Setup]
AppId={{K-BOT-2024-109-ABSOLUTE-MONOLITH}
AppName=K-BOT
AppVersion=109.0.0
AppVerName=K-BOT v109
AppPublisher=K-BOT Team
AppPublisherURL=https://k-bot.dev
AppSupportURL=https://k-bot.dev/support
AppUpdatesURL=https://k-bot.dev/updates
DefaultDirName={autopf}\K-BOT
DefaultGroupName=K-BOT
AllowNoIcons=yes
LicenseFile=LICENSE.txt
InfoBeforeFile=README.md
OutputDir=Output
OutputBaseFilename=K-BOT-Setup
SetupIconFile=kbot_icon.ico
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
LZMANumBlockThreads=4
LZMADictionarySize=65536
LZMAMatchFinder=BT
InternalCompressLevel=ultra64
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
MinVersion=10.0.0
UninstallDisplayIcon={app}\kbot_icon.ico
UninstallDisplayName=K-BOT v109
DisableWelcomePage=no
DisableDirPage=no
DisableProgramGroupPage=no
DisableFinishedPage=no
DisableReadyPage=no
WizardStyle=modern
ShowLanguageDialog=yes
LanguageDetectionMethod=locale
UsePreviousLanguage=no
VersionInfoVersion=109.0.0
VersionInfoCompany=K-BOT Team
VersionInfoDescription=K-BOT Metin2 Farming Bot
VersionInfoCopyright=Copyright (C) 2024 K-BOT Team
VersionInfoProductName=K-BOT
VersionInfoProductVersion=109.0.0

; ═══════════════════════════════════════════════════════════════════════════════
; DILLER
; ═══════════════════════════════════════════════════════════════════════════════
[Languages]
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

; ═══════════════════════════════════════════════════════════════════════════════
; GOREVLER
; ═══════════════════════════════════════════════════════════════════════════════
[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

; ═══════════════════════════════════════════════════════════════════════════════
; DOSYALAR
; ═══════════════════════════════════════════════════════════════════════════════
[Files]
; Ana EXE
Source: "the_absolute_monolith.exe"; DestDir: "{app}"; Flags: ignoreversion nocompression
Source: "config.json"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist
Source: "hedef_kilit.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "sparda_hedef_kilit.png"; DestDir: "{app}"; Flags: ignoreversion

; RTEN modelleri (OCR)
Source: "text-detection.rten"; DestDir: "{app}"; Flags: ignoreversion
Source: "text-recognition.rten"; DestDir: "{app}"; Flags: ignoreversion

; YOLO Modelleri
Source: "yolo_modelleri\*.onnx"; DestDir: "{app}\yolo_modelleri"; Flags: ignoreversion recursesubdirs

; Captcha Sablonlari
Source: "captcha_sablonlari\*"; DestDir: "{app}\captcha_sablonlari"; Flags: ignoreversion recursesubdirs createallsubdirs

; Dokumanlar
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "REHBER.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion

; Remote Controller
Source: "remote_controller.html"; DestDir: "{app}"; Flags: ignoreversion

; Ikon dosyasi
Source: "kbot_icon.ico"; DestDir: "{app}"; Flags: ignoreversion

; ═══════════════════════════════════════════════════════════════════════════════
; IKONLAR
; ═══════════════════════════════════════════════════════════════════════════════
[Icons]
Name: "{group}\K-BOT"; Filename: "{app}\the_absolute_monolith.exe"; IconFilename: "{app}\kbot_icon.ico"
Name: "{group}\{cm:ProgramOnTheWeb,K-BOT}"; Filename: "https://k-bot.dev"
Name: "{group}\{cm:UninstallProgram,K-BOT}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\K-BOT"; Filename: "{app}\the_absolute_monolith.exe"; IconFilename: "{app}\kbot_icon.ico"; Tasks: desktopicon
Name: "{userappdata}\Microsoft\Internet Explorer\Quick Launch\K-BOT"; Filename: "{app}\the_absolute_monolith.exe"; IconFilename: "{app}\kbot_icon.ico"; Tasks: quicklaunchicon

; ═══════════════════════════════════════════════════════════════════════════════
; KAYIT DEFTERI
; ═══════════════════════════════════════════════════════════════════════════════
[Registry]
Root: HKCU; Subkey: "Software\K-BOT"; ValueType: string; ValueName: "InstallPath"; ValueData: "{app}"
Root: HKCU; Subkey: "Software\K-BOT"; ValueType: string; ValueName: "Version"; ValueData: "109"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "K-BOT"; ValueData: "{app}\the_absolute_monolith.exe"; Flags: uninsdeletevalue; Check: IsAutoStart

; ═══════════════════════════════════════════════════════════════════════════════
; CALISTIR
; ═══════════════════════════════════════════════════════════════════════════════
[Run]
Filename: "{app}\the_absolute_monolith.exe"; Description: "{cm:LaunchProgram,K-BOT}"; Flags: nowait postinstall skipifsilent

; ═══════════════════════════════════════════════════════════════════════════════
; KALDIRMA
; ═══════════════════════════════════════════════════════════════════════════════
[UninstallDelete]
Type: filesandordirs; Name: "{app}\yolo_modelleri"
Type: filesandordirs; Name: "{app}\captcha_sablonlari"
Type: filesandordirs; Name: "{app}\logs"
Type: filesandordirs; Name: "{app}\Output"
Type: files; Name: "{app}\*.log"
Type: files; Name: "{app}\*.txt"
Type: dirifempty; Name: "{app}"

; ═══════════════════════════════════════════════════════════════════════════════
; KOD
; ═══════════════════════════════════════════════════════════════════════════════
[Code]
function IsAutoStart: Boolean;
begin
  Result := False;
end;

function InitializeSetup: Boolean;
var
  Uninstaller: String;
  ErrorCode: Integer;
begin
  // Eski versiyon kontrolu
  if RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'Software\Microsoft\Windows\CurrentVersion\Uninstall\{K-BOT-2024-109-ABSOLUTE-MONOLITH}_is1',
    'UninstallString', Uninstaller) then
  begin
    Uninstaller := RemoveQuotes(Uninstaller);
    if MsgBox('K-BOT zaten kurulu. Eski versiyonu kaldirip devam etmek ister misiniz?', mbConfirmation, MB_YESNO) = IDYES then
    begin
      Uninstaller := Uninstaller + ' /SILENT';
      Exec(Uninstaller, '', '', SW_HIDE, ewWaitUntilTerminated, ErrorCode);
    end;
  end;
  Result := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    // Kurulum sonrasi islemler
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Kaldirma sonrasi temizlik
    if MsgBox('Tum ayarlar ve log dosyalari silinsin mi?', mbConfirmation, MB_YESNO) = IDYES then
    begin
      DelTree(ExpandConstant('{app}'), True, True, True);
    end;
  end;
end;