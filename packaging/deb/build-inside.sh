#!/bin/bash
# Runs inside the Ubuntu build container. Stages a Debian tree and writes
# the .deb to /out (the host dist/ directory).
set -euo pipefail

SRC="${SRC:-/src}"
OUT="${OUT:-/out}"
TARGET="${CARGO_TARGET_DIR:-/build/target}"

if [[ ! -f "$SRC/Cargo.toml" ]]; then
    echo "erro: não achei $SRC/Cargo.toml — monte o repositório em /src" >&2
    exit 1
fi

# shellcheck source=/dev/null
. /etc/os-release
UBUNTU_ID="${VERSION_ID:?}"
ARCH="$(dpkg --print-architecture)"
CARGO_VER="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$SRC/Cargo.toml" | head -n1)"
if [[ -z "$CARGO_VER" ]]; then
    echo "erro: não achei version em Cargo.toml" >&2
    exit 1
fi
DEB_VER="${CARGO_VER}-1~ubuntu${UBUNTU_ID}"
PKG_NAME="expressa"
DEB_FILE="${PKG_NAME}_${DEB_VER}_${ARCH}.deb"

echo "Ubuntu ${UBUNTU_ID} (${ARCH}), Expressa ${CARGO_VER} → ${DEB_FILE}"

mkdir -p "$TARGET" "$OUT"
export CARGO_TARGET_DIR="$TARGET"
export CARGO_HOME="${CARGO_HOME:-/usr/local/cargo}"

cd "$SRC"
echo "Compilando release…"
cargo build --release --locked \
    -p expressa \
    -p expressa-aula \
    -p expressa-aula-server \
    -p expressa-tutorial

BIN="$TARGET/release"
for name in expressa expressa-grafico expressa-aula expressa-aula-server expressa-tutorial; do
    if [[ ! -x "$BIN/$name" ]]; then
        echo "erro: faltou $BIN/$name" >&2
        exit 1
    fi
done

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
ROOT="$STAGE/root"
mkdir -p \
    "$ROOT/usr/bin" \
    "$ROOT/usr/share/expressa" \
    "$ROOT/usr/share/expressa-aula/language-specs" \
    "$ROOT/usr/share/expressa-aula/styles" \
    "$ROOT/usr/share/gtksourceview-5/language-specs" \
    "$ROOT/usr/share/gtksourceview-5/styles" \
    "$ROOT/usr/share/applications" \
    "$ROOT/usr/share/mime/packages" \
    "$ROOT/usr/share/icons/hicolor/256x256/apps" \
    "$ROOT/usr/share/pixmaps" \
    "$ROOT/usr/share/doc/$PKG_NAME"

install -m0755 "$BIN/expressa" "$ROOT/usr/bin/expressa"
install -m0755 "$BIN/expressa-grafico" "$ROOT/usr/bin/expressa-grafico"
install -m0755 "$BIN/expressa-aula" "$ROOT/usr/bin/expressa-aula"
install -m0755 "$BIN/expressa-aula-server" "$ROOT/usr/bin/expressa-aula-server"
install -m0755 "$BIN/expressa-tutorial" "$ROOT/usr/bin/expressa-tutorial"
strip --strip-unneeded \
    "$ROOT/usr/bin/expressa" \
    "$ROOT/usr/bin/expressa-grafico" \
    "$ROOT/usr/bin/expressa-aula" \
    "$ROOT/usr/bin/expressa-aula-server" \
    "$ROOT/usr/bin/expressa-tutorial"

cp -a "$SRC/exemplos" "$ROOT/usr/share/expressa/exemplos"
cp -a "$SRC/tutorial" "$ROOT/usr/share/expressa/tutorial"
find "$ROOT/usr/share/expressa/exemplos" -type d -name saida -prune -exec rm -rf {} +
find "$ROOT/usr/share/expressa/exemplos" -name .expressa-grava -delete

install -m0644 "$SRC/aula/ui/res/language-specs/expressa.lang" \
    "$ROOT/usr/share/expressa-aula/language-specs/expressa.lang"
install -m0644 "$SRC/aula/ui/res/language-specs/expressa.lang" \
    "$ROOT/usr/share/gtksourceview-5/language-specs/expressa.lang"
install -m0644 "$SRC/aula/ui/res/styles/expressa-aula.xml" \
    "$ROOT/usr/share/expressa-aula/styles/expressa-aula.xml"
install -m0644 "$SRC/aula/ui/res/styles/expressa-aula-dark.xml" \
    "$ROOT/usr/share/expressa-aula/styles/expressa-aula-dark.xml"
