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

`fy do` does not start Pi. Watch reads `inbox.md` + spec + GitHub and chooses A/B/C/E.

If `fy start` is not running, the files are still written. Watch sees them on next start.

## Spec gate

- `spec.md` already in the GitHub repo or in `projects/<repo>/spec.md` → watch can run A (or skip to C if already approved).
- no spec.md → watch stays blocked-on-spec until someone adds one (or a later intake rule writes a draft). v0 does not treat the prompt as a spec automatically.

## CLI list

`fy help` includes `fy do URL TEXT`.
