# RFC 0074: `int` and `float`, the default number types

**Status:** Accepted
**Date:** 2026-11-15

The project owner's spec binds this design: Rules 1 to 3, the places to update and the two open questions. The owner answered decisions 0 and 4 early: `int` is a built-in shown name, and `float` joins this RFC by the same column, with the prelude trait `float` renamed `floating`. The owner's exploration of `float`, PR 597, is one option among those weighed here, and "Differences from PR 597" gives each point where this design departs from it. Code references are against `origin/main` at `5234c8817`.

## Problem

Most code just wants a number. Knave's default integer type is `i64` and its default float type is `f64`, and the user sees those names everywhere: `(infer 1)` is `"i64"`, `(infer 1.5)` is `"f64"`, every diagnostic prints them, and the prelude's docstrings, `docs/language.md`, the explanation pages (`knave doc <code>`, `knave doc --write --errors`) and the doc pages all teach them.

The owner's spec sets three rules, which this RFC applies to both pairs:

- **Rule 1.** `int` and `i64` are one type, and `float` and `f64` are one type.
- **Rule 2.** The compiler prints `int` and `float` everywhere.
- **Rule 3.** The docs keep `i64`, `f64` and the other widths only where the width is the point.

The emitted Rust stays `i64` and `f64`. Literals keep inferring their width from context. `f32` and the other widths keep their names. A `byte` name for `u8` is left out, as a possible later row of the same column.

Knave cannot say this today:

- **No form binds a second name to a type.** `(deftype meters f64)` without a `rust` clause is `L0345` (`prelude/definers.knv:424-432`). `(rust-type int "i64")` binds a `BindingKind::RustType`, which lowers to `TypeExprKind::Named(binding, [])` (`expand/src/expand/type_expr.rs:59-67`). That is a new nominal type. It does not unify with `Type::Primitive(I64)`, so it breaks Rule 1.
- **A macro cannot help in a type position.** `lower_type` resolves a symbol straight to its binding in the type namespace and never expands a macro (`type_expr.rs:17-32`, `95-108`).
- **`use` and `export` cannot rename.** The forms export a binding under its written name only (`docs/language.md:689`). The registry itself can hold one binding under several names, because `ModuleRecord::export` keys its map by name and namespace (`expand/src/modules/registry.rs:37-43`), so `module-exports` (`expand/src/expand/host.rs:93-100`) returns every name with no change.
- **The name `float` is taken.** The prelude trait `float` (`prelude/number.knv:21`, `(rust-trait float "crate::rt::Float" f32 f64)`) lives in the type namespace, where the type would go (`type_expr.rs:64`).
- **One kernel string is both the shown name and the Rust name.** `PrimitiveType::name()` (`expand/src/core/kernel.rs:373-375`, rows at `271-366`) is the name the kernel exports, the name every printer shows, and the name the emitter writes. The table below lists every caller. Changing that one string would rename the type in the emitted Rust too.

| Caller of `PrimitiveType::name()` | What it feeds | Audience | After this RFC |
|---|---|---|---|
| `src/types/display.rs:51` (`TypeDisplay`) | every type that the type stage, ownership, emit diagnostics, `infer`, `knave doc`, `knave types` and the REPL print (see "Printers") | user | `int`, `float` |
| `src/types/finalize.rs:247,255` | `out_of_range` wording | user | `int` (`:255`), `float` (`:247`) |
| `src/types/pinning.rs:531` | `literal_default_note` (`src/types/problems.rs:274`): `` `limit` defaulted to `i64` `` | user | `int`, `float` |
| `src/types/pinning.rs:657` | the fallback of `LiteralSignature::shown_type`, the signature that `literal_default_help` suggests (`src/types/problems.rs:307`) | user | `int`, `float` |
| `expand/src/core/printer.rs:372` (`CorePrinter`) | `knave expand` | user | `int`, `float` (decision 7) |
| `expand/src/core/kernel.rs:456` (`KernelType::name`) | kernel export names (`expand/src/expand/module_loading.rs:445-456`), and the test helper `names_the_prelude_documents_by_type` (`expand/src/expand/prelude_state.rs:391-392`) that the test `each_kernel_type_and_constant_has_a_page_headed_by_its_shape` (`:507`) uses | binding | one binding per primitive, named by its shown name and exported under both names |
| `src/emit/types.rs:805-806` | Rust type | Rust | `i64`, `f64` |
| `src/emit/number_literal.rs:20` | literal suffix `3_i64` | Rust | `i64`, `f64` |
| `src/emit/structural_impls.rs:85` | generated parameter names such as `i64_2` and `string_1` | Rust identifiers | the kernel spelling, `i64`, `f64`, `string` (`names().spelling()`). The Rust name would give `String_1` and the invalid `()_1` |
| `expand/src/core/template_operation.rs:325` | strips a Rust integer suffix such as `_i64` from template text | Rust | `i64` |

`name()` goes away, and `PrimitiveType::names()` returns a `PrimitiveNames` value with `shown()`, `rust()` and `spelling()`, the kernel spelling. Every caller above must then pick one, so the compiler finds any caller that this table missed, user-facing or Rust-facing.

## Options

### C. The kernel row carries a shown name (recommended, and the owner's choice for decision 0)

Each `PrimitiveRow` holds a `PrimitiveNames` (`expand/src/core/primitive_names.rs`), a value with the kernel spelling, the shown name and the Rust name. Most rows read `PrimitiveNames::same("bool")`. The `string` and `unit` rows add `.emitted_as("String")` and `.emitted_as("()")`, which replaces the `match` in `RustTypes::primitive`. PR 2 adds a constructor for a differing shown name, so the `I64` row reads, for example, `PrimitiveNames::same("i64").shown_as("int")`. The names live in their own value because `PrimitiveType` would otherwise pass the design rules' limit of 7 public methods. The generated parameter names of structural impls keep the spelling, so `string` gives `string_1`, not `String_1`.

**One binding, two exported names.** `builtin_exports` (`module_loading.rs:445-456`) creates one binding per primitive, `BindingKind::KernelType(KernelType::Primitive(I64))`, named by its shown name `int`, and exports it under the shown name and, where it differs, under the kernel spelling `i64`. Both names lower to `TypeExprKind::Primitive(I64)` (`type_expr.rs:41-43`), so the checker, the ownership pass and the emitter see one type. Rule 1 holds with no conversion.

One binding, not two bindings of the same kind, because Rule 1 must hold for macros too. `same-binding?` is how a macro tells which name a form is: the prelude calls it 20 times, and `prelude/documentation.knv:414` matches a page to its term with it. With two bindings, `(same-binding? t (quote int))` is false for `i64`, so a macro would see one type as two, and the docs would need two pages that say the same thing. With one binding, `knave doc i64` prints the `int` page, and a macro sees one type.

