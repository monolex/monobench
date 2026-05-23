#!/usr/bin/env bash
# monobench niia runner — drive a model CLI INTERACTIVELY via the niia headless terminal (off `claude -p`).
#   runners/niia.sh <repo_dir> <prompt_file> <marker> <out_prefix>
# Env: MONOBENCH_NIIA_SESSION (pty session id; auto-finds a live [ws] one if unset)
#      MONOBENCH_CLI=claude|codex|gemini   (the model CLI to spawn; default claude)
# Produces: <out_prefix>.answer.txt  and  <out_prefix>.meter.json (tokens + cache; cost best-effort)
set -uo pipefail
N=niia
HARNESS="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # …/harness
REPO="$1"; PF="$2"; MARKER="$3"; OUT="$4"
CLI="${MONOBENCH_CLI:-claude}"; EFF="${MONOBENCH_EFFORT:-}"
case "$CLI" in
  claude) SPAWN="claude${EFF:+ --effort $EFF}";;
  codex)  SPAWN="codex${EFF:+ -c model_reasoning_effort=$EFF}";;
  *)      SPAWN="$CLI";;
esac
S="${MONOBENCH_NIIA_SESSION:-}"
[ -z "$S" ] && S=$($N serve --list 2>/dev/null | awk '/\[ws\]/{for(i=1;i<=NF;i++) if($i ~ /^niia-/){gsub(/,/,"",$i); print $i; exit}}')
[ -z "$S" ] && { echo "no live niia [ws] session — start one with: niia serve"; exit 1; }
echo "▶ niia runner · session=$S · spawn='$SPAWN' · repo=$REPO"

# scope the model CLI to the repo (so its session JSONL is in the repo's claude project dir)
$N write --session "$S" "cd $REPO"$'\r' >/dev/null 2>&1; $N wait-idle --session "$S" >/dev/null 2>&1
SINCE=$(date +%s)

# spawn the model CLI, wait until it's idle at the prompt, clear a first dialog if any
$N write --session "$S" "$SPAWN"$'\r' >/dev/null 2>&1; sleep 3; $N wait-idle --session "$S" >/dev/null 2>&1; sleep 2
$N write --session "$S" $'\r'      >/dev/null 2>&1; $N wait-idle --session "$S" >/dev/null 2>&1; sleep 1

# type the task (single-lined) — then a SEPARATE Enter to actually submit (the trailing \r is absorbed)
$N write --session "$S" "$(tr '\n' ' ' < "$PF")"$'\r' >/dev/null 2>&1; sleep 2
$N write --session "$S" $'\r' >/dev/null 2>&1
$N wait-idle --session "$S" >/dev/null 2>&1; sleep 3

# read the answer (everything from the marker line to the bottom)
$N get-answer --session "$S" "$MARKER" 2>/dev/null | grep -ivE "^\[INFO\]|NEXT" > "$OUT.answer.txt"

# meter: the run's claude session = newest JSONL created during the run window
PROJ_GLOB="$HOME/.claude/projects"
F=$(find "$PROJ_GLOB" -name "*.jsonl" -newermt "@$SINCE" 2>/dev/null | head -1)
[ -z "$F" ] && F=$(ls -t "$PROJ_GLOB"/*/*.jsonl 2>/dev/null | head -1)
if [ -n "$F" ]; then
  "$HARNESS/../bin/monobench" meter "$F" > "$OUT.meter.json"
  SID=$(node -e "try{console.log(require('$OUT.meter.json').session_id)}catch(e){}" 2>/dev/null)
  monometer daemon recompute >/dev/null 2>&1; sleep 2
  COST=$(monometer sessions --recent 40 --json 2>/dev/null | node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{try{const x=JSON.parse(s).find(y=>y.session_id==='$SID');console.log(x?x.cost_usd:'pending')}catch{console.log('pending')}})")
  echo "  meter: $(cat "$OUT.meter.json")  cost_usd=$COST"
fi

# exit the model CLI (Ctrl-C ×2) → leave the session at a shell for the next run
$N write --session "$S" $'\x03' >/dev/null 2>&1; sleep 1; $N write --session "$S" $'\x03' >/dev/null 2>&1
echo "  answer → $OUT.answer.txt"
