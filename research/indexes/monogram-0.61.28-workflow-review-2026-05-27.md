---
title: "monogram 0.61.28 수정 전체 작업 흐름 리뷰"
created: 2026-05-27T10:03:34+09:00
tags: [monobench, monogram, haiku, workflow, review, scoring]
project: monolex-006
source: codex
status: review-draft
---

# monogram 0.61.28 수정 전체 작업 흐름 리뷰

이 문서는 `claude/haiku + monogram` 재귀 benchmark loop 중 `monogram 0.61.28`로
마무리한 수정, 설치, 검증, 결과 판독 흐름을 리뷰하기 위한 작업 기록이다.

범위는 다음 네 가지다.

- `monogram` 쪽 generalized scoring / grep triage 수정 흐름
- `monogram 0.61.28` 설치와 version capture 확인 흐름
- `monobench` Haiku prepared matrix 결과와 trace/audit/integrity 판독 흐름
- 다음 리뷰에서 봐야 할 남은 리스크와 후속 개선 후보

이 문서는 solver prompt가 아니라 maker/review 문서다. benchmark 정답 literal을
solver prompt나 일반 사용자-facing skill로 옮기면 안 된다.

## 1. 결론 요약

```text
monogram 0.61.28
  -> runtime source preference 과적용을 anchor-gated로 축소
  -> grep free-site triage에 lifecycle/free + receiver state mutation pair 감지 추가
  -> context/rootcause/answer_ready marker를 더 직접적으로 노출
  -> docs/initiate/flow-guide/Cargo/openclis version surface 동기화
  -> release build 후 OpenCLIs local install 갱신
  -> HTMLRewriter UAF 2/2 FULL CLEAN
  -> toThreadSafe holdout 2/2 FULL CLEAN
  -> 단, broad_output_or_fanout_loop maker recommendation은 계속 남음
```

`0.61.28`은 "최종 완성"이라기보다 `0.61.27`에서 보인 HTMLRewriter 회귀를
회복하고, paired lifecycle/state mutation이라는 다음 steering rail을 제공한
버전으로 해석해야 한다.

핵심 판정:

- 성공: `0.61.27`에서 HTMLRewriter 한 run이 `NO_RESULT`였던 회귀는 `0.61.28`에서 회복됨.
- 성공: `bun-1.3.10-toThreadSafe` holdout은 `0.61.28`에서도 `2/2 FULL`로 유지됨.
- 성공: 모든 최종 4개 run은 `integrity CLEAN score=0`.
- 미완: HTMLRewriter r2는 여전히 `Edit`를 수행했고, 새 `paired_lifecycle_state_mutation` marker를 실제 solver가 직접 밟은 증거는 없음.
- 미완: 두 instance 모두 `monobench monogram-audit`가 `broad_output_or_fanout_loop count=2`를 계속 추천함.

## 2. 적용한 원칙

이번 수정은 기존 loop 원칙에 맞춰 다음 기준으로 진행했다.

```text
close the path first, then search within it
```

해석:

- `region`은 query와 구조 evidence가 붙은 candidate space를 좁힌다.
- `grep`은 raw line을 보여주되, containing function과 lifecycle proof marker로 solver가
  wrapper/finalizer 주변을 과도하게 도는 것을 줄인다.
- `context`와 `[NEXT]`는 root boundary를 고정하고, answer-ready 상태를 표시해 solver가
  edit/test/history 작업으로 새지 않게 한다.
- benchmark answer key의 exact file/function/field literal을 functional routing이나 scoring
  formula에 넣지 않는다.

이번 변경의 방향:

- 하드코딩 rail 추가가 아니라 일반화된 score gate와 proof marker를 추가한다.
- 한 benchmark에서 보인 이름을 정답으로 고정하지 않고, "lifecycle/free call"과
  "receiver state mutation"의 같은 함수 내 co-occurrence를 일반 규칙으로 잡는다.
- runtime/bug/lifecycle query라고 해서 declaration/generated/API surface를 무조건 내리지 않는다.
  source implementation preference는 anchor coverage가 있을 때만 준다.

## 3. 사전 컨텍스트

### 3.1 기존 loop 문서

관련 기준 문서:

- `monologue/demo/monobench/research/indexes/loop-flow.md`
- `.claude/skills/monolex-monobench-maker/Full-Guide.md`
- `.agents/skills/monolex-monobench-maker/SKILL.md`
- `.agents/skills/monolex-monogram-maker/SKILL.md`

핵심 규칙:

- 같은 instance / same CLI / same model / same prepared policy / same monogram version끼리 비교한다.
- Haiku single run은 proof가 아니라 scout signal이다.
- success/failure rail을 trace로 비교하고, 반복되는 command/result pattern만 maker proposal로 승격한다.
- `show --spoil` 또는 ground truth literal은 solver-facing prompt나 generic tool logic으로 옮기지 않는다.

### 3.2 작업 축

이번 최종 검증 축:

```text
tool:      monogram
cli:       claude
model:     haiku
via:       direct
prepared:  true
version:   monogram 0.61.28
jobs:      2
runs:      2 per instance
```

검증한 instance:

- `bun-30196-htmlrewriter-uaf`
- `bun-1.3.10-toThreadSafe`

