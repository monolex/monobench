# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**oven-sh/bun PR #28907** — threading: fix lost-wakeup race in ThreadPool.notify() on aarch64

**fix commit:** 770893d28ff9 · **base (merge^):** 1b009ee4bd1a · merged 2026-04-08

## Changed source files (test/fixture files filtered out)
- src/threading/ThreadPool.zig

## PR body

`ThreadPool.notify()`'s fast path reads `sync.notified` with `.monotonic` and returns early if set. Under the C11 memory model this permits the scheduler to observe a stale `notified=true` — skipping the wake — while the worker that consumes that notification pops `run_queue` without observing the task the scheduler just pushed. The task is stranded in `run_queue` with every worker parked on `idle_event`; the awaiting caller hangs forever.

This manifests as sporadic 90 s timeouts in `test/js/bun/util/bun-file.test.ts` ("writer.end() …") on aarch64 CI (macOS 13/14 and Alpine), and affects every `WorkPool.schedule()` caller — `fs.promises`, `Bun.file().text()`, `Bun.write()`, `crypto.subtle`, the package manager, etc. x86_64 is not affected because `lock cmpxchg` on the preceding push is a full fence.

**Fix:** replace the load with `fetchOr(0, .release)` — a no-op RMW. An RMW participates in `sync`'s modification order, so if we read `notified=true` here, the worker's acquire-CAS in `wait()` that later clears it synchronizes-with this release (every write to `sync` is a CAS, so the release sequence is unbroken), and the push happens-before that worker's pop. A plain load — even `.seq_cst` — is insufficient under C11.

Most likely fixes #28048.

<details>
<summary><b>Verification</b></summary>

### The race

The scheduler does `push to run_queue (.release CAS)` → `read sync.notified (.monotonic)`; a worker does `clear sync.notified (.acquire CAS)` → `read run_queue (.monotonic)`. With relaxed loads on both sides, each can miss the other's write — the scheduler sees a stale `notified=true` and skips the wake, and the worker sees a stale empty queue and goes back to sleep. Task stranded.

### Hung state observed with lldb

Reproducer: a 100 k-iteration `fsPromises.open` / `Bun.file(fd).writer().end()` / `fsPromises.close` / `Bun.file().text()` loop on a release build under 24-way CPU load, macOS aarch64. lldb attached to four independent hung processes, `ThreadPool` decoded via `*self` from a worker's `wait` frame:

| Capture | `sync` | `run_queue.stack` | `idle_event` | Pool threads | Stranded callback |
|---|---|---|---|---|---|
| 1 | `idle=3 spawned=3 notified=false state=pending` | `≠0` (task present) | `WAITING` | 3, all `__ulock_wait2` | `NewAsyncFSTask('close').workPoolCallback` |
| 2 | `idle=5 spawned=5 notified=false state=pending` | `≠0` | `WAITING` | 5, all `__ulock_wait2` | same |
| 3 | `idle=4 spawned=4 notified=false state=pending` | `≠0` | `WAITING` | 4, all `__ulock_wait2` | same |
| 4 | `idle=5 spawned=5 notified=false state=pending` | `≠0` | `WAITING` | 5, all `__ulock_wait2` | same |

Every capture: a task sits in `run_queue`, every worker is idle-parked, and `notified=false`. Nothing will ever check the queue again.

### herd7 memory-model check

Litmus test abstracting the race (`P0` = scheduler, `P1` = worker):

```
P0: release-store stack=1 ; relaxed-load sync → r0
P1: acquire-exchange sync=0 ; relaxed-load stack → r1
exists (r0=1 /\ r1=0)    // scheduler sees notified=true AND worker sees queue empty
```

| Scheduler op on `sync` | C11 | AArch64 | x86_64 | Verdict |
|---|---|---|---|---|
| `load(.monotonic)` — current | **Sometimes** | **Sometimes** | Never | **bug** (aarch64-only) |
| `load(.acquire)` | Sometimes | — | — | insufficient |
| `load(.seq_cst)` | Sometimes | Never | — | insufficient under C11 |
| `fetchOr(0,.monotonic)` | Sometimes | Sometimes | — | insufficient (but passes hardware repro — timing bandaid) |
| **`fetchOr(0,.release)`** | **Never** | **Never** | Never | **minimal correct fix** |
| `fetchOr(0,.acq_rel)` / `.seq_cst` / fences / all-seq_cst | Never | — | — | correct but stronger than needed |

### GenMC whole-algorithm model check

A faithful C11 port of the algorithm (full `Node.Queue` tagged-pointer logic, `Buffer.consume`, `notify`/`notifySlow`/`wait`/`Thread.run`/`shutdown` state machine), model-checked under RC11:

| | `load(.monotonic)` | `fetchOr(0,.monotonic)` | `load(.seq_cst)` | `fetchOr(0,.release)` |
|---|---|---|---|---|
| GenMC `--check-liveness` | **Liveness violation** (0.35 s) | **Liveness violation** | **Liveness violation** | **No errors** (271 126 executions, 31 s) |
| Hardware repro (aarch64) | 10/10 hangs | 0/10 | 0/10 | 0/10 |

GenMC correctly rejects both timing-bandaid variants that the hardware reproducer cannot distinguish from a real fix. The baseline witness trace shows the exact race: scheduler's relaxed load of `sync` reads stale `notified=1`, worker's relaxed load of `run_queue` misses the push, task coherence-last in `run_queue_stack`.

### Controls

| Build | `notify` fast-path codegen | Hangs | Note |
|---|---|---|---|
| baseline | `casl; ldr` | 10/10 | the bug |
| `nop×3` perturbation | `casl; nop;nop;nop; ldr` | 10/10 | timing change alone doesn't fix |
| `fetchOr(0,.release)` | `casl; ldsetl wzr` | 0/10 | the fix |

### Standalone reproduction

A pure-Zig binary linking only `ThreadPool.zig` + stdlib (no Bun runtime), scheduling the same awaitable / fire-and-forget / awaitable shape: baseline **15/15 hangs** under load, fixed **0/15**, with lldb showing the identical stranded state.

### CI

`bun-file.test.ts` does not appear as flaky or failing in any of the three PR builds (172 completed aarch64 `test-bun` shard runs). Pre-fix it flaked on aarch64 in builds 20000, 35700, 36000, 41824, 42042, 43754.

</details>
