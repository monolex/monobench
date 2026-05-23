# monobench

**Does giving an AI coding agent a code-intelligence tool actually help it find the ROOT CAUSE of a
real bug — when the symptom and the cause are in different files/languages, linked by *structure*
(call graph / ownership / FFI boundary) rather than *text*?**

Most "tool helps" claims are tested on grep-solvable problems, where a strong model + plain grep wins
and the tool adds nothing. monobench only admits problems where a strong baseline — *told to dig just
as hard* — still struggles, then measures whether a tool (and which model) changes the outcome.

Think *"SWE-bench, but for cross-boundary fault localization, comparing code-intelligence tools."*
Two dimensions are pluggable: the **tool** under test and the **model runner**.

## Layout
```
monobench/
├── bin/monobench            # the CLI — a single Rust binary (no-arg → help). Commands:
│                            #   list·tools·status·clean·show·run·matrix·grade·trace·report·adoption·add
├── Cargo.toml  src/         # Rust core: grade·report·adoption·trace·meter + util, and the native run.rs.
│                            #   One dependency (serde_json). `cargo build --release` → bin/monobench.
├── initiate/
│   ├── SKILL.md             # AI-discovery skill (mono-series / OpenCLIs initiate convention)
│   └── initiate.md          # full command reference (what `monobench` prints)
├── instances/<id>/          # the problem set (DATA, not code)
│   ├── instance.json        # repo + tag + ground_truth + grading + admission
│   ├── symptom.md           # the task given to the agent — NO root-cause spoilers
│   └── ground_truth.md      # the real fix (GATED — never fed to the solver)
├── harness/
│   ├── prompts/depth.md      # shared "dig deep" directive — EVERY arm gets it (fairness)
│   ├── tools/<name>/         # PLUGGABLE tool adapters: tool.json + injected docs (skill.md/initiate.md/lead.md)
│   │   ├── baseline/  monogram/  monogram-mcp/  codegraph/  _TEMPLATE/
│   ├── run.sh                # legacy shell runner (the Rust `run` reimplements this natively; being retired)
│   └── runners/niia.sh       # interactive model-CLI runner (PTY; off metered `-p`; metered by monometer)
├── results/<id>/             # run logs + grades (gitignored)
├── SPEC.md   CANDIDATES.md
```
The CLI is a **Rust binary** (analysis + the native runner) — its only build dependency is `serde_json`,
no Node. Repos-under-test are **cloned on demand** at the pinned tag (not vendored) → the bench stays small.

## Quickstart
```bash
monobench list                                 # instances
monobench tools                                # adapters (baseline · monogram · monogram-mcp · codegraph · yours)
monobench show  ghostty-8208-split-flicker     # the task (no spoilers)

monobench matrix ghostty-8208-split-flicker \  # the benchmark: parallel, git-worktree isolated, n runs/cell
    --tools baseline,monogram,monogram-mcp --models opus,sonnet,haiku --runs 5 --jobs 6
monobench report   ghostty-8208-split-flicker  # per-MODEL grid: FULL Hit-rate · med $/tokens/time · mono%
monobench adoption ghostty-8208-split-flicker  # per-run tool-call breakdown + git-integrity
monobench trace    ghostty-8208-split-flicker monogram-r1        # one run's tool-call timeline
monobench run   ghostty-8208-split-flicker baseline 1            # a single run (matrix is the usual path)
monobench clean ghostty-8208-split-flicker baseline              # wipe an arm before a fresh re-bench
```
Requires `git`, the tool under test, and a **runner**: `claude` (default headless `-p`) or the niia
headless terminal (`MONOBENCH_RUNNER=niia`, off metered `-p`). The CLI binary itself has no Node dependency.

## Two pluggable dimensions
**Tools** — each is a drop-in adapter `harness/tools/<name>/tool.json`
(`index` cmd · `skill` to inject · `deliver`: none|cli|mcp · `forfeit_grep`). Add one:
`cp -r harness/tools/_TEMPLATE harness/tools/<name>`. Shipped: **baseline** (control/admission gate),
**monogram** (CLI + skill), **codegraph** (MCP; FORFEITs on repos it can't index, e.g. Zig).

**Runners** (`MONOBENCH_RUNNER`) — `claude-p` (headless, cost from JSON) or **`niia`** (interactive
model CLI via the niia headless terminal — off metered `-p`, works with `claude`/`codex`/`gemini` via
`MONOBENCH_CLI`, metered per-run by **monometer including cache** tokens/hit-rate/cost).

So the full grid is **instances × tools × runners(models)** — e.g. reproduce *"Codex 5.3 solved it,
Opus 4.6 didn't"* tool-by-tool.

## What it measures
- **root-cause Hit-rate (FULL)** — named the true root-cause function *with* the correct mechanism.
  `NAME_ONLY` = right function, weak mechanism · `DECOY` = the adjacent trap · `MISS` = neither.
- **tokens-per-correct-root-cause** — the load-bearing efficiency metric.
- **cache breakdown** — fresh input vs cache_read vs cache_creation + hit-% (matters: tool arms inject
  more context, which caches differently; `monometer daily` `no_cache_usd` = the un-cached cost).
- **tool-adoption** — tool calls the agent actually made (a tool it never called wasn't tested).
- **forfeit** — the tool couldn't index the repo at all.

## Fairness rules (see SPEC.md)
1. **Every arm gets the same depth directive** — only the *tool* differs.
2. **Admission gate** — run baseline first; if it reliably solves the instance cheaply, the instance
   is non-discriminating and is down-weighted. Only hard problems count.
3. **Symptom ≠ cause, no text bridge** — reject instances where grep on the symptom finds the cause.
4. **The answer key is never shown to the solver** — it gets only `symptom.md`; the grader holds
   `instance.json.grading` + `ground_truth.md`. An AI may *orchestrate* runs but must not be the
   *solver* in a context that has seen the key.
5. **No reading the fix from git history (git-blocked)** — every arm runs with `git` denied
   (`--disallowedTools "Bash(git:*)"`). The repo is cloned with full history, which *includes the fix
   commit*; without this an agent simply runs `git log --all --grep=<symptom>` → `git show <fix>` and
   copies the answer. We caught baselines doing exactly this (3/3 "FULL" that were pure git-cheat — see
   `monobench trace`), so all comparisons are git-blocked and `monobench adoption` reports the git
   attempts and confirms each was denied.
6. **The tool is forced into the prompt, not merely offered** — its full reference (`initiate.md` +
   `skill.md`) is shoved into the `-p` prompt, and for a CLI tool the agent is told to run it FIRST.
   Models ignore a tool they're only told exists (observed adoption ~5–13%); with the docs in-prompt +
   a run-first directive, adoption *and* correctness both jump. A tool the agent never called wasn't
   tested — `monobench adoption` makes that visible (mono% share + first-use call#).

## Adding an instance
`monobench add <id>` → fill `instance.json` (repo, tag, `ground_truth`, `grading`), `symptom.md`
(no spoilers), `ground_truth.md` (gated). Confirm it meets C1–C6 in SPEC.md. No code changes.

## Instances
- **`bun-1.3.10-toThreadSafe`** — cross-thread string refcount UAF that crashed Claude Code (Zig↔C++;
  fixed upstream by PR #30049). Category: memory-safety.
- **`ghostty-8208-split-flicker`** — the GTK split-tree flicker open ~6 months that **Codex 5.3
  solved and Opus 4.6 failed** (Zig/GTK; #8208). Category: UI async-ordering. codegraph forfeits (Zig).

See `CANDIDATES.md` for the backlog.