## 4. 변경 대상 파일

### 4.1 monogram source

```text
tauri-apps/lib-monogram/src/region.rs
tauri-apps/lib-monogram/src/bin/monogram.rs
```

### 4.2 monogram docs / initiate surface

```text
tauri-apps/lib-monogram/src/bin/initiate/initiate.md
tauri-apps/lib-monogram/src/bin/initiate/SKILL.md
tauri-apps/lib-monogram/src/bin/initiate/flow-guide.md
```

### 4.3 version surfaces

```text
tauri-apps/lib-monogram/Cargo.toml
tauri-apps/lib-monogram/openclis.json
tauri-apps/Cargo.lock
```

### 4.4 installed binary surfaces

```text
tauri-apps/lib-monogram/bin/monogram
~/.openclis/versions/monogram/0.61.28/20260527000000/monogram/monogram
~/.openclis/bin/monogram -> ~/.openclis/versions/monogram/0.61.28/20260527000000/monogram/monogram
```

## 5. 변경 1: runtime source preference 축소

### 5.1 문제

`0.61.27`에서 runtime/bug/source preference가 너무 넓게 적용되면 broad runtime symptom만으로
declaration/generated/API surface가 과도하게 밀리는 문제가 생겼다.

관찰된 증상:

- HTMLRewriter에서 `0.61.27` run 하나가 `NO_RESULT`.
- source implementation을 밀어주는 방향 자체는 필요했지만, broad symptom query만으로
  surface penalty를 주면 declaration/API 쪽 valid candidate가 불필요하게 demote될 수 있음.

### 5.2 수정 위치

파일:

```text
tauri-apps/lib-monogram/src/region.rs
```

핵심 현재 코드 위치:

```text
region.rs:1576
  let runtime_surface_preference = scoring.runtime_bug_query && anchor_coverage >= 0.5;

region.rs:1577-1582
  implementation_bonus = 0.35 only when runtime_surface_preference
  and implementation region path match.

region.rs:1629-1638
  generated_surface_penalty = 0.42 only when runtime_surface_preference
  and declaration/generated path has no boundary/graph support.
```

### 5.3 변경 전 개념

```text
runtime/bug query detected
  -> implementation path gets preference
  -> generated/declaration path can be penalized
```

문제:

- query가 broad symptom이면 source preference가 너무 빨리 켜짐.
- "runtime bug" 자체는 anchor가 아니며, anchor term coverage 없이 implementation path만 밀면
  open-index name drift가 생길 수 있음.

### 5.4 변경 후 개념

```text
runtime/bug query detected
AND anchor_coverage >= 0.5
  -> implementation path gets small source nudge
  -> generated/declaration path can be penalized only under same gate

runtime/bug query detected
BUT anchor_coverage < 0.5
  -> implementation_bonus = 0.0
  -> generated_surface_penalty = 1.0
```

즉 source preference는 "runtime"이라는 generic query class만으로 켜지지 않고,
query가 실제 code anchor를 어느 정도 잡았을 때만 켜진다.

### 5.5 테스트

추가/교체된 테스트:

```text
region.rs:2329
  unanchored_runtime_bug_queries_keep_surface_preference_neutral
```

검증 의도:

- broad HTMLRewriter/runtime symptom query가 anchor 없는 상태에서는 declaration/generated surface를
  blanket demote하지 않아야 함.
- implementation path도 anchor 없는 상태에서는 bonus를 받지 않아야 함.

테스트의 핵심 assert:

```text
declaration_debug.generated_surface_penalty == 1.0
implementation_debug.implementation_bonus == 0.0
```

## 6. 변경 2: paired lifecycle/state mutation grep triage

### 6.1 문제

HTMLRewriter UAF 계열에서는 solver가 finalizer/destructor/wrapper 주변 raw evidence를 보고
root boundary를 더 좁히지 못하고 주변 proof를 반복하는 경향이 있었다.

반복된 failure/near-success pattern:

- `Response.finalize` 같은 finalizer call은 crash-site 또는 mechanism evidence일 수 있음.
- 실제 root boundary는 같은 function 안에서 lifecycle/free call과 receiver state mutation이
  함께 일어나는 곳일 수 있음.
- 기존 grep output은 lifecycle/free evidence와 state mutation evidence를 같은 local boundary로
  묶어 solver에게 "여기가 answer-ready boundary"라고 충분히 말해주지 못했다.

### 6.2 수정 위치

파일:

```text
tauri-apps/lib-monogram/src/bin/monogram.rs
```

핵심 현재 코드 위치:

```text
monogram.rs:4729-4734
  LifecycleCallsiteCandidate now has function start/end line.

monogram.rs:4851-4859
  code_hit_belongs_to_lifecycle_candidate checks same file/function and line range.

monogram.rs:4861-4884
  code_hit_has_state_mutation detects receiver state assignment to undefined/null/result/etc.

monogram.rs:4886-4913
  lifecycle_state_mutation_distance computes distance between lifecycle/free line and mutation line.

monogram.rs:4915-4935
  paired_lifecycle_state_mutation_candidate picks nearest candidate.

monogram.rs:5266-5271
  grep command computes lifecycle candidates and paired candidate.

monogram.rs:6183-6190
  grep output prints paired_lifecycle_state_mutation marker and answer_ready/root lock hints.
```

