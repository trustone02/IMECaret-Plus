; IME Caret 공식 설치 프로그램 스크립트 (Inno Setup)
#define MyAppName "IME Caret"
#define MyAppVersion "2.5.1"
#define MyAppExeName "IMECaret.exe"
#define MyAppPublisher "shinz"

[Setup]
AppId={{B5A1E6C2-9B1A-4B3B-8E8B-7B1D7C9C9A11}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\IMECaret
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; 관리자 권한(UAC) 없이 사용자 계정에 안전하고 빠르게 설치
PrivilegesRequired=lowest
OutputDir=Output
OutputBaseFilename=IMECaret-Setup
Compression=lzma2/max
SolidCompression=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
WizardStyle=modern

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Tasks]
Name: "startupicon"; Description: "Windows 시작 시 자동으로 실행 (권장)"; GroupDescription: "시작 옵션:"; Flags: checkedonce
Name: "desktopicon"; Description: "바탕 화면에 바로가기 아이콘 만들기"; GroupDescription: "바로가기 옵션:"; Flags: unchecked

[Files]
Source: "..\dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\ime-caret.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\IMECaret.ini"; DestDir: "{app}"; Flags: onlyifdoesntexist
Source: "..\dist\*.wav"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; 사용자가 설치 시 "Windows 시작 시 자동으로 실행"에 체크했을 때만 시작프로그램(Run 키)에 등록
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "IMECaret"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "설치 완료 후 IME Caret 실행하기"; Flags: nowait postinstall skipifsilent
