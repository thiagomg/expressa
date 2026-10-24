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
ChangesEnvironment=yes

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "envpath"; Description: "Adicionar expressa e expressa-grafico ao PATH"; GroupDescription: "Terminal:"; Flags: checkedonce

[Files]
; A pasta inteira: editor, GTK, exemplos e bin\ (expressa.exe e expressa-grafico.exe).
Source: "dist\expressa-aula\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Copia para Documentos os exemplos que ainda nao existem. Nao substitui
; arquivo que o aluno ja tenha. Desinstalar nao apaga essa pasta.
Source: "dist\expressa-aula\exemplos\*"; DestDir: "{userdocs}\Expressa Aula\exemplos"; Flags: ignoreversion recursesubdirs createallsubdirs onlyifdoesntexist uninsneveruninstall

[Icons]
Name: "{group}\Expressa Aula"; Filename: "{app}\expressa-aula.exe"
Name: "{autodesktop}\Expressa Aula"; Filename: "{app}\expressa-aula.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\expressa-aula.exe"; Description: "Abrir Expressa Aula"; Flags: postinstall nowait skipifsilent

[Code]
function NormDir(const S: string): string;
begin
  Result := Trim(S);
  while (Result <> '') and (Result[Length(Result)] = '\') do
    Delete(Result, Length(Result), 1);
end;

function BinDir: string;
begin
  Result := NormDir(ExpandConstant('{app}\bin'));
end;

function DirInPath(const Path, Dir: string): Boolean;
var
  Work, Piece: string;
  P: Integer;
begin
  Result := False;
  Work := Path;
  while Work <> '' do
  begin
    P := Pos(';', Work);
    if P = 0 then
    begin
      Piece := Work;
      Work := '';
    end
    else
    begin
      Piece := Copy(Work, 1, P - 1);
      Work := Copy(Work, P + 1, Length(Work) - P);
    end;
    if CompareText(NormDir(Piece), Dir) = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

function RemoveDirFromPath(const Path, Dir: string): string;
var
  Work, Piece: string;
  P: Integer;
begin
  Work := Path;
  Result := '';
  while Work <> '' do
  begin
    P := Pos(';', Work);
    if P = 0 then
    begin
      Piece := Work;
      Work := '';
    end
    else
    begin
      Piece := Copy(Work, 1, P - 1);
      Work := Copy(Work, P + 1, Length(Work) - P);
    end;
    if CompareText(NormDir(Piece), Dir) <> 0 then
    begin
      if Result <> '' then
        Result := Result + ';';
      Result := Result + Piece;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Orig, NewPath: string;
begin
  if CurStep <> ssPostInstall then
    Exit;
  if not WizardIsTaskSelected('envpath') then
    Exit;
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', Orig) then
    Orig := '';
  if DirInPath(Orig, BinDir) then
    Exit;
  if Orig = '' then
    NewPath := BinDir
  else if Orig[Length(Orig)] = ';' then
    NewPath := Orig + BinDir
  else
    NewPath := Orig + ';' + BinDir;
  RegWriteExpandStringValue(HKCU, 'Environment', 'Path', NewPath);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Orig, NewPath: string;
begin
  if CurUninstallStep <> usPostUninstall then
    Exit;
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', Orig) then
    Exit;
  if not DirInPath(Orig, BinDir) then
    Exit;
  NewPath := RemoveDirFromPath(Orig, BinDir);
  if NewPath = '' then
    RegDeleteValue(HKCU, 'Environment', 'Path')
  else
    RegWriteExpandStringValue(HKCU, 'Environment', 'Path', NewPath);
end;
