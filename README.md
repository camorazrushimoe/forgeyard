# Forgeyard

Spec-driven AI software factory. Laptop is the remote. SSH host is the workbench.
Watch is deterministic. Pi is the only LLM process. Three roles: tech-pm, developer, qa.

**Status:** v0 spec + Rust workspace on `main`. Install with the curl line below (or `cargo build --release` + `FORGEYARD_BIN_SRC=target/release sh install.sh` until a GitHub Release exists).

## Test scenario (what you will run)

1. You create a **separate** GitHub repo with `spec.md` in the root and a `Makefile` target `qa` (`make qa`).
2. Do **not** turn on required approving reviews on that repo.
3. Factory is installed and onboarded on a Mac. `fy start` is running.
4. You type `fy do <repo-or-issue-url> implement the spec`.
5. Watch: A review spec → (B if large) → C implement on the SSH host → laptop pushes PR → QA `make qa` on that sha → watch merges.

## Install (Mac, Apple Silicon)

Need on the laptop: `git`, `ssh`, `gh`. Pi is separate (see below). Missing Pi does not fail `fy` install; `forge run` then exits `runner_missing`.

```sh
curl -fsSL https://raw.githubusercontent.com/camorazrushimoe/forgeyard/main/install.sh | sh
```

What lands:

```text
~/.forgeyard/bin/{fy,forge,yard,watch}
~/.forgeyard/pack/
~/.forgeyard/factory/tokens/tokens.toml   # from example, never overwritten
~/.forgeyard/factory/projects/
```

`fy` is symlinked into `~/.local/bin` (override with `FORGEYARD_PREFIX`).

Until a [GitHub Release](https://github.com/camorazrushimoe/forgeyard/releases) publishes `forgeyard-darwin-arm64.tar.gz`, build locally:

```sh
git clone https://github.com/camorazrushimoe/forgeyard
cd forgeyard
cargo build --release
FORGEYARD_BIN_SRC=target/release sh install.sh
```

```text
installed.

next:
  fy onboard
  fy help
```

Uninstall:

```sh
rm -rf ~/.forgeyard ~/.local/bin/fy ~/.local/bin/forge ~/.local/bin/yard ~/.local/bin/watch
```

Does not uninstall Pi.

## Install Pi (required for `forge run`)

Official docs: https://pi.dev/docs/latest/quickstart

```sh
npm install -g --ignore-scripts @earendil-works/pi-coding-agent
```

or

```sh
curl -fsSL https://pi.dev/install.sh | sh
```

Check: `command -v pi`. Forgeyard talks to Pi with `pi -p --mode json` and `OPENAI_BASE_URL` / `OPENAI_API_KEY` from onboard.

## Onboard

```sh
fy onboard
```

One field at a time, tokens hidden:

1. Telegram bot token (empty = skip yard)
2. LLM endpoint — any OpenAI-compatible base URL
3. LLM token
4. GitHub PAT (`repo`: issues, PRs, merge)
5. SSH host (`user@host` or `user@host:port`)
6. SSH password

Secrets land in `~/.forgeyard/factory/tokens/tokens.toml` mode 0600. Never in the event log.

## Use

Two terminals.

```sh
# terminal 1 — keep open; this is the factory heartbeat
fy start
```

```sh
# terminal 2 — give work
fy do https://github.com/YOU/toy implement the spec.md in this repo
```

`fy do` does not call the LLM. It binds the project, writes `inbox.md`, appends the log. Watch picks it up.

```text
fy help
fy status
fy stop
fy bind URL
```

## What the factory expects from your project repo

| file | why |
|---|---|
| `spec.md` | without it watch stays blocked-on-spec |
| `Makefile` with `qa` | QA runs `make qa` on the cluster checkout |
| default branch unprotected by required reviews | watch merges with your PAT |

On the SSH host the clone lives at `/srv/forgeyard/<repo-name>/repo/` (`pack/bootstrap-host.sh`).

## Pieces

| piece | job |
|---|---|
| `fy` | what you type |
| `forge` | hooks, envelope, spec gate, log |
| `watch` | state machine, no LLM |
| `yard` | Telegram |
| `pi` | one session per run |

## Spec map

- [SPEC.md](SPEC.md) — kernel
- [spec/watch.md](spec/watch.md) — A–E
- [spec/intake.md](spec/intake.md) — `fy do`
- [spec/onboard.md](spec/onboard.md) / [spec/cli.md](spec/cli.md) / [spec/install.md](spec/install.md)
- [spec/cluster.md](spec/cluster.md) / [spec/github-auth.md](spec/github-auth.md)
- [spec/llm.md](spec/llm.md) / [spec/qa-command.md](spec/qa-command.md)
- [spec/adversarial-review-v0.md](spec/adversarial-review-v0.md)
- [roles/](roles/) [skills/](skills/)

## License

MIT. See [LICENSE](LICENSE).
