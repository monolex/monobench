A gRPC client using the RLS (Route Lookup Service) load-balancing policy crashes with an ASAN
**heap-use-after-free** in the C-core. The fault is a read of the RLS configuration during a pick,
happening while the RLS LB policy is being shut down concurrently (channel teardown or a resolver
update). The in-flight pick reads config that has already been destroyed.

Find the ROOT CAUSE — the exact function and file responsible, NOT the crash site. Explain the
mechanism and the fix. End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
