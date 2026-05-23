TODO — author the symptom. RULE: do NOT name the buggy subsystem/API (it greps straight to the cause).
Give only the crash trace + observable behavior; make the agent infer the subsystem.

(reference, do NOT paste verbatim — PR title: threading: fix lost-wakeup race in ThreadPool.notify() on aarch64)

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
