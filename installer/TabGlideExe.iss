#ifndef AppVersion
#define AppVersion "1.1.0"
#endif

[Setup]
AppId=TabGlide
AppName=TabGlide
AppVerName=TabGlide
AppVersion={#AppVersion}
AppPublisher=Philipp Wallrafen
AppPublisherURL=https://github.com/philippwallrafen/TabGlide
AppSupportURL=https://github.com/philippwallrafen/TabGlide/issues
AppUpdatesURL=https://github.com/philippwallrafen/TabGlide/releases
DefaultDirName={localappdata}\Programs\TabGlide
UsePreviousAppDir=no
PrivilegesRequired=lowest
MinVersion=10.0
DefaultGroupName=TabGlide
SetupIconFile=..\icons\TabGlide.ico
UninstallDisplayIcon={app}\TabGlide.exe
OutputDir=..\build
OutputBaseFilename=TabGlide-{#AppVersion}-windows-x64-setup
DisableWelcomePage=yes
DisableFinishedPage=yes
DisableReadyPage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableStartupPrompt=yes
CreateAppDir=yes
Uninstallable=yes
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
CloseApplicationsFilter=TabGlide.exe
RestartApplications=no

[Tasks]
Name: "autostart"; Description: "Start TabGlide when I sign in"; Flags: checkedonce

[Files]
Source: "..\target\release\TabGlide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userstartup}\TabGlide"; Filename: "{app}\TabGlide.exe"; WorkingDir: "{app}"; Tasks: autostart
Name: "{userprograms}\TabGlide"; Filename: "{app}\TabGlide.exe"; WorkingDir: "{app}"

[InstallDelete]
Type: files; Name: "{userstartup}\TabGlide.lnk"; Tasks: not autostart

[Run]
Filename: "{app}\TabGlide.exe"; Description: "Start TabGlide"; Flags: nowait postinstall skipifsilent

[Code]
function LegacyInstallPresent(): Boolean;
begin
  { Old elevated setup can delete the new roaming configuration on uninstall. }
  Result := FileExists(ExpandConstant('{userappdata}\TabGlide\TabGlide.exe')) or
    RegKeyExists(HKLM32, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\TabGlide_is1') or
    RegKeyExists(HKLM64, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\TabGlide_is1');
end;

function StopTabGlide(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  if FileExists(ExpandConstant('{app}\TabGlide.exe')) then
  begin
    Result := Exec(ExpandConstant('{app}\TabGlide.exe'), '--exit', '', SW_HIDE,
      ewWaitUntilTerminated, ResultCode);
    if Result then Result := ResultCode = 0;
  end;
  if CheckForMutexes('Local\TabGlide.Rust.Instance') then Result := False;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if LegacyInstallPresent() then
  begin
    Result := 'Legacy AHK TabGlide was detected. Back up its configuration outside ' +
      'the old TabGlide folder, exit and uninstall the old version, then run this ' +
      'installer again. The old uninstaller can delete the new configuration.';
    Exit;
  end;
  if not StopTabGlide() then
    Result := 'Please exit TabGlide before updating.';
end;

function InitializeUninstall(): Boolean;
begin
  Result := StopTabGlide();
  if not Result and not UninstallSilent then
    MsgBox('Please exit TabGlide before uninstalling.', mbError, MB_OK);
end;
