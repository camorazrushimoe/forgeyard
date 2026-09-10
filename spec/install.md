# Install

Goal: one command on a Mac installs the factory files. Secrets come after, via `fy onboard`.

v0 installer target: `darwin-arm64`. Linux later.

```
curl -fsSL https://raw.githubusercontent.com/camorazrushimoe/forgeyard/main/install.sh | sh
```

`install.sh` is in the repo. It copies bins from `FORGEYARD_BIN_SRC`, `dist/$ARCH`, or the latest GitHub Release tarball.

## What lands on disk

```
~/.forgeyard/
  bin/fy bin/forge bin/yard bin/watch
  pack/
  factory/
    tokens/tokens.toml     # created empty from example, never overwritten
    projects/
```

PATH symlink: `fy` (and the internal bins) under `~/.local/bin` (`FORGEYARD_PREFIX`).
Pi is installed if missing. Failure to install Pi does not fail `fy` itself; `fy start` then shows `runner: missing`.

End of installer, no questions:

```
installed.

next:
  fy onboard
  fy help
```

## Human commands

See [spec/cli.md](cli.md) and [spec/onboard.md](onboard.md).

## Uninstall

```
rm -rf ~/.forgeyard ~/.local/bin/fy ~/.local/bin/forge ~/.local/bin/yard ~/.local/bin/watch
```

Does not uninstall Pi.
