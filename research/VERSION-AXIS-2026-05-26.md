# monogram version axis (`monogram_version`) — branch `version-axis`

## Problem

`report` derives arm identity from `parse_arm(label).tool`, which was just `"monogram"` for every
run regardless of which monogram build produced it. Runs from monogram 0.51.x and 0.52.x silently
merged into one arm, blending medians. monogram ships multiple versions per week, so cross-day
corpora were averaging across different tools.

## What shipped (core, first-class axis — semver identity)

Label grammar gains an optional version segment right after the tool:

```
<tool>-<version>-<cli>-<model>[-<effort>]-rN-t<ms>
monogram-0.52.1-claude-haiku-r1-t1779701244954
```

- `util.rs`
  - `Arm` gains `version: String`.
  - `parse_arm` splits a trailing semver-shaped segment off the tool region (`split_tool_version` +
    `is_version_token`). Keyword-anchored, so a dotted *model* (`gpt-5.4`) is never mistaken for a
    version. Legacy labels (no version segment) → `version == ""`, parsed identically to before.
  - `full_arm_name(tool, version, cli, model, effort)` inserts the version when non-empty.
  - `capture_semver(bin)` resolves the binary on PATH, follows symlinks, and reads the semver from
    the OpenCLIs install path `…/versions/<name>/<semver>/…`. Returns `""` when not OpenCLIs-installed
    (e.g. `target/debug`/worktree builds) — it never fabricates a version.
- `run.rs` — at run start, reads `version_bin` from tool.json, captures the semver, inserts it into
  the label, and records it in the run meta.
- `run_meta.rs` — `StartMeta.monogram_version` → written to `<run>.meta.json` as `monogram_version`.
- `report.rs` — `arm_display(a)` = `tool` + ` @<version>`; aggregate + cross-instance summary group
  by tool **and** version, so each version is its own arm row with its own medians.
- `harness/tools/{monogram,monogram-mcp,monogram-thin}/tool.json` — declare `"version_bin": "monogram"`.
  baseline omits it → no version segment.

## Backward compatibility

Legacy runs (the existing 81-run corpora) have no version segment → `version == ""` →
`arm_display` is the bare tool name → reports render exactly as before. Verified live against
`bun-1.3.10-toThreadSafe`. 67/67 tests pass.

## Decisions

- **Identity = semver only** (chosen). Same-semver rebuilds (unreleased monogram behavior
  experiments that share a version) still merge into one arm — disambiguate those with `--tag` as
  before. A binary fingerprint would separate them but was not chosen.
- **Captured semver = monogram on PATH at run start.** For `--prepared` runs the index was built by
  a possibly-different monogram than the one the solver invokes; the label records the runtime
  monogram, not the index's. Note when interpreting prepared runs.

## Deferred (to avoid conflict with concurrent `main.rs` WIP)

This branch is off `ba2220b` (grok). `main.rs`, `README.md`, `SPEC.md`, `initiate/*` deliberately
**untouched** — they carry heavy uncommitted WIP (`--since`, grok, etc.) in the working tree.
Add after that WIP lands:

- `report <id> --version <semver>` filter (alongside the WIP's `--since`).
- `report <id> --diff-versions <tool>` side-by-side per-version medians.
- Doc-surface updates: label grammar + the version axis in README / SPEC / `initiate/initiate.md`,
  and the Axes Invariant table in the `monolex-monobench-maker` skill.
