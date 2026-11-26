; Windows installer of sinden-driver (and, optionally, sinden-driver-ui).
; Inno Setup 6: see README.md for the choice and the build.
;
; Defines (iscc /DName=value):
;   DriverVersion  the driver's version (driver.lock)            required
;   DriverExe      sinden-driver.exe          default: dist\driver\x86_64-pc-windows-msvc\sinden-driver.exe
;   UiExe          sinden-driver-ui.exe       optional: without it, no UI component
;   UiVersion      the UI's version (ui/package.json)            with UiExe
;   OutputDir      where the installer goes    default: dist\packages
;   Sign           defined: sign the installer and uninstaller with the
;                  sign tool named "signtool" (iscc /Ssigntool=...)

#ifndef DriverVersion
  #error "Pass /DDriverVersion=X.Y.Z (driver.lock's version)"
#endif
#ifndef DriverExe
  #define DriverExe "..\..\dist\driver\x86_64-pc-windows-msvc\sinden-driver.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist\packages"
#endif
#ifdef UiExe
  #ifndef UiVersion
    #error "Pass /DUiVersion=A.B.C with /DUiExe"
  #endif
#endif

[Setup]
; Never change AppId: upgrades and the uninstaller entry rely on it.
AppId={{6F3E2B7A-4C1D-4E8B-9A57-3D2C1B0E9F42}
AppName=Sinden Driver
AppVersion={#DriverVersion}
AppVerName=Sinden Driver {#DriverVersion}
AppPublisher=trois-six
AppPublisherURL=https://github.com/trois-six/sinden-ui
AppSupportURL=https://github.com/trois-six/sinden-ui/issues
DefaultDirName={autopf}\SindenDriver
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=sinden-driver-{#DriverVersion}-setup-x86_64
SetupIconFile=..\..\ui\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\sinden-driver.exe
UninstallDisplayName=Sinden Driver
InfoBeforeFile=..\..\NOTICE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; The running tray application is closed (Restart Manager) before its file
; is replaced.
CloseApplications=yes
#ifdef Sign
SignTool=signtool
SignedUninstaller=yes
#endif

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[CustomMessages]
en.TypeFull=Driver and tray application
fr.TypeFull=Pilote et application de la zone de notification
en.TypeDriver=Driver only
fr.TypeDriver=Pilote seul
en.ComponentDriver=Driver service (required)
fr.ComponentDriver=Service du pilote (obligatoire)
en.ComponentUi=Tray application: settings and status of the guns
fr.ComponentUi=Application de la zone de notification : réglages et état des pistolets
en.TaskAutostart=Start the tray application at logon
fr.TaskAutostart=Lancer l'application de la zone de notification à l'ouverture de session
en.LaunchUi=Open the settings now
fr.LaunchUi=Ouvrir les réglages maintenant
en.ServiceFailed=The driver service could not be registered (%1, code %2). Run "sinden-driver service install" as an administrator to see why.
fr.ServiceFailed=Le service du pilote n'a pas pu être enregistré (%1, code %2). Lancez « sinden-driver service install » en administrateur pour en voir la raison.
en.Registering=Registering the driver service...
fr.Registering=Enregistrement du service du pilote...

[Types]
Name: "full"; Description: "{cm:TypeFull}"
Name: "driver"; Description: "{cm:TypeDriver}"
Name: "custom"; Description: "Custom"; Flags: iscustom

[Components]
Name: "driver"; Description: "{cm:ComponentDriver}"; Types: full driver custom; Flags: fixed
#ifdef UiExe
Name: "ui"; Description: "{cm:ComponentUi} ({#UiVersion})"; Types: full
#endif

#ifdef UiExe
[Tasks]
Name: "autostart"; Description: "{cm:TaskAutostart}"; Components: ui
#endif

[Dirs]
Name: "{commonappdata}\SindenDriver"

[Files]
Source: "{#DriverExe}"; DestDir: "{app}"; DestName: "sinden-driver.exe"; Components: driver; Flags: ignoreversion
; The configuration is the user's: installed once, kept on upgrade and uninstall.
Source: "..\common\LightgunMono.exe.config"; DestDir: "{commonappdata}\SindenDriver"; Components: driver; Flags: onlyifdoesntexist uninsneveruninstall
Source: "..\..\NOTICE"; DestDir: "{app}"; DestName: "NOTICE.txt"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
#ifdef UiExe
Source: "{#UiExe}"; DestDir: "{app}"; DestName: "sinden-driver-ui.exe"; Components: ui; Flags: ignoreversion
#endif

#ifdef UiExe
[Icons]
Name: "{autoprograms}\Sinden Driver"; Filename: "{app}\sinden-driver-ui.exe"; Components: ui

[Run]
; The UI keeps the login item per user (tauri-plugin-autostart): the
; installing user's choice is applied as that user.
Filename: "{app}\sinden-driver-ui.exe"; Parameters: "--autostart=on"; Tasks: autostart; Flags: runasoriginaluser runhidden waituntilterminated
Filename: "{app}\sinden-driver-ui.exe"; Parameters: "--autostart=off"; Components: ui; Tasks: not autostart; Flags: runasoriginaluser runhidden waituntilterminated
Filename: "{app}\sinden-driver-ui.exe"; Description: "{cm:LaunchUi}"; Components: ui; Flags: postinstall nowait skipifsilent runasoriginaluser
#endif

[UninstallRun]
Filename: "{app}\sinden-driver-ui.exe"; Parameters: "--autostart=off"; RunOnceId: "UiAutostartOff"; Flags: runhidden waituntilterminated skipifdoesntexist
Filename: "{sys}\taskkill.exe"; Parameters: "/f /im sinden-driver-ui.exe"; RunOnceId: "StopUi"; Flags: runhidden waituntilterminated
Filename: "{app}\sinden-driver.exe"; Parameters: "service stop"; RunOnceId: "StopService"; Flags: runhidden waituntilterminated
Filename: "{app}\sinden-driver.exe"; Parameters: "service uninstall"; RunOnceId: "UninstallService"; Flags: runhidden waituntilterminated

[Code]
function RunDriver(const Args: String; var Code: Integer): Boolean;
begin
  Result := Exec(ExpandConstant('{app}\sinden-driver.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, Code);
end;

// An upgrade stops the running service before its binary is replaced.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Result := '';
  if FileExists(ExpandConstant('{app}\sinden-driver.exe')) then
    RunDriver('service stop', Code);
end;

// Registers (or re-registers) and starts the service with the driver's own
// command, reporting a failure instead of ignoring it.
procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
begin
  if CurStep <> ssPostInstall then
    Exit;
  WizardForm.StatusLabel.Caption := CustomMessage('Registering');
  if not RunDriver('service install', Code) or (Code <> 0) then
  begin
    SuppressibleMsgBox(FmtMessage(CustomMessage('ServiceFailed'), ['service install', IntToStr(Code)]), mbError, MB_OK, IDOK);
    Exit;
  end;
  if not RunDriver('service start', Code) or (Code <> 0) then
    SuppressibleMsgBox(FmtMessage(CustomMessage('ServiceFailed'), ['service start', IntToStr(Code)]), mbError, MB_OK, IDOK);
end;
