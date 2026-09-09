---
name: git-branch-discipline
description: Feature branch, PR, never main.
---

# Git branch discipline

Never commit or push to `main`/`master`.

Branch: `feature/<issue>-<slug>` from main.
Commit small, English messages.
Push the branch and open a PR. Do not merge it.

Forbidden: `git push` to main, `git reset --hard`, `git clean -fd`, deleting an in-flight feature branch, `git add -A`.
