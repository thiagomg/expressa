# Monta dist\expressa-aula-freya\ sem DLLs do GTK.
# Compilar no Windows nativo (MSVC + rust-skia). Nao ha cross a partir do Linux.
#
# Na raiz do repositorio:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\empacotar-freya.ps1
#
# O zip sai em dist\expressa-aula-freya-windows.zip.
# O editor GTK continua em empacotar.ps1 / criar-instalador.ps1.

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
Set-Location $root

$release = Join-Path $root "target\release"
$dest = Join-Path $root "dist\expressa-aula-freya"
$zip = Join-Path $root "dist\expressa-aula-freya-windows.zip"

Write-Output "Compilando expressa, expressa-aula-freya, expressa-aula-server e expressa-tutorial (release)..."
cargo build --release --locked -p expressa -p expressa-aula-freya -p expressa-aula-server -p expressa-tutorial
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

foreach ($name in @("expressa.exe", "expressa-grafico.exe", "expressa-aula-freya.exe", "expressa-aula-server.exe", "expressa-tutorial.exe")) {
    if (-not (Test-Path (Join-Path $release $name))) {
        Write-Error "Faltou $release\$name."
    }
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

Copy-Item (Join-Path $release "expressa-aula-freya.exe") $dest
Copy-Item (Join-Path $release "expressa-aula-server.exe") $dest
Copy-Item (Join-Path $release "expressa-tutorial.exe") $dest
$bin = Join-Path $dest "bin"
New-Item -ItemType Directory -Path $bin | Out-Null
foreach ($name in @("expressa.exe", "expressa-grafico.exe")) {
    Copy-Item (Join-Path $release $name) $bin
}

Copy-Tree (Join-Path $root "exemplos") (Join-Path $dest "exemplos")
Copy-Tree (Join-Path $root "tutorial") (Join-Path $dest "tutorial")
Copy-Item (Join-Path $root "packaging\freya\LEIA-ME.txt") (Join-Path $dest "LEIA-ME.txt")

$bytes = (Get-ChildItem $dest -Recurse -File | Measure-Object Length -Sum).Sum
Write-Output ("Pasta pronta: {0} ({1:N0} MB) — sem GTK" -f $dest, ($bytes / 1MB))

if (Test-Path $zip) {
    Remove-Item $zip -Force
}
tar -a -c -f $zip -C (Join-Path $root "dist") "expressa-aula-freya"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
$zipItem = Get-Item $zip
Write-Output ("Zip: {0} ({1:N0} MB)" -f $zipItem.FullName, ($zipItem.Length / 1MB))
Write-Output "Descompacte e de duplo clique em expressa-aula-freya.exe. Se a GPU falhar, FREYA_RENDERER=software."
