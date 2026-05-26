# Grok CLI — monometer / monobench Support Research

**Date:** 2026-05-26
**CLI under study:** `grok` 0.1.219 ("Grok Build TUI"), xAI official, installed today at `~/.grok/bin/grok` (Mach-O arm64; also linked in `~/.local/bin/grok`).
**Method:** Live machine inspection of `~/.grok/`, the help/subcommand surface, one minimal headless `grok -p` probe, plus source mapping of monometer (`rust-src/monometer-daemon/`, `tauri-apps/lib-monometer*`) and monobench (`monologue/demo/monobench/src/`).
**Note:** A real interactive grok session was open during research (session `019e62c8-…b58b`, cwd app-monolex) — used as the primary real-data sample. A throwaway probe session `019e62dc-…bb05` (cwd `/tmp/grok-probe`) was created by the headless probe.

---

## TL;DR (the one finding that reframes everything)

> **Grok is NOT a per-token / per-cost CLI like Claude Code or Codex.**
> It authenticates by **OAuth session (grok.com / SuperGrok subscription)**, exposes **one model (`grok-build`)**, and **persists ZERO cost and ZERO per-message input/output/cache token breakdown** locally. `grep` across all of `~/.grok` finds no `input_tokens`/`output_tokens`/`prompt_tokens`/`completion_tokens`/`cost_usd`/`cache_*` keys — none.
>
> What grok DOES give us is **session-level metrics** (`signals.json`): `contextTokensUsed` (live context occupancy, not billed I/O), `turnCount`, `toolCallCount`, `toolsUsed`, `sessionDurationSeconds`, TTFT/latency, lines added/removed. Plus a clean **headless JSON envelope** (`text` + `sessionId`).

**Consequence:**
- **monobench**: easy + well-shaped. grok has `-p/--single` headless with `--output-format json` → clean answer capture. Telemetry = **agy-style "tokens/cost unavailable"**, but *enrichable* with real `signals.json` metrics (tool calls, turns, duration, context size) keyed by the returned `sessionId`.
- **monometer**: a **design fork**. monometer's whole model is `entries(input_tokens, output_tokens, cache_*, cost_usd)` summed per message → cost via pricing table. Grok provides none of that granularity and no cost. grok is the first **"metrics-only, cost-less"** provider and does not fit the existing token/cost schema cleanly. See [§5](#5-monometer-integration--the-design-fork).

---

## 1. Grok CLI at a glance (all evidence-backed)

| Aspect | Finding | Source |
|---|---|---|
| Binary | `~/.grok/bin/grok`, Mach-O arm64, v`0.1.219` (`c9b7cdec23a`) | `which`, `file`, `--version` |
| Branding | "Grok Build TUI" | `--help` |
| Auth | OAuth **session** via `auth.x.ai` (`grok.com`), bearer | `auth.json`, `models_cache.json` (`auth_method:"session"`), `grok models` ("logged in with grok.com") |
| Models | **only `grok-build`** (default). `context_window: 512000`, `base_url: cli-chat-proxy.grok.com/v1`, `agent_type: grok-build-plan`, `supports_reasoning_effort: **false**` | `models_cache.json`, `grok models` |
| Billing | No per-token cost anywhere (subscription) | `grep` (§4) |
| Config | `~/.grok/config.toml` → `permission_mode="always-approve"`, `fork_secondary_model="grok-build"`, `yolo=false`, `compact_mode=false` | `config.toml` |
| Self-positioning | Help text explicitly maps flags to **"Claude Code:"** equivalents (`--allow`↔`--allowedTools`, `--system-prompt-override`↔`--system-prompt`). Session store mirrors Claude Code's URL-encoded-cwd layout. | `--help`, `~/.grok/sessions/` |

### 1.1 Invocation modes (what we can drive non-interactively)

- **`grok -p/--single <PROMPT>`** — single-turn, prints response to stdout, exits. ← **monobench answer path**
- `--prompt-file <PATH>`, `--prompt-json <JSON>` — alternate single-turn inputs.
- **`--output-format plain|json|streaming-json`** (default `plain`). ← `json` gives a parseable envelope.
- `-m/--model <MODEL>`, `--effort low|medium|high|xhigh|max`, `--reasoning-effort <EFFORT>`.
- `--cwd <CWD>`, `-c/--continue`, `-r/--resume [SID]`.
- Permissions: `--allow/--deny/--always-approve/--disallowed-tools/--tools`, `--permission-mode default|acceptEdits|auto|dontAsk|bypassPermissions|plan`, `--sandbox <PROFILE>` (env `GROK_SANDBOX`).
- Agents: `--agent/--agents/--no-subagents/--max-turns`, `--best-of-n <N>` (headless only), `--check` (self-verify, headless only).

