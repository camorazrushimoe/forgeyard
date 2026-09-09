---
name: adversarial-review
description: Review a spec before any implementation. Product lens. Publish a verdict on GitHub.
---

# Adversarial review

Read `spec.md` and the originating GitHub issue or PR. Do not write application code.

Look for:

- missing or untestable acceptance criteria
- contradictions
- hidden dependencies and unnamed owners
- scope that cannot be finished in one PR
- user-facing holes (value, completeness, convenience)

Write only an evaluation. Do not propose a different product.

Publish on the GitHub issue/PR:

```
## Spec review

**Verdict:** approve | needs-changes | blocked
**Blocking (max 3):**
- ...
**Non-blocking:**
- ...
```

Done means the comment exists. The wrapper records the artifact URL on hook stop.
