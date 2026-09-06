#!/bin/bash
# Add Vista local DNF repo (requires sudo)
set -e
REPO_SRC="$(dirname "$0")/vista.repo"
REPO_DST="/etc/yum.repos.d/vista.repo"
echo "Installing Vista repo $REPO_SRC -> $REPO_DST"
sudo cp "$REPO_SRC" "$REPO_DST"
echo "Refreshing metadata..."
sudo dnf makecache --repo=vista || sudo dnf5 makecache --repo=vista || true
echo "Done. Try: sudo dnf install -y vista vista-test-hello"
echo "List: dnf --repo=vista list available"
