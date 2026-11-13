# RFC 0074: `float` is the everyday name for `f64`

**Status:** Proposed
**Date:** 2026-11-13

## Problem

Most programs that write a decimal number want the default float type, not a
choice about width. They write `f64` today because that is the only spelling the
compiler knows. `float` reads naturally, pairs with the language's integer
names, and says what the program means unless width is the point.

The feature is one type under two names:

```lisp
(defstruct point
  :x float
  :y f64)

(defn distance-squared ((a point) (b point))
  (+ (* (- (:x a) (:x b)) (- (:x a) (:x b)))
     (* (- (:y a) (:y b)) (- (:y a) (:y b)))))
```

`float` and `f64` must mix with no conversion. The compiler must print one
canonical name, `float`, in `infer`, diagnostics and the prompt. `f64` stays
available for Rust interop, GPU and graphics code, and large arrays where width
matters. `f32` stays unchanged.

Three current rules complicate the change:

1. The prelude already binds a trait named `float` that marks `f32` and `f64`
   ([`prelude/number.knv`](../prelude/number.knv)). A type and a trait share the
   type namespace, so the type cannot exist until the trait has another name.
2. `PrimitiveType::name` is both the language name and the Rust name. `F64`
   cannot be named `float` in the language while emitting `f64`.
3. `CoreModule::exports` stores only `BindingId` values. If `float` and `f64`
   share one binding, the core IR cannot tell which spelling a module exported,
   so `expand` would print one name twice or the wrong name.

## Options

### A: Keep `f64` canonical and accept `float` as a hidden alias

The compiler accepts `float` but keeps printing `f64` everywhere.

- It adds a spelling without changing the language's vocabulary.
- It fails the feature's main user-visible goal: a program that writes `float`
  still reads `f64` in every type error and every `infer` result.

### B: Give `float` and `f64` separate bindings

Both names bind `KernelType::Primitive(F64)`, but each has its own `BindingId`.

- `same-binding?` would say the two names are different, so macro code would
  distinguish one type's two names.
- Documentation would need two pages, and the pages would drift.
- The alias would be true at the type level but false wherever the language
  compares names.

### C: One binding, canonical `float`, alias `f64`, trait renamed `floating`

- `float` and `f64` are one binding of `PrimitiveType::F64`, so they mix with
  no conversion and `same-binding?` is true.
- `PrimitiveType::name()` is the canonical language name, `float`.
- `PrimitiveType::rust_name()` is the emitted Rust name, `f64`.
- `PrimitiveType::ALIASES` lists the other written names.
- The prelude trait `float` becomes `floating`. Generic code over both float
  widths keeps its bound.
- Named exports preserve the written spelling, so `(export float f64)` survives
  `expand`.
- One canonical documentation page serves both names: `(doc f64)` shows the
  `float` page.
- A trait implementor list names the concrete primitive `f64`, because it
  enumerates the types a trait accepts rather than printing an inferred type.

This is the recommended option.

### D: Remove the trait instead of renaming it

Generic code over `float` and `f32` would have no single bound. It would write
`(n number)` and admit integers, or duplicate the code per width. The language
already uses the trait to mean "a floating-point type", and the name
`floating` says exactly that.

### E: Make `f64` a Rust-interop spelling only

Programs that write `f64` in a type position would stop compiling. That
contradicts the rule that `f64` stays available where width matters, and it
would break the Rust templates and documentation that already name it.

## Recommendation

Take option C. It is the only option that makes the alias true at every layer:
binding, type, documentation, macro name comparison, and export text.

The implementation separates three pieces of knowledge that are one today:

- the canonical language name, `float`
- the emitted Rust name, `f64`
- the written aliases, `f64`

`PrimitiveType::ALIASES` is the single table that will carry later aliases,
such as `int` for `i64`, so this does not need a second design.

The prelude cannot define this alias itself: `def` binds a value and `deftype`
names a Rust type, not another type. The kernel table is therefore the smallest
place that makes `float` a pure alias of `f64` rather than a new type.

## Migration

`f64` stays a legal spelling. A program that used `float` as a trait bound
moves it to `floating`, and the `L0212` help names that fix. The rename is a
breaking change and takes the `release:minor` label.

## Decision

Pending the project owner's decision.

## Rejected options and why

- **A:** It leaves every compiler message teaching `f64`, which is the opposite
  of the feature.
- **B:** Separate bindings make one type's two names distinguishable to
  `same-binding?` and force duplicate documentation, so the name is not a true
  alias.
- **D:** It removes a working capability and makes generic float code less
  precise.
- **E:** It breaks existing `f64` code and the Rust-interop use the feature
  explicitly keeps.

## Prototype

The pull request carrying this RFC contains the complete implementation:

- `expand/src/core/primitive.rs` holds the primitive table, canonical names,
  Rust names and the alias table.
- `create_builtin` creates one binding per primitive and exports every spelling
  to it.
- `CoreExport` carries an export name and a binding, and `CorePrinter` prints
  the names that the source exported.
- Emitted Rust uses `rust_name()`, so every existing program emits the same
  Rust as before.
- The prelude trait is `floating`, and the canonical `float` page explains the
  alias. The doc lookup falls back from an alias to the note of the term with
  the same binding, so `f64` resolves to the same page without a second one.
- The prelude uses `float` where it used `f64` before. The concrete primitive
  stays `f64` in the alias table, and trait implementor lists name it `f64`.
- Regression tests pin the shared binding, shared page, named exports, canonical
  `infer` text, diagnostics, implementor names, the `floating` migration help,
  prompt output and emit text.

The prototype does not change the Rust runtime, the emitted Rust, or the
generator's findings.

## Rule

[docs/language.md: Kernel forms](../docs/language.md#kernel-forms) and
[docs/language.md: Literals](../docs/language.md#literals)
