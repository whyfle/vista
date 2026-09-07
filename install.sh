#!/bin/bash
# Vista installer — picks the native binary for your distro from GitHub Release 1.
#
#   sudo ./install.sh                    # install (asks once)
#   sudo ./install.sh --yes              # install, no prompt
#   curl -fsSL https://raw.githubusercontent.com/whyfle/vista/main/install.sh | sudo bash
#   ./install.sh --dry-run               # show what would happen (no root needed)
#   ./install.sh --download-only /tmp/v  # fetch + verify checksums only
#
# Picks: Fedora/RHEL/openSUSE -> .rpm | Debian/Ubuntu -> .deb | Arch -> .pkg.tar.zst
set -e

REPO="whyfle/vista"
TAG="${VISTA_TAG:-1}"
YES=0
DRY_RUN=0
DOWNLOAD_ONLY=""

while [ $# -gt 0 ]; do
  case "$1" in
    -y|--yes) YES=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --download-only) DOWNLOAD_ONLY="$2"; shift 2 ;;
    --tag) TAG="$2"; shift 2 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1 (try --help)" >&2; exit 2 ;;
  esac
done

BASE="https://github.com/$REPO/releases/download/$TAG"

detect_distro() {
  # echoes: <family> where family is rpm|deb|arch
  local id="" like=""
  if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    id=$(printf '%s' "${ID:-}" | tr '[:upper:]' '[:lower:]')
    like=$(printf '%s' "${ID_LIKE:-}" | tr '[:upper:]' '[:lower:]')
  fi
  case " $id $like " in
    *" arch "*|*" manjaro "*|*" endeavouros "*|*" garuda "*|*" artix "*) echo arch; return ;;
    *" debian "*|*" ubuntu "*|*mint*|*" pop "*|*" zorin "*) echo deb; return ;;
    *" fedora "*|*" rhel "*|*" centos "*|*alma*|*" rocky "*|*" ol "*|*" suse "*|*" opensuse"*) echo rpm; return ;;
  esac
  # Fall back to available package managers
  if command -v pacman >/dev/null 2>&1; then echo arch;
  elif command -v apt-get >/dev/null 2>&1; then echo deb;
  elif command -v dnf >/dev/null 2>&1 || command -v zypper >/dev/null 2>&1; then echo rpm;
  else echo unknown; fi
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo x86_64 ;;
    *) echo "$(uname -m)" ;;
  esac
}

FAMILY=$(detect_distro)
ARCH=$(detect_arch)

if [ "$ARCH" != "x86_64" ]; then
  echo "Error: Release $TAG ships x86_64 binaries only (your arch: $ARCH)." >&2
  echo "Build from source instead: https://github.com/$REPO" >&2
  exit 1
fi

case "$FAMILY" in
  rpm)  ASSET="vista-0.1.0-1.fc44.x86_64.rpm" ;;
  deb)  ASSET="vista_0.1.0-1_amd64.deb" ;;
  arch) ASSET="vista-0.1.0-1-x86_64.pkg.tar.zst" ;;
  *)
    echo "Error: could not detect a supported distro (need dnf/zypper, apt or pacman)." >&2
    exit 1 ;;
esac

if [ "$DRY_RUN" = "1" ]; then
  echo "distro family: $FAMILY ($ARCH)"
  echo "asset: $ASSET"
  case "$FAMILY" in
    rpm)  echo "install: dnf install -y $ASSET  (or zypper install -y $ASSET)" ;;
    deb)  echo "install: apt-get install -y ./$ASSET" ;;
    arch) echo "install: pacman -U --noconfirm $ASSET" ;;
  esac
  echo "source: $BASE/$ASSET (+ checksums.txt)"
  exit 0
fi

WORK="${DOWNLOAD_ONLY:-$(mktemp -d)}"
mkdir -p "$WORK"
trap 'if [ -z "$DOWNLOAD_ONLY" ]; then rm -rf "$WORK"; fi' EXIT

echo "Downloading $ASSET ..."
curl -fsSL --retry 3 -o "$WORK/$ASSET" "$BASE/$ASSET"
curl -fsSL --retry 3 -o "$WORK/checksums.txt" "$BASE/checksums.txt"

echo "Verifying checksum ..."
(cd "$WORK" && sha256sum -c --status <(grep -F "  $ASSET" checksums.txt)) \
  || { echo "Error: checksum mismatch for $ASSET" >&2; exit 1; }
echo "Checksum OK."

if [ -n "$DOWNLOAD_ONLY" ]; then
  echo "Saved to $DOWNLOAD_ONLY/$ASSET"
  exit 0
fi

if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    echo "Re-running with sudo ..."
    exec sudo "$0" ${YES:+--yes} --tag "$TAG"
  fi
  echo "Error: install needs root. Re-run with sudo." >&2
  exit 1
fi

if [ "$YES" != "1" ]; then
  printf "Install vista 0.1.0 (%s, %s) from %s? [Y/n] " "$ASSET" "$FAMILY" "$BASE"
  read -r answer
  case "$answer" in
    ""|[Yy]*) ;;
    *) echo "Aborted."; exit 0 ;;
  esac
fi

case "$FAMILY" in
  rpm)
    if command -v dnf >/dev/null 2>&1; then dnf install -y "$WORK/$ASSET";
    else zypper install -y "$WORK/$ASSET"; fi ;;
  deb) apt-get update && apt-get install -y "$WORK/$ASSET" ;;
  arch) pacman -U --noconfirm "$WORK/$ASSET" ;;
esac

hash -r 2>/dev/null || true
vista --version
echo "Vista installed."
