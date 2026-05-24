Type `monogram` on the terminal. ← That is your VERY FIRST action, before you read any file, before any grep or find.

    monogram

Run it with no arguments; it prints monogram's full command reference. This repository is
ALREADY INDEXED by monogram. monogram — NOT grep, NOT find, NOT scanning files by hand — is how you
investigate this bug: it traces the call graph, cross-language bindings, ownership call sites, and
dead/orphan contracts that link the symptom to its true cause. On EVERY step after that, reach for
monogram.

Do not stop at `search`. `search` ranks files; it is only the doorway. Prefer the staged
region-first flow before deep chains:

    monogram search "<terms from the symptom>"     → find the files that own the behavior
    monogram region "<natural-language intent>" -n 5
                                                    → rank likely implementation regions
    monogram context <top-symbol> --code 80         → read one bounded source region
    monogram chain <top-symbol> --callers --depth 1 → one inbound hop
    monogram chain <top-symbol> --callees --depth 2 → bounded downstream tree
    monogram coupling --domain ffi --pattern <sym> --all
                                                    → C/Zig/Rust/Swift FFI definition/call sites

Use `chain --depth 3+` only after a concrete symbol is proven and monogram's fan-out NEXT says it
is safe. If monogram prints a budget/cap/fanout warning, follow the staged NEXT rather than adding
`-r`, higher `-n`, or a deeper chain.

For memory, ownership, refcount, UAF, cross-thread, or FFI bugs, use the ownership recipe immediately:

    monogram region "ownership boundary ref deref leakRef isolatedCopy" -n 5 --score-debug
    monogram refgrep "isolatedCopy" --chain --depth 2
    monogram refgrep "leakRef" --chain --depth 2
    monogram refgrep "deref" --chain --depth 2
    monogram refgrep "ref" --chain --depth 2
    monogram coupling --domain ffi --pattern "<candidate symbol>" --all

Symptom file is not root-cause proof. If the crash is in one language but ownership crosses into
another, pivot by verbs (`ref`, `deref`, `leakRef`, `isolatedCopy`, `retain`, `release`, `free`) and
then compare adjacent helpers. A sibling helper with similar words is a decoy until you prove the
ownership balance. Broad words like `String`, `toSlice`, `fromJS`, `ref`, or `deref` are ecosystem
symbols: use region and bounded context before expanding callers.

Run `monogram` now. Then use it to solve the task. The full reference and your task follow below.

Stay in the current working directory. monobench already placed you in the prepared repo worktree;
do not `cd /tmp/monobench-work/...`, do not invent a worktree path from a run id, and do not copy
paths from older traces as commands. If you need location proof, ask monogram from the current cwd:

    monogram stats
    monogram search "<terms>" --cwd -n 10

If your shell tool asks for a working directory, set it to `.`. Do not run `find`, `list_dir`,
permission discovery, or home-directory scans to locate the repo. If a command is launched as a
background task, wait for it and read its output before answering.

================================================================================
