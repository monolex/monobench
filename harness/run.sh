#!/usr/bin/env bash
# monobench runner — run ONE instance under ONE tool. The TOOL is a drop-in adapter (tools/<arm>/tool.json),
# so baseline / monogram / codegraph / <your-own-tool> all run the same way.
#   run.sh <instance-id> <tool> [run-no]        tool = any dir under harness/tools/
# Env: MONOBENCH_MODEL=opus  MONOBENCH_CAP=6  MONOBENCH_WORK=/tmp/monobench-work
#      MONOBENCH_RUNNER=claude-p|niia   MONOBENCH_CLI=claude|codex|gemini (niia)   MONOBENCH_CODEGRAPH='node …/codegraph.js'
#      MONOBENCH_ISOLATE=worktree   → per-run git worktree (own working tree + index) ⇒ PARALLEL-SAFE
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ID="${1:?usage: run.sh <instance-id> <tool> [run-no]}"; ARM="${2:?tool (see: ls harness/tools)}"; RUN="${3:-1}"
INST="$ROOT/instances/$ID"; [ -d "$INST" ] || { echo "no instance '$ID'"; exit 1; }
TOOLDIR="$ROOT/harness/tools/$ARM"; [ -f "$TOOLDIR/tool.json" ] || { echo "no tool adapter '$ARM' (see harness/tools/_TEMPLATE)"; exit 1; }
field(){ node -e "try{const v=require('$TOOLDIR/tool.json');console.log(v['$1']??'')}catch(e){console.log('')}"; }

REPO_URL=$(node -e "console.log(require('$INST/instance.json').repo)")
TAG=$(node -e "console.log(require('$INST/instance.json').tag)")
WORK="${MONOBENCH_WORK:-/tmp/monobench-work}"; mkdir -p "$WORK"
export CODEGRAPH="${MONOBENCH_CODEGRAPH:-codegraph}"
OUT="$ROOT/results/$ID"; mkdir -p "$OUT"
# result label includes the model + effort so model×effort×tool runs don't collide (opus/default stay bare)
LABEL="$ARM"
[ "${MONOBENCH_MODEL:-opus}" != "opus" ] && LABEL="$LABEL-${MONOBENCH_MODEL}"
[ -n "${MONOBENCH_EFFORT:-}" ] && LABEL="$LABEL-${MONOBENCH_EFFORT}"
RUNID="$LABEL-r$RUN"

# portable mkdir-lock (serializes only the quick git-worktree add/remove across parallel runs)
_lock(){ until mkdir "$WORK/.wtlock" 2>/dev/null; do sleep 0.2; done; }
_unlock(){ rmdir "$WORK/.wtlock" 2>/dev/null; }

# 1. repo: shared clone (default) OR per-run git worktree (MONOBENCH_ISOLATE=worktree ⇒ parallel-safe)
if [ "${MONOBENCH_ISOLATE:-}" = "worktree" ]; then
  BASE="$WORK/$(basename "$REPO_URL" .git)-base"
  _lock
  [ -d "$BASE/.git" ] || git clone --filter=blob:none --quiet "$REPO_URL" "$BASE"
  git -C "$BASE" worktree prune 2>/dev/null
  _unlock
  WT="$WORK/wt/${RUNID}-$$"; rm -rf "$WT"; mkdir -p "$WORK/wt"
  _lock; git -C "$BASE" worktree add --quiet --force --detach "$WT" "$TAG" 2>/dev/null; _unlock
  export REPO="$WT"; CLONE="$WT"
  cleanup(){ _lock; git -C "$BASE" worktree remove --force "$WT" 2>/dev/null; rm -rf "$WT"; _unlock; }
  trap cleanup EXIT
else
  export REPO="$WORK/$(basename "$REPO_URL" .git)"; CLONE="$REPO"
  [ -d "$CLONE/.git" ] || { echo "cloning $REPO_URL …"; git clone --filter=blob:none --quiet "$REPO_URL" "$CLONE"; }
  ( cd "$CLONE" && git checkout --quiet "$TAG" 2>/dev/null && git checkout -- . )
fi

# 2. tool adapter: index the repo for the tool (+ FORFEIT if it can't)
INDEX=$(field index); SKILL=$(field skill); DELIVER=$(field deliver); FGREP=$(field forfeit_grep)
echo "▶ $ID / $LABEL r$RUN  (deliver=${DELIVER:-none}, runner=${MONOBENCH_RUNNER:-claude-p}, isolate=${MONOBENCH_ISOLATE:-shared})"
if [ -n "$INDEX" ]; then
  IDXLOG=$( cd "$CLONE" && bash -c "$INDEX" 2>&1 )
  if [ -n "$FGREP" ] && echo "$IDXLOG" | grep -qiE "$FGREP"; then
    echo "  FORFEIT — '$ARM' could not index this repo" | tee "$OUT/$RUNID.forfeit"; exit 0
  fi
