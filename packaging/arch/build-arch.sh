#!/bin/bash
# Build vista Arch package (.pkg.tar.zst) without makepkg (for non-Arch hosts).
# On Arch, prefer: makepkg -s (uses packaging/arch/PKGBUILD).
set -e
cd "$(dirname "$0")/../.."
ROOT="$PWD"
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
REL="${REL:-1}"
ARCH="${ARCH:-x86_64}"
OUTDIR="$ROOT/packaging/arch"
WORK="/tmp/vista-arch-build"

echo "==> Building release binary..."
cargo build --release --locked

echo "==> Staging Arch package (vista ${VERSION}-${REL} ${ARCH})..."
rm -rf "$WORK"
mkdir -p "$WORK/pkg/usr/bin" "$WORK/pkg/usr/share/doc/vista" "$WORK/pkg/usr/share/licenses/vista"
cp target/release/vista "$WORK/pkg/usr/bin/vista"
chmod 0755 "$WORK/pkg/usr/bin/vista"
cp README.md "$WORK/pkg/usr/share/doc/vista/" 2>/dev/null || true
cp packaging/config-example.toml "$WORK/pkg/usr/share/doc/vista/" 2>/dev/null || true
cp LICENSE "$WORK/pkg/usr/share/licenses/vista/" 2>/dev/null || true

SIZE=$(du -sb "$WORK/pkg/usr" | cut -f1)
BUILDDATE=$(date +%s)
cat > "$WORK/.PKGINFO" <<EOF
pkgname = vista
pkgver = ${VERSION}-${REL}
pkgdesc = GitHub-backed universal Linux package manager
url = https://github.com/whyfle/vista
builddate = ${BUILDDATE}
packager = Vista Contributors <vista@local>
size = ${SIZE}
arch = ${ARCH}
license = MIT
depend = gcc-libs
optdepend = flatpak: Flathub fallback backend
optdepend = dnf: RPM installation backend (Fedora/RHEL)
EOF

# Minimal .MTREE (mtree v1 format pacman/libalpm accepts)
pushd "$WORK/pkg" >/dev/null
{
  echo "#mtree"
  echo "/set type=file uid=0 gid=0 time=${BUILDDATE}"
  echo ". type=dir mode=755 time=${BUILDDATE}"
  find usr -type d | sort | while read -r d; do
    echo "./$d type=dir mode=755 time=${BUILDDATE}"
  done
  find usr -type f | sort | while read -r f; do
    sz=$(stat -c%s "$f")
    sum=$(sha256sum "$f" | cut -d' ' -f1)
    if [ -x "$f" ]; then mode=755; else mode=644; fi
    echo "./$f type=file mode=$mode size=$sz sha256digest=$sum time=${BUILDDATE}"
  done
} > "$WORK/.MTREE"
popd >/dev/null

echo "==> Packing .pkg.tar.zst..."
mkdir -p "$OUTDIR"
PKG="$OUTDIR/vista-${VERSION}-${REL}-${ARCH}.pkg.tar.zst"
tar --owner=0 --group=0 --mtime="@${BUILDDATE}" \
  -I 'zstd -19 -T0 -q' -cf "$PKG" -C "$WORK" .PKGINFO .MTREE -C "$WORK/pkg" usr
echo "==> Built: $PKG"
ls -lh "$PKG"
echo "Contents:"
tar -I zstd -tf "$PKG"
