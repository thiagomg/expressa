# O Cargo chama este script para `cargo run` e `cargo test` nesta máquina.
# Coloca deps/gtk/bin no PATH antes do programa, para achar gtk-4-1.dll.
$bin = Join-Path $PSScriptRoot "gtk\bin"
if (Test-Path $bin) {
    $env:PATH = "$bin;$env:PATH"
}
if ($args.Count -lt 1) {
    exit 1
}
$exe = $args[0]
if ($args.Count -gt 1) {
    & $exe @($args[1..($args.Count - 1)])
} else {
    & $exe
}
exit $LASTEXITCODE
