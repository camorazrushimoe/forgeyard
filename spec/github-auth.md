# GitHub PAT — keep it off the cluster

The PAT lives only in laptop `tokens.toml` (`github.pat`).
Do **not** write it to the SSH host (`~/.netrc`, remote git config, remote env files).

Reason: the cluster is a workbench. Disk there is easier to leak than the laptop token file.

## Split

| action | where | token |
|---|---|---|
| edit, commit, test, run app | SSH host | no |
| `git push` to GitHub | laptop | `github.pat` |
| `gh pr create` / comment / merge | laptop | `github.pat` |

After implement commits on the host:

1. laptop fetches the branch from the host over SSH (git remote or `git fetch ssh://…`)
2. laptop `git push` to GitHub with the local PAT (header or `gh auth`)
3. laptop `gh pr create` if needed
4. wrapper then runs `gh pr list` (see github-facts.md)

The token must not appear in `events.jsonl`, SSH argv that gets logged, or Pi's printed output.

If a later version must push from the host, pass the token only in a one-shot env for that process and never persist it. Not v0.
