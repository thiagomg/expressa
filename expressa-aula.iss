; Instalador do Expressa Aula. Quem gera o exe e criar-instalador.ps1
; (Inno Setup 7). A versao vem de Cargo.toml via --define=AppVersion=...
;
; O diretorio instalado tem de ficar inteiro: o editor acha o GTK, os
; schemas e res\ ao lado de expressa-aula.exe.

#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif

[Setup]
; Nao mude o AppId: e ele que faz uma instalacao nova substituir a anterior.
AppId={{F145DB50-3724-4363-A30E-5E414E188CB1}}
AppName=Expressa Aula
AppVersion={#AppVersion}
AppPublisher=Thiago Massari Guedes
DefaultDirName={localappdata}\Programs\Expressa Aula
DefaultGroupName=Expressa Aula
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=dist
OutputBaseFilename=ExpressaAula-{#AppVersion}-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=res\expressa-2.ico
UninstallDisplayIcon={app}\expressa-aula.exe
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "dist\expressa-aula\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Expressa Aula"; Filename: "{app}\expressa-aula.exe"
Name: "{autodesktop}\Expressa Aula"; Filename: "{app}\expressa-aula.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\expressa-aula.exe"; Description: "Abrir Expressa Aula"; Flags: postinstall nowait skipifsilent
