# RFC 0063: One interpreter for programs and macro code

**Status:** Accepted
**Date:** 2026-11-12

## Problem

**The main goal is the lowest footprint for 20 to 30 agent sessions working in parallel, each in its own worktree.** Local sessions use the interpreter in most cases, and CI and deployment use the compiler. Every choice below is judged by three numbers per session: peak memory, CPU time per check-and-test cycle, and disk per worktree, which must be zero beyond the source files. The model for feel is a babashka Clojure project, where 30 sessions cause no trouble.

Today every `run`, `test`, `eval` and prompt expression pays for a rustc run of 0.25 to 1.3 s of CPU and 150 to 230 MB of memory. On disk, a Knave project costs little already. Each entry that `build` or `run` compiles writes about 1 MB under `target/knave/` in its worktree, and crates build once into the shared Cargo target directory in the user cache (`CargoCompiler::shared_target`), which every worktree shares, as Go's build cache is shared. **So the interpreter's gain is mainly CPU and memory, and only a small gain on disk.** The compiler repository's own Cargo workspace takes 1 to 9 GB per worktree, but that is the compiler developers' cost, and this RFC does not change it.

Two more problems share the fix:

- **Macro code runs in a second, dynamic interpreter.** `expand/src/interpret` walks the untyped core IR over its own values: forms, lists, booleans, unit, closures and built-ins. Macro code cannot use structs, maps, `match`, `try` or any `rust` definition, so a macro cannot call `+` or `<`. A mistake in macro code shows as a shape error when the macro runs, not as a type error.
- **`knave task` checks the task tree twice** ([RFC 0066](0066-project-tasks.md)). The first check exists only so that `run-task` can read the inferred signature of a task, because a macro cannot ask for the type of a definition.

### Method

- **Machine.** 18 cores and 30 GB, shared with other sessions.
- **Limits.** Every measurement ran under one cgroup capped at 4 GB and four cores, at `nice 19` and idle I/O priority.
- **Sessions.** Each session is a short-lived process tree started in its own worktree copy. Sessions run in batches, and the batch size is given for each row.
- **What is measured.** CPU is the user and system time of the whole tree. "Largest process" is the kernel's maximum resident size of the largest process in the tree. PSS is sampled every 20 ms across the tree, and divides each shared page among the processes that map it. A process that ends inside 20 ms is not sampled, so for those rows only the largest-process figure counts.
- **Totals for 30 sessions.** A total is measured when 30 sessions ran at once. When the 4 GB cap allowed only batches of 10, the total is the batch peak times three, labelled as an extrapolation.
- **Tools.**
  - `knave` 0.14.22, the installed release, for the toolchain table.
  - For both paths of the same-cycle table, the prototype at `546fb97b0`, built with `--profile local` (release with 16 codegen units and no LTO). A debug build's `knave check` takes about 11 times the CPU.
  - Go 1.26.5, babashka 1.13.229 and rustc 1.95.0.
- **Inputs and logs.** Kept in the worktree of the branch `research-interpreter`, under `target/scratch/rfc-0063/`:
  - `knave-proj/`, `go-proj/`, `bb-proj/` and `rust-proj/` are the sample project, its test module and its ports.
  - `proto/` holds the probe programs.
  - `measure.py` is the batch-and-sample script, and `sharing.py` measures shared pages.
  - `speed2.py` times one program at a time, best of five, from `getrusage` of the children. `check_only.py` times `expand` against `check`.
  - `results/` holds the raw JSON of every row.
  - `census/` holds the census of prelude macro code.

### Baselines by toolchain

The sample is the corpus entry `warehouse` as a project: 609 lines in five modules, with structs, enums, traits, `match`, `sorted-map` and `try`, plus a test module of three tests, `knave-proj/tests/warehouse.knv`. The corpus entry has no tests of its own. The Go and Clojure ports print the same bytes as the compiled Knave program.

| Toolchain and cycle | Batch | CPU per cycle | Largest process | PSS per session | Disk per worktree | Shared disk |
|---|---|---|---|---|---|---|
| `knave check` alone | 30 | 0.054 s | 16 MB (2.4 MB heap) | not sampled | 0 | none |
| `knave check && knave test` | 10 | 1.40 s | 230 MB (rustc) | 60 MB median, 145 MB at most | 0 after `test`, about 1 MB after `run` | crates only, in the shared Cargo target |
| `cargo check && cargo test`, own `target/`, no crates, first cycle | 1 | 1.8 s median (1.7 to 7.5) | 220 MB | 150 to 245 MB | 1.9 MB | none |
| the same after an edit | 1 | 1.5 s median | 220 MB | 140 to 235 MB | 1.9 MB | none |
| `go vet && go test -count=1`, first checkout | 1 | 45 s | 291 MB | 317 MB | 0 | 105 MB build cache |
| the same with the cache warm, or after an edit | 10 | 1.36 s | 85 MB | 35 to 43 MB median | 0 | 105 MB build cache |
| `bb test` | 30 | 0.061 s | 49 MB | not sampled | 0 | none |

The compiled test program of `warehouse` runs its three tests in 1.77 ms of CPU, of which 1.42 ms is starting a process. The tests' own work is 0.35 ms. The Cargo rows ran one at a time because this repository's dev shell admits two compiling `cargo` commands at once. That queue is a property of the dev shell, not a cost a Knave user pays.

### The same cycle on both paths

The interpreter prototype does not yet run `warehouse` (see the prototype section), so the comparison between paths uses programs both can run. Each session runs `knave check` and then `knave run` or `knave interpret` on ten programs: `hello`, `fib` (recursive `fib 27`), `loop_sum` (a 3,000,000-step `loop`), `vectors` (200,000 `push!` calls and a sum), `records`, `overflow`, `mut_param`, `closure_counter`, `move_copy` and `field_set`. Every program prints the same bytes on both paths.

| Cycle | Path | Batch | CPU per session | Largest process | PSS per session | 30 sessions together | Disk per worktree |
|---|---|---|---|---|---|---|---|
| all ten programs | compile and run | 10 | 4.7 s | 175 MB | 52 MB median, 88 MB at most | about 1.3 GB, extrapolated from 432 MB per batch of 10 | 5.4 MB |
| all ten programs | interpret | 30 | 10.2 s | 17 MB | 7 MB median, 11 MB at most | 65 MB, measured | 0 |
| the seven light programs | compile and run | 10 | 3.18 s | 176 MB | 51 MB median | about 1.3 GB, extrapolated | 5.4 MB |
| the seven light programs | interpret | 30 | 0.135 s | 16 MB | not sampled | not sampled | 0 |

**Where the interpreter costs more CPU today.** On the full cycle it uses 2.2 times today's CPU: 10.2 s against 4.7 s per session, and 308 CPU-seconds against 151 for 30 sessions. The three compute-heavy programs cause it. `results/r-speed2.json` times each program alone, one at a time, best of five:

| Program | Interpret | Compiled binary | Work ratio | `knave run` (check, build and run) |
|---|---|---|---|---|
| `fib 27` | 0.70 s | 0.0039 s | 258 | 0.28 s |
| `loop_sum` | 3.66 s | 0.014 s | 285 | 0.28 s |
| `vectors` | 0.53 s | 0.0033 s | 248 | 0.24 s |
| `hello` | 0.0041 s | 0.0012 s | | 0.27 s |
| the five other light programs | 0.0052 to 0.0074 s each | 0.0009 to 0.0010 s each | | 0.25 to 0.29 s each |

