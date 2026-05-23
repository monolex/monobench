# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**oven-sh/bun PR #20093** — fix `NapiHandleScopeImpl` race condition

**fix commit:** 4534f6e635d2 · **base (merge^):** c62a7a77a3fe · merged 2025-05-31

## Changed source files (test/fixture files filtered out)
- src/bun.js/bindings/napi_handle_scope.cpp

## PR body

### What does this PR do?
fixes #16146,

`NapiHandleScopeImpl` functions can be accessed from multiple threads (e.g., during GC sweeping), which makes m_storage prone to race conditions.
<!-- **Please explain what your changes do**, example: -->

<!--

This adds a new flag --bail to bun test. When set, it will stop running tests after the first failure. This is useful for CI environments where you want to fail fast.

-->

- [ ] Documentation or TypeScript types (it's okay to leave the rest blank in this case)
- [x] Code changes

### How did you verify your code works?
ran test_handle_scope.c

<!-- **For code changes, please include automated tests**. Feel free to uncomment the line below -->

<!-- I wrote automated tests -->

<!-- If JavaScript/TypeScript modules or builtins changed:

- [ ] I included a test for the new code, or existing tests cover it
- [ ] I ran my tests locally and they pass (`bun-debug test test-file-name.test`)

-->

<!-- If Zig files changed:

- [ ] I checked the lifetime of memory allocated to verify it's (1) freed and (2) only freed when it should be
- [ ] I included a test for the new code, or an existing test covers it
- [ ] JSValue used outside of the stack is either wrapped in a JSC.Strong or is JSValueProtect'ed
- [ ] I wrote TypeScript/JavaScript tests and they pass locally (`bun-debug test test-file-name.test`)
-->

<!-- If new methods, getters, or setters were added to a publicly exposed class:

- [ ] I added TypeScript types for the new methods, getters, or setters
-->

<!-- If dependencies in tests changed:

- [ ] I made sure that specific versions of dependencies are used instead of ranged or tagged versions
-->

<!-- If a new builtin ESM/CJS module was added:

- [ ] I updated Aliases in `module_loader.zig` to include the new module
- [ ] I added a test that imports the module
- [ ] I added a test that require() the module
-->
