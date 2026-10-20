; IME Caret(SSMS용 전자/반자 경고 기능 포함) 설치 프로그램을 만드는 Inno Setup 스크립트.
#define MyAppName "IME Caret"
#define MyAppVersion "2.5.1"
#define MyAppExeName "ime-caret.exe"
#define MyAppPublisher "shinz"

[Setup]
AppId={{B5A1E6C2-9B1A-4B3B-8E8B-7B1D7C9C9A11}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\IMECaret
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; 관리자 권한 없이 현재 사용자 계정에만 설치한다(트레이에 상주하는 개인용
; 도구라 UAC 팝업 없이 바로 설치되게 하는 게 더 편하다).
PrivilegesRequired=lowest
OutputDir=Output
OutputBaseFilename=IMECaret-Setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
WizardStyle=modern

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Tasks]
Name: "startupicon"; Description: "Windows 시작 시 자동 실행"; Flags: checkedonce
Name: "desktopicon"; Description: "바탕 화면에 바로가기 만들기"; Flags: unchecked

[Files]
Source: "..\dist\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

; "Windows 시작 시 자동 실행"에 체크했을 때만 HKCU Run 키에 등록한다.
; uninsdeletevalue 덕분에 제거 프로그램 실행 시 이 키도 같이 삭제된다.
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "IMECaret"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
