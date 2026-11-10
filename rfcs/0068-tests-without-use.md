# RFC 0068: Tests without a use

**Status:** Accepted
**Date:** 2026-11-10

## Problem

RFC 0056 decision 1 keeps `deftest`, `assert` and `run-tests` in `prelude/test`, which the always-loaded prelude does not import, so a test module writes `(use prelude/test assert deftest)`. Every test file opens with that line, and a file that forgets it gets `L0201` on `deftest`. Since RFC 0066 the prelude exports `deftask`, one letter away, so that `L0201` suggests `deftask`, which is never what a test file means.

A task needs no `use`: `deftask` is a small macro of `prelude/syntax` whose expansion writes a `use` of `prelude/task`. That way cannot serve `assert` or `run-tests`, because `use` is a top-level form and both are written inside expressions: `assert` in a test's body or in any function, `run-tests` in a `main`. No compile-time function loads a module: `module-exports` and `module-symbol` refuse a module that the program has not loaded with `L0204`.

## Options

### A: import `prelude/test` into the prelude

The prelude writes `(use prelude/test assert deftest run-tests)` and exports the three names.

```lisp
(deftest doubles
  (assert (= (twice 2) 4)))
```

```rust
// the emitted Rust does not change
```

`empty_program` measured +42,091 bytes of peak memory (+7.76%), +49,896 allocated bytes (+4.44%) and +146,948 instructions (+2.73%), because every program loads the module's macros and their helpers.

### B: a `require-module` builtin and three small prelude macros

`(require-module path)` loads the public prelude module at `path`, unless the program has loaded it, and returns the symbol that `module-symbol` returns. `(symbol (require-module (quote prelude/test)) "assertion")` is then a name with that module's scopes, which refers to its private `assertion` macro. `prelude/syntax` defines `deftest`, `assert` and `run-tests` as macros that hand their call, as written, to such a macro:

```lisp
(def assert
  (macro (form)
    (call-in-module (quote prelude/test) "assertion" form)))
```

The macro of `prelude/test` reads the call as the user wrote it, so a failure prints `(assert ...)`, `deftest` in a body is still `L0208`, and every error points where it did. A program that writes none of them never loads `prelude/test`. `deftask` hands its call to `define-task` the same way, so the prelude has one way to reach a lazy module. `prelude/test` exports the prelude's three macros, so `(use prelude/test assert deftest)` keeps working and `doc` finds one page for each.

`require-module` skips the checks that `use` makes on a path: a library's own module paths, an unlisted library, a file outside a library's checkout. So it lives in `prelude/internal/code`, a compiler-provided module under `prelude/internal/`, which only the prelude may `use`, and it loads only a public prelude module. `code` does not hold it. A program or library cannot import it, but a macro that builds a name with the scopes of a prelude module that imports it still reaches it. That macro then loads only a public prelude module, which a `use` loads anyway, so this RFC accepts it. `pinned-at` from RFC 0066 moves there too.

In CI, `empty_program` measured +13,210 instructions (+0.25%), +2,713 bytes of peak memory (+0.50%) and +2,960 allocated bytes (+0.26%), for the three macros and their documentation.

### C: a small eager `prelude/assert` module and lazy `deftest`

`assert` and its checks move to a module that the prelude imports, and `deftest` expands to a `use` as `deftask` does. `run-tests` keeps its `use`, because it is written inside `main`.

## Recommendation

B. It serves all three names in every position they are written, costs `empty_program` at most 0.5% on every counter, and needs one builtin that only the prelude can call.

## Decision

The project owner chose option B on 2026-11-10:

1. **`deftest`, `assert` and `run-tests` are small macros of `prelude/syntax` that the prelude exports,** so a test module needs no `use`. Each hands its call, as written, to a macro of `prelude/test`, which a program loads only when it writes one of them. `prelude/test` exports the same three macros, so `(use prelude/test assert deftest)` keeps working.
2. **`require-module` is a compile-time builtin that only the prelude can import.** It lives with `pinned-at` in `prelude/internal/code`, a compiler-provided module under `prelude/internal/`, and loads only a public prelude module.
3. **`deftask` reaches `prelude/task` the same way,** so the prelude has one way to reach a module it loads lazily.

## Rejected options and why

- **A, import `prelude/test` into the prelude:** every program pays +7.76% of peak memory for macros that only test modules call, past the 1% that the memory gate allows.
- **C, an eager `prelude/assert`:** it still loads `assert` into every program, splits one module in two, and leaves `run-tests` behind a `use`.
- **A `use` inside the expansion, as `deftask` had:** `use` is a top-level form, so it cannot serve `assert` or `run-tests`.

## Rule

[docs/language.md: Tests](../docs/language.md#tests)
