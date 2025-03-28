; Inno Setup Script for TabGlide - silent installer, user-level, auto-start, config creation
; Save this file as TabGlide.iss and open it in Inno Setup Compiler

[Setup]
AppName=TabGlide
AppVersion=1.0.0
DefaultDirName={userappdata}\TabGlide
DefaultGroupName=TabGlide
UninstallDisplayIcon={app}\TabGlide.exe
OutputDir=.
OutputBaseFilename=TabGlideSetup_{#AppVersion}
DisableWelcomePage=yes
DisableFinishedPage=yes
DisableReadyPage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableStartupPrompt=yes
SilentInstall=yes
DisableStartupPrompt=yes
AlwaysCreateUninstallIcon=yes
CreateAppDir=yes
Uninstallable=yes
Compression=lzma
SolidCompression=yes

[Files]
Source: "TabGlide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "config.ini"; DestDir: "{app}"; Flags: onlyifdoesntexist

[Icons]
Name: "Startup\TabGlide"; Filename: "{app}\TabGlide.exe"; WorkingDir: "{app}"

[Run]
Filename: "{app}\TabGlide.exe"; Description: "Start TabGlide"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\tabglide.log"
Type: filesandordirs; Name: "{app}"

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True; // run silently
end;