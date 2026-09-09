# Roles and skills

Source of the old pack: [camorazrushimoe/dev-crew](https://github.com/camorazrushimoe/dev-crew) agents `tech-pm`, `developer`, `qa`.

Dropped on purpose:

- role `devops` (staging / pre-prod / factory infra)
- skills `linear-workflow`, `task-dispatch` (doors + Linear bus)
- Hermes `SOUL.md` homes, webhook doors, shared Redis

Kept and rewritten as files in this repo: three roles, skills that do not depend on Linear or doors.

## One runner, three masks

```
pack.toml  → role + skill list for this step
roles/<role>.md              → who you are this session
skills/<skill>/SKILL.md      → how you do this kind of work
forge envelope               → project + step + rules
pi -p                        → one session, then die
```

There is no always-on tech-pm process. The same Pi binary plays PM, then later developer, then QA, in separate runs.

## Roles

| id | job | first workflow |
|---|---|---|
| `tech-pm` | specs, triage, adversarial review, tickets as GitHub issues | `spec-review` |
| `developer` | implement on a feature branch, TDD, self-review, PR | later `implement` |
| `qa` | test against spec, diagnose, QA report, merge verdict | later `qa` |

Adversarial spec review in v0 is **only** `tech-pm`. Developer and QA lenses from the old SOULs wait until `crew` can schedule extra review runs if we want them. One reviewer per spec, by contract.

## Skill catalog (v0 pack)

Shared (mounted when the role lists them):

- `git-branch-discipline` — never push main, feature branch + PR
- `code-review` — standards + spec fidelity
- `tdd` — red-green-refactor

tech-pm:

- `adversarial-review` — first workflow skill
- `to-spec` — turn known context into spec.md + GitHub issue
- `triage` — classify incoming GitHub issues
- `domain-modeling` — CONTEXT.md / ADR, no Linear

developer:

- `implement`
- `codebase-design`
- `resolving-merge-conflicts`

qa:

- `qa-report`
- `diagnosing-bugs`

Not in v0 pack (may come back as files later): `wayfinder`, `grilling`, `prototype`, `research`, `deploy-dev`, `handoff`.

## Mapping to old Dev Crew paths

| old | new |
|---|---|
| `agents/tech-pm/hermes-home/SOUL.md` | `roles/tech-pm.md` |
| `agents/developer/hermes-home/SOUL.md` | `roles/developer.md` |
| `agents/qa/hermes-home/SOUL.md` | `roles/qa.md` |
| `agents/*/skills/<name>/SKILL.md` | `skills/<name>/SKILL.md` (one copy, referenced by many roles) |
| Linear ticket | GitHub issue in the bound repo |
| door / bus | `forge hook` + `events.jsonl` |

## Writing rules for this pack

- English in role and skill files (same as Dev Crew).
- No Hermes paths, no `/opt/crew`, no Linear API, no webhook doors.
- A skill is one folder `skills/<id>/` with `SKILL.md`. Extra reference files allowed.
- Promoting a new skill = PR to this repo. The runner must not rewrite pack files.
