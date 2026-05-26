# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `engine/.../gles/reactor_gles.cc :: ReactorGLES::CreateHandle` (PR #170284, fixes
#152459; fix `2d30ce56feb7`, base `ee0eb22cdbea`).

An external `SurfaceTexture` GL handle was registered with the reactor as a normal owned handle.
`SurfaceTexture.detachFromGLContext` (Android/JNI) already releases the texture, so the reactor's
later collection frees it a **second time** (after foregrounding), corrupting a handle that may be in
active use.

**Decoy:** `TextureGLES::~TextureGLES` (issues the engine-side free) and `detachFromGLContext` (the
JNI release) look responsible; the defect is `CreateHandle` not distinguishing external handles.

**Fix:** track external handles (`CreateHandle` gains an `external_handle` arg) so the reactor does
not free them.

**Admission (C1–C6):** C1 ✓ crash (reactor collection) ≠ cause (handle registration). C2 ✓ symptom
names SurfaceTexture, not `CreateHandle`. C3 ✓ PR #170284. C4 — niche Android/Impeller path.
C5 — baseline. C6 — C++↔JNI ownership. ⚠ **changed files are engine C++/JNI only — this does NOT
exercise monogram's Dart extractor; use a dart-lang/sdk bug for pure-Dart-language coverage.**