These rows were measured alone. The cycle table ran in batches, and in the result files both paths cost about 1.7 to 2.2 times more CPU in a batched run than alone, at a batch of 10 as well as 30: the interpreted cycle takes 10.9 s per session at a batch of 10 and 10.2 s at 30, against about 4.9 s alone, and the compiled cycle 4.7 s at a batch of 10 against about 2.7 s alone. The cause was not isolated. Alone, the nine programs other than `overflow` take 4.9 s interpreted against 2.5 s through `knave run`, the same 2.0 to 2.3 times as in the batches.

**The work ratio** is the measure of interpreted speed in this RFC: interpreted CPU less `knave check` of the same program, against the compiled binary's CPU less the empty program's binary (`hello`, 0.0012 s). It leaves out the fixed cost of checking on one side and of starting a process on the other, which dominate a light program: the five light programs have whole-process ratios of only 5 to 7. On the three compute-bound programs the prototype's work ratio is 248 to 285, and 277 summed over the three. An interpreted run costs less than `knave run`'s 0.24 to 0.29 s only while the compiled program's own work stays under about 0.8 to 1.1 ms: `knave run`'s cost, less the interpreter's check, divided by the work ratio. A session uses more CPU than today once its runs average more than that. The `warehouse` tests' 0.35 ms of work is under that point: about 90 to 100 ms interpreted at the prototype's speed, against 1.3 s of rustc. On memory and disk the interpreter wins on every program: a largest process of 16 MB against 175 MB, and nothing written.

### Shared pages of the embedded prelude

The prelude state is 1.1 MB of `bincode` embedded in the binary's read-only data. With 30 `knave` processes alive, each maps 5.85 MB of the binary. All of those pages are shared and clean, so each process pays 0.29 MB of PSS for them. The OS shares every embedded byte. Each process pays privately only for its heap, which peaks at 2.4 MB for `knave check` on the sample, and about 1 MB of other dirty pages. So the design keeps prelude-derived data, native tables included, in the binary's read-only data, checked prelude macro code included (decision 2), or builds it lazily in each process, and never writes it to a cache file. Whatever a process decodes from it into its heap is private and counts against the private heap target.

## Options

### A: One interpreter over the typed, ownership-checked IR, for programs and macro code

`run`, `eval`, the prompt, `test` and `task` check the program as today and then walk the `TypedProgram`. `build` compiles. Macro code is type-checked in dependency order before it runs, and runs in the same interpreter.

```lisp
(defn bump! (n) (set! n (+ n 1)))

(defn main ()
  (local x 1)
  (bump! x)
  (println x))
```

```text
$ knave run bump.knv          # interprets, writes nothing
2
$ knave build bump.knv        # compiles, as today
```

#### Values and places

- **Integers** carry their primitive type from the typed IR. Arithmetic checks the exact range and panics as a debug build does, with `attempt to add with overflow` and exit 101. A `u128` above `i128::MAX` has its own representation, so every integer type fits.
- **Floats** carry `f32` or `f64` and round to their type after every operation. They print as Rust's `Display` prints them.
- **Heap values** are shared behind reference counts: strings, vectors, maps, struct and enum values, and closures. Moving a value or taking a shared borrow of it shares it. The ownership checks of [RFC 0065](0065-no-failure-reaches-rustc.md) prove the sharing unobservable: no place changes while a shared borrow of it lives. `clone` copies deeply.
- **Places.** A local lives in a cell. A `mut` borrow of a local, of a field or of an element is a reference to that place, not to its value. A write through it, such as `set!`'s `*{0} = {1}`, changes the place. The typed IR already decides each case: `TypedArgument::borrow` is `Mutable` where a call lends a place as `mut` and `Passthrough` where a `mut` parameter passes on, and `Borrow(ReferenceKind::Mutable, place)` is a written `mut`.
- **Closures.** A closure captures its locals' cells, so `(fn () (set! count (+ count 1)))` changes the enclosing `count`. A `move` closure copies its captures into cells of its own when it is made, so its own changes stay its own. Because the emitter writes `move` only on a closure in a function body's tail position, nothing in that body runs after the closure is made, and a later change of the same local cannot happen in Knave. The emitter alone decides this today. The interpreter needs the same fact, so the type stage records a capture mode on each `TypedLambda` and the emitter reads it, by RFC 0065's rule that one stage decides each fact.
- **Maps.** A `hash-map` iterates in an order drawn from a seed for each run. The seed is printed when a run fails, and `KNAVE_SEED` sets it to repeat the run. A `sorted-map` iterates in key order.

#### Natives for prelude `rust` leaves

Each `rust` form that the prelude writes, or that a prelude macro builds, names a native operation that the interpreter implements once. Decision 1 chooses how the form names it.

#### Macro code

When the expander reaches a macro whose code is not yet checked, the macro's definition and every global it reaches are checked as one group, in dependency order. The group is type-checked and ownership-checked, and then run in the interpreter. The section on the `code` module gives the types and the ownership rule. By decision 2, the prelude's own macro groups are checked when `build.rs` builds the prelude state and saved in the binary, so no session checks them. The section on build cost measures what that costs the compiler's own builds.

Running macro code calls back into the expander through the interpreter's one `Host` trait (`expand/src/interpret/host.rs`), with its eight groups: `GlobalValues`, `ModuleQueries`, `DefinitionQueries`, `ModuleNotes`, `NameComparison`, `ExpansionCallbacks`, `SourceQueries` and `Warnings`. The typed interpreter keeps all eight, because every `code` built-in keeps its meaning. `GlobalValues` changes most: a global's value comes from its checked definition. `DefinitionQueries` gains `definition-type`.

#### Asking the type of a definition

A macro may ask for the inferred type of a definition earlier in dependency order. `(definition-type name)` checks that definition and what it reaches, if they are not checked yet, and returns the signature text, as `infer` writes it.

#### What the interpreter does not run

- **Crate calls.** [RFC 0065](0065-no-failure-reaches-rustc.md) decided that an input that reaches a crate is not interpreted: it compiles against prebuilt dependencies. Such inputs keep today's cost until RFC 0064, compiled crate bindings, gives the interpreter compiled crate bindings. **The footprint targets below cover inputs without crates.**
- **A user's own `rust` template** has no native, so a program that reaches one compiles for that command, with a note that names the template, and its errors stay mapped rustc errors, as ruled on 2026-11-09 and decided in decision 3. The language reference recommends keeping user Rust in a Knave library, so that only programs that use the library compile, and RFC 0064 later compiles such a library once per library commit into the shared store.
- **What "reaches" means.** An input reaches a crate or a user template when the call graph from its entry contains one. For `run` and `eval` the entry is `main` or the forms. For `task` it is the task's runner. For `test` the entry is each selected test, and decision 6 chooses whether one such test sends the whole test program to the compile path or only that test.
- **`knave task`** interprets as `run` does, under the same rule, so its interface stays as [RFC 0066](0066-project-tasks.md) promised and a task writes nothing to the worktree.

### B: Keep compiling, as today

Every command keeps calling rustc. Crates keep building once into the shared Cargo target directory that every worktree already uses, and each worktree keeps about 1 MB per compiled entry under `target/knave/`. Macro code stays in the dynamic interpreter, and `knave task` keeps its double check.

```text
$ knave test     # rustc on every cycle: 1.3 s of CPU, 150 to 230 MB
```

