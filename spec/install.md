# Install

Goal: one command on a laptop installs the factory.

```
curl -fsSL https://raw.githubusercontent.com/camorazrushimoe/forgeyard/main/install.sh | sh
```

Until `install.sh` exists, this file is the contract the script must implement.

## What the command installs

Three layers, one prefix (`$FORGEYARD_HOME`, default `~/.forgeyard`):

```
~/.forgeyard/
  bin/forge
  bin/yard
  pack/
    pack.toml
    roles/
    skills/
    AGENTS.md            # default instructions dropped into new checkouts
  factory/               # runtime root (or FORGEYARD_ROOT)
    tokens/tokens.toml   # created from example, mode 0600, empty values
    projects/
```

Also on PATH (symlink into `~/.local/bin` or `/usr/local/bin`):
`forge`, `yard`.

Pi is **not** vendored in the Rust binary. The installer runs the upstream Pi install if `pi` is missing:

```
curl -fsSL https://pi.dev/install.sh | sh
```

or `npm install -g --ignore-scripts @earendil-works/pi-coding-agent`.

If Pi cannot be installed, `forge` and `yard` still install. `forge run` then fails with `runner_missing` until Pi is present. Status command must show `runner: missing`.

## What is inside the binary vs outside

| inside static musl binaries | outside, pulled as files |
|---|---|
| bind, hooks, envelope, log, tokens, status | `roles/*.md` |
| yard Telegram poller | `skills/*/SKILL.md` |
| pack.toml parser | `pack.toml` |
| | `pi` executable |
| | runtime `factory/` |

Roles and skills must stay files. Editing a skill is a git commit to this repo, not a rebuild of `forge`.

## Installer steps (normative)

1. Detect os/arch. v0: `linux-x86_64` and `darwin-arm64`.
2. Download `forge` and `yard` from GitHub Releases of this repo (when they exist). Until then, installer may build from source if Rust is present, or print `binaries not released yet`.
3. Copy `pack/` from the same git ref as the binaries.
4. Create `factory/tokens/tokens.toml` from the example if missing. Never overwrite an existing tokens file.
5. Ensure `pi` on PATH or install it.
6. Print next steps:

```
forge bind https://github.com/owner/repo
# fill tokens: forge tokens set --name llm.default --from-stdin
# optional: yard
```

## Uninstall

```
forge uninstall        # reserved
# or
rm -rf ~/.forgeyard ~/.local/bin/forge ~/.local/bin/yard
```

Does not uninstall Pi (shared tool).

## Version pin

`~/.forgeyard/VERSION` records `forgeyard=<git-sha-or-tag>` and `pi=<pi --version>`.
`forge version` prints both.