### 6.3 변경 후 output contract

조건:

```text
same candidate function contains:
  lifecycle/free/finalize/release/deref/close/etc. line
  AND receiver state mutation assignment line
```

출력:

```text
paired_lifecycle_state_mutation
  [marker: paired_lifecycle_state_mutation]
  [marker: context_root_lock]
  [marker: rootcause_label_guard]
  [marker: answer_ready]
  A lifecycle/free call and receiver state mutation occur in the same function...
```

의도:

- finalizer-only wrapper보다 local boundary를 우선 보게 한다.
- rootcause label을 finalizer/destructor helper로 옮기지 않게 한다.
- solver가 "더 조사" 대신 root boundary proof를 닫도록 한다.
- edit/test/history/repro로 새지 않게 한다.

### 6.4 일반화 기준

이 규칙은 특정 benchmark 함수명에 의존하지 않는다.

사용한 일반 신호:

- lifecycle/free keyword
- function boundary
- receiver state assignment
- line distance
- same file/function range

사용하지 않은 것:

- benchmark answer key
- exact root function as routing key
- exact source file as routing key
- exact field name as routing key

### 6.5 테스트

추가 테스트:

```text
monogram.rs:6586
  paired_lifecycle_candidate_prefers_same_function_state_clear
```

테스트 의도:

- lifecycle call만 있는 wrapper보다 lifecycle call + state mutation이 같은 function 안에 있는
  candidate를 우선 선택해야 함.

핵심 synthetic evidence:

```text
wrapper:
  response.finalize();

runCleanup:
  if (!is_async) response.finalize();
  sink.response = undefined;

expected:
  runCleanup
```

## 7. 변경 3: root lock / answer-ready marker 표면 보강

### 7.1 목적

Haiku는 correct neighborhood에 도착한 뒤에도 다음 행동을 넓히는 경향이 있다.

반복 패턴:

- `context --code 100`
- broad `search --explain`
- `chain --depth >= 2`
- shell `find` / `grep`
- 드물게 `Edit`

따라서 output에서 "여기가 root boundary다", "여기서 answer 가능하다"를 더 명시해야 했다.

### 7.2 사용 marker

```text
context_root_lock
rootcause_label_guard
answer_ready
paired_lifecycle_state_mutation
```

의도:

- `context_root_lock`: 지금 잡은 containing function/boundary를 유지하라.
- `rootcause_label_guard`: helper/wrapper/finalizer로 root label을 옮기지 말라.
- `answer_ready`: 더 넓히기보다 현재 proof로 답을 작성할 수 있다.
- `paired_lifecycle_state_mutation`: lifecycle/free와 state mutation pair가 같은 local boundary에 있다.

### 7.3 리뷰 기준

좋은 marker는 다음 조건을 만족해야 한다.

- output을 줄여야지 늘리기만 하면 안 됨.
- marker가 exact benchmark literal을 포함하면 안 됨.
- marker가 잘못된 root를 과신하게 만들면 안 됨.
- marker 후 `[NEXT]`가 더 좁은 proof command를 제안해야 함.

## 8. 변경 4: docs/initiate/version surface 동기화

### 8.1 version

동기화된 version surface:

```text
tauri-apps/lib-monogram/Cargo.toml:3
  version = "0.61.28"

tauri-apps/lib-monogram/openclis.json:3
  "version": "0.61.28"
```

`Cargo.lock`도 `lib-monogram` version 반영을 위해 갱신됐다.

### 8.2 docs

변경한 docs:

```text
tauri-apps/lib-monogram/src/bin/initiate/initiate.md
tauri-apps/lib-monogram/src/bin/initiate/SKILL.md
tauri-apps/lib-monogram/src/bin/initiate/flow-guide.md
```

반영한 문구의 핵심:

```text
Runtime/bug/lifecycle queries add only an anchored source nudge;
broad symptoms do not blanket-demote declaration/generated schema/API surfaces.

Grep lifecycle triage promotes same-function free/state-mutation pairs.
```

`flow-guide.md`에는 lifecycle grep triage가 `paired_lifecycle_state_mutation`을 감지하고
finalizer-only wrapper보다 local boundary를 lock한다는 흐름을 추가했다.

### 8.3 monogrid

문서 박스 정렬 검증:

```bash
monogrid check tauri-apps/lib-monogram/src/bin/initiate/initiate.md
monogrid check tauri-apps/lib-monogram/src/bin/initiate/flow-guide.md
```

결과:

```text
pass
```

## 9. build/install/version capture 흐름

### 9.1 빌드

실행한 검증 계열:

```bash
cd /Users/macbook/Projects/monolex/monolex/tauri-apps
cargo check -p lib-monogram --bin monogram
cargo test -p lib-monogram --bin monogram paired_lifecycle_candidate_prefers_same_function_state_clear
cargo test -p lib-monogram region::tests::unanchored_runtime_bug_queries_keep_surface_preference_neutral
cargo test -p lib-monogram region::tests::compound_anchor...
cargo build --release --bin monogram
```

주의:

- workspace warning은 있었지만 `lib-monogram` check/test/build를 막는 error는 없었다.
- 실제 installed PATH binary와 source build artifact를 분리해서 확인했다.

### 9.2 설치

설치 상태:

```text
~/.openclis/bin/monogram
  -> ~/.openclis/versions/monogram/0.61.28/20260527000000/monogram/monogram
```

local copy:

```text
tauri-apps/lib-monogram/bin/monogram
```

### 9.3 version 확인

현재 `monogram --version`은 supported command가 아니다. hand-rolled dispatcher가
`--version`을 unknown command로 처리하면서도 no-arg/help banner 안에 `monogram 0.61.28`을
출력한다.

리뷰 포인트:

- `monobench` version capture가 `tool.json`의 `version_bin`으로 PATH binary를 resolve하고,
  OpenCLIs layout 또는 banner text에서 semver를 파싱하는 구조라면 현재 결과는 정상이다.
- 하지만 `monogram --version`을 공식 UX로 기대하면 안 된다. 필요하면 별도 version command를
  추가하는 것이 더 명확하다.

확인된 monobench metadata:

```json
"label": "monogram-0.61.28-claude-haiku",
"monogram_version": "0.61.28"
```

## 10. prepared index 흐름

최종 두 instance 모두 prepared snapshot을 사용했다.

### 10.1 HTMLRewriter prepared manifest

```text
source_root     /private/tmp/monobench-work/bun
source_db       /Users/macbook/.monolex/monogram/bun-3a1071.db
snapshot_db     results/bun-30196-htmlrewriter-uaf/_prepared/monogram/monogram.db
tool_version    0.61.28
created_ms      1779841142132
db size         252M
```

### 10.2 toThreadSafe prepared manifest

```text
source_root     /private/tmp/monobench-work/bun
source_db       /Users/macbook/.monolex/monogram/bun-3a1071.db
snapshot_db     results/bun-1.3.10-toThreadSafe/_prepared/monogram/monogram.db
tool_version    0.61.28
created_ms      1779841622020
db size         241M
```

### 10.3 왜 prepared를 썼나

prepared mode의 목적:

- 각 run worktree에서 동일한 warmed monogram DB snapshot을 사용한다.
- index build 시간/변동성을 solver result에서 분리한다.
- `monogram_version`과 snapshot `tool_version`을 같은 축으로 맞춘다.

이번 결과에서 중요한 점:

- `tool_version 0.61.28`로 준비된 snapshot이 실제 run metadata의 `monogram_version 0.61.28`과 일치한다.
- `Files: 0`류 stale/missing index 상태가 아니라 prepared DB를 넣고 run했다.

## 11. 최종 run 1: HTMLRewriter UAF

### 11.1 실행 명령

```bash
monobench matrix bun-30196-htmlrewriter-uaf \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06128-paired-lifecycle-htmlrewriter-r2j2 \
  --note "monogram 0.61.28 paired lifecycle/state mutation grep triage plus narrowed runtime source preference; expect answer_ready runOutputSink without edit/fanout regression"
```

### 11.2 run metadata

공통:

```json
"cli": "claude",
"model": "haiku",
"tool": "monogram",
"via": "direct",
"prepared": true,
"label": "monogram-0.61.28-claude-haiku",
"monogram_version": "0.61.28",
"repeat_total": 2,
"tag": "haiku-v06128-paired-lifecycle-htmlrewriter-r2j2"
```

### 11.3 결과 요약

```text
monogram-0.61.28-claude-haiku-r1-t1779841169894
  grade: FULL
  cost:  $0.27
  time:  172s
  calls: 13
  monogram calls: 6
  grep/find: 1
  git: 0
  integrity: CLEAN score=0

monogram-0.61.28-claude-haiku-r2-t1779841169894
  grade: FULL
  cost:  $0.29
  time:  195s
  calls: 11
  monogram calls: 5
  grep/find: 0
  git: 0
  integrity: CLEAN score=0
```

Aggregate:

```text
monogram @0.61.28
  n: 2
  FULL: 2/2
  median cost: $0.28
  median tokens: 952k
  median time: 184s
  calls min-med-max: 11-12-13
  mono%: 46%
```

### 11.4 trace r1

Command sequence:

```text
1. monogram stats
2. monogram search "HTMLRewriter" -n 10 --explain
3. Read
4. monogram search "ValueBufferer" -n 5
5. monogram context ValueBufferer --code 100 -n 3
6. Read
7. monogram grep "pub fn finalize" --file "Response.zig" -A 5
8. Read
9. monogram grep "pub fn ref\|pub fn unref" --file "Response.zig" -A 3
10. Read
11. Read
12. Read
13. shell find test file
```

Root cause line:

```text
ROOTCAUSE: src/bun.js/api/html_rewriter.zig::BufferOutputSink.runOutputSink
```

Interpretation:

- FULL이지만 new paired lifecycle marker를 직접 밟았다고 보기는 어렵다.
- `ValueBufferer`, `Response.zig` ref/finalize proof로 돌아가는 경향이 남았다.
- shell `find`가 1회 존재한다.

### 11.5 trace r2

Command sequence:

