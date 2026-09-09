# Watch

`watch` is a deterministic loop. No LLM inside it.
It reads `plan.json`, `state.json`, `events.jsonl`, and GitHub.
If a transition is legal, it calls `forge run` or merges the PR itself.

Where code is edited and run: [spec/cluster.md](cluster.md).
Watch does not SSH. It only requires artifacts (PR URL, verdict line, `hook stop`).

## Who merges, who deploys

- Developer never merges `main`/`master`.
- Every ticket is a **feature branch** → **PR into main**.
- Developer works on the **dev cluster** clone (create branch, edit, run, push, open PR).
- When the PR exists, watch starts **QA** on the **same host**, same clone, PR sha.
- Only `Verdict: merge` from QA lets watch merge on GitHub.
- `Verdict: no-merge` sends the same ticket back to developer on the same branch/PR.
- After merge, the ticket is `done`. Watch picks the next ready item.

## Artifacts watch trusts

| fact | how it is proven |
|---|---|
| spec present | `spec.md` non-empty |
| spec accepted | GitHub comment with `Verdict: approve` from a `tech-pm` run |
| plan exists | `projects/<repo>/plan.json` valid |
| PR open | `artifact` is a PR URL and GitHub says `open` |
| QA merge gate | PR comment contains exactly one line `Verdict: merge` or `Verdict: no-merge` |
| merged | GitHub PR `merged == true` |
| run finished | `events.jsonl` `hook=stop` for the live `run_id` |

No artifact → the step did not happen.

## plan.json

```json
{
  "schema": 1,
  "project": "my-app",
  "spec": "spec.md",
  "source": "https://github.com/owner/my-app/issues/14",
  "items": [
    {
      "id": "T1",
      "issue": 21,
      "title": "auth module",
      "branch": "feature/21-auth",
      "pr": null,
      "needs": [],
      "status": "ready"
    }
  ]
}
```

`status` on an item:

```
ready
implementing
pr_open
qa_running
no_merge
merging
merged
done
blocked
```

Watch writes `status`, `branch`, `pr`. Tech-pm writes the item list in scenario B.
One item in flight per project (v0).

## Item state machine (scenario C)

```text
ready
  -> forge run developer implement          # cwd = cluster clone
  -> implementing

implementing
  -> stop ok + PR URL     -> pr_open
  -> stop crash           -> retry implementing
  -> retries exhausted    -> blocked

pr_open
  -> forge run qa qa-on-cluster             # same host, fetch PR sha
  -> qa_running

qa_running
  -> Verdict: merge       -> merging
  -> Verdict: no-merge    -> no_merge
  -> stop crash           -> retry qa_running

no_merge
  -> forge run developer implement          # same issue, branch, PR, cluster
  -> implementing

merging
  -> watch runs `gh pr merge` (no LLM)
  -> merged on GitHub     -> done
  -> merge failed         -> blocked

done
  -> pick next item whose needs are all done
```

Developer may self-review inside the implement session. That is not a watch state.
QA on the cluster **is** the merge gate.

## Scenarios

### 0. bind

Input: GitHub URL. `forge bind`. No agent. Then A if `spec.md` exists, else blocked-on-spec.

### A. spec-review

1. `forge run --agent tech-pm --step adversarial_review`  (laptop project dir)
2. Comment: `Verdict: approve | needs-changes | blocked`
3. approve → B if more than one PR, else one-item plan and C
4. needs-changes → wait for new spec.md, then A
5. blocked → human

### B. breakdown

1. `forge run --agent tech-pm --step breakdown` (laptop)
2. Artifact: valid `plan.json` plus GitHub issues
3. First free item → C

### C. implement-ticket

See the state machine. Branch `feature/<issue>-<slug>` or `fix/`.
First implement may `git clone` onto the host. No extra approve.

QA comment:

```
## QA on cluster

**Cluster:** <host>
**Deployed sha:** <git sha>
**Tests:** <short>
Verdict: merge
```

Retry cap implement ↔ no-merge: 5. Then `blocked`.

### D. plan-done

All items `done`. Log `plan_done`. Optional tech-pm close comment. Idle.

### E. bugfix

Triage on laptop, then C on `fix/` branch, same cluster rules.

### crash-retry and busy

`stop=crash` → same step, new `run_id`. Cap 3 per step, then `blocked`.
Busy → watch sleeps. One role at a time so the single clone stays consistent.

## Watch loop

```text
loop every N seconds:
  if project busy: continue
  load plan.json + state.json + last stop event + GitHub PR

  if no plan and spec unreviewed: run A
  if spec approved and no plan: run B
  if plan complete: D

  item = first item with status not in {done, blocked} and needs done
  match item.status:
    ready / no_merge     -> run developer implement
    implementing         -> if PR open: set pr_open
    pr_open              -> run qa qa-on-cluster
    qa_running           -> parse Verdict on the PR
    merging              -> gh pr merge
    else                 -> sleep
```

## Illegal moves

- merge by developer or QA process
- implement/QA on the laptop when cluster is configured
- start QA before an open PR (unpushed work is invisible)
- start implement without accepted spec (except triage in E)
- two busy runs on one project
- invent plan items

## Status line

```
plan:     T2/5
item:     T2  qa_running  feature/22-billing  pr#18
cluster:  deploy@203.0.113.10
```

## Non-goals

- exact process manager on the host
- exact test commands (skill `qa-on-cluster`)
- extra bug issues from QA (v0: PR comment)
- parallel tickets
- production deploy
