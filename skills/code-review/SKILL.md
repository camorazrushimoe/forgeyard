---
name: code-review
description: Review a diff against repo standards and the spec.
---

# Code review

Two axes only:

1. Does the diff match `spec.md` / the issue?
2. Does it match the repo's own style and tests?

Output:

- Blocking findings (must change)
- Non-blocking
- Verdict: approve | needs-changes

Do not merge. Do not rewrite the spec in the review.
