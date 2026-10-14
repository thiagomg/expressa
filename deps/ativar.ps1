# Coloca no processo atual o GTK 4, o GtkSourceView 5 e o protoc que estão em deps/.
# Dot-source na raiz do repositório. Não chame com `&`: assim as variáveis somem
# quando o script termina.
#
# O PowerShell vem com execução de script desligada. Neste processo:
#   Set-ExecutionPolicy -Scope Process Bypass
#   . .\deps\ativar.ps1
#
# Nesta máquina o .cargo/config.toml já aponta o Cargo para deps/, então
# `cargo build` e `cargo run` não precisam deste script.
#
# O binário do GTK precisa continuar no PATH enquanto a IDE estiver aberta:
# o Windows carrega gtk-4-1.dll e gtksourceview-5-0.dll de deps/gtk/bin, e o
# GLib acha temas, ícones e schemas a partir dessa pasta.

$deps = $PSScriptRoot
$gtk = Join-Path $deps "gtk"
$protoc = Join-Path $deps "protoc"
$pkgconf = Join-Path $gtk "bin\pkgconf.exe"
$protocExe = Join-Path $protoc "bin\protoc.exe"

if (-not (Test-Path $pkgconf) -or -not (Test-Path $protocExe)) {
    Write-Error "deps incompleto. Esperava $pkgconf e $protocExe."
    return
}

$env:PATH = "$(Join-Path $gtk 'bin');$(Join-Path $protoc 'bin');$env:PATH"
$env:PKG_CONFIG = $pkgconf
$env:PKG_CONFIG_PATH = Join-Path $gtk "lib\pkgconfig"
$env:PROTOC = $protocExe

$gtkLib = Join-Path $gtk "lib"
if ($env:LIB) { $env:LIB = "$gtkLib;$env:LIB" } else { $env:LIB = $gtkLib }
$gtkInc = Join-Path $gtk "include"
if ($env:INCLUDE) { $env:INCLUDE = "$gtkInc;$env:INCLUDE" } else { $env:INCLUDE = $gtkInc }

$gtkVer = & $env:PKG_CONFIG --modversion gtk4
$svVer = & $env:PKG_CONFIG --modversion gtksourceview-5
$protocVer = & $env:PROTOC --version
Write-Host "deps ativo: gtk4 $gtkVer, gtksourceview-5 $svVer, $protocVer"