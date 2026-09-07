# Vista — Universal Linux Package Manager

> Tell Vista what you want. Vista figures out how your Linux system should install it.

```bash
vista add user@repo
vista install firefox
vista install fastfetch-cli@fastfetch --dry-run --yes
```

Priority: **native package → Flatpak/Flathub fallback → clear error**. Never installs incompatible arch/format.

## Status (v0.1.0)

Rust, single binary (~3.7 MB release). 11 tests pass (7 unit + 4 resolver integration).
Verified on Fedora 44 x86_64: `fastfetch-linux-amd64.rpm` scores 195 (native 100 + arch 50 + stable 25 + github 20), `discord` Flathub search returns `com.discordapp.Discord`.

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

## Layout

`src/{cli,resolver,github,flathub,distro,packages,installers,security,database,cache,config,downloader}` + `packaging/{rpm,repo,flatpak,test-packages}` + `tests/`.
