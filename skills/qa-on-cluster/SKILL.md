---
name: qa-on-cluster
description: On the SSH host, fetch the PR sha in /srv/forgeyard/<project>/repo, run it, write merge/no-merge on the PR. Do not merge.
---

# QA on cluster

Same host as developer. Same clone. Not a second product.

1. Do not merge. Do not push to main.
2. `git fetch` and checkout the PR head sha. Do not test unpushed files.
3. Run this project's checks against that sha.
4. Comment on the PR:

```
## QA on cluster

**Cluster:** user@host
**Deployed sha:** …
**Tests:** …
Verdict: merge
```

or `Verdict: no-merge` and what broke.

Watcher parses the `Verdict:` line only.
