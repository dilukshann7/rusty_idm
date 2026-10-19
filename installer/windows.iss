#ifndef SourceRoot
  #define SourceRoot ".."
#endif
#ifndef PayloadRoot
  #define PayloadRoot SourceRoot + "\dist\payload"
#endif
#define AppVersion "0.1.0"

[Setup]
AppId={{D407FE27-761D-4EF5-8565-A3221FAE6423}
AppName=Rusty Download Manager
AppVersion={#AppVersion}
AppPublisher=Rusty Download Manager contributors
DefaultDirName={localappdata}\RustyIDM
DefaultGroupName=Rusty Download Manager
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#SourceRoot}\dist
OutputBaseFilename=RustyDownloadManager-{#AppVersion}-windows-x64-setup
SetupIconFile={#SourceRoot}\assets\branding\rusty.ico
UninstallDisplayIcon={app}\bin\rusty-win32.exe
UninstallDisplayName=Rusty Download Manager
VersionInfoProductName=Rusty Download Manager
VersionInfoVersion=0.1.0.0
VersionInfoDescription=Rusty Download Manager setup
LicenseFile={#SourceRoot}\LICENSE
InfoAfterFile={#SourceRoot}\docs\SETUP.txt
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=no
RestartApplications=no
AppMutex=Local\RustyDownloadManager.Running,Local\RustyDownloadManager.NativeHost.Running

[Tasks]
Name: "startup"; Description: "Start Rusty in the background when I sign in"
Name: "desktop"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#PayloadRoot}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Rusty Download Manager"; Filename: "{app}\bin\rusty-win32.exe"
Name: "{group}\Connect Chrome"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -STA -File ""{app}\scripts\chrome-integration.ps1"""; IconFilename: "{app}\branding\rusty.ico"
Name: "{group}\User guide"; Filename: "{app}\docs\USER_GUIDE.md"
Name: "{autodesktop}\Rusty Download Manager"; Filename: "{app}\bin\rusty-win32.exe"; Tasks: desktop

[Run]
Filename: "{app}\bin\rusty-win32.exe"; Parameters: "--background"; Description: "Start Rusty in the background"; Flags: postinstall nowait skipifsilent
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -STA -File ""{app}\scripts\chrome-integration.ps1"""; Description: "Connect Chrome for automatic download capture"; Flags: postinstall nowait skipifsilent runhidden; Check: NeedsChromeSetup

[Code]
function BinariesInUse: Boolean;
var Locator, Services, Processes, Process: Variant; I: Integer; ExePath: String;
begin
  Result := False;
  try
    Locator := CreateOleObject('WbemScripting.SWbemLocator');
    Services := Locator.ConnectServer('', 'root\CIMV2');
    Processes := Services.ExecQuery('SELECT ExecutablePath FROM Win32_Process WHERE Name=''rusty-win32.exe'' OR Name=''rusty-native-host.exe''');
    for I := 0 to Processes.Count - 1 do begin
      Process := Processes.ItemIndex(I);
      if not VarIsNull(Process.ExecutablePath) then begin
        ExePath := Process.ExecutablePath;
        if (CompareText(ExePath, ExpandConstant('{localappdata}\RustyIDM\bin\rusty-win32.exe')) = 0) or
           (CompareText(ExePath, ExpandConstant('{localappdata}\RustyIDM\bin\rusty-native-host.exe')) = 0) then Result := True;
      end;
    end;
  except
    // If process inspection is unavailable, refuse replacement rather than
    // guessing whether an older installation has our newer installer mutex.
    Result := True;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if BinariesInUse then Result := 'Exit Rusty using its tray menu and close Chrome before continuing. Running downloads will not be stopped by setup.';
end;

function InitializeUninstall: Boolean;
begin
  Result := not BinariesInUse;
  if not Result then MsgBox('Exit Rusty using its tray menu and close Chrome before uninstalling. Running downloads will not be stopped.', mbInformation, MB_OK);
end;

function NeedsChromeSetup: Boolean;
begin
  Result := not FileExists(ExpandConstant('{localappdata}\RustyIDM\browser\registration-owner.json'));
end;

procedure ConfigureIntegration(Mode: String; Startup: String);
var ResultCode: Integer; Params: String;
begin
  Params := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\scripts\release-integration.ps1') + '" -Mode ' + Mode + ' -EnableStartup ' + Startup + ' -InstallRoot "' + ExpandConstant('{app}') + '"';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    RaiseException('Windows could not configure Rusty integration.');
  if ResultCode <> 0 then
    RaiseException('Rusty could not update its owned startup/Chrome settings. Existing unrelated settings were preserved. Run Connect Chrome for details.');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Startup: String;
begin
  if CurStep = ssPostInstall then begin
    Startup := '0';
    if WizardIsTaskSelected('startup') then Startup := '1';
    ConfigureIntegration('Install', Startup);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then ConfigureIntegration('Uninstall', '0');
end;
