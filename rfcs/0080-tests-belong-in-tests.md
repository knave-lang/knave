# RFC 0080: Tests belong in `tests/`

**Status:** Accepted
**Date:** 2026-11-19

## Problem

RFC 0055 decision 2 lets a `deftest` sit at the top level of any module, beside the code it tests or in a module of its own. In a project, `knave test` with no path reads the `tests` directory at the project root, and each file there is an entry: its module and every module it reaches through `use` run their tests. A project's tests can therefore live in two places, and a test beside the code reaches private names. Nothing in the language points a user toward one place for tests or toward one way to test a module.

## Options

The emitted Rust of every program is the same under each option: a test emits Rust only under `knave test`, as today. The example is a project with one source module and one test file.

```lisp
; src/stock.knv
(export restock)

(defn add-units (count amount)
  (+ count amount))

(defn restock (count amount)
  "Adds `amount` units to `count`.

  (restock 2 3)
  => 5"
  (add-units count amount))

(deftest adds-units
  (assert (= (add-units 1 1) 2)))
```

```lisp
; tests/restocking.knv
(use stock restock)

(deftest restocks
  (assert (= (restock 4 6) 10)))
```

### Today's rule

`knave test` runs `restocks`, `adds-units` and the docstring example of `restock`, because `tests/restocking.knv` reaches `stock` through `use`. No warning names `adds-units`.

```text
running 3 tests
test stock/doc:restock ... ok
test stock/adds-units ... ok
test restocking/restocks ... ok
3 passed, 0 failed
```

### Tests only in `tests/`, with a lint

`knave test` with no path runs only the `deftest`s written in files under `tests/`, and the docstring examples of every module the run loads. `adds-units` expands and type checks, as every test does under every command, and does not run. A lint warns at `adds-units`, with a help that moves it to `tests/`.

```text
running 2 tests
test stock/doc:restock ... ok
test restocking/restocks ... ok
2 passed, 0 failed
```

### The lint alone

`knave test` keeps today's rule for every path and for no path, so it runs the three tests above. A lint warns at `adds-units`, with a help that moves it to `tests/`, and `knave test` prints the warning, so the run names the misplaced test.

## Recommendation

The lint alone. It leads code to one place for tests without a second rule for which tests a run calls.

## Decision

The project owner decided on 2026-11-19:

1. **`knave test` does not change, for any path or for no path.** Every rule of RFC 0055 and RFC 0061 about which tests run stays as it is.
2. **A `deftest` in a file of a project outside its `tests/` directory is a lint warning, on by default, with a help that moves the test to `tests/`.** It is a lint as RFC 0069 defines one: it has an `L08xx` code and a level in the project file's `lints` section, like every lint. Outside a project there is no `tests/` directory, so the lint does not apply. The build chooses the lint's code, its name and its wording.
3. **The lint prints under every command that prints lints, as `L0803` does, `knave test` included.** A test run then names each misplaced test.
4. **A module is tested through its exported functions.** A private helper is tested through the exported functions that use it. When a misplaced test names a private function, the help points to testing through the exported functions, because a test moved to `tests/` cannot see a private name.
5. **The change carries no migration text.** No note, deprecation or transition period serves code that keeps tests in source files. The lint is new and breaks no program, so the build needs no `release:minor`.

## Rejected options and why

- **Today's rule:** nothing leads a project's tests to one place, or a module's tests to its exported functions.
- **Tests only in `tests/`, with a lint:** changing `knave test` would make `knave test` and `knave test tests` run different tests, or need a special case for the `tests` path. The lint alone already leads code to the practice.

## Rule

[docs/language.md: Tests](../docs/language.md#tests) states the current rules of `knave test`, which this RFC leaves unchanged. The build of this RFC adds the lint to the table of [Lints](../docs/language.md#lints).
