#define AppVersion "1.0"

[Setup]
AppId=TabGlide
AppName=TabGlide
AppVerName=TabGlide
AppVersion={#AppVersion}
AppPublisher=Philipp Wallrafen
AppPublisherURL=https://github.com/e4zyphil/TabGlide
AppSupportURL=https://github.com/e4zyphil/TabGlide/issues
AppUpdatesURL=https://github.com/e4zyphil/TabGlide/releases
DefaultDirName={userappdata}\TabGlide
DefaultGroupName=TabGlide
SetupIconFile=..\icons\TabGlide.ico
UninstallDisplayIcon={app}\TabGlide.exe
OutputDir=..\build
OutputBaseFilename=TabGlideInstaller
DisableWelcomePage=yes
DisableFinishedPage=yes
DisableReadyPage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableStartupPrompt=yes
CreateAppDir=yes
Uninstallable=yes
Compression=none
SolidCompression=no
ArchitecturesInstallIn64BitMode=x64

[Files]
Source: "..\src\TabGlide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userstartup}\TabGlide"; Filename: "{app}\TabGlide.exe"; WorkingDir: "{app}"
Name: "{commonprograms}\TabGlide"; Filename: "{app}\TabGlide.exe"; WorkingDir: "{app}"

[Run]
; Kill TabGlide.exe silently (if already running)
Filename: "{app}\TabGlide.exe"; Description: "Start TabGlide"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/IM TabGlide.exe /F"; Flags: runhidden

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True; // run silently
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    Sleep(1000); // wait 1 second to ensure process is terminated
  end;
end;