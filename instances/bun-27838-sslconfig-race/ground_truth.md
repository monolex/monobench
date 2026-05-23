# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**oven-sh/bun PR #27838** — fix: SSLConfig intern/deref race causing segfault in proxy tunnel setup

**fix commit:** 7b8aabb30b6a · **base (merge^):** 7ce412010a66 · merged 2026-03-08

## Changed source files (test/fixture files filtered out)
- packages/bun-usockets/src/crypto/openssl.c
- src/bun.js/api/server/SSLConfig.zig
- src/bun.js/webcore/fetch.zig
- src/bun.js/webcore/fetch/FetchTasklet.zig
- src/http.zig
- src/http/AsyncHTTP.zig
- src/http/HTTPContext.zig
- src/http/HTTPThread.zig

## PR body

## Problem

Segfault at address `0x0` in `create_ssl_context_from_bun_options` during proxy tunnel setup (Bun v1.3.11, linux x86_64_baseline). The crash is `strlen(NULL)` on a freed cert/key string.

## Root cause

`SSLConfig.GlobalRegistry` is a dedup cache keyed by TLS option content. It stored raw pointers without contributing to refcount — a weak cache — but `intern()` called plain `ref()` on what it found, which blindly does `fetchAdd(1)` with only a `debugAssert(old > 0)` (no-op in release).

| HTTP thread (`deref`, last holder) | JS thread (`intern`, new fetch same TLS opts) |
|---|---|
| `fetchSub` 1→0, enters `destroy()` | |
| | locks mutex, `getOrPut` finds dying entry (still in map) |
| | `ref()` → **0→1** (resurrection, no check in release) |
| | unlocks, returns dangling pointer to new fetch |
| locks mutex in `remove()`, evicts, unlocks | |
| `deinit()` frees cert/key strings | |
| `allocator.destroy()` | |
| | proxy tunnel builds SSL ctx from freed config → `strlen(NULL)` |

Proxy amplifies this: `ProxyTunnel.start()` creates a *second* SSL context from `tls_props` *after* the CONNECT round-trip, giving a multi-millisecond window for another request's teardown to win the race.

## Fix: Arc/Weak split refcounting

This is the Rust `Arc<T>`/`Weak<T>` pattern (and C++ `shared_ptr`/`weak_ptr`): two atomic counters instead of one.

### New primitive: `ThreadSafeWeakableRefCount`

Added to `src/ptr/ref_count.zig` alongside `ThreadSafeRefCount`. General-purpose, reusable for any type that needs weak references.

```zig
strong: atomic u32    // live users; 1->0 calls drop_contents()
weak:   atomic u32    // weak holders + (1 if strong > 0); 1->0 calls free_memory()
```

The `+1` on weak is the "collective" weak ref held on behalf of all strong refs. It guarantees the struct allocation stays live across the `strong 1→0 → drop_contents()` window, so `upgrade()` is memory-safe as long as *any* weak ref exists.

| Method | Does |
|---|---|
| `ref()` / `deref()` | bump/drop strong; at strong 1→0: `drop_contents()` then drop collective weak |
| `weakRef()` / `weakDeref()` | bump/drop weak; at weak 1→0: `free_memory()` |
| `upgrade()` | CAS-loop on strong, only increments if currently > 0 — **never revives a dead object** |

### SSLConfig wiring

```zig
const RC = bun.ptr.ThreadSafeWeakableRefCount(@This(), "ref_count", dropContents, freeMemory, .{});

fn dropContents(this: *SSLConfig) void {  // strong 1->0
    GlobalRegistry.remove(this);          // while content intact (map eql needs it)
    this.deinit();                        // free strings
}
fn freeMemory(this: *SSLConfig) void {    // weak 1->0
    bun.default_allocator.destroy(this);
}
```

### Registry now holds weak refs

**`intern()`**
1. `getOrPut` by content.
2. Found existing? `upgrade()` it:
   - **Success** (strong was > 0): got a real ref. Free `new_config`, return existing.
   - **Fail** (strong is 0, dying): registry `weakDeref`s the old entry, replaces the slot with `new_config`.
3. Registry `weakRef`s the winning entry.

**`remove()`** (called from `dropContents`)
- Look up by content hash, check **pointer identity**. If `intern()` replaced our slot while we were blocked on the mutex, the pointer won't match → no-op (intern already dropped our weak ref).
- Otherwise evict and `weakDeref`.

### Why `upgrade()` on strong==0 is not a UAF

The weak count holds the allocation. Registry holds a weak ref → `weak_count ≥ 1` → struct memory is live. The CAS reads a live atomic. Contents may be garbage (mid-`deinit()`), but `upgrade()` only touches `strong_count` — never the content.

This is the **key difference** from a single-counter design: memory safety is compositional (refcounts alone guarantee it), not entangled with mutex ordering. The mutex here only protects map structure and the invariant that entry content is intact while in the map.

### Corrected race

| HTTP thread | JS thread | Result |
|---|---|---|
| strong 1→0, blocked in `remove()` | `upgrade()` fails, `weakDeref` old (2→1), replace slot, `weakRef` new | old: `remove()` ptr-mismatch no-op → `deinit` → collective `weakDeref` (1→0) → free. JS thread has a fresh, valid config. |

## Also

- `packages/bun-usockets/src/crypto/openssl.c`: NULL guards before `strlen(content)` in `us_ssl_ctx_use_privatekey_content` and `us_ssl_ctx_use_certificate_chain`. Defense-in-depth — turns a segfault into a clean SSL error if a NULL ever slips through.
- `test/js/web/fetch/fetch-proxy-tls-intern-race.test.ts`: stress test firing overlapping waves of 8 concurrent proxy fetches with identical TLS options and `keepalive: false` (forces immediate deref on completion, no keepalive pool masking the race).


---

## Reproduction & verification

See #27863 for the full reproduction recipe. With a debug+ASAN build and `BUN_DEBUG_SSLConfig=1` (which widens the race window via stderr logging), the repro script crashes **3/3 on main** and **passes 3/3 on this branch**.

**Why it proves the same crash as production:**

The production segfault at `openssl.c:1173` and our debug assertion failure at `ref_count.zig:476` are the **same root cause**, caught at different points:

```
  [HTTP thread]                   [JS thread]
  deref() fetchSub 1→0
    ↓ (race window)
                                   intern() finds dying config
                                   ref() → old_count = 0
                                   ┌────────────────────────────┐
                                   │ DEBUG: assertValid → panic │ ← repro catches here
                                   │ RELEASE: silently succeeds │
                                   └────────────────────────────┘
  destroy/deinit/free              returns dangling *SSLConfig
                                   stored in client.tls_props
                                   ... proxy CONNECT ...
                                   tls_props.?.* → reads freed struct
                                   strdup(garbage) → segfault at 0x0  ← production crash
```

In release, `debugAssert(old_count > 0)` is a no-op. `ref()` silently does 0→1, `intern()` returns the dangling pointer, and the crash surfaces ~200μs later when `startProxyHandshake` dereferences the freed struct. Same race, same dangling pointer — debug catches it at the source, release at the symptom.

**On this branch:** `upgrade()` uses a CAS loop that refuses the 0→1 bump. When it fails, `intern()` replaces the map slot with a fresh config instead of resurrecting the dying one. The repro logs show many clean `deref 1 - 0` to different addresses (fresh allocations) with zero `ref 0 - 1`.
