#!/bin/sh
# Gera .deb da Expressa (interpretador, gráficos e Aula) com Docker.
#
# Na raiz do repositório:
#   ./empacotar-deb.sh              Ubuntu 24.04 e 26.04
#   ./empacotar-deb.sh 24.04        só uma versão
#   ./empacotar-deb.sh 24.04 26.04  as duas, nesta ordem
#
# Os pacotes saem em dist/expressa_<versão>-1~ubuntu<YY.MM>_<arch>.deb
# Cada Ubuntu usa um volume de target próprio (glibc/GTK diferentes).
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
cd "$ROOT"

if ! command -v docker >/dev/null 2>&1; then
    echo "erro: precisa do Docker no PATH" >&2
    exit 1
fi

if [ "$#" -eq 0 ]; then
    set -- 24.04 26.04
fi

for ver in "$@"; do
    case "$ver" in
        24.04|26.04) ;;
        *)
            echo "erro: Ubuntu '$ver' não suportado (use 24.04 ou 26.04)" >&2
            exit 1
            ;;
    esac
done

mkdir -p "$ROOT/dist"
IMAGE_BASE="expressa-deb"

for ver in "$@"; do
    image="${IMAGE_BASE}:${ver}"
    echo "=== imagem Docker Ubuntu ${ver} ==="
    docker build \
        --build-arg "UBUNTU_VERSION=${ver}" \
        -t "$image" \
        -f "$ROOT/packaging/deb/Dockerfile" \
        "$ROOT/packaging/deb"

    echo "=== compilando e empacotando Ubuntu ${ver} ==="
    docker run --rm \
        --network=host \
        -e CARGO_TARGET_DIR=/build/target \
        -e CARGO_HOME=/usr/local/cargo \
        -e "HOST_UID=$(id -u)" \
        -e "HOST_GID=$(id -g)" \
        -v "$ROOT:/src:ro" \
        -v "$ROOT/dist:/out" \
        -v "expressa-cargo-registry:/usr/local/cargo/registry" \
        -v "expressa-cargo-git:/usr/local/cargo/git" \
        -v "expressa-deb-target-${ver}:/build/target" \
        "$image"
done

echo "pacotes em $ROOT/dist:"
ls -l "$ROOT/dist"/expressa_*.deb 2>/dev/null || ls -l "$ROOT/dist"
