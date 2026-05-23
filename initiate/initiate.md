monobench — code-intelligence-tool benchmark
=============================================
Does giving an AI agent a code-intelligence tool (monogram, codegraph, …) actually help it find the
ROOT CAUSE of a real bug — vs a baseline told to dig just as hard — when the symptom and the cause
are linked by STRUCTURE (call graph / ownership), not TEXT?  "SWE-bench, for tool comparison."

USAGE
  monobench <command> [args]

COMMANDS
  list                       List benchmark instances (id + title).
  tools                      List tool adapters you can run as arms (baseline/monogram/codegraph/yours).
  status <id>                Live status of an instance's runs (done/running/forfeit + active workers).
  clean <id> [arm-prefix]    Delete recorded runs (all, or just one arm e.g. `monogram`) + scratch dirs.
                             Use before a fresh re-bench so old/stale-config runs don't pollute results.
  show <id> [--spoil]        Print the task (symptom) given to the agent. --spoil reveals the gated
                             ground truth (NEVER pass --spoil output into a solver).
  run <id> <arm> [n]         Run ONE arm of ONE instance (clones repo@tag pristine, indexes, runs,
                             grades). arm = baseline | monogram. n = run number (default 1).
  grade <id> [run]           Grade all runs (or one run + its ROOTCAUSE) → FULL/DECOY/MISS + cost/tokens.
  trace <id> <run> [max]     Ordered tool-call timeline of ONE run ([M]onogram/[g]rep/[git] marked).
  report <id>                Per-MODEL comparison: FULL Hit-rate, median $/tokens/time, mono% adoption.
                             Failures (FORFEIT/NO_RESULT) are listed separately at the bottom.
  summary                    Cross-INSTANCE leaderboard: FULL hit-rate per arm × instance (+ overall).
  adoption <id>              Per-run tool-call + monogram-subcommand breakdown (calls/share/first-use/
                             fails/mix) — for CLI and MCP delivery. "Did the agent actually use it?"
  add <id>                   Scaffold a new instance from instances/_TEMPLATE/.
  help                       This text.

ARMS = TOOL ADAPTERS (pluggable — define your own)
  Each tool is a drop-in adapter: harness/tools/<name>/tool.json
    { index: "<cmd run in repo, or ''>", skill: "<skill.md to inject, or ''>",
      deliver: "none|cli|mcp", mcp: {command,args with ${REPO}/${CODEGRAPH}}, forfeit_grep: "<regex>" }
  Shipped adapters:
    baseline      index:'' deliver:none        — control (builtins only). The admission gate.
    monogram      index:'monogram index .' cli — CLI; skill leads with structural cmds, "run first".
    monogram-mcp  index:'monogram index .' mcp — SAME index as forced MCP tools (monogram serve).
                                                 Lifts adoption on weak models (CLI suggestion → tools).
    codegraph     index:'codegraph init' mcp   — first-class MCP tools; FORFEITs if it can't index (Zig OOM).
  Add a tool:  cp -r harness/tools/_TEMPLATE harness/tools/<name> && edit tool.json (+ skill.md)
  EVERY arm gets the same depth directive (prompts/depth.md); only the tool differs.

RUNNERS (pluggable model invocation — env MONOBENCH_RUNNER)
  claude-p   headless `claude -p` (cost/tokens from --output-format json). Default.
  niia       interactive model CLI over the niia headless terminal (write/wait-idle/get-answer) —
             OFF metered -p, works with claude/codex/gemini (MONOBENCH_CLI), metered per-run by
             monometer incl. CACHE (tokens, cache_read, cache_hit %, cost). For niia users / post-`-p`.

METRIC
  root-cause Hit-rate (FULL) · tokens-per-correct-root-cause · tool-call count · tool-ADOPTION
  (a tool the agent never called was not tested) · FORFEIT (tool could not index the repo).

FAIRNESS (enforced — see SPEC.md)
  1. every arm gets the SAME depth directive; only the tool differs.
  2. ADMISSION GATE: run baseline first; if it solves the instance cheaply, the instance is
     non-discriminating and is down-weighted. Only hard problems count.
  3. the answer key (ground_truth.md + instance.json `grading`) is NEVER shown to the solver; the
     solver gets only symptom.md. The grader holds the key. An AI may ORCHESTRATE runs, but must not
     be the SOLVER that also sees the key — keep them in separate processes.
  4. GIT-BLOCKED: every arm runs with `git` denied — the clone has full history incl. the fix commit,
     so an unblocked agent just `git show`s the fix. `adoption` must report git attempts "all denied".
  5. the tool's full docs (initiate.md + skill.md) are shoved into the prompt + a CLI tool is told to
     run it FIRST — a tool merely listed in a skill gets ~5–13% adoption (≈ untested).

EXAMPLES
  monobench list
  monobench show bun-1.3.10-toThreadSafe
  monobench run  bun-1.3.10-toThreadSafe baseline 1
  monobench run  bun-1.3.10-toThreadSafe monogram 1
  monobench report bun-1.3.10-toThreadSafe
  monobench add  myorg-repo-vX-shortname     # then edit the 3 files

ENV (axes: model × EFFORT × tool × runner)
  MONOBENCH_MODEL=opus|sonnet|haiku     MONOBENCH_EFFORT=low|medium|high|xhigh|max   MONOBENCH_CAP=6 (USD/run)
  MONOBENCH_RUNNER=claude-p|niia        MONOBENCH_CLI=claude|codex|gemini (niia only) MONOBENCH_ISOLATE=worktree (⇒ parallel)
  MONOBENCH_WORK=…/monobench-work       MONOBENCH_CODEGRAPH='node …/codegraph.js'
  effort → claude `--effort`, codex `-c model_reasoning_effort=`. The result label encodes model+effort,
  so e.g. `monobench run <id> monogram` with MODEL=codex(niia)/EFFORT=xhigh → results/monogram-codex-xhigh-r1.

Instances are DATA (instances/<id>/). Repos-under-test are cloned on demand at the pinned tag.
Methodology + admission criteria: SPEC.md. Backlog of bugs to add: CANDIDATES.md.
