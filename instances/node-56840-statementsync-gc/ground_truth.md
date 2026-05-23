# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**nodejs/node PR #56840** — sqlite: fix use-after-free in StatementSync due to premature GC

**fix commit:** cebf4c8a9f23 · **base (merge^):** 316014d17273 · merged 2025-02-05

## Changed source files (test/fixture files filtered out)
- src/node_sqlite.cc
- src/node_sqlite.h

## PR body

This patch updates `StatementSync` to store a strong reference to the database base object.

`DatabaseSync` may be garbage collected and freed while `StatementSync` is using it (due to `MakeWeak()`). The following code crashes with a segmentation fault:

```js
import { DatabaseSync } from "node:sqlite";

const db = new DatabaseSync(':memory:');
db.exec('CREATE TABLE test (value INTEGER)');

const stmt = db.prepare('INSERT INTO test VALUES (?)');

for(;;) stmt.run(0);
```

```
* thread #1, queue = 'com.apple.main-thread', stop reason = EXC_BAD_ACCESS (code=1, address=0x38)
    frame #0: 0x000000010036e358 node`node::sqlite::StatementSync::Run(v8::FunctionCallbackInfo<v8::Value> const&) + 432
node`node::sqlite::StatementSync::Run:
->  0x10036e358 <+432>: ldr    x1, [x8, #0x38]
    0x10036e35c <+436>: ldr    x22, [x8, #0x78]
    0x10036e360 <+440>: ldrb   w8, [x19, #0x40]
    0x10036e364 <+444>: ldr    x24, [x25, #0xa8]
```
