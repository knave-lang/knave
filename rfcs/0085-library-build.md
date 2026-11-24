# RFC 0085: `knave build` builds a library project's library root

**Status:** Accepted
**Date:** 2026-11-24

## Problem

A library project has a library root, `src/lib.knv` unless `:lib` names another file, and no entry. `knave build` in such a project fails with "cannot read `src/main.knv`", because `build` always starts from the entry. The workflow that `knave new` writes runs `knave build`, so a library fails CI the day it is created. [RFC 0083](0083-library-root-in-check-and-lint.md) let `check` and `lint` start from the library root, and kept `build` and `run` failing in its rule 7.

The build cannot simply start from the library root either: emission starts from `main`, so a file without `main` fails with `L0601`, and rustc and Cargo build only binaries and test runners. rustc never sees the library's code, so a fault that only rustc finds stays hidden until a program uses the library.

When a project holds both files and the entry never uses the library root, `knave build` builds the entry alone, and the library root is never built.

## Options

### A library crate from the library root

`knave build` with no file, in a project whose entry the project file does not write and that has no entry file, builds the library root as a Rust library crate, as `cargo build` builds `lib.rs`. The library root is the crate root. Emission starts from every runtime definition of the project's own modules that the library root loads, writes no `main`, and marks each export `pub`. rustc builds with `--crate-type lib`, or Cargo with a `[lib]` target, into `target/knave/<name>/library/lib<crate>.rlib`.

```lisp
(defproject shelf)
```

```lisp
; src/lib.knv
(export distance-squared)

(defstruct point
  :x f64
  :y f64)

(defn distance-squared (a b)
  (+ (square (- (:x a) (:x b))) (square (- (:y a) (:y b)))))

(defn square (n)
  (* n n))
```

```rust
#[derive(Clone, Copy, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

pub fn distance_squared(a: Point, b: Point) -> f64 {
    square(a.x - b.x) + square(a.y - b.y)
}

fn square(n: f64) -> f64 {
    n * n
}
```

The Rust is a sketch from the emitted binaries of today: whether `square`, which no caller outside the library pins, emits over `f64` or as a generic function is what the prototype settles. Either is a valid `lib.rs`.

In a project with both files, `knave build` builds the entry's binary, then the library root as a library crate unless the entry's program loads it, so each item builds once and each error prints once. `knave run` in a library project prints that the library has no entry to run and exits 2.

### A check-only library crate

The same emission, but rustc runs with `--emit=metadata` and Cargo with `cargo check --lib`, leaving no rlib. It is faster, and it is `cargo check`, not `cargo build`: an error that only code generation finds passes.

### A generated entry

`build` writes a program whose `main` names each export of the library root and builds it as a binary. It needs no change to emission or to the build stage. A generic export cannot be named without a type, so it fails with `L0407` on source the user never wrote, a private function that no export reaches is never emitted, and the emitted Rust holds a `main` that the library does not have.

## Recommendation

Build a library crate from the library root. It does what `cargo build` does for `lib.rs`, so rustc checks every item that Knave emits for the library, and the emitted crate reads as a hand-written `lib.rs`. A check-only crate builds less than `cargo build` does, and a generated entry fails on generic exports and labels code the user did not write.

Rule 11 is needed because a library crate emits every definition, so without it a definition that `knave check` passes can fail in rustc with `E0121`.

The rules it would set:

