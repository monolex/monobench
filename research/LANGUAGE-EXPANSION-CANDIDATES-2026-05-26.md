# monobench — Language-Expansion Candidates (2026-05-26)

Goal: extend monobench beyond its current language pairs with **famous-repo** bugs (so the
benchmark goes viral) that still pass the C1–C6 admission bar. Driven by the monogram call-graph
work in progress: the chain-gap languages (`cpp, c, ruby, java, csharp, lua, php`) and the
no-calls languages (`kotlin, dart, swift`) are exactly the languages monobench does **not** yet
exercise. Authoring instances here = a live test of that call-graph fix.

## The bar (do not relax it)

From `SPEC.md` C1–C6 + the hard-won rules in `CANDIDATES.md`:

- **Famous REPO, obscure BUG.** The repo must be instantly recognizable (viral). The *bug* must
  NOT be a headline CVE — those are contaminated. `ksmbd-37899` (o3's CVE) was solved **FULL by
  baseline-haiku with no tool** because the famous CVE lives in training data as prose →
  non-discriminating. The scalable seam is **merged UAF/race/lifetime fix-PRs**, not CVE headlines.
- **crash-fn ≠ cause-fn** (C1). Reject bugs where the crash is *inside* the buggy function. Want:
  crash deep in shared infra (GC, allocator, finalizer, parser, mutex), cause in a feature binding
  far away — ideally across a language boundary (C6).
- **Symptom must not name the API** (C2). Give the crash trace + behavior only; make the agent
  infer the subsystem. If the symptom says "HTMLRewriter", grep bridges straight to the file.
- **Objective gold patch** (C3) + **contamination recorded** (C4) + **defeats the baseline** (C5).

## Coverage map: have vs. gap

| Language pair | Current instances | Gap this doc fills |
|---|---|---|
| C++↔JS | node ×5 | — |
| Zig / Zig↔C++ | bun ×7, ghostty | — |
| C↔Python | cpython ×3 | (alt: micropython) |
| Rust↔JS | deno ×1 | — |
| C (kernel) | ksmbd | redis (C, new repo) |
| **Ruby↔C** | — | ✅ ruby/ruby, nokogiri |
| **Ruby↔Rust** | — | ✅ ruby/ruby YJIT |
| **Java↔native** | — | ✅ netty |
| **C#↔native** | — | ✅ dotnet/runtime ×2 |
| **PHP↔C** | — | ✅ php/php-src |
| **Lua↔C** | — | ✅ openresty, neovim, redis-lua |
| **Go / Go↔cgo** | — | ✅ moby, kubernetes, mattn/go-sqlite3 |
| **Dart↔C++** | — | ✅ flutter, dart-lang/sdk |
| **Swift** | — | ✅ swiftlang/swift, vapor, swift-nio |
| **Kotlin** | — | ✅ ktor (thin — see gaps) |
| **pure C++** | — | ✅ grpc, envoy |
| **Python↔C (FFI)** | — | ✅ cryptography, Pillow |

---

## Full-language regression matrix (monogram support × instance)

**This set doubles as a per-language regression suite for the call-graph fix.** Each instance forces
the solver to follow a root-cause edge *in that language*, so a baseline-MISS / monogram-FULL split
is direct evidence the extractor now traces that language's calls. All 17 monogram languages now
have ≥1 candidate; the 10 broken ones (7 chain-gap + 3 no-calls) are the priority.

| monogram tier | lang | instance(s) | edge under test |
|---|---|---|---|
| OK | rust | ruby-16128, deno* | Rust `&mut` aliasing across fns |
| OK | ts | tauri (doc), node* (shared js/tsx extractor) | TS→Rust `invoke` |
| OK | js | node* ×5 | JS↔C++ callback |
| OK | py | cpython* ×3, numpy-31314 | Python↔C re-entrancy |
| OK | go | gosqlite-1301 | Go↔cgo finalizer |
| OK | zig | bun* ×7, ghostty* | Zig / Zig↔C++ |
| OK | scala | spark-43188 | Scala dispatch (logic; OK-tier = regression guard) |
| CHAIN GAP | cpp | grpc-39316, envoy-45153, pytorch-73029 | C++ callback/shutdown lifetime |
| CHAIN GAP | c | redis ×2, php-19591, openresty-2483, neovim-37647, numpy-31314, ksmbd* | C re-entrancy/ownership |
| CHAIN GAP | ruby | ruby-16128, ruby-16964 | Ruby↔C / Ruby↔Rust |
| CHAIN GAP | java | netty-12036 | Java↔native (Unsafe) |
| CHAIN GAP | csharp | dotnet-124796, dotnet-125293 | C#↔native (COM) keepalive/handle |
| CHAIN GAP | lua | openresty-2483, neovim-37647 | Lua↔C callback lifetime |
| CHAIN GAP | php | php-19591 | PHP↔lexbor (C) lifecycle |
| NO CALLS | kotlin | ktor-5626 | Kotlin coroutine/JVM resource lifecycle (pure .kt) |
| NO CALLS | dart | dart-3095 (pure .dart), flutter-170284 (engine C++/JNI) | Dart Uri backslash normalization / engine interop |
| NO CALLS | swift | vapor-2500 (pure .swift), swift-88509 (C++ toolchain) | Swift FileMiddleware order-of-ops / Demangler C++ |

`*` = already-existing instance. **Scala** is OK-tier already, so its instance is a regression guard,
not a fix-test; it's also the thinnest seam (Spark is the only big pure-Scala repo). **ts** shares
monogram's js/tsx extractor (node* exercises it; Tauri adds the TS→Rust hop).

**Authored this batch — ALL 21 as full instance dirs** (`instance.json` with ground_truth+grading,
`symptom.md`, `ground_truth.md`), each answer key naming the exact graded function pulled from the PR
diff: ruby-16128, ruby-16964, dotnet-124796, dotnet-125293, netty-12036, redis-14929, redis-15095,
php-19591, openresty-2483, neovim-37647, grpc-39316, envoy-45153, numpy-31314, pytorch-73029,
flutter-170284, swift-88509, vapor-2500, ktor-5626, spark-43188, kubernetes-3177, dart-3095.
Benchmark grew **18 → 39 instances**. (`gosqlite-1301` dropped — PR #1301 is a perf change, not a
bug.) JSON validated; what remains is the **admission gate** (run baseline n≥3 per instance; keep
the discriminating ones). Two carry honest caveats in their admission note: `pytorch-73029`
(crash≈cause, 2022 contamination) and `kubernetes-3177`/`vapor-2500` (famous CVEs → verify baseline
doesn't solve from prose).

---

## TIER A — author-ready (verified by me; crash≠cause UAF/race fix-PRs; famous repo, obscure bug)

These are the recommendations. Every commit below was fetched directly via `gh` (PR body +
merge commit + base = merge's first parent). They match the existing node/bun/cpython instance
shape almost exactly, just in new languages.

### A1 ⭐⭐⭐ ruby/ruby #16128 — YJIT `version_map` UAF from mutable-aliasing UB  `[Ruby↔Rust↔C]`
- **langs:** rust, ruby, c · **repo ~22k stars** · merge `e730ac41be4d` · base `906176adb49a`
- **crash site (decoys, all wrong):** "crashes in various YJIT operations — **block lookup, GC
  marking, block removal**" (3 built-in decoys).
- **root cause:** `get_iseq_payload()` called twice → overlapping `&'static mut IseqPayload` →
  LLVM `noalias` caches a stale `version_map` Vec header → points to freed backing storage.
  Fix sites: `rb_yjit_tracing_invalidate_all` (invariants.rs), `add_block_version` (core.rs).
- **why baseline misses:** the crash is in three different ops; the cause is aliasing UB created
  earlier in unrelated call sites. No text token bridges crash→cause. Pure ownership/aliasing —
  monogram's home turf.
- **viral:** 5 (Rust UB *inside Ruby's JIT* is a great story). **monogram-tier:** chain-gap (ruby) + Rust.

### A2 ⭐⭐⭐ dotnet/runtime #124796 — missing `GC.KeepAlive` → premature-GC UAF  `[C#↔native]`
- **langs:** csharp, c++ · **repo ~16k stars** · merge `adc191279b42` · base `0f125741186d` · **1 file**
- **root cause:** `WMIInterop.cs` passes `IntPtr` fields to native callbacks without rooting the
  managed object holding them; GC collects it mid-call → native deref of freed memory. The exact
  premature-GC-UAF shape of your `node-56840`, but managed↔native.
- **🔥 provenance:** PR body literally says *"Opus highlighted these as potential issues."* — an
  **AI-found** bug that is still obscure (an interop file, not a headline CVE) → both a viral hook
  *and* discriminating. Caveat: a defensive fix ("wasn't able to prove" the crash), so frame as
  ownership-correctness, not a confirmed crash.
- **why baseline misses:** symptom is a native heap corruption; cause is an absent keepalive in
  the C# interop shim. Single-language analyzers see neither side of the boundary. **viral:** 5.

### A3 ⭐⭐⭐ ruby/ruby #16964 — `IO::Buffer#&` UAF on an invalidated slice  `[Ruby↔C]`
- **langs:** ruby, c · merge `2552db04ddc4` · base `356c0cd0e795` · **1 file** (`io_buffer.c`)
- **root cause:** `io_buffer_and` reads `buffer->base` directly; a slice whose parent was `free`d
  retains a stale base pointer. Decoy = the `&` operator crash site; cause = the parent `free`
  happening elsewhere in the flow.
- **why baseline misses:** crash in the bitwise-and path ≠ cause (lifetime of the parent buffer).
  Clean single-file grade. **viral:** 4 (Ruby is everywhere). **monogram-tier:** chain-gap (ruby).

### A4 ⭐⭐⭐ redis/redis #14929 — module re-entrancy UAF in `restoreCommand`  `[C, new famous repo]`
- **langs:** c · **repo ~74k stars** · merge `40c140bf16ca` · base `bbc0dcbb9af7` · 2 files
- **root cause:** a module's keyspace-notification callback calls `RedisModule_SetKeyMeta` →
  `kvobjSet` reallocs the kvobj and frees the old one → `restoreCommand`'s local `kv` dangles →
  `kv->type` reads freed memory. Re-entrancy across the module C-API boundary.
- **why baseline misses:** crash reading `kv` after RESTORE ≠ cause (a callback three hops away on
  the notification path). **viral:** 5 (Redis). **monogram-tier:** chain-gap (c).

### A5 ⭐⭐⭐ envoyproxy/envoy #45153 — async-file callback-after-cancel UAF  `[pure C++]`
- **langs:** c++ · **repo ~26k stars** · merge `96294652f7f2` · 4 files
- **root cause:** `FileStreamer::start()` captures `[this]` into `AsyncFileManager::stat()/read()`
  callbacks; the manager guarantees the callback fires *even after cancel*, so if the
  `FileStreamer` is destroyed before the callback runs, the bare `this` dangles.
- **why baseline misses:** crash in the async-file callback ≠ cause (the cancel/destroy path). The
  link is a captured-`this` lifetime edge across the callback boundary. **viral:** 4.

### Rest of Tier A (same bar; compact)

| id | repo · PR | langs / boundary | crash ≠ cause | commit (merge / base) | viral |
|---|---|---|---|---|---|
| grpc-39316-rls-shutdown-uaf | grpc/grpc #39316 | C++ | config read ≠ LB-policy shutdown freeing it | `6c472088f7ce` / first-parent | 4 |
| netty-12036-unsafe-bytebuffer-uaf | netty/netty #12036 | Java↔native(Unsafe) | native-mem deref ≠ iteration that didn't retain it | `71d829a34236` / `5b0e0a9820f1` | 4 ⚠ 6-file fix |
| dotnet-125293-gchandle-doublefree-race | dotnet/runtime #125293 | C#↔native | GCHandle double-free ≠ non-atomic dispose check | `28c5a4dcc838` / `133f6a80839a` | 4 |
| php-19591-lexbor-mraw-uaf | php/php-src #19591 | PHP↔lexbor(C) | UAF on live `Url` obj ≠ periodic `lexbor_mraw_clean()` | `423960aad30b` / `90822f7692e7` | 4 |
| openresty-2483-luapipe-quic-uaf | openresty/lua-nginx-module #2483 | Lua↔C | `ngx_http_lua_pipe` cleanup ≠ pool destroy on QUIC close | `cac9cbc3b700` / first-parent | 4 |
| gosqlite-1301-rows-finalizer | mattn/go-sqlite3 #1301 | Go↔cgo | finalizer frees stmt ≠ Rows lifetime | `c61eeb5d1d1c` / first-parent | 3 |
| redis-15095-stream-pel-doublefree | redis/redis #15095 | C | teardown double-free ≠ duplicate-PEL error branch | `fab099cdcffb` / `0d9576435f83` | 4 |
| flutter-170284-surfacetexture-doublefree | flutter/flutter #170284 | Dart↔C++/JNI | reactor 2nd free ≠ detachFromGLContext already released | `2d30ce56feb7` / first-parent | 4 ⚠ engine-C++ side |
| neovim-37647-deferfn-luv-leak | neovim/neovim #37647 | Lua↔C(libuv) | leaked luv handle ≠ schedule-failure path in defer_fn | `1906da52dbc9` / first-parent | 3 |
| swift-88509-demangler-uaf | swiftlang/swift #88509 | Swift/C++ | `Words[]` not saved across nested demangle calls | verify commit | 3 |
| numpy-31314-nditer-getitem-segfault | numpy/numpy #31314 | Python↔C | C nditer setter segfault ≠ Python `__getitem__` raising | `fa6d67432de1` / first-parent | 4 |

---

## TIER B — strong logic/security bugs in famous frameworks (breadth + virality)

Mostly single-language (weaker C6 cross-boundary), but several are excellent **"missing-check-arm"
structural** bugs (C1/C2 hold: the bug is *absent code* on one of several enumerated paths, which
grep cannot surface). ✓ = commit verified by me; ⧗ = agent-reported, verify before authoring.

| repo · CVE | lang | bug shape (why it fits) | commit |
|---|---|---|---|
| moby/moby CVE-2024-41110 | Go | AuthZ bypass: body stripped before plugin dispatch (cross-layer) | `9659c3a52bac` ✓ |
| kubernetes/k8s CVE-2024-3177 | Go | `envFrom` secret check **missing** from 1 of 5 enumerated arms | `b722d017a34b` ✓ (merge) |
| symfony/symfony CVE-2024-51996 | PHP | RememberMe: token valid but **username ownership unchecked** | `81354d392c5f` ⧗ |
| laravel/framework CVE-2024-52301 | PHP | env forced via `$argv` from query string (SAPI taint) | `18b326d22d83` ⧗ |
| rails/rails CVE-2025-24293 | Ruby | ActiveStorage cmd-injection via allowlisted transforms→mini_magick | `1b1adf6ee6ca` ⧗ |
| rails/rails CVE-2022-23633 | Ruby | CurrentAttributes thread-local leak on dropped `body#close` | `10c64a472f2f` ⧗ |
| django/django CVE-2024-42005 | Python | SQLi via JSONField column-alias through 3 ORM layers | `c87bfaacf8fb` ⧗ |
| pallets/jinja CVE-2025-27516 | Python | sandbox escape: `|attr` bypasses `Environment.getattr` | `90457bbf33b8` ⧗ |
| pallets/werkzeug CVE-2024-34069 | Python | debugger CSRF→RCE: origin check absent before PIN logic | `3386395b24c7` ⧗ |
| psf/requests CVE-2023-32681 | Python | Proxy-Authorization leaked to dest on redirect | `74ea7cf7a6a2` ⧗ |
| aiohttp CVE-2024-23334 | Python | static `follow_symlinks=True` missing `is_relative_to(root)` | `1c335944d6a8` ⧗ |
| heartcombo/devise CVE-2026-32700 | Ruby | confirm-email TOCTOU: token vs `unconfirmed_email` desync | `02527772bd9a` ⧗ |
| sparklemotion/nokogiri CVE-2022-29181 | Ruby↔C | Ruby→C SAX type confusion (no `Check_Type`) | `db05ba9a1bd4` ✓ |
| pyca/cryptography CVE-2020-36242 | Python↔C | Python unbounded int → 32-bit C overflow in OpenSSL | `82b6ce28389f` ✓ |
| python-pillow/Pillow CVE-2026-42311 | Python↔C | int-overflow **bypasses** an earlier bounds-check patch | `58f9a1d166dc` ⧗ |
| dart-lang/sdk CVE-2022-3095 | Dart | `Uri` treats `\`≠browser → allowlist auth bypass | `c4c802eeb6fe` ✓ |
| dart-lang/sdk CVE-2022-0451 | Dart | HttpClient leaks auth headers on cross-origin redirect | `57db739be0ad` ✓ |
| vapor/vapor CVE-2020-15230 | Swift | path traversal: `..` check **before** percent-decode | `cf1651f7ff76` ✓ |
| swiftlang/swift-nio CVE-2022-3215 | Swift | CRLF injection: validator stage absent from encoder | `a16e2f54a25b` ✓ ⚠ 13-file |
| ktorio/ktor CVE-2020-5207 | Kotlin | request smuggling: chunked + Content-Length both accepted | `d937a1e46172` ✓ |
| tauri-apps/tauri (ACL/remote-origin) | TS↔Rust | IPC origin enforcement gap (your own app's architecture) | `1b26769f92b5` ✓ ⚠ verify CVE map |

> **Why Tauri is special:** it's TS↔Rust IPC — the *exact* boundary monogram's `coupling --domain
> tauri-ipc` targets, and the architecture of app-monolex itself. A Tauri instance dogfoods the
> tool on its own stack.

---

## TIER C — viral-narrative only (CONTAMINATION-FLAGGED, likely fail C5)

The entire "AI cracked a famous bug" genre is **C memory-corruption in infra**. Great marketing
("monobench includes the bugs o3 / Big Sleep found"), but per the contamination rule these are in
the models' training data as prose → expect baseline FULL → **down-weight / hold out, do not count
as tool-value signal.**

| repo | CVE | AI finder | note |
|---|---|---|---|
| torvalds/linux ksmbd | CVE-2025-37899 | OpenAI o3 | already proven baseline-FULL (the canonical contamination example) |
| torvalds/linux ksmbd | CVE-2025-37778 | o3 + Claude 3.7 | krb5 re-auth TOCTOU |
| sqlite/sqlite (series.c) | none (pre-release) | Big Sleep | high gradeability, no CVE |
| sqlite/sqlite | CVE-2025-6965 | Big Sleep | first AI-blocked in-the-wild exploit |
| redis/redis (RediShell) | CVE-2025-49844 | Wiz Research | Lua UAF→RCE in `deps/lua/src/lparser.c` ✓ commit `155519b195d8`; huge name, but headline |
| openssl/openssl | CVE-2024-9143 | OSS-Fuzz-Gen | 20-yr-old GF(2^m) OOB |
| FFmpeg/FFmpeg | CVE-2025-59733/59734 | Big Sleep | EXR / SANM decoders |
| ImageMagick | CVE-2025-55004 | Big Sleep | MNG alpha over-read |
| curl/curl | CVE-2025-9086 | Big Sleep | cookie-path OOB |

---

## Multi-language combos (your explicit ask) — the headline grid

| boundary | best candidate | tier |
|---|---|---|
| Ruby ↔ Rust | ruby/ruby #16128 (YJIT aliasing UAF) | A1 |
| Ruby ↔ C | ruby/ruby #16964; nokogiri CVE-2022-29181 | A3 / B |
| Python ↔ C | cryptography CVE-2020-36242; Pillow CVE-2026-42311 | B |
| C# ↔ native | dotnet #124796 (Opus-found), #125293 | A2 / A |
| Java ↔ native | netty #12036 | A |
| Lua ↔ C | openresty #2483; neovim #37647; redis RediShell | A / C |
| Go ↔ cgo | mattn/go-sqlite3 #1301 | A |
| Dart ↔ C++ | flutter #170284 | A |
| PHP ↔ C | php/php-src #19591 | A |
| TS ↔ Rust | tauri ACL/remote-origin | B (dogfood) |

---

## Verification ledger (honest)

**Dropped — commit was wrong or not gradeable in-repo:**
- `denoland/deno` CVE-2024-27933 — the cited hash `55fac9f5ead6` is "child_process IPC on
  Windows", NOT the fd-close security fix. Re-find the real commit before use.
- `rustls/rustls` CVE-2024-32650 — cited hash `a74f9d531b49` is "deps: update cargo deps", NOT
  the `complete_io` loop fix. Re-find.
- `lovell/sharp` CVE-2023-4863 — `eefaa998725c` is a **release commit** (libwebp dep bump); the
  actual bug lives upstream in libwebp (different repo) → not gradeable *in sharp*.

**Caveats:** swift-nio (13-file merge — large grade surface); tauri (real security fix, exact CVE
mapping unconfirmed); kubernetes/neovim entries are merge commits (use first-parent as base).

**Agent "NOT FOUND" (no public gold patch → unusable until disclosed):** aspnetcore CVE-2024-35264
/ CVE-2025-24070, jellyfin, etcd CVE-2026-44283, containerd CVE-2025-47290, kafka CVE-2024-27309,
ZeroPath's 12 OpenSSL 0-days, Anthropic Mythos curl find (CVE planned curl 8.21.0).

## Gaps by language (set expectations)

- **Swift / Dart / Kotlin / Scala:** ARC/GC means few UAF/race; the gradeable bugs are *logic*
  (path traversal, smuggling, auth bypass) not memory. Star counts lower (dart-sdk ~11k,
  swift-nio ~8k, ktor ~14k). Use Tier-B logic bugs for these.
- **Lua (pure):** scarce. The real Lua bugs live in the **embedded-Lua C** (`redis lparser`,
  openresty C) or at the **Lua↔C boundary** (neovim treesitter `dlopen`, `ngx_http_lua_pipe`).
- **AI-found genre:** ~100% C. Your other-language wedge is *unoccupied territory* — nobody
  benchmarks AI bug-finding in Ruby/C#/PHP/Lua/Dart. That is the viral angle.

## backlog.tsv block (Tier A — paste-ready)

```
ruby-16128-yjit-aliasing-uaf|ruby/ruby|16128|rust,ruby,c|mutable-aliasing UB UAF (YJIT version_map; crash in block lookup/GC mark/removal ≠ cause)
dotnet-124796-wmiinterop-keepalive|dotnet/runtime|124796|csharp,c++|premature-GC UAF (missing GC.KeepAlive managed↔native; AI-found by Opus)
ruby-16964-iobuffer-and-uaf|ruby/ruby|16964|ruby,c|UAF on invalidated slice (IO::Buffer#&; parent freed elsewhere)
redis-14929-restorecmd-meta-uaf|redis/redis|14929|c|re-entrancy UAF (module notification reallocs kvobj; restoreCommand kv dangles)
envoy-45153-filestreamer-cancel-uaf|envoyproxy/envoy|45153|c++|async-callback-after-cancel UAF (FileStreamer [this] capture)
grpc-39316-rls-shutdown-uaf|grpc/grpc|39316|c++|config-after-shutdown UAF (RLS LB policy)
netty-12036-unsafe-bytebuffer-uaf|netty/netty|12036|java,c|native-memory lifetime UAF (forEachReadable siphoned ByteBuffer)
dotnet-125293-gchandle-doublefree-race|dotnet/runtime|125293|csharp,c++|GCHandle double-free race (non-atomic dispose)
php-19591-lexbor-mraw-uaf|php/php-src|19591|php,c|UAF (periodic lexbor_mraw_clean frees live Url objects)
openresty-2483-luapipe-quic-uaf|openresty/lua-nginx-module|2483|lua,c|UAF (ngx_http_lua_pipe cleanup after pool destroy on QUIC close)
gosqlite-1301-rows-finalizer|mattn/go-sqlite3|1301|go,c|finalizer/lifetime (SetFinalizer on SQLiteRows; cgo)
redis-15095-stream-pel-doublefree|redis/redis|15095|c|double-free (duplicate consumer PEL frees shared nack)
flutter-170284-surfacetexture-doublefree|flutter/flutter|170284|dart,c++|double-free interop texture (reactor second free after foreground)
neovim-37647-deferfn-luv-leak|neovim/neovim|37647|lua,c|luv handle leak (defer_fn timer not closed when schedule fails)
numpy-31314-nditer-getitem-segfault|numpy/numpy|31314|python,c|re-entrancy segfault (nditer multi_index setter missing NULL check when __getitem__ raises)
```

## Cross-analysis: the NIIA planning doc

The parallel NIIA artifact (`…/work/monolex-006/wip/research/monobench-monogram-call-graph-language-gaps/00-OVERVIEW…md`)
is **strategically right, mechanically incomplete.**

- **Right:** fame + multi-language + targeting the chain-gap/no-calls languages is the correct axis;
  its gap-cause analysis (C++ templates/virtuals, Java lambdas/method-refs, Ruby blocks, PHP dynamic
  calls; no caller edges for Kotlin/Dart/Swift) is accurate and reinforces these candidates.
- **The core flaw:** it proposes **navigation tasks** ("trace a `torch.nn.functional` call through
  the ATen dispatcher into the kernel"), but monobench grades `ROOTCAUSE file::fn` + `FIX` against a
  **merged gold patch** (SPEC §5) and admits only **symptom≠cause** bugs that **defeat a hard
  baseline** (C3, C5). A "trace X→Y" task has no objective answer key and is navigation-only — which
  `CANDIDATES.md` explicitly calls "mostly grep-solvable → most FAIL the admission gate." It is a
  capability wishlist, not an instance spec.
- **The fame paradox it misses:** famous *repo* = viral (good); famous *bug/CVE* = training-data
  prose → baseline solves it → non-discriminating (bad, see `ksmbd-37899`). Famous repo, obscure bug.

| NIIA pick | verdict | the gradeable version |
|---|---|---|
| PyTorch (#1) | viral, but UAF/race PRs mostly 2018 (contaminated) | pytorch #73029 NNC `run()` data race (2022); B-shape, cause≈crash |
| Flutter (#2) | "trace MethodChannel" = navigation, ungradeable | flutter #170284 Dart↔C++ texture double-free (real gold patch, A-tier) |
| NumPy | high-value ✓ AND authorable | numpy #31314 nditer `__getitem__` segfault (Python↔C) — added to Tier A |
| Node.js core | **already covered** (5 instances exist) | n/a |
| Neovim | good | neovim #37647 / Treesitter `dlopen` RCE |
| Linux beyond ksmbd | viable | obscure merged fix-PRs, NOT headline CVEs |
| Ruby/Java/Kotlin/Swift | directionally right | specific PRs in Tier A/B above |

**Factual nits:** it's `pytorch/pytorch`, not `facebookresearch/pytorch`; `flutter/engine` was folded
into the `flutter/flutter` monorepo in 2025 (`#170284` paths are `engine/src/flutter/…`).

**Bottom line:** the NIIA doc = the *aspiration*; this candidate list = the *executable* version
(15 verified gold-patch fix-PRs, crash≠cause, same strategy). Use the NIIA doc for viral framing and
repo targeting; author from these PRs.

## Recommended next step

Author the top 5 (A1–A5) via the recipe: pin `base_commit` = merge's first parent, read the PR
body for root-cause fn + the correct sibling path (= decoy), **de-keyword the symptom** so the
crash trace doesn't grep to the cause, run the baseline admission gate (n≥3, thin arm), keep only
the discriminating ones. A1 (Ruby↔Rust) and A2 (C#↔native, Opus-found) are the highest
viral×discriminating bets and cover two brand-new pairs at once.
