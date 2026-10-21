# Gera o instalador do Expressa Aula com Inno Setup 7.
#
# Na raiz do repositorio:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\criar-instalador.ps1
#
# O script faz tudo: compila o release, monta dist\expressa-aula, copia o
# runtime do Visual C++ para junto do exe, regrava o zip e compila
# expressa-aula.iss. O resultado e dist\ExpressaAula-<versao>-Setup.exe.
#
# Exige deps\gtk, deps\protoc, o Inno Setup 7 e o redistribuivel do Visual
# C++ (o Build Tools ou o Visual Studio trazem as DLLs).

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
Set-Location $root

function Find-Iscc7 {
    $candidates = New-Object System.Collections.Generic.List[string]
    $keys = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 7_is1",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 7_is1",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 7_is1"
    )
    foreach ($key in $keys) {
        if (Test-Path $key) {
            $loc = (Get-ItemProperty $key).InstallLocation
            if ($loc) {
                $candidates.Add((Join-Path $loc "ISCC.exe"))
            }
        }
    }
    $candidates.Add((Join-Path $env:ProgramFiles "Inno Setup 7\ISCC.exe"))
    foreach ($path in $candidates) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            return $path
        }
    }
    return $null
}

function Find-VcCrtDir {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path -LiteralPath $vswhere) {
        $hits = & $vswhere -latest -products * -find "VC\Redist\MSVC\*\x64\Microsoft.VC*.CRT\vcruntime140.dll"
        if ($hits) {
            $dll = @($hits) | Select-Object -Last 1
            if ($dll -and (Test-Path -LiteralPath $dll)) {
                return (Split-Path -Parent $dll)
            }
        }
    }
    return $null
}

function Get-AppVersion {
    $cargo = Get-Content -LiteralPath (Join-Path $root "Cargo.toml") -Raw
    if ($cargo -match '(?m)^version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    Write-Error "Nao achei version em Cargo.toml."
}

Write-Output "Montando a pasta portatil..."
& (Join-Path $root "empacotar.ps1")
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$dest = Join-Path $root "dist\expressa-aula"
if (-not (Test-Path -LiteralPath (Join-Path $dest "expressa-aula.exe"))) {
    Write-Error "empacotar.ps1 nao deixou $dest\expressa-aula.exe."
}

$crt = Find-VcCrtDir
if (-not $crt) {
    Write-Error @"
Nao achei vcruntime140.dll no redistribuivel do Visual C++.
Instale o Build Tools ou o Visual Studio com o componente 'MSVC v143' e rode de novo.
Sem essas DLLs ao lado do exe, um Windows limpo abre o editor e fecha com um erro de DLL.
"@
}
Write-Output "Copiando o runtime Visual C++ de $crt"
Copy-Item -Path (Join-Path $crt "*.dll") -Destination $dest -Force

$zip = Join-Path $root "dist\expressa-aula-windows.zip"
if (Test-Path -LiteralPath $zip) {
    Remove-Item -LiteralPath $zip -Force
}
tar -a -c -f $zip -C (Join-Path $root "dist") "expressa-aula"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$iscc = Find-Iscc7
if (-not $iscc) {
    Write-Error "Nao achei o ISCC do Inno Setup 7. Instale-o de https://jrsoftware.org/isdl.php e rode de novo."
}
$banner = & $iscc --help 2>&1 | Select-Object -First 1
if ("$banner" -notmatch "Inno Setup 7") {
    Write-Error "O compilador em $iscc nao e o Inno Setup 7 (resposta: $banner)."
}

$version = Get-AppVersion
Write-Output "Compilando o instalador $version com $iscc"
& $iscc "--define=AppVersion=$version" (Join-Path $root "expressa-aula.iss")
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$setup = Join-Path $root "dist\ExpressaAula-$version-Setup.exe"
if (-not (Test-Path -LiteralPath $setup)) {
    Write-Error "O ISCC terminou sem deixar $setup."
}
$item = Get-Item -LiteralPath $setup
Write-Output ("Instalador pronto: {0} ({1:N0} MB)" -f $item.FullName, ($item.Length / 1MB))
Write-Output "Quem instalar nao precisa de PATH, de deps\ nem de permissao de administrador para abrir o editor."
Write-Output "O terminal fica em bin\: expressa.exe e expressa-grafico.exe entram no PATH do usuario."