### C: A typed interpreter for programs, and the dynamic interpreter kept for macros

Option A's program interpreter, with macro code left in `expand/src/interpret`, untyped.

## Recommendation

Take Option A, on the condition that the interpreter reaches the speed target before any command interprets by default. The prototype's same-cycle result has two sides:

| Cycle | Interpret | Compile and run | Result |
|---|---|---|---|
| seven light programs | 0.135 s per session | 3.18 s per session | 24 times less CPU |
| all ten, with three heavy programs | 10.2 s per session | 4.7 s per session | 2.2 times more CPU |

On both cycles the interpreter's largest process is 16 to 17 MB against 175 MB, 30 sessions together peak at 65 MB against an extrapolated 1.3 GB, and it writes nothing.

**What the targets must reach for the interpreter to win overall.** All figures here use the work ratio and the programs measured alone. The compiled path costs about 0.27 s per program whatever the program does, so the interpreter wins whenever its work ratio times a program's compiled work stays under that cost less its own check. On the nine timed programs of the cycle, all but `overflow`, the programs' compiled work adds up to about 17.6 ms, all of it in the three heavy programs, the fixed checks to about 0.1 s, and the compiled path to 2.5 s. The interpreter ties at a work ratio of about 135, about twice the prototype's speed. At the target work ratio of 30, the same cycle takes about 0.6 s interpreted against 2.5 s compiled, four times less CPU, and a single run wins while its compiled work stays under about 8 to 9 ms. The 30-times target therefore carries the main goal: without it, a session whose runs are heavy loses CPU. Slot resolution and native function pointers in pull request 4 are the work that buys the speed. Decision 4 keeps `run --compiled` for programs heavier than that.

Option B keeps rustc in every cycle. Option C keeps two interpreters with two value models, keeps macro code untyped, and keeps the `deftask` double check.

### The prototype

The prototype is two commits on the branch `research-interpreter`, never pushed: `eef8089bb` and `546fb97b0`. It adds a hidden `knave interpret` command and `src/run/`, about 780 lines, written in about three hours.

**What it covers:**

- `defn` and `fn`, recursion, `recur` and `loop`, `if`, `do` and `local`.
- Closures that capture by reference, and `move` closures in tail position.
- `mut` arguments and `set!` on locals, parameters and fields.
- Integer arithmetic with the debug build's overflow and division panics, floats, strings and comparisons.
- `vec`, `push!`, `len`, `at` and `clone`, and struct construction and field reads.
- The `format!`, `println!`, `print!` and `eprintln!` templates that `str` and `println` build.

**What it does not cover:**

- `match` and enums, `option` and `result`, traits and impls.
- Maps, iterators, and natives that call back into Knave code.
- `assoc` and struct updates, `try`, and `rust` clauses for specific types.
- Crate calls and the per-run seed.
- Standard input, arguments, and exit codes other than a panic's.
- Macro code.
- Floats other than `f64`, and `u128`.

On `warehouse` it stops at the first template it lacks, `{0}.as_str()`.

**Agreement.** It runs 11 probe programs. The ten valid ones print the same bytes as `knave run`'s debug build:

- The overflow program exits 101 on both paths.
- `mut_param` prints `2`.
- `closure_counter` prints `2`.
- `move_copy` prints `11 20 2`: a returned `move` closure keeps its own `count` across calls. Its `adder` part takes `base` by value, so the later `(set! base 20)` in `main` does not test capture.
- `field_set` prints `6 2`.

The eleventh, `move_local`, shows the risk that RFC 0065 exists for. `check` accepts `(local show (fn () base)) (set! base 20)`, the emitted `let show = || base; base = 20;` fails in rustc with `E0506`, and the interpreter prints `20 20`. The loan rule of RFC 0065 must cover a closure's borrow of its captures, and this program joins the harness's fixtures.

**What the prototype found.**

1. **Many `rust` leaves are not prelude definitions.** `str`, `println`, `print`, `eprintln`, `vec`, the map literals, `char`, `show` and `display` build their template text at each call site. `(println "fib 27 = " (fib 27))` becomes a `rust` form with the text `println!("fib 27 = {{}}", {0})`, so a native table keyed by definition cannot cover them. The prototype reads the format string out of the template text, which works but depends on how the prelude writes it. Decision 1 is about this.
2. **The typed IR carries what overflow and places need.** A literal's and a call's type name the exact primitive, and a call argument's `Borrow` says where a place is lent as `mut`.
3. **The typed IR lacks two facts.** One is a closure's capture mode, which the emitter decides by position. The other is the impl that a trait call selects, which the emitter leaves to rustc. An interpreter can dispatch on the receiver's value for a method with `self`, but not for a method without a receiver value, such as `from-str` or `default`. The type stage must record both.
4. **Natives that take a closure call back into the interpreter.** `map`, `filter`, `reduce`, `sort-by` and the other iterator forms call Knave closures. Iterators stay lazy, as Rust's are, or a program that takes 5 items of an endless iterator diverges.

### Build cost of checking prelude macros at build time

By decision 2, `build.rs` checks the prelude's macro code. Building the prelude state also expands the whole prelude, which runs every prelude macro: `defn`, `defrule`, `document` and the rest. Once pull request 14 deletes the dynamic interpreter, that run needs the new interpreter. So the type, ownership and interpreter stages all become build dependencies, in one crate of their own beside `knave-expand`. Every edit to them then rebuilds them for the build script as well as for the binary, and reruns `build.rs`.

The interpreter's size is not yet known. The prototype is about 780 lines, and today's dynamic interpreter is 5,354 lines, so the estimate below takes 5,000 to 10,000 lines. With `src/types` and `src/ownership` at 34,265 lines, the crate is about 39,000 to 44,000 of the 65,073 lines of `src/`.

`knave-expand` is already a build dependency, so it shows the mechanism. Each run below is one rebuild of the `knave` binary after touching one file, measured with `cargo build --timings` and the method above. The unit times come from Cargo's timing report, which gives wall time per unit. CPU is the whole build's, and single runs on this shared machine vary by about a third. Results are under `results/r-rebuild-*.json`, `r-units-*.json` and `timing-*.html`.

| Profile and edit | Build-script settings | CPU | `knave-expand` units | `knave` library unit |
|---|---|---|---|---|
| `local`, `expand/src/lib.rs` | today's default, opt-level 0 | 306 s | 92.5 s and 19.7 s, so twice | 136.9 s |
| `local`, `expand/src/lib.rs` | opt-level 3 and 16 codegen units, as the target | 413 s | 109.8 s and 109.3 s, still twice | 121.5 s |
| `local`, `expand/src/lib.rs` | the same, and `resolver = "1"` | 312 s | 66.1 s, once | 115.1 s |
| `dev`, `expand/src/lib.rs` | today's default | 116 s | 32.9 s and 33.4 s, so twice | 42.2 s |
| `dev`, `src/types/checker.rs` | today's default | 40 s | none | 28.8 s |
| `local`, `src/types/checker.rs` | today's default | 173 s | none | |

**What it shows.**