One binding has one cost, through decision 7(a). `CoreModule::exports` stores only `BindingId`s, and `CorePrinter` prints each by its binding's name (`expand/src/core/printer.rs:33-39`), so `knave expand` would print the prelude's export list with `int` twice and no `i64`. So a module's exports become a `ModuleExports` that `CoreModule` owns. It keeps each exported binding and, when the export was written under a name other than the binding's own, that name, and `CoreModule::push_export` is the only way to add one, so the names cannot drift from the bindings. `ModuleExports` stays in `module.rs`, the file of the type it serves: a file of its own would join the core module cycle, which `cargo xtask design-rules` refuses. That grows the saved prelude state and every module's export list a little, which the allocation gate measures (question 5). PR 1 makes this change with no output change, so its cost is measured alone.

**Printing.** Every user-facing printer calls `names().shown()`. The emitter calls `names().rust()`, and generated Rust identifiers call `names().spelling()`. The prelude re-exports `int` and `float` like every other kernel type, and documents each on one page under its shown name (see "What changes").

```lisp
(type add (fn (int int) int))
(defn add (x y) (+ x y))
(defstruct point :x float :y float)
(println (infer 1))
(println (infer 1.5))
(the i64 (add 1 2))   ; i64 is still a valid spelling of the same type
(the f64 1.5)         ; and f64 of float
```

```rust
fn add(x: i64, y: i64) -> i64 { x + y }
struct Point { x: f64, y: f64 }
// in main:
println!("{}", "int");
println!("{}", "float");
add(1, 2)
```

The kernel already names its own types. `ROWS` states `i64` and `f64` today, and the prelude only re-exports and documents them. Under C the prelude states nothing new about the identity of `int` or `float`. The kernel states each once, in the type's own row, so the rule against restating a prelude fact in Rust does not apply. The printing rule is uniform: a kernel type prints by its shown name.

C does create one exception to a rule. Names rule 5 ([RFC 0046](0046-names.md), `docs/language.md:1114-1129`) says "One concept has one name", and `int` and `i64`, like `float` and `f64`, are two prelude exports of one type. The owner's spec requires both names. `docs/language.md` adds the exception to the section's list of names that break a rule: "`int` and `i64` name one type, and so do `float` and `f64`, an exception to rule 5. `int` and `float` are the names that print, and `i64` and `f64` are the spellings where the width is the point."

**The trait `float` becomes `floating`.** The trait keeps its Rust trait, `crate::rt::Float`, and its implementors, `f32` and `f64`. Only its Knave name changes, in `prelude/number.knv:10,21,220,227` and `prelude/prelude.knv:38,76`. `floating` says what the trait means, a floating-point type, and it does not collide with `float`. Generic code over both widths keeps its bound: `(where ((n floating)) (fn (n) n))`.

**A primitive written as a bound gets help.** `(where ((n float)) ...)` now resolves `float` to a type, which reports `L0212`, `` expected a trait, found `float` `` (`wrong_namespace`, `expand/src/expand/problems.rs:744-750`), with no help today. The RFC adds help to `L0212` when the name is a numeric kernel type, and it reads the trait names from the prelude's own rows, because `AGENTS.md` forbids restating a prelude trait's name in Rust. The expand stage holds those rows: the installed prelude state carries each prelude module's `CoreModule` in `ExpansionTables::expanded` (`expand/src/expand/mod.rs:270`, installed at `prelude_state.rs:161`), and each `ItemKind::RustTrait` keeps its rows as `ImplementorRows` on its `RustTraitDeclaration` (`expand/src/core/module.rs:90-100`). The help considers only the traits that the `prelude` module exports, read from the module registry (`ModuleRecord::exports`, `expand/src/modules/registry.rs:45-49`), so it never names a trait that a user cannot write, such as `step`, which `prelude/iterator` keeps private (`iterator.knv:22-27`), and never a user's own trait. Of those, it names each `rust-trait` whose rows are only numeric primitives and include every primitive of the written type's family, as the primitive's `is_integer` and `is_float` say. Traits with fewer rows come first, and traits with equally many rows sort by name. On today's prelude that gives `floating`, `signed` and `number` for `float`, `f32` and `f64`, and `integer` and `number` for `int` and the other integer types (`prelude/number.knv:14-24`). When no trait qualifies, as in a program that does not load the prelude, `L0212` carries no help. The final wording, with the names filled in from that search:

```
error[L0212]: expected a trait, found `float`
help: write a trait that every floating-point type implements, such as `floating`, `signed` or `number`
```

```
error[L0212]: expected a trait, found `int`
help: write a trait that every integer type implements, such as `integer` or `number`
```

The names join as one name, `a` or `b`, or `a`, `b` or `c`. `ImplementorRows::all_rows_primitive` skips the other traits without decoding their rows, and the search runs only when `L0212` is reported. The type stage's `implemented_exactly_by` (`src/types/pinning.rs:634-642`) cannot serve, because its trait table is built after expansion. The help is permanent, not a migration aid. `(where ((n int)) ...)`, meant as "any integer", is the same mistake, and the same help fixes it. Its wording lives in a problems type: PR 3 moves `wrong_namespace` and the new help out of `ExpandProblems`, which sits at its `methods_per_type` baseline of 99, into a type of their own for namespace errors.

Compiler surface: one column in a 19-row table, the `name()` split, one binding per primitive exported under two names, the written name on an export entry, the `L0212` help, and the page test `each_kernel_type_and_constant_has_a_page_headed_by_its_shape` (`prelude_state.rs:507`) changed so that `i64` and `f64` find the page headed by `int` and `float`. `KernelType::shape` (`expand/src/core/kernel.rs:438-446`) returns `KernelType::name` for a primitive, not `PrimitiveType::name`, so the `name()` split does not flag it: PR 1 moves it to the shown name. `KernelType::name` has one more caller, `ExpandProblems::malformed_type` (`expand/src/expand/problems.rs:875`), which never receives a primitive, because `lower_type_binding` returns `TypeExprKind::Primitive` before it reaches that arm (`type_expr.rs:41-47`). No new kernel form, binding kind, build-time table or user feature. The printers need no new path to the shown name, because each one already holds a `PrimitiveType`.

### A. A kernel `alias` form, with `int` declared in the prelude

A new kernel form `(alias name type)` binds `name` in the type namespace to the lowered `type`, as `BindingKind::Alias(TypeExprKind)`. `lower_type_binding` returns the target's kind, so `int` lowers to `Primitive(I64)` before the checker runs. The prelude declares `int` through a definer (see decision 1 for which one):

```lisp
(defalias int
  "Holds an integer, Knave's default. ..."
  i64)
```

```rust
// an alias emits no item; `int` emits `i64`
fn add(x: i64, y: i64) -> i64 { x + y }
```

