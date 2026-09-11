# GitHub facts (no LLM)

After `implement`, the wrapper — not Pi — asks GitHub if a PR exists for the ticket branch:

```
gh pr list --repo owner/repo --head <branch> --state open --json url,number
```

- one open PR → write its URL into `hook stop` artifact / plan item `pr`
- none → implement did not finish; watch retries or blocks
- never parse the model transcript for a PR number

QA merge gate: watch reads only a validated `runs/<run-id>/outcome.json` of kind `qa` for that PR (`verdict` = `merge` | `no_merge`). The wrapper also publishes `Verdict: merge` or `Verdict: no-merge` as a PR comment with the run ID. That comment is a human-visible audit trail and is never parsed by watch.
