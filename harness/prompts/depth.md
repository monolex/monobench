# ============ HOW HARD THIS IS — APPLIES TO YOU ============
This is a DIFFICULT memory-safety bug (use-after-free / reference-count mismanagement) and there is
a DECOY — an adjacent function that looks like a plausible culprit but is NOT the bug. Shallow
guessing WILL be wrong. Therefore:
  • Do NOT stop at the first plausible candidate. Keep digging until you can prove the mechanism.
  • The crash SITE (the panic location) is NOT the root cause — trace OUTWARD from it.
  • Build the COMPLETE ownership picture for the data structure across the language/FFI boundary:
    who ALLOCATES it, who takes a REF, who DEREFs it, who FREES it. The bug is a refcount imbalance.
  • VERIFY your root-cause hypothesis (find the exact function whose ref/deref handling is wrong)
    before you answer. Read as many files as you need; be thorough, not fast.
