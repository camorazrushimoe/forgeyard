# Adversarial review — spec v0 (pre-binaries)

Date: 2026-09-09
Scope: SPEC.md + spec/* after PR #1 merge.

## Verdict: needs-changes (applied in this PR)

The factory story is implementable. Several files still described an older world (Pi cwd on the host, laptop workspace as source of truth, watch only `spec-review`, no intake, no `make qa`). Those contradictions would have shipped into the first Rust crate. This PR aligns them.

## Blockers that were real

1. **SPEC.md stale vs watch/cluster.** Kernel still said first workflow is only spec-review, `workspace/` on the laptop, binaries `forge`+`yard` only. Implementer following SPEC.md would ignore `fy`, `watch`, cluster, intake.
2. **Watch loop ignored `fy do`.** Inbox existed; watch never read it. First command after install would queue work that never starts.
3. **PR fact vs implement cwd.** Watch still implied push/PR from the host. PAT policy is laptop-only. Without github-facts + github-auth in the kernel path, implement would either leak the PAT or never push.
4. **QA command was a non-goal in watch and a goal in qa-command.md.** First QA session would improvise.
5. **Verdict line "exactly one"** on a PR that already has a previous `no-merge`. Parser must use the **latest** matching comment, not "exactly one in the thread".

## Accepted risks (do not block coding)

- Single GitHub user: no official Approve review. Comments + API merge only. Repo must not require approving reviews.
- Password SSH: wrapper must supply sshpass/askpass. Some hosts refuse password auth — operator problem, not spec.
- Pi flag names: we alias `OPENAI_BASE_URL` + `OPENAI_API_KEY`. Confirm on the installed Pi version in code, not more spec.
- `/srv/forgeyard` may need sudo on a fresh host. First implement creates it; if permission denied → `cluster_path` fail, human.
- `gh` CLI is a laptop dependency like `pi`. Not listed in old install spec.
- No toy repo in this tree yet. Need one with `spec.md` + `make qa` before an end-to-end run.
- Default branch `main` vs `master`: detect from GitHub, do not hard-code only `main`.
- state.json still grows new fields in watch (`plan.json` item status). Kernel state remains the busy lock; plan is the workflow source.

## What v0 will still not do

Parallel tickets, prod deploy, Pi on the server, PAT on the server, Telegram as the only intake.
