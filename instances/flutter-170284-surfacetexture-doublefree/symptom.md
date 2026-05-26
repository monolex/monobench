A Flutter Android app using the Impeller GLES backend crashes shortly after the app returns to the
foreground, when a platform view / external texture (Android `SurfaceTexture`) is involved. A GL
texture handle is freed twice; the second free corrupts a handle that another part of the system may
still be using.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
