# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `plugin/pkg/admission/serviceaccount/admission.go :: limitSecretReferences` (PR
#124322, CVE-2024-3177; fix `b722d017a34b`, base `cae35dba5a30`).

`limitSecretReferences` enforced the mountable-secrets allowlist for **volumes**, **env
`valueFrom.secretKeyRef`**, and **imagePullSecrets**, but never checked **`container.EnvFrom[].SecretRef`**.
A pod could reference a non-allowed Secret via `envFrom` and pass admission. (`limitEphemeralContainerSecretReferences`
had the same gap.)

**Decoy:** the three present, correct loops look complete — the bug is the **absent** fourth arm.

**Fix:** add the missing `envFrom.SecretRef` check loop in both functions.

**Admission (C1–C6):** C1 — no crash; symptom (a pod that should be rejected is admitted) ≠ cause
(missing arm). C2 ✓ STRONG: grep `envFrom` in the buggy plugin returns nothing (the code is absent).
C3 ✓ PR #124322. C4 — **famous CVE-2024-3177 → check contamination**, down-weight if baseline solves
from prose (C5). C6 — Go logic; the missing-arm shape is monogram-relevant (enumerate all consumers).