**Subcommands** (`grok <cmd>`):
`agent` (→ `stdio` | `headless` | `serve` | `leader` — run without UI), `export` (session→Markdown), `import`, `inspect` (config discovery), `models`, `sessions` (`list`/`search`), `trace` (export/upload session trace), `mcp`, `memory`, `plugin`, `login`/`logout`, `setup`, `ssh`, `trace`, `update`, `worktree`, `completions`.

### 1.2 On-disk layout (`~/.grok/`)

```
~/.grok/
├── bin/grok                      # the binary
├── auth.json (+ .lock)           # OAuth tokens (DO NOT read/commit)
├── config.toml                   # permission_mode, default model
├── models_cache.json             # model catalog (grok-build only)
├── agent_id, version.json, CHANGELOG.{json,md}
├── logs/
│   ├── unified.jsonl             # app logs: turn.phase_transition, shell.turn.inference_*, auth
│   └── mcp/
├── sessions/
│   ├── session_search.sqlite     # FTS5 search index ONLY (session_docs + _fts). No tokens.
│   └── %2FUsers%2F…%2Fapp-monolex/        # URL-encoded cwd (Claude-Code-style)
│       ├── prompt_history.jsonl
│       ├── permission_*.toml
│       └── <session-uuid>/                # ONE dir per session
│           ├── chat_history.jsonl         # conversation (assistant/user/tool_result/system)
│           ├── events.jsonl               # phase_changed, tool_started/completed, permission_*, first_token, turn_*
│           ├── updates.jsonl              # streaming updates; carries running `totalTokens`
│           ├── signals.json               # ★ per-session metrics (see §4)
│           ├── summary.json               # ★ session metadata (model, counts, git, request_id)
│           ├── system_prompt.txt, prompt_context.json
│           ├── plan_mode.json, rewind_points.jsonl, resources_state.json, announcement_state.json
│           └── terminal/call-<uuid>-N.log # per-tool-call terminal output
├── skills/  bundled/  marketplace-cache/  completions/  docs/  downloads/  worktrees.db  upload_queue/
```

---

## 2. Headless JSON envelope (the monobench answer source)

`grok -p "Reply with exactly: ok" --cwd /tmp/grok-probe --output-format json --always-approve --no-subagents` →

```json
{
  "text": "ok",
  "stopReason": "EndTurn",
  "sessionId": "019e62dc-1535-7be1-9771-3850ed8dbb05",
  "requestId": "df1dc71f-ca9d-474d-a08f-75d9c5ff7801",
  "thought": "The user query is: \"Reply with exactly: ok\"\n"
}
```

- `text` → **the answer** (write to `{runid}.answer.txt`; grade as usual).
- `sessionId` → **locate `~/.grok/sessions/<enc-cwd>/<sessionId>/signals.json`** for telemetry.
- **No tokens, no cost** in the envelope (matches the on-disk reality).

---

## 3. Transcript shape (for tool-event grading, if wanted)

`chat_history.jsonl` lines (135 in the sample), `type` ∈ {`assistant`, `user`, `tool_result`, `system`}:

- `assistant`: keys `type, content, reasoning{text, encrypted}, tool_calls, model_id, model_fingerprint` (reasoning text is partially **encrypted**; final answer is in `content`).
- `tool_result`: `type, tool_call_id, content`.
- `user`: `type, content`.

`events.jsonl` (6755 lines) is the structured event stream: `phase_changed` (6354), `tool_started`/`tool_completed` (70 each), `permission_requested`/`permission_resolved` (70), `first_token` (54), `turn_started`/`turn_ended` (5). → **Tool-call counting for monobench can come straight from `signals.json.toolCallCount`/`toolsUsed`** without writing a JSONL parser.

---

## 4. Telemetry reality — available vs. NOT

**Confirmed by `grep -rIE '"(input_tokens|output_tokens|prompt_tokens|completion_tokens|cost_usd|total_cost|cache_creation|cache_read_input)"' ~/.grok` → ZERO matches. No `cost`/`usd`/`price` quoted keys anywhere either.**

### `signals.json` — the metrics goldmine (per session)
```
turnCount, userMessageCount, assistantMessageCount,
contextWindowUsage (%), contextTokensUsed, contextWindowTokens (512000),
toolCallCount, toolsUsed[], modelsUsed[], primaryModelId,
sessionDurationSeconds, avgTimeToFirstTokenMs, avgResponseTimeMs,
minTimeToFirstTokenMs, maxTimeToFirstTokenMs, totalChunkCount, itl* (inter-token latency),
agentLinesAdded/Removed, agentFilesTouched, humanLines*, peakRssBytes,
doomLoop* (loop detection), gcsQueue* (trace upload to Google Cloud Storage),
compactionCount, totalTokensBeforeCompaction, errorCount, toolFailureCount
```
Live session sample: `contextTokensUsed:131264 / 512000 (25%)`, `turnCount:7`, `toolCallCount:70`, `sessionDurationSeconds:1103`.
Probe (1-turn "ok") sample: `contextTokensUsed:18103`, `turnCount:1`, `toolCallCount:0`, `sessionDurationSeconds:4`, `avgTimeToFirstTokenMs:1710`.

