# Intake — how work enters

Work enters from a second terminal while `fy start` tails the log in the first.
Telegram is optional in v0.

```
fy do <github-url> <prompt...>
```

Examples:

```
fy do https://github.com/acme/toy/issues/4 add login as in the spec

fy do https://github.com/acme/toy implement the spec.md already in this repo
```

## What `fy do` does (deterministic, no LLM)

1. Parse owner/repo from the URL. Project name = repo name.
2. `forge bind` if this project is new.
3. Save `projects/<repo>/inbox.md` with the raw prompt + URL + time.
4. Append `events.jsonl` `hook=intake` (`agent=human`, summary truncated).
5. If the URL points at an issue/PR, store that as `source` in `plan.json` later.
6. Print one line: `queued <project> — watch will pick it up`.

`fy do` does not start Pi. Watch reads `inbox.md`, refreshes the spec cache from GitHub, then chooses A/B/C/E.

If `fy start` is not running, the files are still written. Watch sees them on next start.

## Spec gate and repository spec cache

Before evaluating the gate for a bound GitHub project, watch deterministically refreshes `projects/<repo>/spec.md` from `spec.md` at the repository's detected default-branch SHA.

1. Resolve the default branch through the authenticated GitHub API or `gh repo view`; do not assume `main`.
2. Fetch exactly `spec.md` at that branch/SHA using the same GitHub credentials already configured for the factory. Public repositories must work without a PAT; private repositories use the factory PAT.
3. Reject a missing, empty, or undecodable file. On success, atomically replace the local cache and write `spec-source.json` with owner/repo, branch, commit SHA, fetch time, and content SHA-256.
4. If GitHub is temporarily unreachable but a non-empty local cache exists, retain the cache and record `spec_refresh_stale`; do not silently replace it with an empty file. If no usable cache exists, record the concrete refresh failure and stay `blocked-on-spec`.
5. Refresh at intake and whenever the default-branch SHA changes. A changed content SHA invalidates the prior tech-pm approval and requires a new review.

The local `projects/<repo>/spec.md` is therefore a cache and audit artifact, not an operator-maintained second source of truth. Manual copying into the factory directory is not a supported workflow.

- a successfully refreshed non-empty `spec.md` → watch can run A (or skip to C only if the cached SHA has an approval).
- no usable repository spec/cache → watch stays `blocked-on-spec`. v0 does not treat the prompt as a spec automatically.

## CLI list

`fy help` includes `fy do URL TEXT`.
