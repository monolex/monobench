# bun-1.3.10-toThreadSafe / gpt-5.3-codex-spark high monogram analysis

Date: 2026-05-23

Source run root:
`/Users/macbook/.monobench/0.1.6-1779528810`

Archived raw logs:
`raw/results/`

Archived instance files:
`raw/instance/`

Integrity file:
`SHA256SUMS`

## Current Result State

The currently archived monobench result differs from the remembered note that r1 was FULL.
In the only completed `0.1.6-1779528810` result set found locally, r1 is graded MISS.

| run | grade | cost | tokens | time | calls | mono calls | cache read | cache pct |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| monogram-gpt-5.3-codex-spark-high-r1 | MISS | $3.19 | 11.56M | 790s | 166 | 165 | 11.15M | 96.4% |
| monogram-gpt-5.3-codex-spark-high-r2 | MISS | $8.38 | 31.26M | 2223s | 489 | 450 | 30.26M | 96.8% |
| monogram-gpt-5.3-codex-spark-high-r3 | FULL | $1.75 | 5.85M | 552s | 67 | 64 | 5.58M | 95.5% |

Monobench report summary:

```text
monogram-gpt-5.3-codex-spark-high-r1 MISS $3.19   790s   166c  .165
monogram-gpt-5.3-codex-spark-high-r2 MISS $8.38  2223s   489c  .450
monogram-gpt-5.3-codex-spark-high-r3 FULL $1.75   552s    67c  .64
```

## Ground Truth

Root cause:

`src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`

Critical ownership bug:

```cpp
auto impl = str->impl.wtf->isolatedCopy();
if (impl.ptr() != str->impl.wtf) {
    str->impl.wtf = &impl.leakRef();
}
```

The old `StringImpl` ref is overwritten without release. The fix releases the previous impl before the pointer swap and removes the compensating Zig-side `orig.deref()`.

Important decoy:

`src/bun.js/bindings/BunString.cpp::toCrossThreadShareable`

## Answers

| run | answer root cause | grade meaning |
|---|---|---|
| r1 | `BunString.cpp::BunString__transferToJS` | Near miss. Correct file and ownership domain, wrong terminal transfer helper. |
| r2 | `src/string.zig::toThreadSafeSlice` | Deep wrong compensating path. It latched onto Zig-side ref balancing instead of the C++ pointer overwrite. |
| r3 | `BunString.cpp::BunString__toThreadSafe` | Correct. It inspected `isolatedCopy` and `leakRef` early, then anchored on the pointer swap. |

## Successful Pattern In r3

r3 found the useful ownership trail early:

```text
monogram grep "isolatedCopy" --chain --depth 2 --json
monogram context toCrossThreadShareable --code 120
monogram context BunString__toThreadSafe --code 140
monogram chain BunString__toThreadSafe --callers --depth 3
```

The decisive context output showed:

```text
BunString__toThreadSafe
calls: isolatedCopy, ptr, leakRef

214 auto impl = str->impl.wtf->isolatedCopy();
216     str->impl.wtf = &impl.leakRef();
```

This is the pattern monogram should amplify: ownership verbs plus mutation of an input-owned pointer field.

## Failure Pattern In r1

r1 stayed in the right ownership neighborhood but selected the wrong endpoint:

```text
ROOTCAUSE: BunString.cpp::BunString__transferToJS
```

Observed pattern:

- It found `transferToJS` before the correct `BunString__toThreadSafe`.
- It did eventually inspect `BunString__toThreadSafe`, but earlier evidence from `transferToJS` remained dominant.
- It treated the terminal `deref()` path as the cause instead of asking where ownership first became imbalanced.

Monogram strengthening target:

When a candidate contains `deref()` but another nearby candidate contains `isolatedCopy + leakRef + pointer overwrite`, rank the latter higher as the origin of ownership imbalance.

## Failure Pattern In r2

r2 is the most valuable failure log. It spent 31.26M tokens and 489 tool calls, then selected:

```text
ROOTCAUSE: src/string.zig::toThreadSafeSlice
```

Observed pattern:

- It repeatedly returned to `toThreadSafeSlice`, `fromBunString`, `SliceWithUnderlyingString`, and `transferToJS`.
- It inspected `BunString__toThreadSafe` early and again late, but did not promote it above the Zig compensating path.
- It used `monogram coupling --domain ffi --pattern toThreadSafeSlice --all` and got 0 keys.

The 0-key result is not proof that FFI extraction is absent. `toThreadSafeSlice` is not itself an FFI binding key. The `context` hint led the agent to check the local Zig function name as an FFI key, which is a bad hint for this case.

