A long-running .NET service that periodically performs system management / instrumentation queries
crashes intermittently with an **access violation in unmanaged code**. The faulting frame is inside
a native COM interface call made while comparing two management objects, or while invoking a method
with input/output signature objects.

It only reproduces under memory pressure (after garbage collections) and never in short test runs.
The managed wrapper classes look correct and already pin their own instance across the native call.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