Printing needs a statement that `int`, not `i64`, is the shown name. `PreludeState::build` in the expand crate, which `build.rs` reaches through `SavedPrelude::build` (`expand/src/expand/saved_prelude.rs`), would collect each alias that the `prelude` module exports whose target is a primitive, into a 19-entry shown-name table. Every printer already takes the `BindingArena`, so the table belongs there, and none of the 11 `TypeDisplay::new` sites, `CorePrinter` or `finalize.rs` needs a new parameter. It cannot sit beside `saved` as it is, because `saved` is `#[serde(skip)]` (`expand/src/core/binding.rs:127-128`). The table must survive `save()` into the embedded state, so it needs either a serialized field of its own or a place in the tables that `SavedBindings` decodes.

Compiler surface beyond C:

- a new kernel form, with its reason recorded in `docs/language.md`
- a new `BindingKind`
- the table and its build step
- a build error for two prelude aliases of one primitive
- a definer, either a new one or a rework of `deftype` and `L0345`
- a public, transparent type alias feature, because every kernel form is reachable through `(use kernel ...)` (`expand/src/modules/sources.rs:35`, `:318`)

It also needs a printing rule for user aliases. If a user alias prints by its target's name, then "a prelude alias of a kernel type is that type's shown name" is a prelude-only exception. If user aliases print by their own names, Rule 2's mixing comes back for user types. A's merit: the `int` page and the statement of its shown name sit together in the prelude, and if the owner wants user aliases anyway, the form is needed regardless.

### B. Rename on import

```lisp
;; prelude/prelude.knv
(use kernel (int i64))
(export int)
```

```rust
fn add(x: i64, y: i64) -> i64 { x + y }
```

`int` would be the `i64` binding under a second name. `use` gains a second shape that every module could use to rename any name. The printer still needs a statement of which name wins, and B cannot name a compound type later. Rejected.

### D. A prelude rewrite of `int` in its own definers

```lisp
;; inside `defn`, `defstruct`, `type` and the other prelude definers:
;; rewrite each `int` in a type position to `i64` before expansion
(defstruct reading :value int)   ; the macro hands `:value i64` to `struct`
```

```rust
struct Reading { value: i64 }
```

No compiler change for spelling. But a user macro that emits kernel forms directly, `(struct ...)` or `(type ...)`, never gets the rewrite, so `int` would work in some type positions and not others. And printing `int` still needs the compiler. Rejected.

## Differences from PR 597

PR 597 is the owner's exploration of `float`, read here as one option. Where this RFC does what 597 did, the reason stands on this RFC's evidence. Where it differs:

| Point | PR 597 | This RFC | Why |
|---|---|---|---|
| Where the names live | `name()` returns the shown name `float`. A separate `ALIASES` table lists `("f64", F64)`. `rust_name()` matches `F64 => "f64"` by hand. | One row holds a `PrimitiveNames` with the kernel spelling, the shown name and the Rust name. `name()` is removed. | The row states `f64` once, where 597 states it in `ALIASES` and again in `rust_name()`. Removing `name()` makes the compiler flag every caller. Under 597's shape, setting `I64`'s name to `int` would leave `template_operation.rs:325` stripping `_int` from template text in place of `_i64`, and nothing would flag it. |
| Bindings | One binding per primitive, exported under both names. | The same. | `same-binding?` and the doc pages (Option C). |
| Export names | `CoreExport` holds a binding and an index into a new `CoreModule::aliases` list. | An export entry records the written name when it differs from the binding's name. The representation is the PR author's, and the allocation gate judges it. | Needed by one binding. 597's index is one compact way to do it. |
| `knave doc f64` | A name fallback, `alias-docstring-notes`, beside `docstring-notes` (`prelude/macros.knv:1245`). | `knave doc i64` and `knave doc f64` print the `int` and `float` pages, pinned by a test. | The page is keyed by its term's binding, and `same-doc-key?` (`macros.knv:1263-1266`) already compares bindings. PR 2 first checks whether that finds the page with no fallback, and adds one only if it does not. |
| Implementor lists in the prelude source | Respelled `f64` to `float` in each `rust-trait` list. | Keep `i64` and `f64`. | The list enumerates widths, so the width is the point (Rule 3). The source spelling does not change what prints. |
| Implementor lists in diagnostics | Print `f64`, through `F64` checks hard-coded in `TypeDisplay::show_implementor` and `scheme_implementor`. | Shown names, `int` and `float`, like every other printed type (decision 11(a)). | Rule 2 holds with no new exception, and a user whose value prints as `int` sees `int` in the list. No primitive is special-cased in `TypeDisplay`. |
| `L0212` help | Only when a float type is written as a bound, with `floating` and `number` written in the Rust wording. | When any numeric kernel type is written as a bound, with the trait names read from the prelude's `rust-trait` rows. | `AGENTS.md` forbids restating a prelude trait's name in Rust, and 597's wording would name a trait that no longer exists after a prelude rename. `(n int)` for "any integer" is the same mistake as `(n float)`, and the row already says which family a primitive is in. |
| Scope | `float` alone. | `int` and `float` together, in one PR sequence and one release. | The owner folded `float` into this RFC. |

These tests from 597 carry over, for both names: `types` prints a written `f64` as `float` (`types_prints_a_written_f64_as_float`), `doc f64` prints the `float` page (`doc_of_an_alias_prints_the_canonical_page`), `same-binding?` is true for both spellings (`tests/fixtures/expand/same_binding_types.knv`), the export list keeps both spellings (`float_alias_exports.knv`), a struct and a function mix both spellings (`emit` and `run` fixtures `float_alias.knv`), the REPL prints `(infer 1.5)` as `"float"`, and the `7.0_f64` suffix (`number_literal.rs`).

## Questions

### 1. Where `int` and `float` live

In the kernel. No existing form can state them in the prelude (see Problem), and C needs no new form, so "Small compiler, big prelude" favors C. A user program cannot declare an alias. The owner chose this in decision 0.

### 2. Which name is shown

Rule 2 settles the behavior: the compiler always prints `int` and `float`, never the written `i64` or `f64`. `(the i64 "x")` reports `` expected `int` ``, and `(the f64 "x")` reports `` expected `float` ``.

Rule 2 covers types that the compiler prints. A name the user wrote prints as written, and the RFC states that as a rule in `docs/diagnostics.md` beside "Use the names the user wrote" (`:45`) and in `docs/language.md` beside Rule 2. It covers:

- a diagnostic's source window
- a message or note that echoes a written name. `(println (i64 "3"))` reports `L0201` with the note `` `i64` is a type; a type goes beside a name in a binding, as in `(local (year int) 2025)` ``, after the respelling of `expand/src/expand/problems.rs:683`. `wrong_namespace` (`problems.rs:744-750`) prints `` expected a trait, found `f64` `` from the written name.
- a doc page's header, under decision 10(a) or (c)

Each of these quotes the user's own text, never a type the compiler inferred.

A diagnostic that lists implementors, such as `partial-eq`'s, the `(parse-error ...)` rows, and missing-trait help, prints them by shown names, `i8 i16 i32 int i128 ... f32 float` (decision 11(a)). The prelude source of those lists keeps `i64` and `f64`, because there the width is the point (Rule 3).

