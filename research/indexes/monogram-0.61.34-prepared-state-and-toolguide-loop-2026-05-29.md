# monogram 0.61.34 prepared-state + tool-guide loop

Date: 2026-05-29 KST

## Context

This loop continued after `monogram 0.61.34` fixed the false prepared-index stale warning.
The target was not another ranking change. The open question was whether the remaining
`query_transport` / shell fallback pressure should be handled by stronger solver-facing monogram
guidance or by audit-only diagnosis.

## Confirmed baseline after 0.61.34

Instance: `dotnet-125293-gchandle-doublefree-race`

Good prepared-state runs before the tool-guide experiment:

| Tag | Runs | Result |
|---|---:|---|
| `monogram-v06134-prepared-state-loop` | 1 | 1/1 FULL |
| `monogram-v06134-prepared-state-loop-r2` | 2 | 2/2 FULL |

Together: 3/3 FULL on `monogram 0.61.34` before changing solver-facing guide text.

Observed improvement versus earlier arms on the same instance:

| Arm | FULL |
|---|---:|
| baseline/haiku | 1/5 |
| monogram 0.61.31/haiku | 1/3 |
| monogram 0.61.33/haiku | 0/1 |
| monogram 0.61.34/haiku before guide experiment | 3/3 |

Unrelated holdout also passed:

| Instance | Run | Result |
|---|---|---|
| `node-59910-diagchannel-gc` | `monogram-0.61.34-claude-haiku-r1-t1779990140781` | FULL |

## Tool-guide experiment

Experiment tag: `monogram-v06134-toolguide-local-proof-r1`

Change tested: add stronger solver-facing prompt text telling the model to keep proof local with
`context --file`, `grep --file`, and `chain --file`, and to avoid shell `find`, shell `grep`/`rg`,
and git/history unless the prepared DB was wrong.

Result:

| Run | Grade | Rootcause Chosen |
|---|---|---|
| `monogram-0.61.34-claude-haiku-r1-t1779990763539` | MISS | `WinHttpRequestCallback.cs::OnRequestHandleClosing` |
| `monogram-0.61.34-claude-haiku-r2-t1779990763539` | MISS | `WinHttpRequestState.cs::ClearSendRequestState` |

Interpretation:

The guide text was harmful for Haiku. It reduced broad shell fallback pressure only partially, but
made the solver over-lock on nearby helpers after reaching the right neighborhood. This is a
prompt-delivery regression, not evidence that monogram ranking regressed.

The guide change was reverted. Do not reintroduce this as solver-facing text without a separate
delivery A/B run.

## Audit-only path after reverting guide text

After reverting the solver-facing guide change, a confirmation run was executed:

| Tag | Run | Grade | Rootcause |
|---|---|---|---|
| `monogram-v06134-audit-only-after-guide-revert` | `monogram-0.61.34-claude-haiku-r1-t1779991361542` | FULL | `WinHttpRequestState.cs::Dispose` |

Trace summary:

- 27 calls
- 18 monogram calls
- 2 shell grep/find calls
- 0 git calls
- integrity: CLEAN, score 0
- prepared-index state appeared normally (`Index state: prepared`); no stale prepared warning was present.

Current aggregate including the failed guide A/B rows:

| Arm | FULL |
|---|---:|
| monogram 0.61.34/haiku | 4/6 |

Do not use this aggregate alone to judge `0.61.34`; the two MISS rows belong to the reverted
tool-guide experiment. For the released/prompt-restored behavior, use the 4 FULL rows outside the
bad guide tag.

## Implemented audit-only change

`monogram-audit` now detects non-scoring transport pressure from fallback commands:

- `shell_file_search_fallback` for shell `grep`, `rg`, or `find` after monogram use.
- `git_denied_fallback` for denied git attempts.

These feed only `query_transport_pressure` in `maker_state_bridge.rs`, which remains diagnostic-only
and does not affect active maker layer scores.

The scope layer action text was also adjusted to match the observed state:

> Correctness is present, but fanout/filter pressure remains. Preserve the chosen file/symbol with
> --file, bounded context, and monogram grep before widening.

This text is printed by `monobench monogram-audit`; it is not injected into solver prompts.

## Decision

Keep the audit-only diagnostic expansion.

Do not add stronger local-proof guidance to the monogram solver prompt. For weak models, more
explicit prompt constraints can make a nearby helper look like the final answer. The next improvement
should be inside monogram output/NEXT itself or in audit/research tooling, where it can be measured
without changing the solver prompt contract.

## Next research candidates

1. Study successful FULL traces versus the two bad guide MISS traces for the exact command where
   `WinHttpRequestState.cs::Dispose` lost to `OnRequestHandleClosing` or `ClearSendRequestState`.
2. Keep `shell_file_search_fallback` and `git_denied_fallback` diagnostic-only until repeated
   holdout evidence proves they should become active pressure.
3. Prefer monogram-side output shaping over prompt text when reducing shell fallback. Candidate:
   make `monogram grep --file <partial-name>` explain whether the file filter is exact, fuzzy,
   path-like, or too broad, without telling the solver to avoid all shell fallback.
4. Re-test prompt-delivery changes only as explicit A/B arms, not as default `monogram` harness text.