⚠️ **`contextTokensUsed` ≠ billed tokens.** It is *current context-window occupancy*. The 1-turn "ok" probe already shows 18 103 — that's system prompt + 6 project instruction files (`grok inspect`: CLAUDE.md×, Agents.md×… ~20k tokens) + the tiny exchange. It grows with conversation length; it is **not additive across turns** and is **not** a sum of input+output the way Claude's per-message `usage` is.

### `summary.json` — session metadata
`id, cwd, session_summary, created_at, updated_at, num_messages, num_chat_messages, current_model_id (grok-build), next_trace_turn, chat_format_version, git_root_dir, git_remotes, head_commit, head_branch, request_id, agent_name (grok-build-plan)`. No tokens/cost.

### `updates.jsonl` — running `totalTokens`
Each update event carries a cumulative `totalTokens` (e.g. 25 020) = running context counter. Same caveat as `contextTokensUsed`.

### Official surfaces
- `grok sessions list [-n N]` → table: `SESSION ID | CREATED | UPDATED | STATUS | SUMMARY`. No tokens. (No `--json` flag.)
- `grok trace <SID> [--local] [-o PATH] [--json]` → tar.gz **bundling the session files** (summary/updates/events/chat_history/signals/terminal/…). Adds nothing beyond what's already on disk; `--local` avoids remote upload. `--json` = machine-readable *export metadata*, not usage.
- `grok export <SID>` → Markdown transcript (human-facing).

**Bottom line:** the richest machine-readable telemetry grok offers locally is **`signals.json`** (per session) — metrics, not cost.

---

## 5. monometer integration — the design fork

### 5.1 How monometer works today (verified)
- **Daemon** `rust-src/monometer-daemon/` parses each CLI's raw transcripts → libsql `~/.monometer/monometer.db`, table `entries(provider, message_id, request_id, timestamp, date, model, session_id, input_tokens, output_tokens, cache_read_input_tokens, cache_creation_5m_tokens, cache_creation_1h_tokens, web_search_requests, cost_usd, source_file)`, PK `(provider, message_id, request_id)`.
- **Providers** at `src/providers/`: `claude_code.rs, codex.rs, gemini.rs, opencode.rs, antigravity.rs`, dispatched in `main.rs`; default data dirs in `db.rs:155-160`:
  ```
  (".claude/projects","claude_code") (".config/claude/projects","claude_code")
  (".codex","codex") (".local/share/opencode","opencode")
  (".gemini/tmp","gemini") (".gemini/antigravity-cli","antigravity")
  ```
- **Readers** `tauri-apps/lib-monometer/` (`reader.rs` `Entry`, `types.rs`) + **analytics** `tauri-apps/lib-monometer-analytics/` (`cost.rs::rates_for_provider`, `pricing.rs`+`pricing.json`).
- **Binaries**: sibling `monometer-{claude,codex,gemini,opencode}` (just a `const PROVIDER` over shared command logic) + unified `monometer --provider …`.