install -m0644 "$SRC/aula/ui/res/styles/expressa-aula.xml" \
    "$ROOT/usr/share/gtksourceview-5/styles/expressa-aula.xml"
install -m0644 "$SRC/aula/ui/res/styles/expressa-aula-dark.xml" \
    "$ROOT/usr/share/gtksourceview-5/styles/expressa-aula-dark.xml"

install -m0644 "$SRC/packaging/deb/expressa-aula.desktop" \
    "$ROOT/usr/share/applications/expressa-aula.desktop"
install -m0644 "$SRC/packaging/deb/expressa.desktop" \
    "$ROOT/usr/share/applications/expressa.desktop"
install -m0644 "$SRC/packaging/deb/expressa-tutorial.desktop" \
    "$ROOT/usr/share/applications/expressa-tutorial.desktop"
install -m0644 "$SRC/packaging/deb/expressa.xml" \
    "$ROOT/usr/share/mime/packages/expressa.xml"
install -m0644 "$SRC/res/expressa-2.png" \
    "$ROOT/usr/share/icons/hicolor/256x256/apps/expressa.png"
install -m0644 "$SRC/res/expressa-2.png" \
    "$ROOT/usr/share/pixmaps/expressa.png"

install -m0644 "$SRC/README.md" "$ROOT/usr/share/doc/$PKG_NAME/README.md"
install -m0644 "$SRC/packaging/deb/copyright" "$ROOT/usr/share/doc/$PKG_NAME/copyright"

{
    echo "${PKG_NAME} (${DEB_VER}) unstable; urgency=medium"
    echo
    echo "  * Pacote gerado a partir da árvore fonte (Ubuntu ${UBUNTU_ID})."
    echo
    echo " -- Thiago Massari Guedes <expressa@localhost>  $(date -R)"
} | gzip -9n > "$ROOT/usr/share/doc/$PKG_NAME/changelog.Debian.gz"

# Shared-library Depends from the four binaries.
WORK="$(mktemp -d)"
mkdir -p "$WORK/debian"
cat > "$WORK/debian/control" <<EOF
Source: ${PKG_NAME}
Section: devel
Priority: optional
Maintainer: Thiago Massari Guedes <expressa@localhost>
Standards-Version: 4.6.2

Package: ${PKG_NAME}
Architecture: ${ARCH}
Depends: \${shlibs:Depends}
Description: linguagem Expressa e editor Aula
EOF
(
    cd "$WORK"
    dpkg-shlibdeps -Tdebian/substvars \
        "$ROOT/usr/bin/expressa" \
        "$ROOT/usr/bin/expressa-grafico" \
        "$ROOT/usr/bin/expressa-aula" \
        "$ROOT/usr/bin/expressa-aula-server" \
        "$ROOT/usr/bin/expressa-tutorial"
)
SHLIBS="$(sed -n 's/^shlibs:Depends=//p' "$WORK/debian/substvars")"
rm -rf "$WORK"

SIZE="$(du -sk "$ROOT" | awk '{print $1}')"

mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${DEB_VER}
Section: devel
Priority: optional
Architecture: ${ARCH}
Maintainer: Thiago Massari Guedes <expressa@localhost>
Installed-Size: ${SIZE}
Depends: ${SHLIBS}
Description: linguagem Expressa e editor Aula
 Expressa é uma linguagem de programação em português brasileiro.
 Este pacote instala o interpretador (expressa), o visualizador de
 gráficos (expressa-grafico), o editor GTK Expressa Aula, os tutoriais
 (expressa-tutorial) e o servidor
 de arquivos da sala.
EOF

cat > "$ROOT/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications || true
    fi
    if command -v update-mime-database >/dev/null 2>&1; then
        update-mime-database /usr/share/mime || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q /usr/share/icons/hicolor || true
    fi
fi
exit 0
EOF
cat > "$ROOT/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = remove ] || [ "$1" = purge ]; then
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications || true
    fi
    if command -v update-mime-database >/dev/null 2>&1; then
        update-mime-database /usr/share/mime || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q /usr/share/icons/hicolor || true
    fi
fi
exit 0
EOF
chmod 0755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"

echo "Montando ${DEB_FILE}…"
dpkg-deb --root-owner-group -Zxz --build "$ROOT" "$OUT/$DEB_FILE"
if [[ -n "${HOST_UID:-}" && -n "${HOST_GID:-}" ]]; then
    chown "${HOST_UID}:${HOST_GID}" "$OUT/$DEB_FILE"
fi
echo "pronto: dist/${DEB_FILE}"
dpkg-deb --info "$OUT/$DEB_FILE"
