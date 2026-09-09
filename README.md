# Forgeyard

Deterministic kernel for a spec-driven AI software factory.

Install on a laptop should feel like one command: binaries + pack (roles, skills) + Pi runner.

| piece | kind | job |
|---|---|---|
| `forge` | static Rust CLI | bind GitHub project, spec gate, hooks, envelope, event log |
| `yard` | static Rust daemon | Telegram control panel |
| `pi` | external harness | the one process that may read/write files and run bash |
| pack | files in this repo | three roles + skills. Not compiled into the binaries |

There is **one** Pi process per run, not three daemons. Role is chosen per session: `tech-pm`, `developer`, or `qa`.

No Hermes. No Redis. No Linear. No DevOps role. If a loop dies, truth is on disk and GitHub.

`crew` (workflow binary) is still later. Until then `forge run` is the wrapper around Pi.

## Invariants

1. Project name = GitHub repository name.
2. Start/stop hooks fire from the wrapper, never from the model.
3. Every Pi prompt goes through `forge envelope` and always contains the project name.
4. No spec → no implementation. `forge require-spec`.
5. One append-only `events.jsonl` per project.
6. One busy lock per project. One role per run.

## Spec map

- [SPEC.md](SPEC.md) — kernel
- [spec/telegram-panel.md](spec/telegram-panel.md) — `yard`
- [spec/tokens.md](spec/tokens.md) — secrets
- [spec/pi-runner.md](spec/pi-runner.md) — how Pi is wrapped
- [spec/install.md](spec/install.md) — one-command install
- [spec/roles-and-skills.md](spec/roles-and-skills.md) — pack layout
- [roles/](roles/) — three role files
- [skills/](skills/) — factory skills

## Status

Specification + pack files. Rust binaries and `install.sh` implementation come next.

## License

MIT. See [LICENSE](LICENSE).
