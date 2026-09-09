---
name: qa-on-cluster
description: On the SSH host, checkout the PR sha, run make qa, write Verdict on the PR. Do not merge.
---

# QA on cluster

1. Do not merge. Do not push to main.
2. Via SSH: fetch and checkout the PR head sha.
3. Run `make qa` in the repo root. That is the v0 gate.
4. Optional smoke skill after that, if present.
5. Comment on the PR:

```
## QA on cluster

**Cluster:** user@host
**Deployed sha:** …
**Tests:** make qa …
Verdict: merge
```

or `Verdict: no-merge`.