**Printers.** Every printer except the rustc relay (below) reaches the shown name through one of the callers in the Problem table. The ones that reach it through `TypeDisplay`, listed here so that the PR author reads their snapshots:

- the type stage diagnostics (`src/types/checker.rs:720,762`, `unification.rs:165`, `pinning.rs:471`, `main_result.rs:85`)
- `infer` (`src/types/type_text_writer.rs`)
- `knave doc` signature lines (`src/types/typed.rs:633-667`, from `src/cli/doc.rs:242` and `src/cli/doc_pages.rs:212`)
- `knave doc --types` and `knave doc --write` headers. These print the type line through `TypeDisplay`, and right beside it the definition's header as written, read through its `header-shape` (`docs/language.md:489-505`). `--write` puts both lines in one fenced block (`:491`). So `(defn area (:width i64 :height i64) "Area." (* width height))` prints `(type area (fn (int int) int))` above `(defn area (:width i64 :height i64))`. Decision 10 chooses what the header prints.
- `knave types` (`entry_signatures`, `src/types/typed.rs:603`)
- REPL and `eval --json` results (`src/types/form_results.rs:84`)
- ownership diagnostics (`src/ownership/returned.rs:179`, `src/ownership/values.rs:426,455`)
- emit diagnostics (`src/emit/untied_borrows.rs:149`)

One printer does not reach the shown name: the rustc relay. `RustcDiagnosticMapper` (`src/build/rustc_errors.rs`) relays rustc's message and children as Knave diagnostics, labeled on Knave source. Its `user_code` (`:144-159`) rewrites each backticked word through `emitted.names`, which maps program names only. So `(def add-mixed (rust "{0} + {1}" (fn (i32 i64) i64)))` called as `(add-mixed 1 2)` reports `` cannot add `i64` to `i32` `` and `` expected `i32`, found `i64` `` in a report whose Knave diagnostics print `int`. Decision 9 chooses what it prints.

`knave fmt` and `knave lint` read source forms (`expand/src/format/`, `src/lint/form_reader.rs`) and print no inferred type. The task runner reads a signature's `infer` text but compares only `result`, `where` and binder names (`prelude/task.knv:358-396`), so `"int"` and `"float"` change nothing there. The repository has no language server.

### 3. Number literals

An unsuffixed integer literal still defaults to `PrimitiveType::I64`, and a float literal to `F64` (`src/types/finalize.rs:209-213`). Each still takes its width from context, so `8080` passed where a `u16` is expected stays `u16`, and `1.5` passed where an `f32` is expected stays `f32`. Each place that prints a literal's type:

| Place | Path | Today | After |
|---|---|---|---|
| `infer` text | `TypeTextWriter::write` → `TypeDisplay::scheme` | `"i64"`, `"f64"` | `"int"`, `"float"` |
| Mismatch, missing-trait and `L0407` diagnostics, and the `the` fix in `L0407`'s help | `TypeDisplay::show` | `` `i64` `` | `` `int` `` |
| The literal-default note | `pinning.rs:531` → `problems.rs:274` | `` `limit` defaulted to `i64` `` | `` `int` `` |
| The literal-default help's signature | `pinning.rs:657` → `problems.rs:307` | `(type count-up (fn (usize) i64))` | `(fn (usize) int)` |
| Literal out of range | `finalize.rs:255` | `` `i64` `` | `` `int` `` |
| Float literal too large for any type | `finalize.rs:247` | `` `f64` `` | `` `float` `` |
| REPL and `eval --json` | `form_results.rs:84` | `"(vec i64)"` | `"(vec int)"` |
| `knave expand` | `CorePrinter` | `i64`, `f64` | `int`, `float` |
| Emitted suffix | `number_literal.rs:20` | `3_i64`, `7.0_f64` | the same |

`infer` is the one place where a printed type reaches the emitted Rust. `(println (infer 1))` emits the string literal `"i64"` today and `"int"` after, as the spec requires, and `(infer 1.5)` emits `"float"`. No corpus entry uses `infer`. Two emit snapshots carry such a string (`stages__emit_fixtures@infer.knv`, `@infer_parameter_mode.knv`), and run fixtures that print `infer` change their standard output.

### 4. What changes

Counts on `5234c8817`. A mechanical change is a respelling. A judgment change needs a Rule 3 decision.

