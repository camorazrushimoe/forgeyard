# Watch

`watch` is a deterministic loop. No LLM inside it.
It reads `plan.json`, `state.json`, `events.jsonl`, and GitHub.
If a transition is legal, it calls `forge run` or merges the PR itself.

Binary name later: `watch` (same role as the postponed `crew`).
Until the binary exists, a human can walk the same table with `forge status` + `forge run`.

## Who merges, who deploys

- Developer never merges `main`/`master`. Developer never deploys the cluster.
- Every ticket is a **feature branch** → **PR into main**.
- When the PR exists, watch starts **QA**.
- QA pulls that branch onto the **dev cluster**, tests there, and writes a verdict on the PR.
- Only `Verdict: merge` from QA lets watch merge.
- `Verdict: no-merge` sends the same ticket back to developer on the same branch/PR.
- After merge, the ticket is `done`. Watch picks the next ready item.

How QA talks to the cluster (SSH layout, per-project dirs) is out of this file. The watcher only cares that QA finished with a parseable verdict and a `hook stop`.

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
  -> forge run developer implement
  -> implementing

implementing
  -> stop ok + PR URL     -> pr_open
  -> stop crash           -> retry implementing
  -> retries exhausted    -> blocked

pr_open
  -> forge run qa qa-on-cluster
  -> qa_running

qa_running
  -> Verdict: merge       -> merging
  -> Verdict: no-merge    -> no_merge
  -> stop crash           -> retry qa_running

no_merge
  -> forge run developer implement   (same issue, same branch, same PR)
  -> implementing

merging
  -> watch runs `gh pr merge` (no LLM)
  -> merged on GitHub     -> done
  -> merge failed         -> blocked

done
  -> pick next item whose needs are all done
```

Developer may self-review inside the implement session. That is not a watch state.
QA on the cluster **is** the merge gate. There is no separate watch step "code-review then deploy". One QA run covers pull-branch, deploy, test, verdict.

## Scenarios

### 0. bind

Input: GitHub URL. `forge bind`. No agent. Then A if `spec.md` exists, else blocked-on-spec.

### A. spec-review

1. `forge run --agent tech-pm --step adversarial_review`
2. Comment on the source issue/PR: `Verdict: approve | needs-changes | blocked`
3. approve → B if more than one PR is needed, else a one-item plan and C
4. needs-changes → wait for a new spec.md, then A again
5. blocked → human

One reviewer.

### B. breakdown

1. `forge run --agent tech-pm --step breakdown`
2. Required artifact: valid `plan.json` plus GitHub issues for each item
3. Missing plan → retry or blocked
4. Watch sets the first item with empty `needs` to `ready` and enters C

Tiny spec: tech-pm may write a one-item plan in the same spirit. Watch does not invent items.

### C. implement-ticket

See the state machine above.

Branch name: `feature/<issue>-<slug>` (bugs: `fix/<issue>-<slug>`).
PR target: `main` (or `master` if that is the default).
QA verdict lives on the PR, not a new issue (v0).

QA comment shape (parseable):

```
## QA on cluster

**Cluster:** <name or host>
**Deployed sha:** <git sha>
**Tests:** <commands + results, short>
Verdict: merge
```

or `Verdict: no-merge` plus what failed. No extra issue required.

Retry cap on the pair implement ↔ no-merge: 5. Then `blocked`.

### D. plan-done

All items `done`. Watch appends `hook=run status=ok summary=plan_done`.
Optional: `forge run tech-pm` with a short close comment on the source issue.
Project idle.

### E. bugfix

Incoming GitHub issue labeled bug, spec already accepted.

1. `forge run tech-pm --step triage`
2. ready → one plan item, then C (`fix/` branch)
3. needs-info / wontfix → human / stop

Same QA-on-cluster gate as C.

### crash-retry and busy

`stop=crash` → same step, new `run_id`. Cap 3 per step, then `blocked`.
If `agent_status=busy`, watch sleeps. No parallel roles in v0.

## Watch loop

```text
loop every N seconds:
  if project busy: continue
  load plan.json + state.json + last stop event + GitHub PR

  if no plan and spec unreviewed: run A
  if spec approved and no plan: run B
  if plan complete: D

  item = first item with status not in {done, blocked}
                 and all needs done
  match item.status:
    ready / no_merge     -> run developer implement
    implementing         -> if PR open: set pr_open
    pr_open              -> run qa qa-on-cluster
    qa_running           -> parse Verdict on the PR
    merging              -> gh pr merge --squash (or merge)
    else                 -> sleep
```

Watch never asks an LLM what the next step is.

## Illegal moves

- merge by developer or by QA process
- deploy by developer
- start QA before GitHub shows an open PR
- start implement without accepted spec (except triage in E)
- two busy runs on one project
- invent plan items

## Status line

`forge status` / yard `/status` add the current item:

```
plan:     T2/5
item:     T2  qa_running  feature/22-billing  pr#18
```

## Non-goals

- how SSH to the cluster works (later)
- what exact tests QA runs (later; skill `qa-on-cluster`)
- opening extra bug issues from QA (v0: PR comment only)
- parallel tickets
- production deploy
