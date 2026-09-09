# Forgeyard

Deterministic kernel for a spec-driven AI software factory.

Install on a laptop should feel like one command: binaries + pack (roles, skills) + Pi runner.

| piece | kind | job |
|---|---|---|
| `forge` | static Rust CLI | bind GitHub project, spec gate, hooks, envelope, event log |
| `yard` | static Rust daemon | Telegram control panel |
| `watch` | static Rust loop | deterministic scenario driver (no LLM) |
| `pi` | external harness | the one process that may read/write files and run bash |
| pack | files in this repo | three roles + skills |

There is **one** Pi process per run. Role is chosen per session: `tech-pm`, `developer`, or `qa`.

Merge gate: developer opens a PR from a feature branch; QA deploys that branch to the dev cluster and writes `Verdict: merge` or `Verdict: no-merge`; `watch` merges only on `merge`.

No Hermes. No Redis. No Linear. No DevOps role.

## Watcher loops

`watch` has no LLM. It reads facts (`spec.md`, `plan.json`, PR, verdict line, event log) and starts the next legal `forge run`.
Full contract: [spec/watch.md](spec/watch.md).

### How the factory walks a spec

```mermaid
flowchart TD
  bind["0 bind GitHub URL"] --> specExists{spec.md?}
  specExists -->|no| humanSpec["human: put spec.md"]
  humanSpec --> specExists
  specExists -->|yes| A["A spec-review\ntech-pm"]
  A -->|needs-changes| specExists
  A -->|blocked| humanA["human"]
  A -->|approve, small spec| oneItem["one-item plan"]
  A -->|approve, large spec| B["B breakdown\ntech-pm writes plan.json + issues"]
  B -->|no plan.json| B
  B --> ready["first ready item"]
  oneItem --> ready
  ready --> C["C implement-ticket loop"]
  C -->|item done, more items| ready
  C -->|all items done| D["D plan-done"]
  C -->|retries gone| humanC["human / blocked"]
```

### A. Spec review

```mermaid
flowchart TD
  spec["spec.md landed"] --> pm["watch starts tech-pm\nadversarial-review"]
  pm --> comment["verdict on GitHub issue/PR"]
  comment -->|Verdict: approve| next["B or one-item C"]
  comment -->|Verdict: needs-changes| wait["wait for new spec.md"]
  wait --> spec
  comment -->|Verdict: blocked| human["human"]
```

### B. Breakdown

```mermaid
flowchart TD
  approved["spec approved"] --> size{bigger than one PR?}
  size -->|no| single["tech-pm writes one-item plan.json"]
  size -->|yes| pm["watch starts tech-pm\nbreakdown"]
  pm --> plan["plan.json + GitHub issues T1..Tn"]
  plan -->|missing plan.json| pm
  plan --> first["watch marks first free item ready"]
  single --> first
  first --> C["C"]
```

### C. Implement ticket — main loop

Feature branch → PR into main. Developer does not merge and does not deploy.
QA deploys that branch to the dev cluster and writes the merge gate on the PR.
Watch merges only on `Verdict: merge`.

```mermaid
flowchart TD
  ready["item ready"] --> dev["watch starts developer\nimplement on feature/issue-slug"]
  dev --> pr{open PR into main?}
  pr -->|no / crash| retryDev["retry same step"]
  retryDev --> capDev{retry cap?}
  capDev -->|no| dev
  capDev -->|yes| blocked["blocked / human"]
  pr -->|yes| qa["watch starts qa\nqa-on-cluster"]
  qa --> deploy["QA deploys PR branch\nto dev cluster and tests"]
  deploy --> verdict{PR comment}
  verdict -->|Verdict: no-merge| dev
  verdict -->|crash| retryQa["retry QA"]
  retryQa --> qa
  verdict -->|Verdict: merge| merge["watch merges PR\nno LLM"]
  merge -->|github merged| done["item done"]
  merge -->|merge failed| blocked
  done --> more{next item with deps done?}
  more -->|yes| ready
  more -->|no, plan finished| D["D plan-done"]
```

### D. Plan done

```mermaid
flowchart TD
  allDone["every item done"] --> log["watch writes plan_done"]
  log --> optional["optional tech-pm close comment\non the source issue"]
  optional --> idle["project idle"]
```

### E. Bugfix

Same C loop. Different entrance and branch prefix (`fix/` instead of `feature/`).

```mermaid
flowchart TD
  bug["GitHub issue labeled bug"] --> triage["watch starts tech-pm\ntriage"]
  triage -->|needs-info / wontfix| human["human"]
  triage -->|ready| item["one plan item"]
  item --> C["C on fix/issue-slug"]
```

## Spec map

- [SPEC.md](SPEC.md) — kernel
- [spec/watch.md](spec/watch.md) — watcher + scenarios A–E
- [spec/telegram-panel.md](spec/telegram-panel.md) — `yard`
- [spec/tokens.md](spec/tokens.md) — secrets
- [spec/pi-runner.md](spec/pi-runner.md) — how Pi is wrapped
- [spec/install.md](spec/install.md) — one-command install
- [spec/roles-and-skills.md](spec/roles-and-skills.md) — pack layout
- [roles/](roles/) — three role files
- [skills/](skills/) — factory skills

## Status

Specification + pack files. Rust binaries and `install.sh` come next.

## License

MIT. See [LICENSE](LICENSE).
