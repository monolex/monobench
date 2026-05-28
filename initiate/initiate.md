monobench — code-intelligence-tool benchmark
=============================================
Does giving an AI agent a code-intelligence tool (monogram, codegraph, …) actually help it find the
ROOT CAUSE of a real bug — vs a baseline told to dig just as hard — when the symptom and the cause
are linked by STRUCTURE (call graph / ownership), not TEXT?  "SWE-bench, for tool comparison."

Monobench is public-shaped, but its first customer is the Monolex recursive tool-development loop:
preserve evidence, compare success/failure rails, propose generalized monogram/NIIA/monokist
improvements, and keep answer-key literals out of solver prompts. Canonical flow:
research/indexes/loop-flow.md.

USAGE
  monobench <command> [args]

COMMANDS
  list                       List benchmark instances (id + title).
  tools                      List tool adapters you can run as arms (baseline/monogram/monogram-mcp/
                             codegraph/yours; legacy controls may also appear).
  status <id> [--detail]     Runs in chronological order (oldest→newest), each with its UTC start time
                             ("MM-DD HH:MMZ"; "—" = legacy run with no embedded start time), state,
                             active phase, workers. --detail adds file size/age.
                             Shows the active matrix (pid · cli/model · elapsed) for this instance.
                             Each running run shows `· idle Ns` (`· ⚠ idle Ns` if its output log has
                             stalled ≥5m) — the real "alive vs hung" signal (CPU sits ~0 while reasoning).
                             --live [--every N] [--count N] refreshes recursively for a terminal pane.
  watch [--live]             Global status across instances (done/FULL/running + active workers).
                             Shows the running matrix (pid · cli/model · elapsed) — no ad hoc tail/pgrep.
                             --live [--every N] [--count N] refreshes recursively for a terminal pane.
                             Works from another cwd by remembering or inferring the real result root.
  stop                       Gracefully stop an active matrix after in-flight runs finish.
  clean <id> [arm-prefix]    Delete recorded runs (all, or just one arm e.g. `monogram`) + scratch dirs.
                             Use before a fresh re-bench so old/stale-config runs don't pollute results.
  show <id> [--spoil]        Print the task (symptom) given to the agent. --spoil reveals the gated
                             ground truth (NEVER pass --spoil output into a solver).
  run <id> <arm> [n|note...] Run ONE arm of ONE instance (clones repo@tag pristine, indexes, runs,
                             grades). n is legacy repeat index (default 1); non-numeric text becomes
                             the run note. Use --tag/--note for explicit metadata.
  prepare <id>               Pre-index non-baseline tools once; monogram arms snapshot the DB.
      [--tools a,b]          Default: monogram. Use before `matrix --prepared`.
  matrix <id>                Run a tool × repeat grid for exactly ONE CLI+model command.
      [--tools a,b]          Tool arms to compare, e.g. baseline,monogram.
      [--cli c]              CLI environment: claude | codex | agy | gemini | grok.
      [--model x]            Full model label for this command. Repeat the matrix command for the next model.
                             (agy: must match ~/.gemini/antigravity-cli/settings.json model, else refused.)
      [--via direct|niia]    direct by default; niia drives the CLI through the headless terminal.
      [--effort e]           Effort label and CLI-specific effort flag where supported.
      [--prepared]           Run `prepare` first. In worktree mode, copy the prepared monogram DB
                             snapshot into each run and skip per-run monogram indexing. Solver-side
                             monogram index/reindex/-r is guarded and should report
                             HARNESS_DB_MISMATCH if the DB is wrong.
      [--isolate shared|worktree]  worktree default. shared is single-lane and reuses the stable DB.
                             `--models x` is accepted as a compatibility alias, but only one value.
      [--runs N] [--jobs J]  Repeats per arm and parallel workers. Uses git-worktree isolation.
      [--tag T] [--note TXT] Store human experiment intent in results/<id>/<run>.meta.json.
  grade <id> [run]           Automatic grade for all runs (or one run) + review status/NEXT.
  judge <id> <run>           Build the final-judge prompt that sees answer + ground truth.
      [--model m] [--write]  --write stores results/<id>/<run>.judge.md; no model is called.
  review <id> <run>          Record final checked grade in results/<id>/<run>.review.json.
      --final GRADE          FULL|NAME_ONLY|DECOY|MISS|NO_RESULT|INVALID|FORFEIT.
      [--reason TEXT] [--judge-model m]
  inspect <id> <run>         Monomento-style run peek: artifact sizes/age, agy conversation,
                             live transcript fallback, event counts, active process, NEXT.
  note <id> <run>            Add/update human metadata for an existing run. Also available as `memo`.
      [note...]              Positional text becomes note. --tag/--note are explicit.
  integrity <id> [run]       Heuristic contamination-risk scan for benchmark validity. Scores
                             git leaks, DB surgery, lock/registry/index mutations, stale prepared DBs.
  evidence <id> [run] [pat]  Focused run evidence search. Replaces ad hoc `rg results/...err`.
                             Without <run>: an INDEX across every run (ans/tool/raw/notable hit
                             counts + each run's conclusion line), ranked so runs that concluded
                             on the pattern come first.
      [--pattern P]          P is pipe-separated OR text; `^term` means line-start.
      [--max N]              Cap results: index rows (default 40), or single-run matches (default 80).
      [--context N]          Single-run only: print N context lines around each raw match.
      [--case]               Case-sensitive match (default is case-insensitive). Works in both modes.
      [--include-prompt]     Include prompt/tool-doc preamble; default raw source matches start at
                             the first observed solver tool command.
  trace <id> <run> [max]     Ordered tool-call timeline of ONE run ([M]onogram/[g]rep/[git] marked).
                             For agy, falls back from .agy.jsonl to live transcript_full.jsonl.
  export <id> <run>          Render one run's full evidence transcript/log to
                             results/<id>/<run>.md for monomento indexing/search.
  report <id> [--since DUR]  Per-CLI/MODEL comparison: FULL Hit-rate, median $/tokens/time, mono% adoption.
                             Per-run rows are chronological (oldest→newest) with a "started" column (UTC
                             start time); failures (FORFEIT/NO_RESULT) listed separately at the bottom.
                             --since 9h|30m|2d windows to runs STARTED in the period (label -t<ms>,
                             mtime fallback for legacy labels) — isolate one session from all-time totals.
  summary [--since DUR]      Cross-INSTANCE leaderboard: FULL hit-rate + median wall time per arm.
  column <arm> [--since DUR] ONE arm's verified grade breakdown across every instance: per-instance +
                             total FULL/MISS/DECOY/NAME_ONLY/INVALID counts, root-cause hit-rate, and
                             review coverage (judged vs unreviewed). The judged detail behind `summary`;
                             <arm> is a full arm name e.g. baseline-codex-gpt-5.4-mini-low.
  adoption <id>              Per-run tool-call + monogram-subcommand breakdown (calls/share/first-use/
                             fails/mix) — for CLI and MCP delivery. "Did the agent actually use it?"
  monogram-audit <id>        Diagnose monogram command/result failure patterns in solver telemetry
      [--tag T] [--run RUN]  Optional filters for one experiment tag or one resolved run label.
                             and print maker recommendations for general score/proof/budget/NEXT
                             changes (not solver hints).
  meter <session.jsonl>      Summarize tokens/cache/cost for a raw model session JSONL.
  add <id>                   Scaffold a new instance from instances/_TEMPLATE/.
  version                    Print the monobench version.
  help                       This text.

ARMS = TOOL ADAPTERS (pluggable — define your own)
  Each tool is a drop-in adapter: harness/tools/<name>/tool.json
    { index_steps: [{command,args}], skill: "<skill.md to inject, or ''>",
      deliver: "none|cli|mcp", mcp: {command,args with ${REPO}/${CODEGRAPH}}, forfeit_grep: "<regex>" }
  Shipped adapters:
    baseline      index_steps:[] deliver:none  — control (builtins only). The admission gate.
    monogram      monogram index .        cli  — CLI; skill leads with structural cmds, "run first".
    monogram-mcp  monogram index .        mcp  — SAME index as forced MCP tools (monogram serve).
                                                 Use only when delivery/interaction is the suspected variable.
    codegraph     codegraph init+index    mcp  — first-class MCP tools; FORFEITs if it can't index (Zig OOM).
  Legacy/diagnostic controls:
    monogram-thin monogram index .        cli  — prompt-load control only. Do not use as the default
                                                 recursive-loop axis; start with real monogram.
  Add a tool:  cp -r harness/tools/_TEMPLATE harness/tools/<name> && edit tool.json (+ skill.md)
  EVERY arm gets the same depth directive (prompts/depth.md); only the tool differs.

CLI / MODEL AXES
  --cli claude   direct `claude -p --model <model>` (cost/tokens from stream-json). Default for Claude aliases.
  --cli codex    direct `codex exec -m <model>`; effort → model_reasoning_effort.
  --cli agy      direct `agy --print` (no --model flag in print mode → the model is whatever
                 ~/.gemini/antigravity-cli/settings.json says). PREFLIGHT refuses the run if that
                 configured model != --model; post-run observed_model is verified, so model_enforced
                 reflects the real match (effort stays label-only). The repo is handed to agy via
                 --add-dir (it ignores cwd), and reads are jailed with sandbox-exec on macOS so agy
                 cannot read the gated ground truth. Cost/tokens unavailable.
  --cli grok     direct `grok -p <prompt> --cwd <clone> --model grok-build --output-format json`
                 (single model grok-build, OAuth/subscription). No per-turn token split or cost →
                 tokens/cost_usd null, *_available false; meter carries honest session metrics from
                 ~/.grok/sessions/<cwd>/<sessionId>/signals.json (turns, tool_calls, context_tokens_used,
                 duration, ttft), found via the sessionId in grok's JSON envelope. effort is label-only.
  --via niia     drive the selected CLI through the niia headless terminal (write/wait-idle/get-answer);
                 picks a live ATTACHED session (detached zombies are skipped). For agy it runs
                 `agy --print` with --dangerously-skip-permissions + --add-dir + the sandbox-exec
                 read-jail, and waits for completion via a sentinel file (no premature empty capture).
                 NOTE: agy answer-capture from the PTY is still lossy vs --via direct — prefer direct
                 for agy.
                 For custom niia commands, MONOBENCH_CLI can override the spawn command.
  Result labels are `<tool>-<cli>-<model>-<effort>-rN-t<start_ms>`, e.g.
    monogram-agy-claude-opus-4.1-low-r1-t1779581234567
  Legacy `<tool>-<cli>-<model>-<effort>-rN` labels still read normally. For trace/grade/export,
  an untimestamped prefix resolves only when it matches exactly one timestamped run.

METRIC
  root-cause Hit-rate (FULL) · final-review status · tokens-per-correct-root-cause · tool-call
  count · tool-ADOPTION (a tool the agent never called was not tested) · FORFEIT.
  Two-stage grading: `grade` is deterministic and automatic; `judge` builds an answer-key-aware
  final-grader prompt for the orchestrating LLM or a human; `review` records the checked result in
  `.review.json`. `judge` itself does not call a model, so the solver process stays separate from
  the reviewer that may see ground truth. Final benchmark truth should use checked `.review.json`.

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

FLOW  (every command ends with [NEXT]; no command dead-ends — monogram-style discovery graph)
  list ──→ show ──→ run ──→ grade ──→ judge / review
    └──→ status ──→ report ──→ summary ──→ column <arm>
           └──→ watch --live
  tools ──→ run / matrix
  {trace · adoption · monogram-audit} ──→ evidence ──→ export / integrity / trace

  compare tool vs baseline:     run <id> baseline → run <id> monogram → report <id>
  investigate a MISS:           report <id> → evidence <id> <run> --pattern ROOTCAUSE → trace <id> <run> → export <id> <run>
  diagnose monogram loop:       monogram-audit <id> → read MAKER RECOMMENDATIONS → evidence <id> --pattern 'region_first_next|systems_lifecycle_next|ROOTCAUSE' → classify path-not-closed vs closed-but-uncalibrated
  validate before counting:     integrity <id> → inspect <id> <run> → rerun if contaminated
  scan conclusions (all runs):  evidence <id> --pattern ROOTCAUSE → evidence <id> <run>
  watch live runs:              matrix <id> … → watch --live  /  status <id> --live
  cross-instance leaderboard:   summary → column <arm> → report <id>
  verify one arm fully judged:  column <arm>   (per-instance FULL/MISS/DECOY/INVALID + judged/unreviewed)
  isolate one session's score:  report <id> --since 9h   (all-time totals conflate old arms/configs)

EXAMPLES
  monobench list
  monobench show bun-1.3.10-toThreadSafe
  monobench status bun-1.3.10-toThreadSafe
  monobench watch --live --every 5
  monobench status bun-1.3.10-toThreadSafe --live --every 5 --detail
  monobench run  bun-1.3.10-toThreadSafe baseline "quick baseline sanity"
  monobench run  bun-1.3.10-toThreadSafe monogram --tag lockfix --note "prepared index lock fix 재검증"
  monobench prepare bun-1.3.10-toThreadSafe --tools monogram
  monobench matrix bun-1.3.10-toThreadSafe --tools baseline,monogram --cli claude --model haiku --runs 3 --jobs 2
  monobench matrix bun-1.3.10-toThreadSafe --tools baseline,monogram --cli codex --model gpt-5.3-codex-spark --effort high --runs 2 --prepared --tag lockfix-spark --note "lock+grep-probe 이후 재검증"
  monobench matrix bun-1.3.10-toThreadSafe --tools baseline,monogram --cli codex --model gpt-5.4-mini --effort low --runs 2 --jobs 2
  monobench matrix ksmbd-37899 --tools baseline,monogram --cli agy --model gemini-3.5-flash-low --runs 2 --jobs 1 --prepared --tag flashlow-ksmbd
  monobench matrix bun-1.3.10-toThreadSafe --tools baseline,monogram --cli agy --model gemini-3.5-flash-medium --runs 2 --jobs 2   # agy model = ~/.gemini/antigravity-cli/settings.json (must match --model)
  monobench report bun-1.3.10-toThreadSafe
  monobench judge  bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567 --model gpt-5.5 --write
  monobench review bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567 --final FULL --reason "root cause and mechanism match" --judge-model gpt-5.5
  monobench inspect bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567
  monobench note bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567 --tag suspect --note "registry race 의심; 실패 분석용"
  monobench integrity bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567
  monobench evidence bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567 --pattern '^/bin/zsh -lc|ROOTCAUSE|StringImpl::isolatedCopy'
  monobench trace  bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567
  monobench export bun-1.3.10-toThreadSafe monogram-codex-gpt-5.4-mini-low-r1-t1779581234567
  monomento index . --project
  monomento search "rootcause decoy monogram grep" --project --read --h2
  monomento peek monogram-codex-gpt-5.4-mini-low-r1-t1779581234567.md --project
  monobench add  myorg-repo-vX-shortname     # then edit the 3 files

RUN ANALYSIS MEMORY
  Use `inspect` before tailing raw logs. It shows artifact sizes/age, agy live transcript fallback,
  event counts, active process hints, observed model, grade/review status, and the next action.
  Use `integrity` before counting a run in benchmark stats. It gives a contamination probability
  score from observed signals: git history access, solver-side sqlite3/lock/registry/index surgery,
  tool process kills, monogram re-indexing, stale prepared DB path/mtime anomalies, and missing
  telemetry. It is not a proof; high scores mean "keep for failure analysis and rerun".
  Use `evidence` when you would otherwise run `rg -n` against a provider log or pipe audit output to
  `tail`. With a <run> it resolves the label, picks the right transcript/log, and shows matching tool
  calls together with raw line-numbered evidence from source, answer, and index artifacts. Without a
  <run> it scans every run and prints an index — answer/conclusion hits (`ans`), matching tool calls
  (`tool`), source+index log hits (`raw`), state/process calls (`notable`) — so you can see which
  runs concluded on the pattern before drilling into one. Like `integrity`, it never mutates anything.
  Use `trace` for a compact ordered tool-call timeline. It prefers structured events, including
  `.agy.jsonl` and live agy `transcript_full.jsonl`, before falling back to stderr logs.
  Use `export` when a run should become reusable evidence. It writes a verbose markdown transcript
  under results/<id>/<run>.md; then run `monomento index . --project` and search/peek it later.
  The export is intentionally full evidence, not a compact memory summary. If a run needs a small
  durable finding, write a separate analysis note under research/ and index that too.

ENV (axes: tool × CLI × model × effort)
  --cli / MONOBENCH_CLI_NAME=claude|codex|agy|gemini
  --via / MONOBENCH_VIA=direct|niia
  --model / MONOBENCH_MODEL=opus|sonnet|haiku|claude-opus-4.1|gpt-5.4-mini|gemini-3.5-flash-low|...
  MONOBENCH_EFFORT=low|medium|high|xhigh|max   MONOBENCH_CAP=6 (USD/run)
  MONOBENCH_RUNNER=claude-p|codex|agy|niia      legacy compatibility only
  MONOBENCH_CLI='codex -m gpt-5.4-mini'         niia custom spawn override
  MONOBENCH_CODEX_MODEL=<same as --model>       MONOBENCH_AGY_TIMEOUT=20m
  MONOBENCH_ISOLATE=worktree (matrix sets this) MONOBENCH_WORK=…/monobench-work
  MONOBENCH_CODEGRAPH='node …/codegraph.js'
  effort → claude `--effort`, codex `-c model_reasoning_effort=`. Keep one CLI+model per matrix command.

Instances are DATA (instances/<id>/). Repos-under-test are cloned on demand at the pinned tag.
Methodology + admission criteria: SPEC.md. Backlog of bugs to add: CANDIDATES.md.