| Area | Size | Mechanical | Judgment |
|---|---|---|---|
| Prelude `.knv`, without `prelude/tests/` | `i64`: 15 files, 101 lines. `f64`: 8 files, 36 lines. Trait `float`: `number.knv:10,21,220,227`, `prelude.knv:38,76` | Signatures and examples: `let-try`'s `(option i64)`, `try->`'s `(n i64)`, `result.knv`'s 16 examples, `parse` examples, the examples in `vec`, `types`, `option`, `map` and `assoc`, and `(circle f64)` in `definers.knv:28`. The comparator in `sort-with!`'s signature, `(fn ((ref a) (ref a)) i64)` (`vec.knv:140`), is a signature where the width is not the point. The trait `float` becomes `floating` in its definition, page, export and `use`. Doc examples run as tests, so each `infer` example whose text changes (`prelude.knv:599` and others) fails until it is updated. | Every `rust-trait` implementor list keeps `i64` and `f64`: `iterator.knv:26-27` (`step`), `map.knv:19-23` (`borrow`), `number.knv:14-58`, `:66-71` (`partial-ord`), `:193-198` (`ord`), `ownership.knv:10-12` (`clone`), `:21-24` (`copy`), `text.knv:40-55` (`display`), `:68-77` (`debug`), `:96-115` (`show`) and `:186-200` (`from-str`). The export list (`prelude.knv:75-77`) keeps both spellings beside the new names. The `i64` page (`prelude.knv:591-600`) becomes the `int` page: it says that `i64` is another name for the type, to write where the width is the point, and it keeps the "literals default" text. The `f64` page becomes the `float` page the same way. The `floating` page names `[f32]` and `[float]`. `the`'s docstring follows decision 5. |
| `prelude/tests/` | `i64`: 4 lines. `f64`: 0 | `option.knv` | none |
| The prelude's written Markdown page (`knave doc --write`) | 2 sections | none | The page lists `module-exports` (`prelude/documentation.knv:31-33`, `:170`), which returns both `int` and `i64`, and each finds the `int` docstring through `same-binding?`. Without a change, the page has two sections that carry one docstring. The page instead gives one section to each binding, headed by the term that the `document` form names (`int`, `float`), and lists the binding's other exported names in that section as another name of it (`i64`, `f64`). The grouping is prelude code in `documentation.knv`, by `same-binding?`. |
| Explanation pages, `expand/src/diagnostic/explanations/*.md` (`knave doc <code>`, `knave doc --write --errors`) | 77 pages. `i64`: 277 lines. `f64`: 27 pages, 110 lines. Trait `float`: `L0441.md:7` | Examples where the type is incidental. Quoted compiler output and help text, which must match Rule 2's output. `L0441.md:7`'s `(n float)` becomes `(n floating)`. | Pages about ranges, literal fit, `the`, widths and Rust interop keep their widths. `L0345.md` and `L0340.md:8` do not change. `L0212.md` gains the help for a numeric type written as a bound. |
| `docs/language.md` | `i64`: 61 lines. `f64`: 17. Trait `float`: `:171`, `:191` | Knave examples. Rust snippets keep `i64` and `f64`. `(n float)` bounds become `(n floating)`. | Names: add both pairs to the list of names that break rule 5 (see option C). Types table: `int` and `float` rows before `i64` and `f64`. Kernel forms (`:85`): the primitive list gains `int` and `float` and says that `i64` and `f64` are other names of them. Literals (`:176`, `:182`): "defaults to `int` or `float`, Rust's `i64` or `f64`", and "a float literal can become `f32` or `float`". Type texts: `(infer 1)` is `"int"`, and decision 8's text. Width-specific passages keep their widths. |
| `docs/diagnostics.md` | 0 lines | none | Beside "Use the names the user wrote" (`:45`): a primitive type prints by its shown name, a name the user wrote prints as written, and implementor lists print shown names (decision 11(a)). The prose words "an integer" and "a float" for an open literal (`src/types/unification.rs:65-66`, `expand/src/interpret/value.rs:40`) and "the float types" for `f32` and `f64` in prelude prose (`number.knv:205,214,265,289`, `text.knv:907`) stay, because they describe a literal or a family, not a type name, and Rule 2 covers backticked type names. |
| README | 0 lines | none | Introduce `int` and `float` first, then the widths (spec). The guide and knave-lang.org are outside this repository. |
| Diagnostic wordings | 8: 5 in Rust, 3 in the prelude. Plus 1 new help | Rust: `expand/src/expand/problems.rs:683`, `:868`, `expand/src/expand/module_body.rs:1346`, `src/types/problems.rs:398`, `:705`. Prelude `fail` help texts: `prelude/definers.knv:286` and `prelude/struct.knv:56` ("write a type after it, as in :x i64"), `prelude/syntax.knv:400` (`(defn add (:x i64 :y i64) (+ x y))`). These are diagnostic output under Rule 2, not docstrings, and they appear in stage snapshots. The help text with `(circle f64)` at `problems.rs:909` becomes `(circle float)`. | The new `L0212` help (option C). `module_body.rs:1346` writes a wording outside a problems type, which breaks the stage rule today. Moving it is out of scope. |
| Unit tests that assert printed text | 5 assertions | `src/types/unification.rs:324`, `:335`, `:393` (`` expected `i64`, found `bool` ``), `src/check.rs:453`, `:515` (`(type triple (fn (i64) i64))`, `(type wrap (fn (i64) (option i64)))`) | none |
| `tests/cli.rs` | 10 expectations of printed types, plus 2 headers. No `f64` | `:169` (`TWICE_TYPE`, `(type twice (fn (i64) i64))`, the type line that `:3471`, `:3481` and `:3652` expect), `:2327-2332` (`knave doc i64`, which now prints the `int` page), `:3008` (`(type size (fn () i64))` in a written page), `:3110` (`(type nothing (option i64))`), `:3550`, `:3559`, `:3568`, `:3624`, `:3661` (`knave doc --types` type lines), `:4614` (`"(vec i64)"`) | `:5650` and `:5656` are headers written with `i64`, which stay under decision 10(c). `:3701` (`Vec<i64>`) is emitted Rust and stays. |
| RFC 0070's candidates | 1 line | none | `rfcs/0070-foreign-names.md:618` (C2) lists Python's `int` and `float` with the hint `(the i64 (unwrap-ok (parse text)))`. RFC 0070's rule 5 (`:81`) judges "bound" in the value namespace only, so `int` and `float` stay unbound foreign names. After this RFC, `(int "3")` reports `L0201` with the "`int` is a type" note added, and the pull request that ships C2 writes its hint as `(the int ...)`. RFC 0070 itself is not edited. |
| CLI snapshots | `i64`: 10 files. `f64`: 5 files | Including `types_prints_inferred_struct_derives`, `repl_prints_an_inferred_type_as_a_string`, two task-error snapshots, `eval_notes_a_value_whose_type_does_not_implement_show` | `runs_the_prelude_tests_and_lists_the_untested_prelude_names` changes with the page names. |
| Stage snapshots | 545 `stages__*` snapshots match `i64` as a word (`git grep -lw i64`), and 164 match `f64`. 79 files and 166 lines have `i64` inside backticks (`` git grep -E '`[^`]*\bi64\b[^`]*`' ``) | Regenerated by the printer and read, not edited | Rust text in emit snapshots must not change (question 5) |
| Fixture sources | `i64`: 748 files, 1285 lines. `f64`: 290 files, 519 lines. Trait `float` as a bound: 9 files | The 9 files with a `float` bound: `check/float_literal_overflow_generic.knv`, `check/generic_literals.knv`, `check/implied_bounds.knv`, `emit/generic_literals.knv`, `emit/max_min_values.knv`, `run/generic_literals.knv`, `run/max_min_values.knv`, `run/function_value_float_items_follow_help.knv`, `run/function_value_literal_beside_generic_follow_help.knv`. They become `floating`. | Keep the `i64` and `f64` fixtures as proof that both still work. New fixtures write `int` and `float`. Add one check fixture where `i64` and `f64` are written and `int` and `float` are printed, and the tests listed under "Differences from PR 597". |
| `xtask/agree` findings | 0 | none | none |

### 5. Cost

**The RFC's own target: no front-end stage rises more than 1% in instructions on any corpus entry.** CI posts the counts on each ready pull request (`docs/pull-requests.md:34`) and never fails on an instruction rise, so the PR author reads the comment and holds the PR to this target. Before a pull request is ready, measure locally with `cargo xtask bench measure --workspace <checkout> --save <file>` for base and branch, then `cargo xtask bench compare`. C adds nothing on a hot path: two more exported names in the kernel and a field read where a type prints.

**The CI gate: allocated bytes and peak live bytes.** The `front-end instruction counts` job fails when a corpus program's allocated or peak live bytes rise more than 1% with no `Counters-Accepted:` trailer naming the program. Two changes grow what every program loads: the written name on an export entry (PR 1), and two more kernel exports (PRs 2 and 3). If the gate trips, the PR author stops and reports the programs and counters to the project owner. No trailer goes in without the owner's approval.

**The baseline gates.** `expand/src/core/kernel.rs` sits at its `production_lines` entry of 506 lines, and `ExpandProblems` at its `methods_per_type` entry of 99. PR 1 moves `PrimitiveType` and its table into `expand/src/core/primitive.rs`, which lowers `kernel.rs` well below its entry before the column grows the table. PR 3 adds the `L0212` help in a problems type of its own, as option C says, so `ExpandProblems` does not grow. Neither PR asks for a `Baseline-Accepted:` trailer. If a gate trips all the same, the PR author stops and reports the entry to the project owner.

