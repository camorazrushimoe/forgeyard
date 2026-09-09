# Forgeyard

Deterministic kernel for a spec-driven AI software factory.

This repository specifies two static Rust binaries and the files they own:

| binary | kind | job |
|---|---|---|
| `forge` | CLI, one-shot | bind a GitHub repo as a project, gate on spec, wrap agent start/stop, append the event log |
| `yard` | long-running | Telegram control panel: status, who is working, token names (not secrets) |

Hermes agents are **not** in this repo. They call `forge` as a harness.
Workflow orchestration (`crew`) is **not** in this repo yet. It will be a third binary that reads the same `state.json`.

There is no Redis, no Linear, no LLM inside `forge` or `yard`. If a loop dies, the only remaining truth is the files on disk and GitHub.

## Invariants

1. Project name = GitHub repository name.
2. Agent start/stop hooks are fired by a process wrapper, never by the model.
3. Every request that reaches an agent is wrapped by `forge envelope` and always contains the project name.
4. No spec file → no implementation work. Enforced by `forge require-spec` exit code.
5. One append-only `events.jsonl` per project. Hooks write short JSON lines under a file lock.

## Why two binaries, not one

`forge` must stay a short-lived, scriptable CLI. Hooks wrap a Hermes process and must return.
`yard` is a long-lived Telegram poller. Different failure domain, different privileges, different lifecycle.

`yard` never writes `state.json` itself except through the same functions `forge` uses (shared crate). It does not invent status. It renders files.

## Spec map

- [SPEC.md](SPEC.md) — source of truth for the kernel
- [spec/telegram-panel.md](spec/telegram-panel.md) — `yard` bot contract
- [spec/tokens.md](spec/tokens.md) — how LLM / bot tokens are stored and rotated
- [examples/](examples/) — sample `PROJECT.toml`, `state.json`, log lines

## Status

Specification only. Implementation of the Rust binaries comes next.

## License

MIT. See [LICENSE](LICENSE).
