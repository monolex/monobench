# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `src/core/load_balancing/rls/rls.cc :: RlsLb::Picker::Pick` (PR #39316, backport of
#39303, fixes #39217; fix `6c472088f7ce`, base `26fe06e53783`).

`RlsLb::Picker::Pick` accessed configuration owned by the `RlsLb` policy without keeping it alive.
After the policy was shut down and destroyed, an in-flight pick dereferenced the freed config → UAF.

**Decoy:** `RlsLb::Cache::Entry::Pick` / `Cache::Entry::Size` also touch freed state and look
plausible; the defect is the Picker not holding a reference to the config across policy shutdown.

**Fix:** have the Picker hold its own `RefCountedPtr` to the config so it outlives policy shutdown.

**Admission (C1–C6):** C1 ✓ crash (pick) ≠ cause (policy shutdown freeing config the picker still
uses). C2 ✓ symptom names RLS/pick but not the ownership fix. C3 ✓ PR #39316. C4 — niche RLS path.
C5 — baseline. C6 ✓ C++ object-lifetime/ownership edge (policy → picker → config).
