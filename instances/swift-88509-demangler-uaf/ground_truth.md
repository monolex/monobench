# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `lib/Demangling/Demangler.cpp :: Demangler::DemangleInitRAII` (PR #88509, fix
`d39d0feb25cd`, base `4be700e13125`).

`Words[]` (substitution words) were not saved/restored when a demangle **re-enters** (nested
demangle), so the inner demangling reused/overwrote storage the outer `Words[]` still referenced →
UAF.

**Decoy:** the nested demangle entry point and `NodeFactory` storage (where the stale pointer is
read) look responsible; the defect is `DemangleInitRAII` not saving/restoring `Words`.

**Fix:** save and restore the `Words` state in `DemangleInitRAII` around nested demangles.

**Admission (C1–C6):** C1 ✓ crash (reading stale `Words[]`) ≠ cause (missing save/restore). C2 ✓
symptom never names `DemangleInitRAII`/`Words`. C3 ✓ PR #88509. C4 — niche demangler path.
C5 — baseline. C6 — C++ object-state lifetime across re-entrancy. ⚠ **Swift toolchain C++ — does NOT
exercise monogram's Swift extractor; Swift-language coverage = vapor-2500 (.swift).**
