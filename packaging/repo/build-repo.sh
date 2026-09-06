#!/bin/bash
# Build vista RPM + test RPM + local DNF repo (no sudo needed to build)
set -e
cd "$(dirname "$0")/../.."
ROOT="$PWD"
OUTDIR="$ROOT/packaging/repo"
RPMBUILD="$ROOT/target/rpmbuild"

echo "==> Building release binary..."
cargo build --release --locked

echo "==> Preparing rpmbuild tree..."
rm -rf "$RPMBUILD"
mkdir -p "$RPMBUILD"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
# Create source tarball from git or dir
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
echo "Version: $VERSION"
TARDIR="/tmp/vista-$VERSION"
rm -rf "$TARDIR"
mkdir -p "$TARDIR"
cp -a Cargo.toml Cargo.lock src README.md LICENSE packaging "$TARDIR/" 2>/dev/null || true
tar -czf "$RPMBUILD/SOURCES/vista-$VERSION.tar.gz" -C /tmp "vista-$VERSION"
cp packaging/rpm/vista.spec "$RPMBUILD/SPECS/"

echo "==> Building vista RPM (rpmbuild -bb)..."
rpmbuild --define "_topdir $RPMBUILD" -bb "$RPMBUILD/SPECS/vista.spec" || {
  echo "rpmbuild with cargo failed (network sandbox?), falling back to binary RPM..."
  # Fallback: build simple binary RPM without cargo rebuild
  mkdir -p /tmp/vista-rpmroot/usr/bin
  cp target/release/vista /tmp/vista-rpmroot/usr/bin/
  mkdir -p /tmp/vista-rpmroot/etc/yum.repos.d
  cp packaging/repo/vista.repo /tmp/vista-rpmroot/etc/yum.repos.d/
  # Use fpm if available? else rpmbuild minimal spec
  cat > /tmp/vista-bin.spec <<SPEC
Name: vista
Version: $VERSION
Release: 1%{?dist}
Summary: Vista universal package manager (binary build)
License: MIT
BuildArch: x86_64
Requires: flatpak
%description
Vista binary package.
%install
mkdir -p %{buildroot}/usr/bin
cp $ROOT/target/release/vista %{buildroot}/usr/bin/vista
mkdir -p %{buildroot}/etc/yum.repos.d
cp $ROOT/packaging/repo/vista.repo %{buildroot}/etc/yum.repos.d/vista.repo
%files
/usr/bin/vista
/etc/yum.repos.d/vista.repo
%changelog
* Sun Sep 06 2026 Vista <vista@local> - $VERSION-1
- Binary build
SPEC
  rpmbuild --define "_topdir $RPMBUILD" -bb /tmp/vista-bin.spec
}

echo "==> Building vista-test-hello RPM..."
rpmbuild --define "_topdir $RPMBUILD" -bb packaging/test-packages/vista-test-hello.spec

echo "==> Copying RPMs to repo dir..."
mkdir -p "$OUTDIR"
find "$RPMBUILD/RPMS" -name "*.rpm" -exec cp -v {} "$OUTDIR/" \;

echo "==> Creating repodata with createrepo_c..."
export LD_LIBRARY_PATH=/tmp/vista-createrepo/usr/lib64:$LD_LIBRARY_PATH
/tmp/vista-createrepo/usr/bin/createrepo_c --update "$OUTDIR" || /tmp/vista-createrepo/usr/bin/createrepo_c "$OUTDIR"

echo "==> Repo contents:"
ls -lh "$OUTDIR"
echo "==> Repodata:"
ls -lh "$OUTDIR/repodata"

echo "Done. To use (needs sudo): sudo ./packaging/repo/add-repo.sh"
echo "Or query without sudo: dnf --repofrompath=vista,$OUTDIR --repo=vista list available"
