#!/bin/sh
# Dry-run contract for install.sh. Exit 0 if layout + banner match spec.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
bins="$tmp/src"
home="$tmp/home"
pref="$tmp/pref"
mkdir -p "$bins"
for b in fy forge yard watch; do
  printf '#!/bin/sh\necho %s\n' "$b" > "$bins/$b"
  chmod +x "$bins/$b"
done
out=$(FORGEYARD_HOME="$home" FORGEYARD_PREFIX="$pref" FORGEYARD_BIN_SRC="$bins" FORGEYARD_PACK_SRC="$root/pack" SKIP_PI=1 sh "$root/install.sh")
echo "$out" | grep -q '^installed\.$'
echo "$out" | grep -q 'fy onboard'
echo "$out" | grep -q 'fy help'
test -x "$home/bin/fy"
test -x "$home/bin/forge"
test -x "$home/bin/yard"
test -x "$home/bin/watch"
test -L "$pref/fy"
test -f "$home/factory/tokens/tokens.toml"
test -d "$home/factory/projects"
# never overwrite tokens
echo 'keep-me' > "$home/factory/tokens/tokens.toml"
FORGEYARD_HOME="$home" FORGEYARD_PREFIX="$pref" FORGEYARD_BIN_SRC="$bins" SKIP_PI=1 sh "$root/install.sh" >/dev/null
grep -q keep-me "$home/factory/tokens/tokens.toml"
echo "check-install ok"
rm -rf "$tmp"
