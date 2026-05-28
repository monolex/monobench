#!/usr/bin/env zsh
set -u

cd /Users/macbook/Projects/monolex/monolex/monologue/demo/monobench

LOG=/Users/macbook/Projects/monolex/monolex/monologue/demo/monobench/research/indexes/monogram-0.61.31-24h-loop-2026-05-28.log
DEADLINE=$(( $(date +%s) + 86400 ))
TAG_PREFIX=haiku-v06131-24h-20260528

exec >> "$LOG" 2>&1

trap 'rc=$?; echo; echo "===== $(date "+%Y-%m-%d %H:%M:%S %Z") :: supervisor exit rc=$rc ====="' EXIT

active_solver_rows() {
  ps -axo pid=,comm=,args= | awk -v me="$$" -v ignore="${IGNORE_PIDS:-}" '
    BEGIN {
      split(ignore, ignored, " ")
      for (i in ignored) {
        if (ignored[i] != "") {
          skip[ignored[i]] = 1
        }
      }
    }
    $1 == me { next }
    $1 in skip { next }
    $2 == "monobench" && ($0 ~ / sweep / || $0 ~ / matrix /) { print; next }
    $2 == "claude" && $0 ~ / -p / { print; next }
    $2 == "monogram" && $0 ~ /search variant compare signature/ { print; next }
  '
}

log_header() {
  echo
  echo "===== $(date '+%Y-%m-%d %H:%M:%S %Z') :: $1 ====="
}

summarize_instance() {
  local id="$1"
  log_header "report $id"
  monobench report "$id" || true
  log_header "adoption $id"
  monobench adoption "$id" || true
}

run_sweep() {
  local label="$1"
  local ids="$2"
  local runs="$3"
  local jobs="$4"
  local tag="${TAG_PREFIX}-${label}-r${runs}j${jobs}"

  log_header "START sweep $label tag=$tag"
  echo "instances: $ids"
  monobench sweep "$ids" \
    --tools monogram \
    --cli claude \
    --model haiku \
    --runs "$runs" \
    --jobs "$jobs" \
    --prepared \
    --tag "$tag" \
    --note "24h clean breadth loop: $label; many-problem Haiku monogram pattern search; no scoring change before repeated clean evidence"
  local rc=$?

  log_header "END sweep $label rc=$rc"
  local arr
  arr=("${(@s:,:)ids}")
  for id in "${arr[@]}"; do
    summarize_instance "$id"
  done
  return "$rc"
}

log_header "24h supervisor boot"
monobench version || true
monogram version || true
which monobench || true
which monogram || true

log_header "initial active solver rows"
active_solver_rows || true

log_header "pre-existing current reports"
summarize_instance dotnet-124796-wmiinterop-keepalive
summarize_instance swift-88509-demangler-uaf
summarize_instance node-62325-zlib-reset-write

while [ "$(date +%s)" -lt "$DEADLINE" ]; do
  rows="$(active_solver_rows || true)"
  if [ -n "$rows" ]; then
    log_header "guard wait: active solver rows present"
    echo "$rows"
    sleep 600
    continue
  fi

  run_sweep fresh-lifetime-a "dotnet-125293-gchandle-doublefree-race,envoy-45153-filestreamer-cancel-uaf,flutter-170284-surfacetexture-doublefree,grpc-39316-rls-shutdown-uaf,openresty-2483-luapipe-quic-uaf,php-19591-lexbor-mraw-uaf" 2 3 || true
  [ "$(date +%s)" -ge "$DEADLINE" ] && break

  run_sweep vocab-hard-b "bun-20093-napi-handlescope-race,node-59910-diagchannel-gc,node-62325-zlib-reset-write,dotnet-124796-wmiinterop-keepalive,swift-88509-demangler-uaf,ksmbd-37899" 2 2 || true
  [ "$(date +%s)" -ge "$DEADLINE" ] && break

  run_sweep broad-mixed-c "numpy-31314-nditer-getitem-segfault,redis-14929-restorecmd-meta-uaf,ruby-16128-yjit-aliasing-uaf,dart-3095-uri-backslash-bypass,vapor-2500-filemiddleware-traversal,ktor-5626-readchannel-close" 2 3 || true
  [ "$(date +%s)" -ge "$DEADLINE" ] && break
done

log_header "24h supervisor finished"
