# Flathub submission draft for Vista

This directory contains a Flathub-ready manifest. Vista is CLI-only, so
Flathub review may ask for justification; the recommended distribution is
native Release 1 binaries (recommended for a package manager needing host
package-manager access).

## Option A (recommended): native binaries from Release 1

`install.sh` (repo root) detects the distro and installs the matching native
package (.rpm / .deb / .pkg.tar.zst) from
[Release 1](https://github.com/whyfle/vista/releases/tag/1), verified against
`checksums.txt`:

```bash
curl -fsSL https://raw.githubusercontent.com/whyfle/vista/main/install.sh | sudo bash
```

Local test repo alternative (this repo):

```bash
./packaging/repo/build-repo.sh
sudo ./packaging/repo/add-repo.sh
sudo dnf install -y vista
```

## Option B: Flathub

Manifest: `io.github.whyfle.Vista.yml`

Flathub requires:
- App ID reverse-DNS you own (`io.github.whyfle.Vista` assumes github.com/whyfle)
- Icon + metainfo (provided)
- No broad `--filesystem=host` unless justified. Vista needs it to invoke
  `dnf/flatpak` on host, so Flatpak is a poor fit — prefer native package.
- Submit PR to https://github.com/flathub/flathub with manifest.

Local test build (needs flatpak-builder):
```bash
flatpak-builder --force-clean build-dir packaging/flatpak/io.github.whyfle.Vista.yml
flatpak-builder --run build-dir packaging/flatpak/io.github.whyfle.Vista.yml vista --version
```

## Decision

Ship native Release 1 binaries as primary. Keep Flatpak manifest for users who explicitly
want sandboxed `vista search/info` but document that `vista install` of
RPM/DEB from inside Flatpak cannot manage host packages.
