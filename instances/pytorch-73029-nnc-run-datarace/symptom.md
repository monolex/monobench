A TorchScript model compiled with the NNC (TensorExpr) fuser and executed concurrently — e.g. via
Static Runtime running the **same** compiled kernel object from multiple threads — intermittently
produces wrong results or trips TSAN with a data race. Calling the compiled kernel repeatedly and in
parallel is supposed to be safe. The race only appears for models with dynamic input shapes.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
