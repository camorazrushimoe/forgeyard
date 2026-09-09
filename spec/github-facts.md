# GitHub facts (no LLM)

After `implement`, the wrapper — not Pi — asks GitHub if a PR exists for the ticket branch:

```
gh pr list --repo owner/repo --head <branch> --state open --json url,number
```

- one open PR → write its URL into `hook stop` artifact / plan item `pr`
- none → implement did not finish; watch retries or blocks
- never parse the model transcript for a PR number

Same idea for the QA verdict: read the latest issue comment that contains a line `Verdict: merge` or `Verdict: no-merge`. Ignore the rest of the prose.
