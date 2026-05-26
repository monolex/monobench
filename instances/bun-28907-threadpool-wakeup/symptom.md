On Apple Silicon / aarch64, a concurrent workload occasionally hangs forever (a CI job hits its
90-second timeout and is killed). There is no crash, no error, no panic — the process is simply
stuck. When captured, a unit of work is sitting queued and ready to run, yet every worker thread is
idle/parked and nothing ever picks it up. The internal "there is pending work" flag reads false even
though work is pending.

It is timing-dependent and never reproduces on x86_64; it only shows up under load on weak-memory
(ARM) hardware. The wait/park side of the workers looks correct on inspection — the work just never
gets handed to them.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
