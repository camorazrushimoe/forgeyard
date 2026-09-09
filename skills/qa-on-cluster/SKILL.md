---
name: qa-on-cluster
description: Deploy the PR branch to the dev cluster, test it there, write merge/no-merge on the PR. Do not merge.
---

# QA on cluster

You receive a project, an issue, a feature branch, and a PR URL.

1. Do not merge. Do not push to main.
2. Take the PR branch onto the dev cluster (SSH details come from factory config later).
3. Deploy that sha. If deploy fails, `Verdict: no-merge`.
4. Run whatever checks this project has. Record commands and outcomes.
5. Comment on the PR:

```
## QA on cluster

**Cluster:** ...
**Deployed sha:** ...
**Tests:** ...
Verdict: merge
```

or `Verdict: no-merge` and what broke.

One verdict line. Watcher parses that line only.
