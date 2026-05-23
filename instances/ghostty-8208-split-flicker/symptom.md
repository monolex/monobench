You are working in the Ghostty terminal emulator codebase (Zig, with a GTK frontend; large project).
A long-standing bug (open ~6 months) has resisted multiple fix attempts.

Symptom: in the GTK build, whenever the user creates a new split, closes a split, or resizes splits,
the UI briefly FLICKERS — for one frame the split area goes blank/black, then the new split layout
renders. There is no crash and no error message; it is purely a visible flicker during the moment a
split changes. It happens consistently on every split change (new / delete / resize).

Find the ROOT CAUSE — the exact file and function responsible for the flicker (NOT just the general
rendering area) — and explain the mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