**Emitted Rust stays byte-identical, apart from `infer` strings and the programs that "Breaking change" names.** No corpus entry, fixture, example or prelude file uses `int` as a type name today (`git grep -w int -- '*.knv'` finds only compound names such as `parse-int-error`, `parse-int` and `takes-int`). The 9 fixtures with a `float` bound change their source but not their emitted Rust, because the bound's Rust trait is still `crate::rt::Float`. So the checks below expect no other difference.

1. Emit snapshots: `git diff` on `tests/snapshots/stages__emit_fixtures@*` shows only the `infer` strings in the two snapshots named above. Read each hunk.
2. Corpus: emit every entry under `tests/fixtures/corpus/` with the base and with the branch, then `diff -r`. The diff must be empty.
   - Base: the probe binary of the base commit, downloaded as `docs/pull-requests.md:61-106` says. AGENTS.md forbids building unedited code to probe it.
   - Branch, before the PR is ready: a local `cargo build --profile local`, which builds edited code.
   - Branch, once the PR is ready: its merge binary, downloaded the same way.
3. Run fixture snapshots keep their output, except for the standard output of fixtures that print `infer`.

### Breaking change

Today `int` names nothing and `float` names a trait. After this RFC, both name kernel types in every module:

- **An implicit `impl` variable named `int` changes meaning.** An implementor treats each name that resolves in neither namespace as a type variable (`expand/src/expand/module_body.rs:1359-1377`, `:1420-1423`, `docs/language.md:1402`). `(impl sized (vec int) ...)` emits `impl<Int> Sized2 for Vec<Int>` today. After this RFC it emits `impl Sized2 for Vec<i64>`, and a call such as `(size (vec "a" "b"))` becomes a type error that `knave check` reports at the call. The program has no binder that a diagnostic could point at. `float` resolves today, as the trait, so it is never an implicit variable and this case does not apply to it.
- **A `float` bound stops compiling.** `(where ((n float)) ...)` reports `L0212`. Its help is option C's help for any numeric type written as a bound, which exists for `(n int)` as much as for this case. `(use prelude/number float)` for the trait now reports that `prelude/number` does not export `float` (`expand/src/expand/problems.rs:621`), because the module exports `floating`. A docstring link `[float]` now links to the type's page.
- **An explicit binder keeps compiling but reads differently.** A `where` binder, as in `(type keep (where (int) (fn (int) int)))`, and a struct, enum or `rust-type` head parameter, as in `(defstruct (holder int) ...)`, bind a type variable that shadows the kernel's `int` in their scope. The same holds for `float`. Their diagnostics then print `int` for the variable, which looks like the default integer. An `infer` text `"int"` inside them pastes as `i64` outside the `where`, which breaks RFC 0054's paste guarantee. Decision 8 covers these.

The owner is the only user and wants no migration aid of any kind: no help, note, alias, hint or deprecation period that exists only to ease the change for existing code. The `L0212` help is not one, because it serves every numeric type written as a bound, permanently, and would exist without the rename. PR 3 carries no fixture or test of a renamed `float` bound as such. The breaks ship as they are, under `release:minor`, which `docs/pull-requests.md:25` defines as a pre-1.0 change that makes a program that compiled and ran fail. The pull requests that export `int` and `float` (steps 2 and 3 below) carry the label.

### 6. Order of work as pull requests

1. **Split the primitive name, and give exports their written names.** Move `PrimitiveType` and its table into `expand/src/core/primitive.rs`, its own file, so that `kernel.rs`, at its `production_lines` baseline of 506, does not grow. Add the shown-name column with every shown name equal to its kernel spelling. Remove `name()` in favor of `names()`, whose `PrimitiveNames` gives `shown()`, `rust()` and `spelling()`, and move each caller in the Problem table, and `KernelType::shape`, to the right one. Make an export entry record its written name when it differs from its binding's name. No output changes. The bench comparison is flat in instructions, and the allocation counters show the cost of the export names alone. Review: each call site's choice.
2. **`int`.** Set `I64`'s shown name to `int`, and export one binding as `int` and `i64` from the kernel and the prelude. Rewrite the `i64` page as the `int` page, and make `knave doc i64` print it. Make the written prelude page carry one `int` section that lists `i64` as another name, and pin it with a test. Update the five Rust wordings and the three prelude `fail` help texts. Regenerate and read the snapshots, and update the ten `tests/cli.rs` expectations. The headers at `:5650` and `:5656` stay, under decision 10(c). Update the five unit-test assertions. Fix the prelude `infer` examples that would fail, and add the "written `i64`, printed `int`" fixture and the `int` versions of the tests from "Differences from PR 597". Rewrite the compiler output that the explanation pages quote. The tests `each_erroneous_explanation_example_reports_its_code_first` and `each_explanation_fix_builds_and_runs` (`tests/stages.rs:350-372`) check codes and builds, not quoted text, so no test would catch a stale quote. Apply decision 9 to the rustc relay, and add a fixture for the implicit `impl` variable that "Breaking change" describes. Run the byte-identity check and read the counters. The PR carries `release:minor`. It is large but mechanical: its review is reading the snapshot diff. The RFC merges with this PR or alone.
3. **`float`.** Rename the trait `float` to `floating` in the prelude, the 9 fixtures, `L0441.md` and `docs/language.md`. Set `F64`'s shown name to `float` and export it as in step 2. Rewrite the `f64` page as the `float` page. Add the `L0212` help for a numeric type written as a bound, which reads its trait names from the prelude's `rust-trait` rows, in a problems type of its own with `wrong_namespace`. Add fixtures for a float and an integer type, one where a renamed prelude trait changes the help, and one that pins the filter: a module-private `rust-trait` over every integer type, like `step`, that the help leaves out. Respell the `(circle f64)` help. Add the `float` tests. Regenerate and read the snapshots, and run the byte-identity check. The PR carries `release:minor`.
4. **Prelude docstrings and signatures.** The mechanical prelude rows, plus `the`'s docstring under decision 5. Implementor lists and the export list stay.
5. **Explanation pages.** The 77 pages, sorted page by page under Rule 3.
6. **`docs/language.md`, `docs/diagnostics.md` and README.** Introduce `int` and `float` first in Types and Literals, respell the Knave examples, and keep the Rust snippets and width-specific passages.
7. **Decision 8(b):** the warning for a type-namespace binding named after a kernel type, with its fixtures and explanation page.

PRs 2 to 7 ship in one release, so the release that introduces `int` and `float` also warns on a binding that shadows them (decision 8(b)), and no release has `knave doc` signatures that print `int` or `float` beside docstrings, explanation prose and `docs/language.md` text that still teach `i64` or `f64`.

### 7. Decisions for the project owner

