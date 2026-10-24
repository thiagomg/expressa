# Monta dist\expressa-aula\ para copiar para outra maquina.
# Duplo clique em expressa-aula.exe. Nao precisa de PATH nem de deps\.
#
# Na raiz do repositorio:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\empacotar.ps1
#
# O script tambem grava dist\expressa-aula-windows.zip.
# Compilar exige deps\gtk e deps\protoc (veja deps\ativar.ps1). Rodar o
# pacote nao exige.
#
# Para o instalador (Inno Setup 7), use criar-instalador.ps1. Ele chama
# este script e gera dist\ExpressaAula-<versao>-Setup.exe.

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
Set-Location $root

$gtk = Join-Path $root "deps\gtk"
$release = Join-Path $root "target\release"
$dest = Join-Path $root "dist\expressa-aula"
$zip = Join-Path $root "dist\expressa-aula-windows.zip"

if (-not (Test-Path (Join-Path $gtk "bin\gtk-4-1.dll"))) {
    Write-Error "Nao achei $gtk\bin\gtk-4-1.dll."
}

Write-Output "Compilando expressa, expressa-aula e expressa-aula-server (release)..."
cargo build --release -p expressa -p expressa-aula -p expressa-aula-server
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

foreach ($name in @("expressa.exe", "expressa-grafico.exe", "expressa-aula.exe", "expressa-aula-server.exe")) {
    if (-not (Test-Path (Join-Path $release $name))) {
        Write-Error "Faltou $release\$name."
    }
}

Get-CimInstance Win32_Process | Where-Object {
    $_.Name -match '^expressa-aula' -and $_.ExecutablePath -and $_.ExecutablePath.StartsWith($dest)
} | ForEach-Object {
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}

if (Test-Path $dest) {
    Remove-Item $dest -Recurse -Force
}
New-Item -ItemType Directory -Path $dest | Out-Null

function Copy-Tree($src, $dst) {
    if (-not (Test-Path $src)) {
        Write-Error "Nao achei $src."
    }
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    Copy-Item -Path (Join-Path $src "*") -Destination $dst -Recurse -Force
}

Copy-Item (Join-Path $release "expressa-aula.exe") $dest
Copy-Item (Join-Path $release "expressa-aula-server.exe") $dest
# So o interpretador entra no PATH. A pasta do editor tem as DLLs do GTK;
# coloca-la no PATH faria outros programas acharem zlib, libpng etc.
$bin = Join-Path $dest "bin"
New-Item -ItemType Directory -Path $bin | Out-Null
# Os dois ficam lado a lado: o expressa procura expressa-grafico.exe na mesma pasta.
foreach ($name in @("expressa.exe", "expressa-grafico.exe")) {
    Copy-Item (Join-Path $release $name) $bin
}

$helpers = @("gdbus.exe", "gspawn-win64-helper.exe", "gspawn-win64-helper-console.exe")
Get-ChildItem (Join-Path $gtk "bin") -File | Where-Object {
    $_.Extension -eq ".dll" -or $helpers -contains $_.Name
} | Copy-Item -Destination $dest

Copy-Tree (Join-Path $gtk "lib\gdk-pixbuf-2.0") (Join-Path $dest "lib\gdk-pixbuf-2.0")
Copy-Tree (Join-Path $gtk "share\glib-2.0\schemas") (Join-Path $dest "share\glib-2.0\schemas")
Copy-Tree (Join-Path $gtk "share\icons") (Join-Path $dest "share\icons")
Copy-Tree (Join-Path $gtk "share\gtk-4.0") (Join-Path $dest "share\gtk-4.0")
Copy-Tree (Join-Path $gtk "share\gtksourceview-5") (Join-Path $dest "share\gtksourceview-5")
Copy-Tree (Join-Path $gtk "etc\fonts") (Join-Path $dest "etc\fonts")
if (Test-Path (Join-Path $gtk "share\licenses")) {
    Copy-Tree (Join-Path $gtk "share\licenses") (Join-Path $dest "share\licenses")
}
foreach ($loc in @("pt", "pt_BR")) {
    $src = Join-Path $gtk "share\locale\$loc"
    if (Test-Path $src) {
        Copy-Tree $src (Join-Path $dest "share\locale\$loc")
    }
}
Copy-Tree (Join-Path $root "aula\ui\res") (Join-Path $dest "res")
# Modelos somente leitura. O editor (e o instalador no Windows) copia para
# Documentos\Expressa Aula\exemplos os arquivos que ainda nao existem;
# o que o aluno ja editou permanece.
Copy-Tree (Join-Path $root "exemplos") (Join-Path $dest "exemplos")

$bytes = (Get-ChildItem $dest -Recurse -File | Measure-Object Length -Sum).Sum
Write-Output ("Pasta pronta: {0} ({1:N0} MB)" -f $dest, ($bytes / 1MB))

if (Test-Path $zip) {
    Remove-Item $zip -Force
}
tar -a -c -f $zip -C (Join-Path $root "dist") "expressa-aula"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
$zipItem = Get-Item $zip
Write-Output ("Zip: {0} ({1:N0} MB)" -f $zipItem.FullName, ($zipItem.Length / 1MB))
Write-Output "Descompacte e de duplo clique em expressa-aula.exe."