```text
1. monogram
2. monogram search "HTMLRewriter" -n 8
3. monogram region "HTMLRewriter buffered transform response finalize error handling rejected promise" -n 5 --score-debug
4. monogram context transform_ --code 100 --file ./src/bun.js/api/html_rewriter.zig
5. monogram symbols "transform" --file ./src/bun.js/api/html_rewriter.zig
6. Read
7. Read
8. Read
9. Read
10. Edit
11. Read
```

Root cause line:

```text
ROOTCAUSE: src/bun.js/api/html_rewriter.zig::BufferOutputSink.runOutputSink
```

Interpretation:

- `region -> context transform_` 흐름은 source side로 잘 들어갔다.
- 하지만 `Edit`가 남아 있다.
- solver가 correct root를 최종 답으로 냈지만, tool output이 "답만 작성하고 멈추라"는
  행동을 완전히 강제하지는 못했다.

### 11.6 monogram-audit

```text
runs=2
calls=11
issues=0
oversized=0
help=1
next-lines=26
json-next=0
```

Subcommands:

```text
search   3
context  2
grep     2
help     1
region   1
stats    1
symbols  1
```

Patterns:

```text
region_first_next        3
context_code_ge_100      2
free_site_triage         2
context_code_capped      1
fanout_preflight         1
regex_alternation_query  1
region_score_debug       1
search_explain           1
success_pattern_next     1
```

Maker recommendation:

```text
broad_output_or_fanout_loop count=2
```

Meaning:

- run은 FULL이지만 output/fanout loop 압축 후보가 계속 남아 있다.
- 다음 개선은 answer literal이 아니라 output budget, staged depth, compact summary, region-first
  narrowing 쪽이어야 한다.

## 12. 최종 run 2: toThreadSafe holdout

### 12.1 실행 명령

```bash
monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06128-paired-lifecycle-tothreadsafe-r2j2 \
  --note "monogram 0.61.28 holdout after paired lifecycle grep triage and narrowed runtime source preference; verify compound anchor toThreadSafe root stability"
```

### 12.2 run metadata

공통:

```json
"cli": "claude",
"model": "haiku",
"tool": "monogram",
"via": "direct",
"prepared": true,
"label": "monogram-0.61.28-claude-haiku",
"monogram_version": "0.61.28",
"repeat_total": 2,
"tag": "haiku-v06128-paired-lifecycle-tothreadsafe-r2j2"
```

### 12.3 결과 요약

```text
monogram-0.61.28-claude-haiku-r1-t1779841622026
  grade: FULL
  cost:  $0.23
  time:  121s
  calls: 20
  monogram calls: 10
  grep/find: 3
  git: 0
  integrity: CLEAN score=0

monogram-0.61.28-claude-haiku-r2-t1779841622026
  grade: FULL
  cost:  $0.20
  time:  131s
  calls: 15
  monogram calls: 8
  grep/find: 1
  git: 0
  integrity: CLEAN score=0
```

Aggregate:

```text
monogram @0.61.28
  n: 2
  FULL: 2/2
  median cost: $0.22
  median tokens: 1.08M
  median time: 126s
  calls min-med-max: 15-18-20
  mono%: 51%
```

### 12.4 trace r1

Command sequence:

```text
1. cd . && monogram stats
2. cd . && monogram search "switch on corrupt value" -n 10
3. cd . && monogram region "ownership boundary ref deref leakRef isolatedCopy" -n 5 --score-debug
4. cd . && monogram context BunString__toThreadSafe --code 100 --file ./src/bun.js/bindings/BunString.cpp
5. cd . && monogram grep "deref" --raw -n 20 --file ./src/bun.js/bindings/BunString.cpp
6. Read denied
7. Read
8. cd . && monogram chain BunString__toThreadSafe --callers --depth 2
9. Read
10. cd . && monogram grep "isolatedCopy" -n 20 --chain --depth 1
11. Read
12. shell grep "switch on corrupt value" src/string.zig
13. cd . && monogram search "switch on corrupt" --cwd -n 10
14. cd . && monogram grep "BunStringTag" -n 10 --file ./src/string.zig
15. shell find BunString.h
16. shell find/grep struct BunString
17. cd . && monogram grep "pub.*tag.*BunStringTag" -n 10
18. Read denied
19. Read
20. Read
```

Root cause:

```text
src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

Interpretation:

- correct root는 유지됐다.
- monogram use가 많고 root lock은 작동했지만, shell grep/find fallback이 아직 있다.
- Read denied가 contamination은 아니지만, solver가 broad proof를 더 얻으려는 행동으로 볼 수 있다.

### 12.5 trace r2

Command sequence:

```text
1. monogram
2. monogram search "switch on corrupt value string" -n 8
3. monogram region "ownership boundary ref deref leakRef isolatedCopy" -n 5 --score-debug
4. monogram context BunString__toThreadSafe --code 100 --file ./src/bun.js/bindings/BunString.cpp
5. monogram grep "deref\|release" --raw -n 20 --file ./src/bun.js/bindings/BunString.cpp
6. Read
7. monogram grep "isolatedCopy" --chain --depth 2
8. monogram context toThreadSafe --code 80 --file ./src/string.zig
9. Read
10. monogram chain "BunString__toThreadSafe" --callers --depth 1
11. Read
12. Read
13. Read
14. shell grep -r "isolatedCopy" .../BunString.cpp
15. Read
```

Root cause:

```text
src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

