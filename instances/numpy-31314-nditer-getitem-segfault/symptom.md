NumPy segfaults (hard interpreter crash) when assigning to an `nditer`'s `multi_index` using a
sequence-like Python object whose element access raises an exception partway through. The crash is
inside NumPy's C iterator code; it is triggered by the Python exception propagating out of the
element lookup.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