- **The build script's copy.** Today every edit to a build dependency compiles it twice. In `local` the build script's copy is cheap, because it is unoptimized: 19.7 s beside 92.5 s. In `dev`, where both copies are unoptimized, the copy costs as much as the original: 33 s each, 29% of the rebuild.
- **Matching settings are not enough.** Giving the build script the target's opt-level and codegen units does not merge the two copies. The two `rustc` command lines for `knave-expand` then differ only in their metadata hash, because Cargo's default feature resolver for edition 2024 keeps build dependencies apart from normal ones. The copy just gets as expensive as the original.
- **Resolver 1 merges them.** With `resolver = "1"` and matching settings, `knave-expand` compiled once. Resolver 1 unifies features between build dependencies and normal dependencies across the whole workspace, so it is safe only while that unification changes no crate's features.

**Option A's cost under each choice,** estimated for the crate of the type, ownership and interpreter stages, from its share of the `knave` library unit. `build.rs` today expands the prelude in 0.31 s with its unoptimized `local` copy and 1.09 s in `dev`, and checking the macro code adds about 1 to 2 s unoptimized:

| Choice | `dev`, after an edit in `src/types` | `local`, after an edit in `src/types` | `release` |
|---|---|---|---|
| keep today's build-script settings | about 17 to 20 s more, from 40 s to about 57 to 60 s, and `build.rs` about 1.5 to 3 s | about 20 to 23 s more for the opt-level 0 copy, from 173 s to about 195 s, and `build.rs` about 1.5 to 3 s | the opt-level 0 copy, about 20 to 23 s, in CI builds only |
| match the build script's settings to each profile | the same as above | about 100 s more: a second optimized copy, as measured for `knave-expand` | as above |
| match settings and use `resolver = "1"` | no second copy, if `dev`'s build script also takes `debug = "line-tables-only"`; `build.rs` about 1.5 to 3 s | no second copy; `build.rs` under 0.5 s, because the checker and the interpreter are optimized | fat LTO compiles normal units differently, so the opt-level 0 copy stays, in CI builds only |
| a smaller crate | already applied: the crate holds only the type, ownership and interpreter stages, not emission, the CLI or the build stage. Prelude expansion needs all three, so a smaller crate would mean splitting a stage | | |

**Recommendation.** Use the crate of the type, ownership and interpreter stages alone, set `resolver = "1"`, and give each profile's build script the profile's own settings, so the crate compiles once in `dev` and `local`. Keep the release profile's default build-script settings, so CI's release build pays one unoptimized copy, about 20 to 23 s. Pull request 12 measures `dev` and `local` before and after. It keeps the change only if `cargo tree -e features` shows the same features with both resolvers and each crate then compiles once. Otherwise it keeps today's settings, which cost about 17 to 20 s per `dev` rebuild and 20 to 23 s per `local` rebuild.

### Where the checked prelude macro groups are saved

`PreludeState` lives in `knave-expand`, and checked macro groups are values of the type stage, which `knave-expand` may not name. So the checked groups go in a second embedded file that the new crate owns:

- The `MacroRuntime` implementation lives in the new crate of the type, ownership and interpreter stages, because `build.rs` cannot depend on its own package's `src/`. `build.rs` and the CLI in `src/` both use that one implementation.
- `build.rs` builds the prelude state with it: it runs `PreludeState::build` with the new crate's `MacroRuntime`, which checks each prelude macro group and runs it during expansion.
- It writes two files to `OUT_DIR`: `prelude-state.bin`, the expander's state as today, and `prelude-macros.bin`, the checked groups, keyed by the binding of each macro and global.
- The expander's saved state refers to a compiled macro by its binding only. The `MacroRuntime` resolves that binding in `prelude-macros.bin` when the macro first runs in a session.
- Both files are embedded with `include_bytes!`, so both stay in the binary's shared read-only pages.

Pull request 12 makes `build.rs` depend on the crate, and pull request 14 writes the second file.

### The types of the `code` module

Macro code is written against one kind of datum. The design types it that way:

- **`form`** is a kernel type for every compile-time datum: a symbol, a keyword, a string, a number and a list. A list of forms is a `form`. A `form` is immutable.
- `bool`, `unit` and function types stay as they are.
- A string, number or boolean literal, or a `(quote x)`, in a position that expects `form` has type `form`. This is a new literal-fit rule beside the integer literal rule.
- `fail` returns `never`, a bottom type that fits every expected type.
- Built-ins with a variable or optional number of arguments get variadic types. Only built-ins have them, and user functions never do.

| Built-in | Type |
|---|---|
| `first`, `rest` | `(fn (form) form)` |
| `nth` | `(fn (form form) form)`: the index is a number form |
| `list`, `concat`, `string`, `symbol` | variadic: any number of `form`s to `form` |
| `prepend` | `(fn (form form) form)` |
| `list?`, `symbol?`, `empty?` | `(fn (form) bool)` |
| `value-eq` | `(fn (form form) bool)` |
| `number-add` and the other number built-ins | `(fn (form form) form)`, or `bool` for comparisons |
| `fail` | `(fn (form form ...) never)`, with the optional help and labels |
| `template` | a kernel form whose value is `form` |

The table covers the built-ins macro code uses most, about 20 of the 75 in `expand/src/core/builtin.rs`. The rest are as easy to type and pull request 13 covers them all: those that return `unit`, such as `add-module-note`, the `declare-` built-ins and `warn`; `set-macro-step-limit`, which returns `bool`; and `read-form`, which returns `form`.

**The census.** Repeat it with `census/macro_census.py`, which finds every macro and every helper reachable from it, quoted symbols included, because generated macros call helpers.

- The prelude holds 5,648 lines of compile-time code: 71 macros in 749 lines and 762 helpers in 4,899 lines.
- The 30 `defrule` definitions add no checked code, because their clauses are quoted data.
- The rest is in the census table under `census/`. The largest files are `prelude/macros` with 1,116 lines, `prelude/documentation` with 703, `prelude/test` with 686, `prelude/syntax` with 637 and `prelude/task` with 605.

**Fit.** The dynamic interpreter already holds macro code to this design in most places: `list`, `prepend` and `concat` accept only code, a literal already evaluates to a form, and `if` already demands a boolean.

- `prelude/task.knv` gets **0 type errors** with the three rules above, done by hand and with `census/t1_check.py`.
- The whole prelude gets about 27 to 30 errors and needs about 50 to 65 lines rewritten:
  - 22 come from `false` used as a "no match" value where a form is expected, in the matcher of `defrule` and `defmacro` (`prelude/macros.knv`, lines 85 to 451).
  - 4 come from an `if` with a `unit` branch and a `form` branch (`definers.knv:187`, `:304`, `:326`, `documentation.knv:67`).
- One pattern the design cannot type is in `prelude/internal/syntax.knv:17-35`. It builds a record out of closures that are applied at several types, which needs rank-2 types, and it has 13 use sites. Decision 9 is about it.
- The checker used for the count is approximate and skips closure parameters, so the count is a lower bound.

**Ownership in macro code.** Macro code checks by the same rules as program code, with one addition: `form` is copy, so using a form twice, as `define-task` uses `form`, moves nothing. Closures follow the closure-kind rule of program code (`docs/language.md`, Ownership, Closures). A closure that only reads its captures can be passed and reused, and one that moves or changes a capture cannot be copied, `L0501`. The two closures that prelude macro code reuses capture only forms, so both come out `Fn` under that rule: `fill-iteration` at `macros.knv:247`, passed through `repetition-count` and `map-count`, and `finished` at `documentation.knv:441`. Nothing in prelude macro code changes a value in place. Under decision 8, macro code that uses vectors or structs gets the ordinary ownership rules for them, so a closure that `push!`es to a captured vector is rejected in macro code as it is in program code.

