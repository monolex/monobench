# Ground truth — ⚠️ SPOILER (do not feed to the agent)

**Root cause:** `src/bun.js/bindings/BunString.cpp` → `BunString__toThreadSafe()`

```cpp
auto impl = str->impl.wtf->isolatedCopy();   // always a FRESH impl
if (impl.ptr() != str->impl.wtf) {
    str->impl.wtf = &impl.leakRef();          // ← old StringImpl ref NEVER released
}
```

`isolatedCopy()` always allocates a new impl, so the branch is always taken and the original
`StringImpl`'s ref is leaked. The Zig side (`src/string.zig` `SliceWithUnderlyingString.toThreadSafe`)
had a *compensating* `orig.deref()` that masked it on `Bun.file` / async-`fs.write` paths — so the
refcount was balanced on some paths and off-by-one on others → **leak** (OOM / 1GB orphans) on direct
callers + **double-free → use-after-free** elsewhere → the `String`'s `tag` byte becomes garbage → an
exhaustive `switch (this.tag)` in `string.zig` (`length()` / `utf8ByteLength`) panics
`switch on corrupt value`.

**Decoy:** `toCrossThreadShareable` (BunString.cpp:322) is an *adjacent* cross-thread string helper.
A shallow search lands here — wrong function.

**Fix — PR #30049, commit `1b82e1d4`:**
```cpp
auto* existing = str->impl.wtf;
auto impl = existing->isolatedCopy();
if (impl.ptr() != existing) {
    existing->deref();                 // release old ref — clean ownership transfer
    str->impl.wtf = &impl.leakRef();
}
```
…and remove the now-redundant `orig.deref()` in `src/string.zig` (else it double-frees).

## Why this is a fair instance
- Symptom (`string.zig` crash) ≠ root cause (`BunString.cpp`) — different file AND language.
- The symptom string leads nowhere near the cause; you must reason "refcount/ownership" first.
- Has a decoy that punishes shallow pattern-matching.
- Cross-language (Zig→C++ FFI) — the hop a call-graph tool *should* bridge.
- Objective ground truth (the merged fix).