Interpretation:

- r1보다 더 compact하지만 shell grep 1회는 남아 있다.
- `region -> context -> grep -> chain` 흐름이 root로 수렴한다.
- holdout 기준으로는 regression 없음.

### 12.6 monogram-audit

```text
runs=2
calls=18
issues=0
oversized=0
help=1
next-lines=34
json-next=0
```

Subcommands:

```text
grep     6
context  3
search   3
chain    2
region   2
help     1
stats    1
```

Patterns:

```text
ownership_verb_redirect  8
region_first_next        6
rootcause_label_guard    5
free_site_triage         4
budget_truncated         2
context_code_capped      2
context_code_ge_100      2
region_score_debug       2
success_pattern_next     2
fanout_preflight         1
generic_symbol_or_query  1
regex_alternation_query  1
```

Maker recommendation:

```text
broad_output_or_fanout_loop count=2
```

Meaning:

- root stability는 유지됐다.
- 하지만 proof 과정에서 `context --code 100`, grep alternation, chain depth, shell fallback이 여전히 있다.
- 다음 개선은 "root를 맞히게 하기"가 아니라 "맞힌 root를 더 빨리 닫게 하기" 쪽이다.

## 13. smoke test 흐름

정식 matrix 전에 좁은 smoke를 통해 `grep` surface가 실제로 원하는 local evidence를 보여주는지 확인했다.

예시:

```bash
monogram grep "response.finalize\|sink.response =" -n 30 --file ./html_rewriter.zig
```

확인한 output shape:

```text
out_response.finalize
response.finalize
sink.response = result
sink.response = undefined
paired_lifecycle_state_mutation
context_root_lock
answer_ready
```

해석:

- 기능 자체는 출력 가능하다.
- 그러나 최종 HTMLRewriter matrix의 두 run에서는 solver가 이 marker path를 직접 사용한 흔적이 약하다.
- 따라서 `0.61.28`의 paired marker는 "available rail"이지 "observed adopted rail"이라고 단정하면 안 된다.

## 14. loop 종료 시점 확인

사용자가 loop 종료를 요청했을 때 수행한 행동:

- 이미 실행 중이던 `bun-1.3.10-toThreadSafe` matrix가 끝날 때까지 기다림.
- 새 benchmark를 시작하지 않음.
- 해당 run의 `trace`, `monogram-audit`, `integrity`만 확인함.
- active process 확인 후 goal을 complete로 닫음.

최종 process 확인:

```bash
ps -ax -o pid,command | rg 'monobench matrix|monobench run|claude.*haiku' || true
```

결과:

```text
active benchmark/haiku runner 없음
```

## 15. 이번 작업에서 실제로 좋아진 점

### 15.1 HTMLRewriter regression recovery

`0.61.27`:

```text
HTMLRewriter final batch에 NO_RESULT 1개 존재
```

`0.61.28`:

```text
HTMLRewriter 2/2 FULL
integrity CLEAN
median cost/time/calls가 과도하게 튀지 않음
```

해석:

- runtime source preference를 anchor-gated로 좁힌 것이 broad runtime surface demotion 회귀를 줄인 것으로 볼 수 있다.
- 단, sample size가 2라서 proof가 아니라 회복 signal이다.

### 15.2 toThreadSafe holdout 유지

`toThreadSafe`는 기존에 root 안정성이 높았지만 version change마다 wrapper/root drift가 생기던 canary였다.

`0.61.28` 결과:

```text
2/2 FULL
CLEAN score=0
rootcause: BunString__toThreadSafe 유지
```

해석:

- HTMLRewriter를 위해 넣은 narrowing이 toThreadSafe ownership/cross-thread rail을 깨지 않았다.
- `ownership_verb_redirect`와 `rootcause_label_guard` pattern이 계속 작동했다.

### 15.3 version axis가 분리됨

run label과 metadata에 `monogram_version: 0.61.28`이 들어갔다.

이게 중요한 이유:

- `monogram` arm과 `monogram @0.61.28` aggregate가 report에서 분리된다.
- 이전 version 결과와 같은 axis처럼 섞어서 median을 읽는 실수를 줄인다.
- prepared snapshot manifest의 `tool_version`과 run metadata가 맞는지 확인할 수 있다.

## 16. 좋아졌다고 말하면 안 되는 부분

### 16.1 paired marker adoption은 아직 미확정

기능 smoke에서는 `paired_lifecycle_state_mutation` marker가 출력됐다.

하지만 최종 HTMLRewriter trace:

- r1은 `ValueBufferer`, `Response.zig`, ref/unref proof 쪽으로 감.
- r2는 `region -> context transform_ -> symbols transform` 후 `Edit`가 있음.

따라서:

```text
paired marker is available
paired marker is not yet proven adopted by Haiku in final matrix
```

### 16.2 edit/fanout elimination은 아직 안 됨

HTMLRewriter r2:

```text
Edit
```

toThreadSafe:

```text
shell grep/find fallback remains
context --code 100 remains
```

따라서:

```text
0.61.28 reduced regression risk but did not fully remove edit/fanout behavior.
```