### The step budget

A step budget bounds what macro code may do to the compiler, so it has to be defined for the walk that the new interpreter does. The new costs:

| Work | Steps |
|---|---|
| evaluating a node of the typed IR, other than an adjustment that adds or removes a borrow | 1 |
| reading a local or a global | nothing beyond its node's 1, because locals are slots and globals are resolved before the run |
| a call, a `recur`, or a call of a closure from a native | 1, beside the nodes the body runs |
| making a closure | 1, and 1 for each capture |
| a `code` built-in | its cost today: 1 for each list item it copies or reads, 1 for every 16 bytes of text, 3 for each form that `read-form` or `string-lines` builds, and the rest of the table in the language reference |
| `expand-call` | 64, and 1 for each form it returns |
| any other native | 1, and 1 for each item or every 16 bytes it copies or reads |

The limit stays at 30,000,000, `with-macro-steps` keeps its meaning, and the byte, list and note limits of `ValueLimits` and `ModuleNotes` stay as they are. The pull request that switches macro code restates the step costs in the language reference. It may change the position where `L0363` is reported in these snapshots, each of which it must read before accepting:

- **Expand fixtures:**
  - The `macro_step_limit` group: `macro_step_limit`, `macro_step_limit_counts_function_calls`, `macro_step_limit_passed_by_a_loop`, `macro_step_limit_reported_once`, `macro_step_limit_through_expand_call`, and the five limit-value fixtures (`of_a_float`, `of_a_negative_number`, `of_a_non_integer`, `of_a_too_large_number`, `of_zero`).
  - The `macro_steps_charge_*` group, twelve fixtures.
  - `expand_call_restores_the_step_limit`.
- **Run fixture:** `macro_steps_raised`.
- **CLI tests:** `eval_reports_a_macro_that_passes_the_step_limit` and `repl_keeps_its_session_after_a_macro_passes_the_step_limit`.

`L0355` and the `expansion_too_*` fixtures count forms, not steps, and do not change.

### `deftask` without the double check

- **The task program.** `run-task` asks `(definition-type (note-function note))` for its task's function and gives `string` to each parameter whose type is still open, by the rule it applies today. `TaskRunners` stops passing a signature.
- **What is removed.** The `task-signature` macro, `TaskCatalog::signatures`, and the first check of the task tree.
- **Checking once.** The type results that `definition-type` produces stay in the check session. The check of the whole program reuses each group that is already checked instead of checking it again. That makes `knave task <name>` and `knave check` check the task tree once.
- **The measure.** The CPU of `knave check` on a project with 20 tasks, before and after the change. It should fall by about the cost of one check of the task tree.
- **Text to update.**
  - The tasks part of `docs/language.md`.
  - RFC 0066's points 9 and 13, and its sentence "Every task command type checks the task tree twice".
  - `TaskCatalog::user_output`'s `DistinctOutput`, which exists because two checks report the same error. It may go.
- **What stays.** Task syntax and behaviour stay the same, and every task fixture keeps its snapshot.

### Panic output

The interpreter's first line is `thread 'main' panicked at <file>.knv:<line>:<column>:`, and the message follows on the next line, as Rust prints it.

- The location is the innermost call in a user's module on the interpreter's stack, so a panic inside a native or a prelude function names the user's call that reached it.
- A panic while a global's value is computed names the global's definition.
- Exit code 101.

Decision 7 chooses whether the compiled program prints the same line.

### Footprint targets

Each target is for inputs without crates, measured with `cargo xtask footprint` and the method above.

| Measure | Measured | Target | Prototype | Today |
|---|---|---|---|---|
| whole check-and-test cycle on `warehouse` with its tests, CPU | at a batch of 10 | at most 0.25 s, and never more than today's path on the same cycle | not yet runnable | 1.40 s |
| the same cycle, largest process | at a batch of 10 | 32 MB | 16 to 17 MB on the probe cycles | 230 MB |
| PSS per session | at a batch of 10 | 12 MB | 9 MB median | 60 MB median |
| PSS of 30 sessions together | 30 at once | 400 MB | 65 MB on the probe cycle | about 1.3 GB on the probe cycle, extrapolated |
| disk per worktree beyond the sources, `run`, `test`, `eval` and `task` included | after a cycle | 0 | 0 | about 1 MB per compiled entry and per compiled task |
| start-up: `hello` from start to exit, CPU, `interpret` against `knave run` | alone | 25 ms | 4.1 ms | 0.27 s |
| work ratio, summed over a fixed set of compute-bound programs: `fib`, `loop_sum` and `vectors`, then programs for maps, iterators, traits and strings as they become runnable | alone | at most 30 | 277 (248 to 285 per program) | |
| private heap for prelude data per session | alone | 4 MB | 2.4 MB for all of `check` | 2.4 MB |

The work-ratio target is what keeps the whole-cycle target true for tests with a few milliseconds of work. At 30, the break-even point moves from about 0.8 to 1.1 ms of compiled work to about 8 to 9 ms. A sum over the run fixtures would weigh start-up instead, because most of them print a few lines, so start-up has its own row.

### The agreement-test harness

The generated programs that check rustc against `check` (RFC 0065, "Testing the claim") and the agreement tests are one harness. The interface is fixed here, so that the work on RFC 0065's checks builds it once and the interpreter adds one backend. It lives in a crate of its own, `knave-agree`, under `xtask/agree/`. `xtask` runs it for generated programs, and the stage tests in `tests/` use it for fixtures.

```rust
pub trait ProgramSource {
    fn programs(&self) -> Box<dyn Iterator<Item = SubjectProgram> + '_>;
}

pub struct SubjectProgram {
    pub name: String,
    pub seed: Option<Seed>,
    pub files: Vec<(PathBuf, String)>,
    pub entry: PathBuf,
    pub arguments: Vec<String>,
    pub stdin: Vec<u8>,
    pub forms: FormCensus,
}

pub struct FormCensus {
    pub kernel_forms: BTreeMap<String, usize>,
    pub prelude_names: BTreeMap<String, usize>,
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;
    fn verdict(&self, program: &SubjectProgram, limit: Duration) -> Verdict;
}

pub enum Verdict {
    Accepted,
    Rejected(Vec<Rejection>),
    Ran(RunOutcome),
    Unsupported(String),
}

pub enum RejectingTool {
    Knave,
    Rustc,
}

pub struct Rejection {
    pub tool: RejectingTool,
    pub code: Option<DiagnosticCode>,
    pub file: PathBuf,
    pub span: Range<usize>,
    pub function: Option<String>,
    pub reached_from_main: bool,
}

pub struct RunOutcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit: ExitKind,
}

pub enum ExitKind {
    Code(i32),
    Panic(String),
    StackOverflow,
    Signal(i32),
    TimedOut,
}

pub struct BackendVerdicts {
    pub program: SubjectProgram,
    pub verdicts: BTreeMap<&'static str, Verdict>,
}

pub struct Finding {
    pub oracle: &'static str,
    pub program: SubjectProgram,
    pub verdicts: BackendVerdicts,
    pub summary: String,
}

pub trait Oracle: Send + Sync {
    fn name(&self) -> &'static str;
    fn judge(&self, verdicts: &BackendVerdicts) -> Option<Finding>;
}

pub trait Reducer {
    fn shrink(&self, program: SubjectProgram, oracle: &dyn Oracle, backends: &[&dyn Backend]) -> SubjectProgram;
}
```

