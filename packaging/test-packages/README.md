# Vista test packages

Two layers:

## 1. Resolver fixtures (offline, no install)

In `tests/fixtures/`:
- `github-release-mixed.json` — v2.4.1 with `x86_64.rpm`, `aarch64.rpm`, `amd64.deb`, `AppImage`, `checksums.txt`
  - Fedora x86_64 must pick `x86_64.rpm` (native +100, arch +50, stable +25, github +20)
  - Debian must pick `amd64.deb`, reject `.rpm`
  - `checksums.txt` must score -1000, incompatible
- `github-release-no-native.json` — only macOS/Windows assets, must trigger Flathub fallback
- `flathub-search.json` — mock Flathub hits

Run: `cargo test`

## 2. Real DNF test package (installable)

`vista-test-hello-1.0.0` — provides `/usr/bin/vista-hello`, built from
`packaging/test-packages/vista-test-hello.spec`.

```bash
./packaging/repo/build-repo.sh
dnf --repofrompath=vista,./packaging/repo --repo=vista list available
sudo dnf install -y vista-test-hello
vista-hello  # -> vista-test-hello 1.0.0 - ok
```

Also used to test `vista list/info/remove` DB recording without touching GitHub.
