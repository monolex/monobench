# Additional Test Analysis: cpython + ksmbd

Date: 2026-05-24

## Summary

Two additional archived tests were inspected:

- `cpython-147962-grouper-reentrant`
- `ksmbd-37899`

They represent different outcomes:

- `cpython-147962-grouper-reentrant` is not a monogram failure. The solver answer names the correct
  root cause and mechanism, but the instance metadata still contains TODO grading fields, so the
  correct benchmark status is `INVALID`.
- `ksmbd-37899` is a real FULL result, but it is non-discriminating as a benchmark signal because
  the baseline also solved it.

## Evidence

### cpython-147962-grouper-reentrant

Source files:

- `research/cases/cpython-147962-grouper-reentrant/README.md`
- `research/cases/cpython-147962-grouper-reentrant/runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1/raw/instance/instance.json`
- `research/cases/cpython-147962-grouper-reentrant/runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1/raw/results/monogram-preindexed-gpt-5.4-mini-low-r1.answer.txt`
- `research/indexes/invalid-instance-incident-2026-05-24.md`

Observed:

- Answer says `ROOTCAUSE: Modules/itertoolsmodule.c::_grouper_next`.
- Answer mechanism mentions re-entrant `__eq__`, borrowed pointers, and strong refs around
  `PyObject_RichCompareBool`.
- Instance metadata still has TODO fields:
  - `ground_truth.root_cause_fn = "TODO"`
  - `grading.full_must_name = ["TODO"]`
  - `grading.mechanism_keywords = ["TODO"]`

Verdict:

`INVALID`, not `MISS`. The archived old grade was a harness/instance-authoring error.

### ksmbd-37899

Source files and commands:

- `research/cases/ksmbd-37899/README.md`
- `research/cases/ksmbd-37899/runs/2026-05-23_gpt-5.5_low_monogram-r1/raw/results/monogram-low-r1.answer.txt`
- `monobench report ksmbd-37899`
- `monobench adoption ksmbd-37899`
- `monobench trace ksmbd-37899 monogram-haiku-r1`
- `monobench trace ksmbd-37899 baseline-haiku-r1`

Observed:

- Baseline: `FULL`, 37 calls, 0 monogram calls.
- Monogram: `FULL`, 47 calls, 16 monogram calls, first monogram call at #1.
- Monogram answer names `smb2pdu.c::smb2_session_logoff`.
- Mechanism: `smb2_session_logoff()` waits only current connection with `ksmbd_conn_wait_idle(conn)`,
  then frees `sess->user` while another SMB3 multichannel connection can still use the session.
- Baseline found the same root cause with plain find/grep/read flow.

Verdict:

Clean monogram success, but weak benchmark signal. This case should remain as a record/regression
example, not as a strong tool-discrimination case.

## Harness checks

Relevant implementation:

- `src/grade.rs` marks TODO/empty grading metadata as `INVALID`.
- `src/run.rs` refuses to run invalid instance metadata.
- `src/run.rs` also refuses `symptom.md` containing TODO.
- `src/report.rs` excludes `INVALID` from medians and hit-rate summaries.

Verification:

`cargo test` in `monobench` passed:

```text
26 passed; 0 failed
```

## Conclusions

1. The cpython run is actually evidence that monogram can solve the intended bug, but the instance is
   not admissible until authored properly.
2. The ksmbd run is evidence that monogram can follow the ownership/session teardown path, but it
   does not prove tool lift because the baseline also solves it.
3. Future matrix scaling should prioritize instances where baseline is not already FULL at low n.
4. The new invalid-instance guard is necessary and working: it prevents TODO scaffolds from becoming
   misleading `MISS` data.

