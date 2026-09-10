#!/bin/sh
# Forgeyard installer. Target: darwin-arm64 (also copies any local bins).
# Dry-run / tests:
#   FORGEYARD_HOME=/tmp/fy FORGEYARD_PREFIX=/tmp/fy-bin FORGEYARD_BIN_SRC=./fake-bins SKIP_PI=1 sh install.sh
set -eu

REPO="${FORGEYARD_REPO:-camorazrushimoe/forgeyard}"
HOME_DIR="${FORGEYARD_HOME:-$HOME/.forgeyard}"
PREFIX="${FORGEYARD_PREFIX:-$HOME/.local/bin}"
BIN_SRC="${FORGEYARD_BIN_SRC:-}"
PACK_SRC="${FORGEYARD_PACK_SRC:-}"
ARCH="${FORGEYARD_ARCH:-}"
SKIP_PI="${SKIP_PI:-0}"
PI_DOCS="https://pi.dev/docs/latest/quickstart"

if [ -z "$ARCH" ]; then
  u=$(uname -s | tr 'A-Z' 'a-z')
  m=$(uname -m)
  case "$u-$m" in
    darwin-arm64|darwin-aarch64) ARCH=darwin-arm64 ;;
    linux-x86_64|linux-amd64) ARCH=linux-amd64 ;;
    linux-aarch64|linux-arm64) ARCH=linux-arm64 ;;
    *) ARCH="$u-$m" ;;
  esac
fi

echo "forgeyard install → $HOME_DIR ($ARCH)"

mkdir -p "$HOME_DIR/bin" "$HOME_DIR/pack" "$HOME_DIR/factory/tokens" "$HOME_DIR/factory/projects" "$PREFIX"

copy_bins() {
  src="$1"
  for b in fy forge yard watch; do
    if [ -f "$src/$b" ]; then
      cp "$src/$b" "$HOME_DIR/bin/$b"
      chmod +x "$HOME_DIR/bin/$b"
    fi
  done
}

if [ -n "$BIN_SRC" ]; then
  copy_bins "$BIN_SRC"
elif [ -d "$(dirname "$0")/dist/$ARCH" ]; then
  copy_bins "$(cd "$(dirname "$0")" && pwd)/dist/$ARCH"
elif [ -d "./dist/$ARCH" ]; then
  copy_bins "./dist/$ARCH"
else
  url="https://github.com/$REPO/releases/latest/download/forgeyard-$ARCH.tar.gz"
  tmp=$(mktemp -d)
  if command -v curl >/dev/null 2>&1 && curl -fsSL "$url" -o "$tmp/pack.tgz" 2>/dev/null; then
    tar -xzf "$tmp/pack.tgz" -C "$tmp"
    if [ -d "$tmp/bin" ]; then copy_bins "$tmp/bin"; else copy_bins "$tmp"; fi
  else
    echo "note: no prebuilt $ARCH release yet; bins will be missing until you cargo build --release" >&2
    echo "      cargo build --release && FORGEYARD_BIN_SRC=target/release sh install.sh" >&2
  fi
  rm -rf "$tmp"
fi

if [ -n "$PACK_SRC" ] && [ -d "$PACK_SRC" ]; then
  cp -R "$PACK_SRC"/. "$HOME_DIR/pack/"
elif [ -d "$(dirname "$0")/pack" ]; then
  cp -R "$(cd "$(dirname "$0")" && pwd)/pack"/. "$HOME_DIR/pack/" 2>/dev/null || true
fi

EXAMPLE="$HOME_DIR/pack/tokens.example.toml"
DEST="$HOME_DIR/factory/tokens/tokens.toml"
if [ ! -f "$DEST" ]; then
  if [ -f "$EXAMPLE" ]; then
    cp "$EXAMPLE" "$DEST"
  else
    printf '[telegram]\nbot_token = ""\nallow_user_ids = []\n\n[llm]\nendpoint = ""\ndefault = ""\n\n[github]\npat = ""\n\n[cluster]\nhost = ""\nuser = ""\nport = 22\npassword = ""\n' > "$DEST"
  fi
  chmod 600 "$DEST" 2>/dev/null || true
fi

for b in fy forge yard watch; do
  if [ -x "$HOME_DIR/bin/$b" ]; then
    ln -sf "$HOME_DIR/bin/$b" "$PREFIX/$b"
  fi
done

if [ "$SKIP_PI" != "1" ] && ! command -v pi >/dev/null 2>&1; then
  echo "runner: missing; install pi from $PI_DOCS" >&2
  echo "  npm install -g --ignore-scripts @earendil-works/pi-coding-agent" >&2
  echo "  or: curl -fsSL https://pi.dev/install.sh | sh" >&2
fi

cat <<'EOF'
installed.

next:
  fy onboard
  fy help
EOF