1. `knave build` with no file skips a missing entry that the project file does not write when the library root exists, and builds the library root as a library crate.
2. A library crate's emission starts from every runtime definition of the project's own modules that the library root loads. A listed library's items emit as they are reached, as in a binary.
3. rustc builds the crate with `--crate-type lib`, and Cargo with a `[lib]` target and `cargo rustc --lib`, into `target/knave/<name>/library/`, with the same edition, profile and capped lints as a binary.
4. Both routes leave the library at `target/knave/<name>/library/lib<crate>.rlib`, where `<crate>` is the project's name with each `-` written `_`. The Cargo route copies the rlib there from Cargo's target directory, as it copies a binary.
5. `knave build -o <path>` in a library project writes the rlib to `<path>`.
6. When the entry exists, `knave build` builds the entry, then the library root as a library crate unless the entry's program loads it, as `knave check` does under RFC 0083.
7. A written `:entry` whose file is missing stays an error, even when the library root exists.
8. With no entry file and no library root, `knave build` with no file prints `nothing to build: the project has no entry and no library root` and exits 0 when the project has a task tree. With no task tree either, it is an error that names the missing entry, the missing library root and the missing tasks, and exits 2.
9. `knave run` with no file, in a project whose entry the project file does not write and that has no entry file, is a usage error that says the library, or the project when it has no library root, has no entry to run, exits 2, and builds nothing.
10. `knave run <file>`, `knave build <file>`, `test`, `types`, `expand`, `emit`, `doc` and `check <file>` keep their behavior.
11. A definition whose bound names a type variable that no parameter and no result holds is `L0407` at the definition, in a program and in a library alike, so `knave check` reports it before rustc does.

This RFC supersedes rule 7 of RFC 0083 for `knave build` and `knave run`.

## Decision

The project owner, 2026-11-24, D140: accepted the library crate from the library root, with rule 11. In a library project, `knave build` builds the library root as a Rust library crate, and `knave run` is a usage error. A definition whose bound names a type variable that no parameter and no result holds is `L0407` at the definition, in a program and in a library alike.

The project owner, 2026-11-24, D143: a project with at least one program, an entry, a library root or a task tree, builds what it has, and a project with none fails. A task tree alone has nothing to build, so `knave build` exits 0 there, and `knave run` is a usage error.

## Rejected options and why

- **A check-only library crate:** it runs `cargo check`, not `cargo build`, so an error that code generation finds passes, and it leaves nothing built.
- **A generated entry:** a generic export fails on generated source, a private function that no export reaches stays unbuilt, and the emitted Rust holds a `main` the library never wrote.

## Prototype

Branch `design-library-build-prototype`, commit `855212fb3`, on loan step 3c (`8efa04446`); it was not pushed.

- `src/cli/`: `build.rs`, `run.rs`, new `project_build.rs` and `library_crate.rs`, `located_project/entry_layout.rs`, `project_crates.rs`, `rustc_route.rs`, `build_routes.rs`, `emission.rs`.
- `src/emit/plan.rs`, `src/emit/entry_point.rs`, `src/emit/items.rs`, `src/emit/mod.rs`: `EntryPoint` holds an optional `main`, a library plan holds none, and the runtime start-up call goes only into a `main`.
- `src/ownership/reach_search.rs`: library roots reuse loan step 3c's root walk, filtered by `ModuleOrigin::is_project`.
- `src/types/typed.rs`, `src/types/typed_module_builder.rs`: each typed module carries its library, copied from `CoreModule::library`.
- `src/build/mod.rs`, `src/build/cargo_manifest.rs`, `src/build/cargo_compiler.rs`: `--crate-type lib`, a `[lib]` target and the rlib artifact.

An export whose parameter nothing pins emits as a generic function, `pub fn keep<A>(item: A) -> A`, and rustc accepts it. The loan checks report as they do with `main`. One case reaches rustc: an export whose bound names a type that no argument holds passes `knave check` and fails with `E0121`, because `L0407` reports such a variable only at a call. Rule 11 closes that case; the prototype does not build it. Every program with an entry emits the same Rust as before: the emit and run fixtures pass with no snapshot change.

## Rule

[docs/language.md: Projects](../docs/language.md#projects): the rules on what `build` and `run` take as their file, where `build` writes its output, and the paragraph of RFC 0083 that says `knave build` and `knave run` still need the entry.

[docs/language.md: Formatting](../docs/language.md#formatting): the emitted Rust of a library crate is formatted as a binary's is.

[docs/language.md: Type texts](../docs/language.md#type-texts): the rule that a variable the definition never fixes is `L0407` at its binding gains the variable that only a bound names, reported at the definition.

[docs/diagnostics.md: Codes](../docs/diagnostics.md#codes): the paragraph "Say so when no annotation can help" reports a bound-only variable at the definition, not only at a call.

[`L0407`](../expand/src/diagnostic/explanations/L0407.md): the explanation page gains the definition whose bound names a type that nothing fixes.