- **Sources.** The generator of RFC 0065 implements `ProgramSource` over the range of seeds it is given, and a `Seed` names its program, such as `seed-3`. A reader of fixtures implements it over a fixed list of paths, so every run fixture and every corpus entry goes through the same oracles. `FormCensus` records which forms a program uses, so a report can say which forms the claim covers.
- **Backends.**
  - `Checker` runs `check` and returns `Accepted` or `Rejected`.
  - `CheckerWithoutOwnership` emits with the hidden flag that skips the ownership checks.
  - `CompiledDebug` emits one file and runs `rustc` on it directly, with the debug profile's overflow checks, in a temporary directory that it deletes, never in the worktree and never through Cargo.
  - `Interpreted` arrives with the interpreter.
  - Each backend runs under the time limit it is given. Because of overflow, `CompiledDebug` is the only compiled backend that agreement compares, as RFC 0065 decided.
- **Normalizing.** A backend returns `ExitKind::Panic` with the panic message alone, without its first line. It removes the same first line from `stderr`, along with the compiled program's `note: run with \`RUST_BACKTRACE=1\` environment variable to display a backtrace` line and the interpreter's line that prints the `hash-map` seed, so `stderr` holds only what the program wrote. A stack overflow is `StackOverflow` on both paths. The generator never lets `hash-map` order reach output.
- **Oracles.**
  - `CheckAcceptsRustcRejects`: `Checker` returns `Accepted` and `CompiledDebug` fails to compile.
  - `FalseOwnershipError`: `Checker` rejects with an ownership code and `CheckerWithoutOwnership` compiles.
  - `BackendsDisagree`: `CompiledDebug` and `Interpreted` both ran and differ in `stdout`, in `exit`, or in the normalized `stderr`.
  - A `TimedOut` or `Unsupported` verdict in any backend is never a finding. It is counted, so the census shows what the interpreter still lacks.
  - The rustc oracles compare only functions that `main` reaches, read from each `Rejection`'s `function`. A `Rejection` names its function by its Knave name, whichever tool rejected it, and records that tool, so a Knave error without a code never counts as rustc's. Whether `main` reaches the function comes from the expanded program, where the expander has resolved every name, so a local that shares a function's name is not a call.
- **Reducing.** The reducer keeps a candidate only while the same oracle finds the same codes at the same functions.
- **Runs.**
  - `cargo xtask agree --seeds <range> --oracle <name>...` runs programs in parallel under a job limit, writes each finding with its seed and its reduced program, and exits nonzero on a finding.
  - Pull requests run a fixed set of seeds, and a scheduled job runs fresh ones.
  - `--test stages run_fixtures` asks the same backends, so every run fixture is compared on both paths once the interpreter exists.

### Order of work, as pull requests

The interpreter trusts `check`, so it comes after the RFC 0065 work in that RFC's order: the harness, the loan rule, regions, and every reference layer in the typed IR. Until then, every open report of emitted Rust that rustc rejects is also a program that runs interpreted and fails compiled, as `move_local` does.

| # | Pull request | Touches |
|---|---|---|
| 1 | The harness above with `Checker`, `CheckerWithoutOwnership` and `CompiledDebug`, as RFC 0065's first step | `xtask/agree/`, `xtask/`, `tests/support/` |
| 2 | The type stage records the selected impl at each trait call and the capture mode of each closure; the emitter reads the capture mode. No output changes | `src/types/`, `src/emit/` |
| 3 | The native clause of decision 1: the `rust` kernel form's lowering, the core `RustTemplate`, the language reference's kernel forms, and the clause written on every prelude `rust` form and every template a prelude macro builds. It merges after Expander pull request 10, which only moves the lowering into `RustTemplates`; this pull request adds the clause there | `expand/src/expand/`, `expand/src/core/`, `prelude/`, `docs/language.md` |
| 4 | `src/run/` with the value and place model, locals resolved to slots and natives to function pointers before a run, the kernel forms, the format-string native and the `Interpreted` backend. A new stage in `DesignRules::STAGE_ORDER`, and a hidden `interpret` command. Run fixtures that use only these forms compare both paths, which with the format native covers most of them, since most run fixtures print with `println`, `print` or `str` | `src/run/`, `src/lib.rs`, `src/cli/`, `xtask/src/design.rs` |
| 5 | Natives for `prelude/number`, `text`, `vec`, `option`, `result` and `pair` | `src/run/` |
| 6 | Traits through the recorded impls, and natives that call closures, with lazy iterators | `src/run/` |
| 7 | Maps: `hash-map` with the per-run seed, `sorted-map`, `assoc-at`, `map-at` and `set-at` | `src/run/` |
| 8 | The program's edges: standard input, arguments, exit codes, `exit`, signals, panics as specified above, stack depth, and the runner records of `prelude/test` | `src/run/`, `src/cli/` |
| 9 | `cargo xtask footprint`, with its CI job for CPU time and peak memory | `xtask/` |
| 10 | `knave test`, `run`, `eval` and `task` interpret by default, with `--compiled`; inputs that reach a crate or a user's template take the compile path with a note. A task then writes nothing under `target/knave/<project>/tasks/`. It merges only when the footprint targets are met | `src/cli/`, `docs/language.md`, `tests/cli.rs` |
| 11 | The prompt replays its buffer through the interpreter (decision 5) | `src/cli/` |
| 12 | A `MacroRuntime` trait in the expander, implemented in the new crate of the type, ownership and interpreter stages, and, by decision 2, the type, ownership and interpreter stages in a crate that `build.rs` can use, with the build-cost measure that the section on build cost recommends. No behaviour changes | `expand/src/expand/`, `src/`, new crate, `build.rs`, `Cargo.toml`, `xtask/src/design.rs` |
| 13 | The `code` module gets its types, and the prelude macro code is rewritten to check: about 50 to 65 lines, and the record of closures in `internal/syntax.knv` by decision 9. The dynamic interpreter still runs macros, and a test checks every prelude macro group | `expand/src/core/`, `src/types/`, `prelude/` |
| 14 | Macro groups are checked in dependency order and run in the interpreter. The step costs are restated, and `build.rs` writes the checked prelude macro groups to their own embedded file. The dynamic interpreter and `CompileTimeCheck` are deleted, and its limits move | `expand/src/`, `src/run/`, `src/cli/`, `build.rs`, `docs/language.md`, the snapshots listed above |
| 15 | `definition-type`; `run-task` moves to it, and the first check of the task tree is deleted | `prelude/task.knv`, `src/cli/task_*.rs`, `docs/language.md`, `rfcs/0066-project-tasks.md` |
| 16 | Compile-time built-ins that duplicate a prelude function are deleted, 26 of the 75 in `expand/src/core/builtin.rs`, in four parts, and their macro callers move to the prelude functions. The plan follows the table | `expand/src/core/builtin.rs`, the `code` natives in `src/run/`, `prelude/`, `docs/language.md` (the `code` module) |

**How pull request 16 bridges `form` and prelude values.** The prelude functions do not take forms, so the bridge is built first:

- **Traits on `form`.** `form` implements `partial-eq`, whose native is today's recursive comparison of `value-eq` in Rust. It also implements the prelude's `sequence` trait for a list form, so `len` and `empty?` work on it.
- **Conversions.** Six new compile-time built-ins stay, because only compile time has forms. `form-integer`, `form-text` and `form-items` fail the macro on any other kind of form, with the message that `first` gives today, and `form-integer` also fails on a negative number or one too large for `usize`.

  | Built-in | Type | Converts |
  |---|---|---|
  | `form-integer` | `(fn (form) usize)` | a number form to a count or an index |
  | `integer-form` | `(fn (usize) form)` | a count or an index to a number form |
  | `form-text` | `(fn (form) string)` | a string form to a `string` |
  | `text-form` | `(fn ((ref str)) form)` | a string to a string form |
  | `form-items` | `(fn (form) (vec form))` | a list form to a vector of its forms |
  | `items-form` | `(fn ((ref (slice form))) form)` | a vector of forms to a list form |

- **Lists that are not lists.** `len`, `empty?` and `form-items` on a form that is not a list fail the macro as `first` does today. A macro that takes a bare symbol apart still fails, so the rule for a macro's name in value position keeps its error (`docs/language.md`, Macros). `=` never fails.
- **Counts and indexes.** Macro code keeps counts and indexes as `usize`, as `len` returns them and `at` takes them. A number written in source becomes a `usize` with `form-integer`, and a count becomes a number form with `integer-form`, so `(number-add (string-length name) 2)` becomes `(integer-form (+ (char-count (form-text name)) 2))`. Subtracting past zero panics, which is a macro failure at the call; no prelude caller needs a negative count today.

**The deleted built-ins.** The number of uses is the count of call sites in the prelude, so it gives the size of each part's rewrite:

| Part | Built-in | Uses | Replacement | Exists today? |
|---|---|---|---|---|
| 16a, fits as is | `bool-not` | 24 | `not` | yes |
| 16a | `value-eq` | 133 | `=`, through `partial-eq` on `form` | `=` yes; the impl is new |
| 16a | `empty?` | 274 | `empty?`, through `sequence` on `form` | `empty?` yes; the impl is new |
| 16b, numbers | `number-add`, `number-sub` | 60 | `+`, `-` on `usize` or `i64` | yes |
| 16b | `number-lt`, `number-le`, `number-gt`, `number-ge` | 27 | `<`, `<=`, `>`, `>=` | yes |
| 16b | `len` | 79 | `len`, through `sequence` on `form`, returning `usize` | yes |
| 16b | `nth` | 226 | `at` on `(form-items form)` | yes |
| 16c, text | `string` | 147 | `str`, then `text-form` where a form is needed | yes |
| 16c | `string-length` | 16 | `char-count` on `form-text`, which counts characters as `string-length` does; prelude `len` counts bytes | no: 16c adds `char-count`, `{0}.chars().count()` |
| 16c | `string-lines` | 9 | `split` with `"\n"` on `form-text`, then `text-form` and `items-form`; like `string-lines` it keeps a final empty line and any `\r`, where prelude `lines` drops them | yes |
| 16c | `string-lowercase` | 4 | `lower-case` | yes |
| 16c | `string-contains?`, `string-drop`, `string-strip-prefix`, `string-strip-suffix`, `string-trim-end`, `string-trim-start` | 55 | `contains?`, `drop-chars`, `strip-prefix`, `strip-suffix`, `trim-end`, `trim-start` on `str` | no: 16c adds each to `prelude/text` with a native and a Rust template, so programs get them too |
| 16d, lists | `first` | 411 | `at` with index 0 on `(form-items form)` | yes |
| 16d | `rest`, `take`, `drop` | 283 | `skip` and `take` on `(iter (form-items form))`, then `collect` and `items-form` | `take` and `collect` yes; `skip` no, 16d adds it |
| 16d | `concat` | 105 | `concat` on vectors of forms, then `items-form` | no: 16d adds `concat` for vectors |

A panic in macro code, such as `at` past the end of a list, is a macro failure at the macro call, with the panic's message and the macro's name, as `fail` reports today. The counts include a few uses in program code, so they are upper bounds. 16d is the largest rewrite and lands one prelude file per pull request.

**RFC 0069's lint rules.** RFC 0069 matches lint rules with `first`, `rest` and `value-eq` and lists the number, string and list built-ins among its tools. If `prelude/lint` exists when a part of pull request 16 is built, that part moves the lint rules to the prelude functions in the same pull request, so no built-in stays for the lint alone. If RFC 0069's build comes later, it is written against the prelude functions and the conversions above. `value-eq`'s recursive comparison in Rust survives as the `partial-eq` native on `form`, so comparing whole subtrees keeps its cost. Both RFCs keep the interpreter's `Host`, as RFC 0069 expects.

**The built-ins that stay, because they only make sense at compile time.** Each reads or changes the expander's state, or builds a form, a symbol or a scope. With the six conversions, 55 built-ins remain:

- Forms: `list`, `prepend`, `symbol`, `symbol-name`, `symbol?`, `list?`, `string?`, `number?`, `keyword?`, `keyword-field`, `keyword-name`, `keyword-trim-end`, `field-keyword` and `pattern-head?`.
- Locations and scopes: `located`, `form-location`, `pinned-at`, `introducing-macro` and `same-binding?`.
- Expansion: `expand-call`, `read-form`, `require-module`, `fail`, `warn` and `set-macro-step-limit`.
- Modules and definitions: `current-module`, `module-definitions`, `module-exports`, `module-symbol`, `module-notes`, `add-module-note`, `program-modules`, `project-modules`, `project-file-name`, `in-tasks-directory?`, `declare-definition`, `declare-macro-definition`, `declare-type-definition`, `definition-call`, `definition-symbol`, `type-definition-call` and `type-definition-symbol`.
- Diagnostics and Rust text: `closest-name`, `sorted-names`, `distinct-leading-lines`, `escape-char`, `escape-format`, `escape-string` and `language-name`.
- Conversions: `form-integer`, `integer-form`, `form-text`, `text-form`, `form-items` and `items-form`.

`closest-name`, `sorted-names` and the `escape-` built-ins could become prelude functions later. They stay here because only diagnostics and emitted Rust text use them.

## Decisions for the project owner

1. **How a `rust` form names its native.**
   - A: the `rust` kernel form takes a `(native name)` clause. The prelude writes it on each definition, and each macro that builds a template writes it on the form it builds. The format natives read the format string from the template, because Rust's format syntax is a fixed language.
   - B: the interpreter recognizes template texts and a fixed set of template shapes, as the prototype does. Nothing in the prelude changes.
   - C: prelude macros stop building templates, and `println` expands to calls of named definitions. This changes the emitted Rust.
   - Recommendation: A. The prelude states each fact once and the interpreter reads it. B is less work now, but the interpreter then depends on how the prelude spells each template, and every prelude change can break it quietly. C writes worse Rust for every program to make the interpreter easier.