0. **Scope: `int` only, or `int` plus user type aliases?** **Decided: (a)**, `int` as a built-in shown name through option C. `float` follows by decision 4.
   - (a) `int` only, through option C.
   - (b) `int` plus transparent user aliases, through option A's form in a later RFC.
   - (c) `int` declared through option A, as the first alias, with a prelude-only printing exception.
1. **If 0(b): which definer declares an alias?** Moot under 0(a).
2. **If 0(b): how does a user alias print?** Moot under 0(a).
3. **If 0(b): aliases with parameters?** Moot under 0(a).
4. **Does `f64` get a `float` name?** **Decided: (b)**, rename the trait to `floating` and give `F64` the shown name `float` by the same column. A `byte` row for `u8` is left out for now.
   - (a) Not in this RFC.
   - (b) Rename the trait to `floating`, and give `F64` the shown name `float`.
   - (c) Show `f64` under another name, such as `real`.
5. **Should the docs of `the` show both the default names and an explicit width?** (Owner's open question.) **Decided: (a).** `(the int n)` does something: a use can pin an unannotated literal to another width, and `the` stops that. With `(let (a 8080 b (the int 8080)) ...)`, passing `a` to a `(fn (u16) u16)` emits `let a: u16 = 8080;`, while passing `b` is `L0401`, `` expected `u16`, found `int` ``. `(the float 1.5)` keeps a float from narrowing to `f32` the same way.
   - (a) Three examples: an explicit width, `(the u16 8080)`; an empty collection, `(the (vec int) (vec))`; and `(the int 8080)` keeping a value from narrowing, with the `L0401` it causes at a `u16` use. **Recommended.** Each one shows a different reason to write `the`, and two of them show `int` first. The `float` page carries its own `(the float 1.5)` example, so `the`'s docstring needs no fourth.
   - (b) `(the int 3)` first with no explanation, then a width. It suggests that `the` is needed for the default, which it is not.
   - (c) Only explicit widths. This hides the pinning case.
6. **A lint or note suggesting a narrower width for a large `(vec int)` or `(vec float)`?** (Owner's open question.) **Decided: (a).**
   - (a) A note, no lint. The `int` and `float` pages and `docs/language.md` say that a large collection of small numbers can use `u8`, `u16` or `i32`, and a large collection of floats can use `f32`, to save memory. **Recommended.** The compiler cannot know a vector's size, so a lint would fire on every `(vec int)` and `(vec float)`, the defaults this RFC promotes.
   - (b) A lint on `(vec int)` and `(vec float)` struct fields only. It still fires with no evidence of size.
   - (c) Nothing.
7. **What does `knave expand` print?** **Decided: (a).**
   - (a) `int` and `float`, like every other printer. Export lists print the names as written, from the export entries (option C). **Recommended.**
   - (b) The written spelling in type positions. `CorePrinter` no longer has it, because lowering has already happened.
8. **What happens when a program binds a kernel type's name in the type namespace?** **Decided: (b).** This covers a type definition, `(defstruct int ...)` or `(deftrait float ...)`, a `where` binder, `(where (int) ...)`, and a struct, enum or `rust-type` head parameter, `(defstruct (holder float) ...)`. Each one silently shadows the kernel type in its scope. A mismatch can then read "expected `int`, found `int`". `(infer 1)` is `"int"` there too, and pasting it elsewhere names the other type, which breaks RFC 0054's "a text pastes into a `type` signature" (`docs/language.md:207`). The same hazard exists today for every kernel name, as `(defstruct i64 ...)` and `(where (i64) ...)` show. `int` and `float` make it likelier, `float` most of all, because a program that kept its own trait or type named `float` now shadows the kernel's. An import shadows the same way: `(use other i64 ->i64)` of a module that exports `(defstruct i64 :n bool)` checks with no warning, and after this RFC a library that exports a type `int` or `float` does the same in every module that imports it. An implicit `impl` variable is not covered here, because under C it no longer binds: it resolves to the kernel type ("Breaking change").
   - (a) Accept it and document it under Type texts.
   - (b) A warning on every type-namespace binding whose name is a kernel type's name, any of them: `int`, `i64`, `float`, `f64`, `string` and so on, when the name is bound to anything other than that kernel type. The prelude's re-exports and `(use prelude i64)` bind the kernel type itself, so they do not warn. **Recommended**, as step 7. It is a permanent rule for every kernel name, not a migration aid. It is uniform, it points at the binder that explains both the odd diagnostic and the broken paste, and it costs nothing in a module that does not shadow. The warning fires at the binding and at each `use` that binds such a name. A warning at the definition alone would leave the importing module silent, and the definition can sit in a library the user cannot edit.
   - (c) An error for the same bindings. That is stricter than the language's rule for values, where a program may shadow a kernel name, as `(let (if 3) ...)` does.
   - (d) Print the Rust name when the printing scope shadows the shown name. This needs a rule for diagnostics that span modules and scopes, and it brings back the mix that Rule 2 forbids.
9. **What do relayed rustc errors print?** **Decided: (a).** Rule 2 names "error messages" without an exception. The rustc relay prints rustc's own text, as "Printers" shows.
   - (a) Relayed rustc text keeps Rust's spelling, `i64` and `f64`. **Recommended.** That text is Rust: it carries a rustc code such as `E0277` and Rust syntax such as `Vec<i64>` and `Add<f64>`, and `i64` or `f64` there names the type that the user's own `rust` template or crate signature wrote. Mapping only the word would print `Vec<int>`, which is neither language. Before 1.0, only user `rust` text and crate calls reach rustc errors ([RFC 0065](0065-no-failure-reaches-rustc.md)). The exception goes into `docs/language.md` beside Rule 2.
   - (b) Map the words `i64` and `f64` to the shown names in `user_code` (`src/build/rustc_errors.rs:144-159`), as program names are mapped today, through a lookup by Rust name in the same column. Rule 2 then holds without an exception, but the relayed text mixes Knave names into Rust syntax.
10. **What does a doc page's header print?** **Decided: (c).** `knave doc --types` and `knave doc --write` print the inferred type line beside the header as written, so one page can show `(fn (int int) int)` above `(:width i64 :height i64)`. That is the mix Rule 2 forbids, on a page rather than in a diagnostic. `tests/cli.rs:3559` pins such a page, and `:5650` and `:5656` pin keyword-parameter headers written with `i64`. The same holds for `f64`.
    - (a) A header is source text and prints as written, like a source window, under the "a name the user wrote prints as written" rule from question 2. This is an exception to Rule 2 for pages.
    - (b) The page prints a header's type positions by their shown names. The header is a read form that prelude code builds (`prelude/macros.knv`, `doc-header`), so this needs new mechanism: either a builtin that resolves a type symbol, or a respelling when the CLI writes the page.
    - (c) As (a), and the RFC and the docs teach `int` and `float` in typed parameters, so that the mix shows only in a program that writes `i64` or `f64` itself. **Recommended.** No prelude definition has a header typed with `i64` or `f64`, so no prelude page shows the mix. A program that writes `i64` gets back the spelling it chose, next to a type line that tells it the shown name, and the page needs no new mechanism. Under (c) the "as written" rule covers headers, so this is the same exception that (a) states, not a new one.
11. **How does a diagnostic print a list of implementors?** **Decided: (a).** Missing-trait help, `partial-eq`'s implementors and the `(parse-error ...)` rows list every type a trait accepts, widths included.
    - (a) By shown names, like every other printed type: `i8 i16 i32 int i128 ... f32 float`. **Recommended.** Rule 2 holds with no new exception, and every name in the list is one the user can write and sees elsewhere. Under (b), a user whose value prints as `int` reads a list that names `i64` and must already know they are one type to see that `int` qualifies, which is the mix that Rule 2 exists to prevent.
    - (b) By kernel spelling: `i8 i16 i32 i64 i128 ... f32 f64`. A list of widths is where the width is the point, so Rule 3 extends to this compiler output, and the list reads as a uniform set of widths. PR 597 chose this for `f64`. It is a fourth exception to Rule 2, and `TypeDisplay` reads the row's kernel spelling for it, for every primitive, rather than checking `F64` by hand.

## Prototype

PR 1 was built first as the prototype: https://github.com/knave-lang/knave/pull/610 (head `a4d241014`, base `41c7adeef`). It confirmed:

- **The caller table.** Removing `name()` flagged exactly the ten callers listed, and no other.
- **No output change.** `knave expand`, `check` and `emit` on all 2587 `.knv` files under `tests/fixtures` and `examples`, the base probe binary against the branch: 7761 runs, 0 differ.
- **The cost.** Instruction totals moved by at most 0.34% on any corpus program. Allocated bytes rose 24 bytes per loaded module, which is 2,208 bytes on a program that loads only the prelude and at most 0.22% on any program, with no new allocation, and peak live bytes at most 0.30%, so the allocation gate passed with no trailer.

It changed two claims, and the sections above now state the built design:

- **Three names, not two.** The generated parameter names of structural impls need the kernel spelling: the Rust name would turn `string_1` into `String_1` and `unit` into the invalid `()_1`. So each row holds a `PrimitiveNames` value with the spelling, the shown name and the Rust name, which also keeps `PrimitiveType` within the design rules' limit of 7 public methods.
- **Where the written export names live.** An exports type in a file of its own joins the core module cycle, which the design rules refuse, so `ModuleExports` lives in `module.rs` beside `CoreModule`, which owns it.

## Recommendation

Option C, as the owner decided for decisions 0 and 4: `int` and `float` are the shown names of `I64` and `F64`, each type is one binding exported under two names, and the trait `float` becomes `floating`. The checker, the ownership pass and the emitter are unchanged, so Rule 1 holds for free. The emitted Rust stays identical apart from `infer` strings and the `release:minor` breaks. C's printing rule is uniform. With the owner's decisions, the RFC makes three exceptions to rules, and `docs/language.md` records each one:

1. **Names rule 5:** `int` and `i64`, and `float` and `f64`, are two names for one concept each.
2. **Rule 2, rustc relay (decision 9(a)):** relayed rustc text keeps Rust's `i64` and `f64`.
3. **Rule 2, written names (question 2, decision 10(c)):** a name the user wrote prints as written, in a source window, an echoing note and a doc page header.

## Decision

The project owner decided:

- **Decision 0: (a).** `int` is a built-in shown name of `I64`, through option C. No user type aliases in this RFC, so decisions 1 to 3 are moot.
- **Decision 4: (b).** The prelude trait `float` becomes `floating`, and `F64` shows as `float` by the same column. This RFC covers both names.
- **`byte`:** left out for now, as a possible later row of the same column.
- **Decision 5: (a).** The docs of `the` show three examples: an explicit width, `(the u16 8080)`; an empty collection, `(the (vec int) (vec))`; and `(the int 8080)` keeping a value from narrowing, with the `L0401` it causes at a `u16` use. The `float` page carries its own `(the float 1.5)` example.
- **Decision 6: (a).** A note and no lint. The `int` and `float` pages and `docs/language.md` say that a large collection of small numbers can use `u8`, `u16` or `i32`, and a large collection of floats can use `f32`, to save memory.
- **Decision 7: (a).** `knave expand` prints `int` and `float`, like every other printer. Export lists print the names as written, from the export entries.
- **Decision 8: (b).** A warning on every type-namespace binding whose name is a kernel type's name, such as `int`, `i64`, `float`, `f64` or `string`, when the name is bound to anything other than that kernel type. It fires at the binding and at each `use` that imports such a name. It ships as step 7. `int` used as a type variable is a breaking change: it ships under `release:minor`, with no migration aid ("Breaking change").
- **Decision 9: (a).** Relayed rustc text keeps Rust's spelling, `i64` and `f64`. `docs/language.md` records the exception beside Rule 2.
- **Decision 10: (c).** A doc page's header prints as written, under the "a name the user wrote prints as written" rule. The RFC and the docs teach `int` and `float` in typed parameters, so the mix shows only in a program that writes `i64` or `f64` itself. `tests/cli.rs:5650` and `:5656` keep their `i64` headers.
- **Decision 11: (a).** A diagnostic prints a list of implementors by shown names, `i8 i16 i32 int i128 ... f32 float`, like every other printed type. The prelude source of those lists keeps `i64` and `f64` (Rule 3).
- **`L0212` help: approved as written** in option C: only the traits the `prelude` module exports, fewest rows first and ties by name, with the wordings "write a trait that every floating-point type implements, such as `floating`, `signed` or `number`" and "write a trait that every integer type implements, such as `integer` or `number`".
- **Acceptance:** the owner accepted this RFC on 2026-11-15, after PR 1 was built first as its prototype and reported which claims it confirmed or changed (see Prototype).

## Rejected options and why

- **`(rust-type int "i64")`:** makes a nominal type that does not unify with `i64`, which breaks Rule 1, and literals would still print `i64`.
- **A for `int` (0(c)):** a kernel form, a table and a prelude-only printing exception, for what C does with a column.
- **B, rename on import:** gives every module a second `use` shape, and it still needs a rule for which name prints.
- **D, a prelude rewrite:** misses type positions that user macros emit, and printing still needs the compiler.
- **Two bindings of one kind per primitive:** `same-binding?` would tell `int` from `i64`, and the docs would need two pages for one type.
- **Echo the written spelling:** settled by Rule 2. It mixes `int` and `i64` for one value.
- **Remove the trait `float` instead of renaming it:** generic code over both float widths would have no bound narrower than `number`, which admits integers.

## Rule

[docs/language.md: Types](../docs/language.md#types), with new text under Kernel forms (the primitive list), Literals (default `int` and `float`), Type texts (`(infer 1)` is `"int"`, and decision 8), Names (the rule 5 exception) and the rustc relay (decision 9). [docs/diagnostics.md](../docs/diagnostics.md), beside "Use the names the user wrote".
