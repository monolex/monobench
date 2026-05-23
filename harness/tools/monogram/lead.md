Type `monogram` on the terminal. ← That is your VERY FIRST action, before you read any file, before any grep or find.

    monogram

Run it with no arguments; it prints monogram's full command reference. This repository is
ALREADY INDEXED by monogram. monogram — NOT grep, NOT find, NOT scanning files by hand — is how you
investigate this bug: it traces the call graph, cross-language bindings, ownership call sites, and
dead/orphan contracts that link the symptom to its true cause. On EVERY step after that, reach for
monogram.

Do not stop at `search`. `search` ranks files; it is only the doorway. Use the structural commands:

    monogram search "<terms from the symptom>"     → find the files that own the behavior
    monogram symbols <Name>                        → exact definition: kind, file:line, signature
    monogram grep "<call expr>" --chain             → call expression → containing function → callers
    monogram chain <fn> --callers --depth 3        → who calls it (trace the symptom OUTWARD to its cause)
    monogram chain <fn> --callees --depth 3        → what it calls (trace a trigger INWARD)
    monogram context <Symbol>                      → one shot: symbol + callers/callees + line# source
    monogram coupling --domain ffi --pattern <sym> --all
                                                    → C/Zig/Rust/Swift FFI definition/call sites

For memory, ownership, refcount, UAF, cross-thread, or FFI bugs, use the ownership recipe immediately:

    monogram grep "isolatedCopy" --chain --depth 2
    monogram grep "leakRef" --chain --depth 2
    monogram grep "deref" --chain --depth 2
    monogram grep "ref" --chain --depth 2
    monogram coupling --domain ffi --pattern "<candidate symbol>" --all

Symptom file is not root-cause proof. If the crash is in one language but ownership crosses into
another, pivot by verbs (`ref`, `deref`, `leakRef`, `isolatedCopy`, `retain`, `release`, `free`) and
then compare adjacent helpers. A sibling helper with similar words is a decoy until you prove the
ownership balance.

Run `monogram` now. Then use it to solve the task. The full reference and your task follow below.
================================================================================
