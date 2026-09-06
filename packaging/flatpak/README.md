# Flathub submission draft for Vista

This directory contains a Flathub-ready manifest. Vista is CLI-only, so
Flathub review may ask for justification; alternative is publishing to
Fedora COPR + native repos (recommended for a package manager needing host
package-manager access).

## Option A (recommended): COPR / native DNF repo

```bash
# local test repo (this repo):
./packaging/repo/build-repo.sh
sudo ./packaging/repo/add-repo.sh
sudo dnf install -y vista
```

For public hosting:
1. Create COPR at https://copr.fedorainfracloud.org/ -> `whyfle/vista`
2. Upload `packaging/rpm/vista.spec` + source tarball
3. COPR builds for Fedora/RHEL/CentOS, generates `.repo` automatically
4. Users: `sudo dnf copr enable whyfle/vista && sudo dnf install vista`

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

Ship DNF/COPR as primary. Keep Flatpak manifest for users who explicitly
want sandboxed `vista search/info` but document that `vista install` of
RPM/DEB from inside Flatpak cannot manage host packages.
