#!/bin/bash
# Build vista .deb without dpkg-deb (uses ar + tar, available everywhere)
set -e
cd "$(dirname "$0")/../.."
ROOT="$PWD"
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
REVISION="${REVISION:-1}"
ARCH="${ARCH:-amd64}"
OUTDIR="$ROOT/packaging/deb"
WORK="/tmp/vista-deb-build"

echo "==> Building release binary..."
cargo build --release --locked

echo "==> Staging .deb (vista ${VERSION}-${REVISION} ${ARCH})..."
rm -rf "$WORK"
mkdir -p "$WORK/DEBIAN" "$WORK/usr/bin" "$WORK/usr/share/doc/vista" "$WORK/etc/apt/sources.list.d"
cat > "$WORK/DEBIAN/control" <<EOF
Package: vista
Version: ${VERSION}-${REVISION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: Vista Contributors <vista@local>
Homepage: https://github.com/whyfle/vista
Recommends: flatpak
Description: GitHub-backed universal Linux package manager
 Tell Vista what you want. Vista figures out how your Linux
 system should install it. GitHub Releases as primary backend
 with native DEB/RPM scoring and Flatpak/Flathub fallback.
EOF
cp target/release/vista "$WORK/usr/bin/vista"
chmod 0755 "$WORK/usr/bin/vista"
cp README.md LICENSE "$WORK/usr/share/doc/vista/" 2>/dev/null || true
cp packaging/config-example.toml "$WORK/usr/share/doc/vista/" 2>/dev/null || true
cat > "$WORK/usr/share/doc/vista/changelog" <<EOF
vista (${VERSION}-${REVISION}) stable; urgency=medium
  * Upstream release ${VERSION}.
 -- Vista Contributors <vista@local>  $(date -R)
EOF
gzip -9 -n -f "$WORK/usr/share/doc/vista/changelog"
cat > "$WORK/etc/apt/sources.list.d/vista.sources" <<EOF
# Vista local APT repo (serve packaging/deb-repo/ over HTTP and point Dir here)
# Types: deb
# URIs: http://localhost:8000/
# Suites: stable
# Components: main
# Signed-By: /usr/share/keyrings/vista-archive-keyring.gpg
EOF
# md5sums for data files (dpkg convention, relative to package root)
(cd "$WORK" && find usr etc -type f -exec md5sum {} + | sed 's| \./| |' > DEBIAN/md5sums)

echo "==> Packing .deb..."
mkdir -p "$OUTDIR"
echo "2.0" > "$WORK/debian-binary"
tar -czf "$WORK/control.tar.gz" -C "$WORK/DEBIAN" control md5sums
tar -czf "$WORK/data.tar.gz" -C "$WORK" usr etc
DEB="$OUTDIR/vista_${VERSION}-${REVISION}_${ARCH}.deb"
ar r "$DEB" "$WORK/debian-binary" "$WORK/control.tar.gz" "$WORK/data.tar.gz"
echo "==> Built: $DEB"
ls -lh "$DEB"
ar t "$DEB"
echo "Contents:"
tar -tzf "$WORK/data.tar.gz" | head -n 20
