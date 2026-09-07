# Vista — Universal Linux Package Manager

> Tell Vista what you want. Vista figures out how your Linux system should install it.

## Quick start

```bash
cargo build --release
./target/release/vista sys-info
./target/release/vista search discord
./target/release/vista install fastfetch-cli@fastfetch --dry-run --yes
```

Auth (raises GitHub limit 60 → 5000/hr, never commit the token):

```bash
export GITHUB_TOKEN="ghp_..."
export VISTA_GITHUB_TOKEN="ghp_..."  # alternative
vista search foo
```

Config: `~/.config/vista/config.toml` (see `packaging/config-example.toml`).

## Install (native binaries, Release 1)

`install.sh` detects your distro and installs the matching native package
(.rpm / .deb / .pkg.tar.zst) from
[Release 1](https://github.com/whyfle/vista/releases/tag/1), verified against
`checksums.txt`:

```bash
curl -fsSL https://raw.githubusercontent.com/whyfle/vista/main/install.sh | sudo bash
# or from a clone:
sudo ./install.sh
sudo ./install.sh --yes   # no prompt
./install.sh --dry-run    # preview only, no root needed
```

## DNF repo (local, tested)

Built: `vista-0.1.0-1.fc44.x86_64.rpm` + `vista-test-hello-1.0.0-1.fc44.noarch.rpm` with `repodata/` via `createrepo_c`.

```bash
./packaging/repo/build-repo.sh
dnf --repofrompath=vista,$PWD/packaging/repo repoquery --repo=vista
# needs sudo:
sudo ./packaging/repo/add-repo.sh
sudo dnf install -y vista vista-test-hello
```

## Flathub

Manifest: `packaging/flatpak/io.github.whyfle.Vista.yml` + metainfo. Note: Vista needs host `dnf/flatpak` access, so Flatpak is a poor fit for `install`; native Release 1 binaries (see Install above) are primary. See `packaging/flatpak/README.md` for submission steps. Flathub search API fixed to `POST /api/v2/search`.

## Test packages

- Offline fixtures: `tests/fixtures/github-release-mixed.json` (rpm/deb/AppImage/checksums), `github-release-no-native.json` (fallback case), `flathub-search.json`.
- Real RPM: `vista-test-hello` (`/usr/bin/vista-hello`). See `packaging/test-packages/README.md`.
- Run: `cargo test`

## CLI

`install` (alias `add`), `remove`, `update`, `upgrade`, `search`, `info`, `list`, `clean`, `sys-info`. Flags: `--github user@repo`, `--repo`, `--default native/flathub`, `--flatpak`, `-y/--yes`, `--dry-run`.

## Behavior notes

- **Root escalation:** `vista install` runs the native package manager with
  `sudo` automatically when you are not root (`dnf`/`apt-get`/`pacman`/
  `zypper`/`apk`; flatpak handles auth itself). A `Re-running with sudo:`
  line means it escalated. Set `VISTA_NO_SUDO=1` to
  disable, or run `sudo vista install <package>` yourself.
- **Selection policy:** Vista only auto-installs what it can install
  (native packages, AppImage, Flatpak). If the best GitHub asset is a
  tarball/zip, Vista checks Flathub instead — `vista install discord`
  resolves to `com.discordapp.Discord`, and an explicit `user@repo` only
  diverts to Flathub on a real name match (never fuzzy junk like Wesnoth
  for `bat`). With no usable build anywhere you get the download path plus
  a manual-install note, never a blind install.
- **Confirmation:** every install asks once (`Install this package? [Y/n]`);
  `-y`/`--yes` (or `auto_confirm` in config) skips it.

## Layout

`src/{cli,resolver,github,flathub,distro,packages,installers,security,database,cache,config,downloader}` + `packaging/{rpm,repo,flatpak,test-packages}` + `tests/`.
