# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**oven-sh/bun PR #29951** — bake: fix use-after-free in DirectoryWatchStore when a client component boundary is demoted

**fix commit:** aa90c281c613 · **base (merge^):** aa3f9803a8c6 · merged 2026-04-29

## Changed source files (test/fixture files filtered out)
- src/bake/DevServer/DirectoryWatchStore.zig
- src/bake/DevServer/IncrementalGraph.zig

## PR body

## Problem

`DirectoryWatchStore.Dep.source_file_path` borrows the key slice from `IncrementalGraph.bundled_files` (via `insertEmpty`, see the comment at `DirectoryWatchStore.zig:69-71`). When a client-component boundary loses its `"use client"` directive, `server_graph.receiveChunk` detects the demotion and calls `client_graph.disconnectAndDeleteFile`, which frees that key string and overwrites the slot with `""`. The `Dep` is never notified, so it keeps pointing at freed memory.

The next time the directory watcher fires for that directory, `HotReloadEvent.processFileList` walks the dep chain and does

```zig
bun.path.dirname(dep.source_file_path, .auto)
```

on the freed slice, then (on resolution success) hashes it into `event.files` and passes it to `IncrementalGraph.invalidate`. Under ASAN this is a `use-after-poison` at `mem.lastIndexOfScalar` → `resolve_path.dirname` → `HotReloadEvent.processFileList:106`.

## Repro

Requires `separateSSRGraph: true` so a `"use client"` file's imports are resolved under the browser target (and therefore its resolution-failure `Dep` borrows the **client** graph's key):

1. `Comp.ts` compiles cleanly as a CCB — `is_client_component_boundary = true` in the server graph, client graph owns the key.
2. Edit `Comp.ts` to import `'./missing'` while still `"use client"` — `trackResolutionFailure(.client, Comp.ts, './missing')` inserts a `Dep` whose `source_file_path` is the client-graph key for `Comp.ts`.
3. Edit `Comp.ts` to drop both the directive and the failing import — server parse succeeds, `receiveChunk` sees `scb=false && was_ccb=true` → `client_graph.disconnectAndDeleteFile` frees the key. The `Dep` from (2) now dangles.
4. Create `components/missing.ts` — the directory watcher fires, `processFileList` reads `dep.source_file_path` → freed memory.

## Fix

- Add `DirectoryWatchStore.removeDependenciesForFile(alloc, file_path)` which walks every watch and drops any `Dep` whose `source_file_path.ptr == file_path.ptr` (pointer identity, since the slice is shared), freeing the watch if it becomes empty.
- Call it from `disconnectAndDeleteFile` just before `g.allocator().free(key)`.
- Relax the cleanup assertion in `freeEntry` to `dependencies.items.len == dependencies_free_list.items.len` (and clear both) so non-sequential dependency frees don't trip the debug assert.

## Verification

```
# without the fix (bun bd, ASAN):
==8747==ERROR: AddressSanitizer: use-after-poison on address 0x74d289f06db2
    #4 bake.DevServer.HotReloadEvent.processFileList (HotReloadEvent.zig:106)
(fail) DEV:bundle-8: removing 'use client' from a component with a pending resolution failure

# with the fix:
(pass) DEV:bundle-8: removing 'use client' from a component with a pending resolution failure [4232ms]
```

All 18 tests in `test/bake/dev/bundle.test.ts` pass.