2. **Where the prelude's macro groups are checked.** Every option has the expander define a `MacroRuntime` trait, implemented in the crate of the type, ownership and interpreter stages, which keeps stage order. They differ in where the checked prelude macro code comes from. The checker alone, without reading and expansion, runs at 27,000 to 71,000 lines a second on the corpus (`results/r-check-only.json`: `call_chain`'s 2,000 lines in 0.073 s, `generic_types`' 1,801 lines in 0.025 s). The prelude macro code is 5,648 lines, and a program with tests reaches about 3,000 of them.
   - A: build time. The type, ownership and interpreter stages move into a crate that `build.rs` uses, and the saved prelude holds checked macro code. No session pays. The compiler's own builds pay: `src/types` and `src/ownership`, 34,265 lines, and the interpreter become build dependencies of `build.rs`. Today an incremental `--profile local` rebuild after an edit in `src/types` takes 173 s of CPU. Under A it also rebuilds those crates for the build script wherever the build script's profile differs from the target's, and in `release` it always does, because fat LTO cannot match a build override. It also reruns `build.rs`, which takes 0.38 s today.
   - B: each session checks the groups it reaches, at first use in each command: about 0.04 to 0.11 s for 3,000 lines and 0.08 to 0.21 s for all of them. That is 1 to 2.5 times a whole `knave check` of `warehouse` (0.043 s), spent on every command in every session, against a 0.25 s cycle target.
   - C: the user cache. The first command after a `knave` install checks the prelude's macro groups, which takes 0.08 to 0.21 s once, and writes the result under the user cache, keyed by the version and commit identifier embedded in the binary at build time, beside the shared Cargo target and the git store. The file is estimated at 1 to 2 MB: macro code is 53% of the prelude's 10,729 lines, the embedded state for the whole prelude is 1.1 MB, and the types on each node may double or triple that share. Later commands in every session and worktree map the file read-only, so its pages are shared across processes as the embedded prelude's are, and decode only the groups the command reaches into the heap. That decoded part is private and counts against the 4 MB private heap target: a program with tests reaches about 3,000 lines, an estimated 0.5 to 1 MB. Nothing is written per worktree, and no build-script dependency is added. It costs a cache file that the CLI must write atomically, because 30 sessions may race to write it first, and must ignore when it is unreadable, falling back to B.
   - D: keep the dynamic macro interpreter, and only type-check macro code.
   - Recommendation: C. It costs a session nothing after the first command on a machine and costs the compiler's builds nothing, where A adds the second compile of 34,265 lines to the compiler developers' builds and B adds up to 2.5 checks of `warehouse` to every command. Take A instead if the owner prefers no cache file in the compiler. D keeps two value models and loses decision 8.
3. **A user's own `rust` template.** Crate calls are settled by RFC 0065.
   - A: the input takes the compile path, with a note that names the template.
   - B: the command fails and suggests `build`.
   - Recommendation: A. The program works under every command, and a template error stays a mapped rustc error either way.
4. **Programs that run long.** The interpreter costs more CPU than compiling once a program's compiled work passes about 0.8 to 1.1 ms at the prototype's speed, and about 8 to 9 ms at the target work ratio.
   - A: `run` always interprets, and `knave run --compiled <file>` compiles, a flag that pull request 10 adds to `run`, `test`, `eval` and `task`. For a task it goes before the task's name, `knave task --compiled <name>`, because every word after the name belongs to the task.
   - B: `run` interprets and switches to compiling after a fixed amount of interpreted work, such as one second of CPU.
   - Recommendation: A. The same command does the same thing every time, and an agent that knows its program is heavy asks for `--compiled`. B saves that knowledge but makes the output's timing and the command's memory depend on a threshold.
5. **What the prompt keeps between inputs.**
   - A: live values, where a definition's value stays computed until it is redefined, as in babashka. This needs rules this RFC does not give: what happens to the values of dependents, to closures that captured an old definition, and to values of a struct type that is redefined.
   - B: the buffer of definitions, replayed for each expression as [RFC 0051](0051-repl-redefinition.md) says, now through the interpreter instead of rustc.
   - Recommendation: B now, and A in its own RFC. B keeps the prompt's rules and costs milliseconds per input instead of a rustc run.
6. **`knave test` when some tests reach a crate or a user's template.**
   - A: the whole test program takes the compile path.
   - B: the tests that reach one compile into one program, and the rest are interpreted, in one report in the usual order.
   - Recommendation: A first. It is one rule with one output, and B can follow once crate projects with tests show the cost.
7. **Panic output of the compiled program.**
   - A: the compiled program also prints `thread 'main' panicked at <file>.knv:<line>:<column>:`, through a panic hook, the separate design RFC 0065 names.
   - B: the compiled program keeps Rust's first line, and the harness compares only the message and the exit code.
   - Recommendation: B now, and A later. The harness works with B, and A is better for users but needs its own design of the hook.
8. **What macro code may do.**
   - A: macro code may use every prelude function that has a native, and structs, maps, `match` and `try`. The language reference drops "macro code cannot run `+`, `<` or any other Rust template".
   - B: macro code keeps today's limits, now type-checked.
   - Recommendation: A. The limit exists only because the dynamic interpreter has no natives.
9. **The record of closures in `prelude/internal/syntax.knv`.** It is applied at several types, which needs rank-2 types.
   - A: rewrite it as separate helper functions, one per use, at its 13 use sites.
   - B: add rank-2 types for compile-time groups only.
   - Recommendation: A. It is one prelude file. B adds a type-system feature for one prelude helper, and only for macro code, which breaks the rule that macro code and program code check alike.
10. **What `definition-type` returns.**
    - A: the signature text, as `infer` writes it today.
    - B: a structured type value of the `code` module.
    - Recommendation: A first. `run-task` already reads that text, and B can follow when a second user needs it.

## Decision

The project owner decided each decision on 2026-11-12:

1. **A.** A `(native name)` clause on the `rust` form names its native. The clause is allowed only in prelude modules; elsewhere it is an error.
2. **A.** The prelude's macro code is checked at build time and saved in the binary. No cache file: `knave` stays fast without one. The section on build cost measures how to keep the compiler's own build cost down.
3. **A.** A program that reaches a user's `rust` template compiles for that command, with a note. The recommended practice is to isolate user Rust in a Knave library, which RFC 0064 later compiles once per library commit into the shared store.
4. **A.** `--compiled` asks for the compile path.
5. **B now.** The prompt replays its buffer through the interpreter. Live values get their own later RFC.
6. **A.** A test program that reaches a crate or a user's template compiles as a whole.
7. **B now.** The compiled program keeps Rust's panic line. A panic hook that prints the Knave location is its own later design.
8. **A.** Macro code may use every prelude function that has a native, with structs, maps, `match` and `try`. Pull request 16 then deletes the compile-time built-ins that duplicate prelude functions.
9. **A.** The record of closures in `prelude/internal/syntax.knv` is rewritten as separate helpers at its 13 use sites.
10. **A.** `definition-type` returns the signature text. A structured type value is left for later.

## Rejected options and why

- **B, keep compiling, as today:** rustc stays in every cycle, at 1.3 s of CPU and 150 to 230 MB per run on the sample, and 30 sessions need about 1.3 to 1.8 GB, extrapolated. Crates already share one target directory, so B has no disk gain left to offer.
- **C, a typed interpreter for programs only:** two interpreters with two value models, untyped macro code, and the `deftask` double check stays.
- **Decision 2 B, checking the prelude's macro groups in each session:** 0.04 to 0.21 s on every command in every session, against the main goal.
- **Decision 2 C, a user-cache file of checked macro code:** the project keeps no cache of the compiler's own work on a user's machine; work that can run once when `knave` is built runs there.
- **Decision 2 D, the dynamic macro interpreter kept:** two value models, and macro code could not use the prelude.
- **A faster code generator, such as Cranelift, behind `run`:** it still builds and links a binary per cycle, writes it to disk, and needs a second backend's agreement with LLVM's.

## Rule

[docs/language.md: Evaluation](../docs/language.md#evaluation), to be written with the build.
