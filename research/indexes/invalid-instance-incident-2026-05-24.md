# Invalid Instance Incident: cpython-147962-grouper-reentrant

Date: 2026-05-24

## Finding

`cpython-147962-grouper-reentrant` was included in reports while its `instance.json` still contained TODO grading metadata.

The archived answer named:

`Modules/itertoolsmodule.c::_grouper_next`

The ground truth also identifies `_grouper_next`, but the old scorer compared against `full_must_name = ["TODO"]` and `mechanism_keywords = ["TODO"]`, causing the run to appear as `MISS`.

## Impact

Affected canonical report:

`/Users/macbook/.monobench/0.1.2-1779431036/results/cpython-147962-grouper-reentrant`

Old visible result:

`monogram-preindexed-gpt-5.4-mini-low-r1 MISS`

Correct status after fix:

`monogram-preindexed-gpt-5.4-mini-low-r1 INVALID`

## Fix

`monobench 0.1.7` now:

- marks instances with TODO/empty grading metadata as `INVALID`
- excludes `INVALID` from medians and hit-rate summaries
- refuses to run an instance whose `instance.json` or `symptom.md` still contains TODO/provisional metadata

## Verification

```text
monobench 0.1.7
grade=INVALID  cost=$0.82  tokens=1921967  time=114s  toolcalls=25  tool-adoption=23
ROOTCAUSE: Modules/itertoolsmodule.c::_grouper_next
```

`cargo test` passed: 19 tests.
