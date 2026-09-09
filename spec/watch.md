# Watch

Deterministic loop. No LLM.
Reads `inbox.md`, `plan.json`, `state.json`, `events.jsonl`, GitHub.
Calls `forge run` or merges the PR. Never SSHes. Never parses model prose for facts.

Where code lives: [cluster.md](cluster.md).
How a PR is proven: [github-facts.md](github-facts.md).
How push happens: [github-auth.md](github-auth.md).
QA command: [qa-command.md](qa-command.md).

## Merge gate

Developer never merges. QA never merges.
Developer commits on the host; laptop pushes and opens the PR.
Watch starts QA only when GitHub shows an open PR for the item branch.
QA runs `make qa` on that sha via SSH (Pi on the laptop).
Latest PR comment line `Verdict: merge` | `Verdict: no-merge` wins
(older verdicts in the thread are ignored).
Watch merges on GitHub only on `merge`.

## Artifacts

| fact | proof |
|---|---|
| work queued | `inbox.md` after `fy do` |
| spec present | `spec.md` non-empty (project dir or fetched from repo) |
| spec accepted | latest tech-pm `Verdict: approve` |
| plan exists | valid `plan.json` |
| PR open | `gh pr list --head <branch>` returns one open PR |
| QA gate | latest `Verdict: merge` or `no-merge` on that PR |
| merged | GitHub `merged == true` |
| run finished | `hook=stop` for live `run_id` |

## plan.json item status

`ready | implementing | pr_open | qa_running | no_merge | merging | merged | done | blocked`

One item in flight. Default branch = GitHub default (`main` or `master`), detected, not assumed.

## C machine

```text
ready / no_merge
  -> forge run developer implement     # Pi laptop, files via SSH
  -> implementing

implementing
  -> wrapper: fetch branch from host, push to GitHub, gh pr list
  -> open PR     -> pr_open
  -> no PR / crash -> retry (cap 3 crash, cap 5 implement↔no-merge) else blocked

pr_open
  -> forge run qa qa-on-cluster        # make qa on host sha
  -> qa_running

qa_running
  -> latest Verdict: merge     -> merging
  -> latest Verdict: no-merge  -> no_merge
  -> crash -> retry QA

merging
  -> watch: gh pr merge on laptop
  -> merged -> done -> next item or D
```

## Watch loop

```text
loop:
  if busy: sleep
  if inbox.md new and project unbound: bind from URL inside it
  if no spec.md: blocked-on-spec (do not treat inbox text as spec)
  if spec unreviewed: A
  if approved and no plan: B or one-item plan
  if plan complete: D
  else drive current item through C
```

## Scenarios A–E

Unchanged intent: A tech-pm review on laptop; B plan.json; C as above;
D plan_done; E triage then C with `fix/`.

## Illegal

- merge by Pi
- implement/QA without SSH when cluster is set
- start QA before GitHub shows the PR
- persist PAT on the host
- two busy runs
- invent plan items
- treat model text as the PR URL