fi

# 3. PROMPT PREAMBLE = the tool's docs DUMPED IN, then the shared depth directive.
#    For monogram: BOTH initiate.md (full command reference) AND skill.md are shoved straight into the
#    -p command prompt (below) so the agent reads the whole tool surface before it does anything.
SYS="$(cat "$ROOT/harness/prompts/depth.md")"
[ -n "$SKILL" ] && [ -f "$TOOLDIR/$SKILL" ] && SYS="$(cat "$TOOLDIR/$SKILL")

$SYS"
[ -f "$TOOLDIR/initiate.md" ] && SYS="$(cat "$TOOLDIR/initiate.md")

$SYS"
# lead.md (if any) goes at the VERY TOP — e.g. monogram's "type `monogram` in the terminal FIRST"
[ -f "$TOOLDIR/lead.md" ] && SYS="$(cat "$TOOLDIR/lead.md")

$SYS"

# 4. MCP config — per-run filename (parallel-safe)
MCPCFG="$OUT/mcp-empty-$RUNID.json"; echo '{"mcpServers":{}}' > "$MCPCFG"
if [ "$DELIVER" = "mcp" ]; then
  MCPCFG="$OUT/mcp-$RUNID.json"
  node -e "const v=require('$TOOLDIR/tool.json').mcp||{};const sub=s=>String(s).replace(/\\\${REPO}/g,process.env.REPO).replace(/\\\${CODEGRAPH}/g,process.env.CODEGRAPH);require('fs').writeFileSync('$MCPCFG',JSON.stringify({mcpServers:{['$ARM']:{command:sub(v.command),args:(v.args||[]).map(sub)}}}))"
fi

Q="$(cat "$INST/symptom.md")"
case "${MONOBENCH_RUNNER:-claude-p}" in
  niia)  # interactive model CLI via the niia headless terminal (off `-p`); metered by monometer
    PF=$(mktemp); printf '%s\n\n%s\n' "$SYS" "$Q" > "$PF"
    "$ROOT/harness/runners/niia.sh" "$CLONE" "$PF" "ROOTCAUSE" "$OUT/$RUNID"
    "$ROOT/bin/monobench" grade "$ID" "$RUNID"
    ;;
  codex) # headless `codex exec` (codex's -p equivalent); answer via -o; metered by monometer (codex)
    PF=$(mktemp); printf '%s\n\n%s\n' "$SYS" "$Q" > "$PF"; ANS="$OUT/$RUNID.answer.txt"; T0=$SECONDS
    ( env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT -u CLAUDE_CODE_SESSION_ID -u CLAUDE_EFFORT -u AI_AGENT -u CLAUDE_CODE_EXECPATH \
      codex exec -C "$CLONE" --skip-git-repo-check --dangerously-bypass-approvals-and-sandbox \
        ${MONOBENCH_CODEX_MODEL:+-m "$MONOBENCH_CODEX_MODEL"} -c model_reasoning_effort="${MONOBENCH_EFFORT:-high}" \
        -o "$ANS" < "$PF" > "$OUT/$RUNID.codexlog" 2>"$OUT/$RUNID.err" )
    MB_DUR=$((SECONDS-T0)); monometer daemon recompute >/dev/null 2>&1; sleep 1
    MB_DUR=$MB_DUR monometer sessions --provider codex --recent 3 --json 2>/dev/null | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{const x=(JSON.parse(s)||[])[0]||{};process.stdout.write(JSON.stringify({tokens:x.total_tokens||null,cost_usd:x.cost_usd||null,duration_s:+process.env.MB_DUR||null,model:(x.models||[])[0]||"codex"}))}catch(e){process.stdout.write("{}")}})' > "$OUT/$RUNID.meter.json"
    "$ROOT/bin/monobench" grade "$ID" "$RUNID"
    ;;
  *)     # headless claude -p
    F="$OUT/$RUNID.jsonl"
    PROMPT="$SYS

════════════════════════════════════════════════════════════════════════════════
# YOUR TASK
$Q"
    ( cd "$CLONE" && env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT -u CLAUDE_CODE_SESSION_ID -u CLAUDE_EFFORT -u AI_AGENT -u CLAUDE_CODE_EXECPATH \
      claude -p "$PROMPT" --output-format stream-json --verbose --permission-mode bypassPermissions \
        --model "${MONOBENCH_MODEL:-opus}" ${MONOBENCH_EFFORT:+--effort "$MONOBENCH_EFFORT"} --max-budget-usd "${MONOBENCH_CAP:-6}" \
        --setting-sources '' --disable-slash-commands --strict-mcp-config --mcp-config "$MCPCFG" \
        --disallowedTools "Bash(git:*)" \
        > "$F" 2>"$OUT/$RUNID.err" )
    "$ROOT/bin/monobench" grade "$ID" "$RUNID"
    ;;
esac