### 16.3 broad_output_or_fanout_loop는 계속 남음

두 final audit 모두 같은 maker recommendation을 냈다.

```text
broad_output_or_fanout_loop count=2
```

이것은 다음 loop의 첫 후보가 root symbol routing이 아니라 output budget/staging이라는 뜻이다.

## 17. 리뷰 리스크

### 17.1 비기능 주석의 benchmark literal

현재 `monogram.rs`에는 다음 역사적 설명 주석이 남아 있다.

```text
response_wrapper_lifecycle_score(...)
  Body removed (monobench overfit - html_rewriter/runOutputSink answer ranking).
```

판정:

- 실행 로직은 아니며 score/routing에 쓰이지 않는다.
- 하지만 anti-overfit rule을 comments까지 엄격하게 적용하면 제거 후보다.
- 이 문서는 maker/review 문서이므로 해당 literal을 리스크로 기록하지만, solver-facing docs나 generic skill에는 옮기면 안 된다.

후속 권장:

```text
주석을 "Body removed because previous lifecycle wrapper ranking was over-specific."
정도로 바꾸고 exact benchmark file/function literal은 제거.
```

### 17.2 `monogram --version` UX 불일치

현재 no-arg banner에는 version이 찍히지만 `--version`은 unknown command다.

리스크:

- 사람이 version 확인을 `--version`으로 기대하면 헷갈릴 수 있다.
- monobench version capture가 banner parse에 의존하는 구조라면 작동은 하지만 UX가 약하다.

후속 후보:

```text
monogram version
monogram --version
```

둘 중 하나를 공식 surface로 추가하고 initiate docs와 `--mcp-schema`를 맞춘다.

### 17.3 answer_ready marker의 과신 가능성

`answer_ready`는 solver가 멈추고 답하도록 유도하는 장점이 있지만, 잘못 쓰면 proof가 부족한데 root를 과신하게 할 수 있다.

리뷰 기준:

- marker는 same-function lifecycle/free + state mutation처럼 실제 code evidence가 있을 때만 출력되어야 한다.
- broad regex match만으로 answer_ready가 나오면 안 된다.
- marker가 helper/finalizer-only candidate에 붙으면 regression이다.

### 17.4 sample size

최종 검증은 각 instance `runs=2`다.

해석:

- 회귀 회복 signal로 충분함.
- 장기 개선 proof로는 부족함.
- 다음 loop에서는 unrelated hard instance를 추가해야 한다.

## 18. 남은 개선 후보

### 18.1 output budget / fanout compression

audit가 계속 말한 핵심 후보:

```text
broad_output_or_fanout_loop
```

가능한 generalized fix:

- `context --code >=100` 이전에 type/member summary를 더 강하게 제안.
- `chain --depth >=2`에서 high fanout이면 staged frontier를 먼저 보여주고 expansion을 제한.
- `grep` alternation query가 broad proof dump로 커지면 split proof NEXT를 더 앞에 노출.
- `search --explain` broad query는 region-first next를 더 강하게 출력.

Validation:

- output bytes 감소
- command count 감소
- shell grep/find fallback 감소
- FULL rate 유지
- rootcause cone width 감소

### 18.2 marker adoption 측정

현재는 marker가 출력 가능한지만 확인했고, solver가 실제로 그 marker를 읽고 행동을 바꿨는지 약하다.

후속 측정:

```bash
monobench evidence <id> --pattern 'paired_lifecycle_state_mutation|answer_ready|context_root_lock|ROOTCAUSE'
monobench export <id> <run>
```

보고 싶은 것:

- marker 이후 solver가 broad search를 멈추는가
- marker 이후 final answer가 바로 나오는가
- marker가 없는 run보다 shell fallback/edit이 줄었는가

### 18.3 unrelated hard instance holdout

다음 loop에서 추가할 수 있는 axis:

- CPython reentrant/lifecycle case
- Ghostty/Vapor/Kubernetes style ownership or lifecycle case
- Node/freeParser style cleanup/error branch case
- getHeapSnapshot 계열 if current index/ground truth가 valid라면

조건:

- same `claude/haiku`
- same `monogram @0.61.28` or next version
- prepared snapshot
- `runs >= 2`, 가능하면 `runs 3-5`
- `monogram-audit` hard gate

## 19. 리뷰 체크리스트

리뷰할 때 다음 순서로 보면 된다.

1. `region.rs`의 `runtime_surface_preference` gate가 너무 약하거나 강하지 않은지 확인한다.
2. `generated_surface_penalty`가 declaration surface query에는 적용되지 않는지 확인한다.
3. `paired_lifecycle_state_mutation_candidate`가 exact benchmark literal 없이 동작하는지 확인한다.
4. `code_hit_has_state_mutation`의 assignment detector가 false positive를 많이 만들지 않는지 확인한다.
5. `answer_ready` marker가 helper/finalizer-only function에 붙을 가능성이 없는지 확인한다.
6. docs/initiate/flow-guide가 실제 command surface와 어긋나지 않는지 확인한다.
7. `monogram --version` UX를 이번에 정리할지 다음 version으로 넘길지 결정한다.
8. 비기능 주석의 benchmark literal을 제거할지 결정한다.
9. `broad_output_or_fanout_loop`를 다음 version의 primary target으로 둘지 결정한다.

