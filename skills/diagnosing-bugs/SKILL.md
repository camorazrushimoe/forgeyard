---
name: diagnosing-bugs
description: Reproduce, minimise, hypothesise, prove. Then report or fix if STEP allows.
---

# Diagnosing bugs

1. Reproduce. If you cannot, say so and stop.
2. Minimise the input.
3. One hypothesis at a time.
4. Gather evidence (log, test, bisect).
5. If STEP is qa: write the bug into the QA report. Do not silently patch product code unless asked.
6. If STEP is implement: fix on the feature branch with a regression test.
