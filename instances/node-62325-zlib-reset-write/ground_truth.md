# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**nodejs/node PR #62325** — zlib: fix use-after-free when reset() is called during write

**fix commit:** 53bcd114b100 · **base (merge^):** dbc74059503b · merged 2026-03-26

## Changed source files (test/fixture files filtered out)
- src/node_zlib.cc

## PR body

The `Reset()` method did not check the `write_in_progress_` flag before resetting the compression stream. This allowed `reset()` to free the compression library's internal state while a worker thread was still using it during an async write, causing a use-after-free.

Add a `write_in_progress_` guard to `Reset()` that throws an error if a write is in progress, matching the existing pattern used by `Close()` and `Write()`.

This does not fall within a threat model because it cannot be exploited from the outside.

Refs: https://hackerone.com/reports/3609132

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