### 5.2 The mismatch
monometer's value = **sum per-message input/output/cache tokens → cost**. Grok exposes **neither per-message tokens nor cost** — only per-**session** metrics. So:
- There is no honest `input_tokens`/`output_tokens`/`cost_usd` to populate. (monobench's own rule — see SPEC — is *never fake telemetry / no `0` without measurement*; same ethic should hold here.)
- `contextTokensUsed` is a context-occupancy gauge, not a billable total — storing it as `input_tokens` would be a lie.

### 5.3 Options (need a decision — see §7)

**Option A — first-class provider, cost-less, metrics-carrying.**
Add `grok` provider reading `~/.grok/sessions/**/signals.json` + `summary.json`. Write one row **per session** (not per message) with `cost_usd=NULL`, token columns left `0`/`NULL`, and surface grok's real metrics (`contextTokensUsed`, `turnCount`, `toolCallCount`, `sessionDurationSeconds`) — which needs **either** new nullable columns **or** a sibling `grok_sessions` table. Reuses daemon/watch/DB plumbing; diverges from the token/cost schema.

**Option B — separate "session metrics" surface, outside the token/cost model.**
Keep `entries` pure (token+cost CLIs only). Add a thin `monometer grok` read view (or a small `monometer-grok` reader) that reads `signals.json` directly and reports turns/tools/context/duration. No schema pollution; grok simply isn't a "cost" provider. Closest to SMPC.

**Option C — defer monometer; do monobench only.**
monobench gets full value now (answer + metrics); monometer waits until xAI exposes real usage (or we accept metrics-only).

> Recommendation: **B (or C first).** Grok's data is genuinely a different kind (metrics, not spend). Bending the cost schema to fit it (Option A) risks the exact "looks-like-support-but-lies" failure mode. A dedicated metrics view keeps each surface honest. If a unified UX is required, expose grok under `monometer` as a metrics-only provider that explicitly reports cost as N/A.

---

## 6. monobench integration — straightforward, well-shaped

### 6.1 Adapter contract today (verified `util.rs`, rest per code map)
String-dispatch, no trait. A CLI is: a token in `CLIS` (`util.rs:6`), a `("direct", <cli>)` arm in `run.rs` dispatch, a `command_words()` case in `niia_runner.rs`, a meter fn, optional telemetry parser, and answer capture (`{runid}.answer.txt` or `.jsonl`).

### 6.2 grok arm design
- **Command (direct):** `grok -p "<sys>\n\n<q>\n" --cwd <clone> --model grok-build --output-format json --always-approve --no-subagents` → stdout is the JSON envelope → parse `.text` into `{runid}.answer.txt`, keep envelope as `{runid}.grok.json`, store `sessionId`.
  - Use `--output-format json` (clean) rather than scraping plain stdout.
  - Keep the git-deny wrapper + `STRIP_ENV` like the other arms (anti-contamination).
- **Answer capture:** `run_answer_text()` already prefers `{runid}.answer.txt` → write `.text` there. No new branch needed.
- **Meter (`grok_meter`):** after the run, read `~/.grok/sessions/<enc-cwd>/<sessionId>/signals.json`.
  - `tokens: null`, `cost_usd: null`, `tokens_available:false`, `cost_available:false`, `meter_error:"grok exposes session metrics only; no per-turn token split or cost"`.
  - **Enrich** with real fields: `tool_calls: signals.toolCallCount`, `turns: signals.turnCount`, `context_tokens_used: signals.contextTokensUsed`, `duration_s`, `ttft_ms: signals.avgTimeToFirstTokenMs`, `observed_model: signals.primaryModelId`.
  - Locate session by the envelope's `sessionId` (deterministic) — better than "newest session" heuristics.
- **Model label:** `grok-build` (only model). Add `"grok"` to `CLIS` and a `grok`→`grok-build` default in `default_cli_for_model`/`is_extended_model_start`.
- **Effort caveat:** `--effort` is accepted but `grok-build` has `supports_reasoning_effort:false` — effort is likely a **no-op**. Record `effort_enforced:false` honestly.

### 6.3 monobench file checklist
1. `src/util.rs:6` — add `"grok"` to `CLIS`; add grok to `is_extended_model_start` (117-134) + `default_cli_for_model`.
2. `src/run.rs` — new `("direct","grok")` arm (model `grok-build`, json envelope → `.answer.txt`, capture `sessionId`); `grok_meter()` reading `signals.json`.
3. `src/niia_runner.rs` `command_words()` — `Some("grok") => { --model, (--effort) }` for the interactive/niia path.
4. `src/telemetry.rs` — optional: only if we want tool-event grading from `chat_history.jsonl`; otherwise `signals.toolCallCount` suffices.
5. Docs: `README.md` / `SPEC.md` / `initiate/initiate.md` — add grok arm + the "metrics-only, no cost" note.

---

## 7. Open decisions (need your call before touching shared source)

Both monometer (`providers/`, `db.rs`, `pricing.json`) and monobench (`util.rs` `CLIS`, `run.rs` dispatch) are **shared files edited by parallel sessions** — so I'm holding edits pending:

1. **monometer shape** — A (first-class cost-less provider, schema change) / **B (separate metrics view, recommended)** / C (defer monometer).
2. **Scope now** — implement monobench grok arm now (it's well-shaped and high-value), or keep this research-only?
3. **Effort axis** — keep `--effort` plumbed for grok (no-op today, future-proof) or omit it until grok-build supports reasoning effort?

---

## Appendix — exact reproduction commands (all read-only except the one probe)
```
grok --version ; grok --help ; grok models ; grok inspect
grok sessions list -n 5
grok trace <SID> --local -o /tmp/grok-trace.tar.gz   # bundles session files, no upload
# headless probe (the only API call made):
grok -p "Reply with exactly: ok" --cwd /tmp/grok-probe --output-format json --always-approve --no-subagents
# token/cost absence proof:
grep -rIE '"(input_tokens|output_tokens|prompt_tokens|completion_tokens|cost_usd|cache_creation|cache_read_input)"' ~/.grok   # → none
```
