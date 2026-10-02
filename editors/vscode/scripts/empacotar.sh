#!/bin/sh
# Gera expressa-<versão>.vsix, a extensão para instalar no VS Code.
#
#   sh scripts/empacotar.sh             só gera o .vsix
#   sh scripts/empacotar.sh --instalar  gera e instala (comando `code`)
#
# Precisa de Node.js 16 ou mais novo (npm baixa o @vscode/vsce na 1ª vez).
set -eu

cd "$(dirname "$0")/.."

if ! command -v node >/dev/null 2>&1; then
    echo "erro: precisa do Node.js (https://nodejs.org)" >&2
    exit 1
fi

if [ ! -x node_modules/.bin/vsce ]; then
    echo "instalando dependências (npm ci)…"
    npm ci --no-audit --no-fund
fi

# Palavras-chave e funções nativas saem do código da linguagem.
node scripts/gerar.js
node --test test/

versao=$(node -p 'require("./package.json").version')
saida="expressa-$versao.vsix"
node_modules/.bin/vsce package --out "$saida"
echo "pronto: editors/vscode/$saida"

if [ "${1:-}" = "--instalar" ]; then
    code --install-extension "$saida" --force
else
    echo "para instalar: code --install-extension editors/vscode/$saida"
fi
