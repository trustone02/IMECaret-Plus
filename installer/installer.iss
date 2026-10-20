; IMECaret Plus 공식 설치 프로그램 스크립트 (Inno Setup)
#define MyAppName "IMECaret Plus"
#define MyAppVersion "1.1.1"
#define MyAppExeName "IMECaretPlus.exe"
#define MyAppPublisher "shinz"

[Setup]
AppId={{B5A1E6C2-9B1A-4B3B-8E8B-7B1D7C9C9A11}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\IMECaretPlus
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; 관리자 권한(UAC) 없이 사용자 계정에 안전하고 빠르게 설치
PrivilegesRequired=lowest
OutputDir=Output
OutputBaseFilename=IMECaretPlus-Setup
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
Source: "..\dist\IMECaret.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\ime-caret.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\IMECaretPlus.ini"; DestDir: "{app}"; Flags: onlyifdoesntexist
Source: "..\dist\IMECaret.ini"; DestDir: "{app}"; Flags: onlyifdoesntexist
Source: "..\dist\*.wav"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
; Run 키만으로 시작프로그램이 실행되지 않는 환경을 대비한 보조 등록 (중복 실행은 프로그램의 단일 실행 잠금이 방지)
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: startupicon

[Registry]
; 사용자가 설치 시 "Windows 시작 시 자동으로 실행"에 체크했을 때만 시작프로그램(Run 키)에 등록
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "IMECaretPlus"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: startupicon

[UninstallDelete]
Type: files; Name: "{app}\*.ini"
Type: files; Name: "{app}\*.log"
Type: files; Name: "{app}\*.wav"
Type: files; Name: "{app}\*.exe"
Type: filesandordirs; Name: "{app}"
Type: filesandordirs; Name: "{group}"
; 프로그램이 남긴 로그 및 아래한글 상태 저장 파일
Type: filesandordirs; Name: "{localappdata}\IMECaretPlus"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "설치 완료 후 IMECaret Plus 실행하기"; Flags: nowait postinstall skipifsilent

[Code]
// 설치 시작 시 기존 실행 중인 프로세스 안전 종료
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  Exec('taskkill.exe', '/F /IM IMECaretPlus.exe /IM IMECaret.exe /IM ime-caret.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(200);
end;

// 구버전(IME Caret) 설치본이 남긴 시작프로그램 항목 정리: 새 항목과 중복 실행되거나
// 삭제된 경로를 가리키는 것을 방지한다.
procedure CleanLegacyStartup();
begin
  RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'IMECaret');
  RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run', 'IMECaret');
  DeleteFile(ExpandConstant('{userstartup}\IMECaret.lnk'));
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
    CleanLegacyStartup();
end;

// 언인스톨 시작 시 실행 중인 프로세스 안전 종료 (파일 Lock 해제)
function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  Exec('taskkill.exe', '/F /IM IMECaretPlus.exe /IM IMECaret.exe /IM ime-caret.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(200);
end;

// Windows 11 작업표시줄 설정(NotifyIconSettings)에 영구 보존되는 유령 트레이 아이콘 캐시 완전 청소
procedure CleanNotifyIconSettings();
var
  Subkeys: TArrayOfString;
  I: Integer;
  ExePath: String;
  TargetAppDir: String;
begin
  TargetAppDir := Uppercase(ExpandConstant('{app}'));
  if RegGetSubkeyNames(HKCU, 'Control Panel\NotifyIconSettings', Subkeys) then
  begin
    for I := 0 to GetArrayLength(Subkeys) - 1 do
    begin
      if RegQueryStringValue(HKCU, 'Control Panel\NotifyIconSettings\' + Subkeys[I], 'ExecutablePath', ExePath) then
      begin
        ExePath := Uppercase(ExePath);
        if (Pos('IMECARETPLUS.EXE', ExePath) > 0) or 
           (Pos('IMECARET.EXE', ExePath) > 0) or 
           (Pos('IME-CARET.EXE', ExePath) > 0) or 
           (Pos(TargetAppDir, ExePath) > 0) then
        begin
          RegDeleteKeyIncludingSubkeys(HKCU, 'Control Panel\NotifyIconSettings\' + Subkeys[I]);
        end;
      end;
    end;
  end;
end;

// 언인스톨 완료 후 남아있는 파일 및 폴더 강제 삭제
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: String;
  GroupDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // 1. 작업표시줄 설정 알림 아이콘 캐시 청소
    CleanNotifyIconSettings();

    // 2. 시작 프로그램 레지스트리 잔여 키 완전 삭제
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'IMECaretPlus');
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'IMECaret');

    // 3. 앱 설치 폴더 및 찌꺼기 완벽 삭제
    AppDir := ExpandConstant('{app}');
    if DirExists(AppDir) then
    begin
      DelTree(AppDir, True, True, True);
    end;

    // 4. 시작메뉴 바로가기 폴더 완벽 삭제
    GroupDir := ExpandConstant('{group}');
    if DirExists(GroupDir) then
    begin
      DelTree(GroupDir, True, True, True);
    end;
  end;
end;
