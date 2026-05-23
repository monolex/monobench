# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**nodejs/node PR #59910** — diagnostics_channel: fix race condition with diagnostics_channel and GC

**fix commit:** 897932c4848a · **base (merge^):** c7b0dfbd7c56 · merged 2025-09-19

## Changed source files (test/fixture files filtered out)
- lib/diagnostics_channel.js

## PR body

When a garbage collector is executed, the callback of `FinalizationRegistry` it is not executed synchronously with the GC, it is executed later, in the next event loop.

That means that there is a corner case in the `WeakRefMap` object in diagnostics channel. Eventually could happen that an event is GC and created again before the execution of the callback of `FinalizationRegistry`. When this happens, the key object is deleted from the `WeakRefMap` even when it has a valid value.

This behavior can be reproduced with this code added in tests:

```javascript
const assert = require('assert');
const { channel } = require('diagnostics_channel');

function test () {
  const testChannel = channel('test-gc');

  setTimeout(() => {
    const testChannel2 = channel('test-gc');

    assert.ok(testChannel === testChannel2, 'Channel instances must be the same');
  });
}

test();

setTimeout(() => {
  global.gc();
  test();
}, 10);
```

This code fails in `main` branch, but it works as expected in the current branch.

<!--
Before submitting a pull request, please read:

- the CONTRIBUTING guide at https://github.com/nodejs/node/blob/HEAD/CONTRIBUTING.md
- the commit message formatting guidelines at
  https://github.com/nodejs/node/blob/HEAD/doc/contributing/pull-requests.md#commit-message-guidelines

For code changes:
1. Include tests for any bug fixes or new features.
2. Update documentation if relevant.
3. Ensure that `make -j4 test` (UNIX), or `vcbuild test` (Windows) passes.

If you believe this PR should be highlighted in the Node.js CHANGELOG
please add the `notable-change` label.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
-->