## 20. 재현 명령 모음

### 20.1 active process 확인

```bash
ps -ax -o pid,command | rg 'monobench matrix|monobench run|claude.*haiku' || true
```

### 20.2 version / install 확인

```bash
monogram
which monogram
readlink "$HOME/.openclis/bin/monogram"

monobench version
which monobench
readlink "$HOME/.openclis/bin/monobench"
```

주의:

```text
monogram --version is not a supported command in 0.61.28.
Use no-arg banner or OpenCLIs path/metadata until version command is added.
```

### 20.3 prepared manifest 확인

```bash
sed -n '1,120p' results/bun-30196-htmlrewriter-uaf/_prepared/monogram/manifest.tsv
sed -n '1,120p' results/bun-1.3.10-toThreadSafe/_prepared/monogram/manifest.tsv
du -h results/*/_prepared/monogram/monogram.db
```

### 20.4 HTMLRewriter matrix

```bash
monobench matrix bun-30196-htmlrewriter-uaf \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06128-paired-lifecycle-htmlrewriter-r2j2 \
  --note "monogram 0.61.28 paired lifecycle/state mutation grep triage plus narrowed runtime source preference; expect answer_ready runOutputSink without edit/fanout regression"
```

### 20.5 toThreadSafe matrix

```bash
monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06128-paired-lifecycle-tothreadsafe-r2j2 \
  --note "monogram 0.61.28 holdout after paired lifecycle grep triage and narrowed runtime source preference; verify compound anchor toThreadSafe root stability"
```

### 20.6 결과 판독

```bash
monobench report bun-30196-htmlrewriter-uaf --tag haiku-v06128-paired-lifecycle-htmlrewriter-r2j2
monobench monogram-audit bun-30196-htmlrewriter-uaf --tag haiku-v06128-paired-lifecycle-htmlrewriter-r2j2
monobench trace bun-30196-htmlrewriter-uaf monogram-0.61.28-claude-haiku-r1-t1779841169894 80
monobench trace bun-30196-htmlrewriter-uaf monogram-0.61.28-claude-haiku-r2-t1779841169894 80
monobench integrity bun-30196-htmlrewriter-uaf monogram-0.61.28-claude-haiku-r1-t1779841169894
monobench integrity bun-30196-htmlrewriter-uaf monogram-0.61.28-claude-haiku-r2-t1779841169894

monobench report bun-1.3.10-toThreadSafe --tag haiku-v06128-paired-lifecycle-tothreadsafe-r2j2
monobench monogram-audit bun-1.3.10-toThreadSafe --tag haiku-v06128-paired-lifecycle-tothreadsafe-r2j2
monobench trace bun-1.3.10-toThreadSafe monogram-0.61.28-claude-haiku-r1-t1779841622026 80
monobench trace bun-1.3.10-toThreadSafe monogram-0.61.28-claude-haiku-r2-t1779841622026 80
monobench integrity bun-1.3.10-toThreadSafe monogram-0.61.28-claude-haiku-r1-t1779841622026
monobench integrity bun-1.3.10-toThreadSafe monogram-0.61.28-claude-haiku-r2-t1779841622026
```

### 20.7 source verification

```bash
rg -n "runtime_surface_preference|generated_surface_penalty|implementation_bonus" \
  tauri-apps/lib-monogram/src/region.rs

rg -n "paired_lifecycle_state_mutation|code_hit_has_state_mutation|answer_ready" \
  tauri-apps/lib-monogram/src/bin/monogram.rs
```

## 21. QED 상태

현재 QED 수준:

```text
source code changed: yes
docs version surface changed: yes
release binary installed: yes
monobench version capture: yes
prepared DB tool_version: yes
HTMLRewriter final run: 2/2 FULL CLEAN
toThreadSafe holdout: 2/2 FULL CLEAN
active loop stopped: yes
```

아직 QED가 아닌 것:

```text
paired_lifecycle_state_mutation adoption by solver: not proven
edit/fanout elimination: not proven
cross-instance generalized lift beyond two Bun instances: not proven
official monogram --version UX: not implemented
comment-level anti-overfit cleanup: pending review
```

## 22. 다음 리뷰 결정

리뷰 후 결정할 수 있는 선택지:

1. `0.61.28`을 현재 상태로 유지하고 다음 target을 `broad_output_or_fanout_loop`로 잡는다.
2. 먼저 비기능 주석의 benchmark literal과 `monogram --version` UX를 작은 patch로 정리한다.
3. paired marker adoption이 약하므로 HTMLRewriter를 다시 `runs 3-5`로 돌려 marker가 실제 행동을 바꾸는지 본다.
4. unrelated hard holdout을 추가해서 `0.61.28`이 Bun-specific 튜닝이 아닌지 확인한다.

추천:

```text
2 -> 1 -> 4
```

이유:

- 주석 literal과 version UX는 작고 명확한 정리다.
- 다음 큰 개선은 root literal이 아니라 audit가 반복 제안한 output/fanout compression이다.
- Bun 두 instance만으로 generalized lift를 말하기에는 부족하므로 unrelated holdout이 필요하다.
