# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**python/cpython PR #142851** — gh-142831: Fix UAF in `_json` module

**fix commit:** 235fa7244a04 · **base (merge^):** d761f539bdae · merged 2026-04-12

## Changed source files (test/fixture files filtered out)
- Misc/NEWS.d/next/Library/2025-12-17-04-10-35.gh-issue-142831.ee3t4L.rst
- Modules/_json.c

## PR body

<!--
Thanks for your contribution!
Please read this comment in its entirety. It's quite important.

# Pull Request title

It should be in the following format:

```
gh-NNNNNN: Summary of the changes made
```

Where: gh-NNNNNN refers to the GitHub issue number.

Most PRs will require an issue number. Trivial changes, like fixing a typo, do not need an issue.

# Backport Pull Request title

If this is a backport PR (PR made against branches other than `main`),
please ensure that the PR title is in the following format:

```
[X.Y] <title from the original PR> (GH-NNNNNN)
```

Where: [X.Y] is the branch name, for example: [3.13].

GH-NNNNNN refers to the PR number from `main`.

-->


<!-- gh-issue-number: gh-142831 -->
* Issue: gh-142831
<!-- /gh-issue-number -->


Fixes #142831