## Coupling CLI Issue Found

r3 also ran:

```text
monogram coupling --domain ffi --pattern "BunString__toThreadSafe|toThreadSafeEnsureRef|Bun__WTFStringImpl__deref|Bun__WTFStringImpl__ref" --all
```

and got:

```text
MONOGRAM COUPLING - 0 key(s)
No bindings indexed. Run `monogram index` first.
```

Current `monogram coupling` code treats `--pattern` as a literal substring:

```rust
results.retain(|r| r.key.contains(p.as_str()));
```

So a regex-looking alternation such as `A|B|C` cannot match any key. This makes the output misleading: the user sees "No bindings indexed" even though the specific filter syntax is the likely cause.

## Monogram Strengthening Candidates

1. Support multi-pattern or regex-looking `--pattern` input for `coupling`.
   At minimum, split `A|B|C` into alternatives or print that `--pattern` is literal.

2. Improve empty-state wording for filtered coupling results.
   Distinguish:
   `no coupling bindings exist in DB` vs `bindings exist, but this filter matched 0`.

3. Make `context` FFI hints boundary-aware.
   For a non-FFI local helper like `toThreadSafeSlice`, prefer:
   `monogram coupling --domain ffi --all`
   or suggest concrete FFI callees/callers such as `BunString__toThreadSafe`, not the local helper name.

4. Add ownership-risk ranking.
   A symbol should get a high-risk marker when the same function contains:
   `isolatedCopy`, `leakRef`, assignment to an existing pointer field, and no nearby `deref`/release of the old field.

5. Add decoy suppression for adjacent pure helpers.
   `toCrossThreadShareable` has the right vocabulary but is a helper returning a new value. `BunString__toThreadSafe` mutates the input struct. Mutation should outrank adjacent helper use in root-cause ranking.

6. Add a loop breaker for repeated deep traces.
   r2 re-expanded the same wrong cone many times. After a high-call run has already inspected a stronger ownership candidate twice, monogram should surface a "compare these candidates" hint.

## Recommended Regression Fixture

Create a compact fixture with this shape:

```zig
extern fn BunString__toThreadSafe(this: *String) void;

pub fn toThreadSafeSlice(this: *String) void {
    bun.cpp.BunString__toThreadSafe(this);
}
```

```cpp
extern "C" [[ZIG_EXPORT(nothrow)]] void BunString__toThreadSafe(BunString* str) {
    auto impl = str->impl.wtf->isolatedCopy();
    if (impl.ptr() != str->impl.wtf) {
        str->impl.wtf = &impl.leakRef();
    }
}
```

Expected monogram behavior:

- `coupling --domain ffi --pattern BunString__toThreadSafe --all` shows both Zig called site and C++ defined site.
- `coupling --domain ffi --pattern "BunString__toThreadSafe|toThreadSafeSlice" --all` does not silently return 0.
- `context BunString__toThreadSafe` emits an ownership-risk hint focused on `isolatedCopy + leakRef + pointer overwrite`.
- `context toThreadSafeSlice` does not suggest `coupling --domain ffi --pattern toThreadSafeSlice` as if it were itself a boundary key.

## Raw Log Inventory

```text
raw/results/monogram-gpt-5.3-codex-spark-high-r1.answer.txt
raw/results/monogram-gpt-5.3-codex-spark-high-r1.codexlog
raw/results/monogram-gpt-5.3-codex-spark-high-r1.err
raw/results/monogram-gpt-5.3-codex-spark-high-r1.meter.json
raw/results/monogram-gpt-5.3-codex-spark-high-r2.answer.txt
raw/results/monogram-gpt-5.3-codex-spark-high-r2.codexlog
raw/results/monogram-gpt-5.3-codex-spark-high-r2.err
raw/results/monogram-gpt-5.3-codex-spark-high-r2.meter.json
raw/results/monogram-gpt-5.3-codex-spark-high-r3.answer.txt
raw/results/monogram-gpt-5.3-codex-spark-high-r3.codexlog
raw/results/monogram-gpt-5.3-codex-spark-high-r3.err
raw/results/monogram-gpt-5.3-codex-spark-high-r3.meter.json
raw/results/mcp-empty-monogram-gpt-5.3-codex-spark-high-r1.json
raw/results/mcp-empty-monogram-gpt-5.3-codex-spark-high-r2.json
raw/results/mcp-empty-monogram-gpt-5.3-codex-spark-high-r3.json
raw/instance/ground_truth.md
raw/instance/instance.json
raw/instance/symptom.md
```
