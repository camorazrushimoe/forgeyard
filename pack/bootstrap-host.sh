#!/bin/sh
# One-time layout on the SSH workbench. Run ON the host (or: ssh user@host < this).
# Does not need the GitHub PAT. Watch never SSHes.
set -eu
PROJECT="${1:?usage: bootstrap-host.sh <project> [git-url]}"
URL="${2:-}"
PATH_REPO="/srv/forgeyard/${PROJECT}/repo"
echo "forgeyard host bootstrap → $PATH_REPO"
sudo mkdir -p "$PATH_REPO"
sudo chown "$(whoami)" "$PATH_REPO" || true
if [ -n "$URL" ] && [ ! -d "$PATH_REPO/.git" ]; then
  git clone "$URL" "$PATH_REPO"
fi
echo "ok $PATH_REPO"
