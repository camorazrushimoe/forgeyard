# Prebuilt binaries

Do not commit large binaries to `main`.

Produce them:

```sh
cargo build --release
mkdir -p dist/$(uname -s | tr A-Z a-z)-$(uname -m)
cp target/release/fy target/release/forge target/release/watch target/release/yard dist/…
```

Or attach `forgeyard-darwin-arm64.tar.gz` to a GitHub Release. `install.sh` looks at:

1. `FORGEYARD_BIN_SRC`
2. `dist/$ARCH` next to the script
3. `https://github.com/camorazrushimoe/forgeyard/releases/latest/download/forgeyard-$ARCH.tar.gz`
