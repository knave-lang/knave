# RFC 0065: No failure reaches rustc

**Status:** Proposed
**Date:** 2026-11-09

## Problem

The project owner ruled that no failure reaches rustc before 1.0. Every program the checker accepts compiles, and every rejection is a Knave diagnostic at the user's source. Today [docs/language.md: Ownership](../docs/language.md#ownership) says the opposite: "Any lifetime or aliasing error that slips past these checks comes from rustc, mapped back to the source and the user's names." [RFC 0060](0060-set-infers-mut.md) accepted rustc's `E0499` for two `mut` arguments of one collection on the same terms.

The program interpreter depends on the rule. It walks the typed, ownership-checked IR and trusts `check`, so a program that rustc rejects would run when interpreted and fail when compiled. An agent that tests with the interpreter would ship a program that CI cannot build.

The probes below ran on a release build of `origin/main` at `021f9eaeb`, with rustc 1.95.0 on Linux.

### What the probes found

**The gap is wide, and the simplest shape falls in it.** The ownership stage tracks moves of names. It does not track loans. A borrow held in a local is invisible to it:

```lisp
(defn main ()
  (local v (vec 1 2 3))
  (local r (at v 0))
  (push! v 4)
  (println r))
```

```rust
let mut v: Vec<i64> = vec![1_i64, 2_i64, 3_i64];
let r = &v[0];
v.push(4);
println!("{}", r)
```

`knave check` accepts it, and rustc reports `E0502` at `(push! v 4)`.

**A generator finds this class at once.** A generator of straight-line `main` bodies over one struct, vectors, borrowing helpers such as `(defn first-make (cs) (:make (at cs 0)))`, `mut` helpers such as `(defn grow! (cs c) (push! cs c))`, and closures that capture a vector, produced 1,000 programs of 14 statements each:

| Outcome | Programs |
|---|---|
| `check` rejects (`L0501`) | 349 |
| `check` accepts, rustc compiles | 490 |
| `check` accepts, rustc rejects (`E0502` 156, `E0505` 5) | 161 |

So 24.7% of the accepted programs fail in rustc. Each program takes 23 ms to check and about 300 ms to check, emit and compile, so the 1,000 ran in 107 seconds on four cores.

**The open bug reports fall into a few classes.** 51 open reports mention rustc or an `E0xxx` code. 38 still fail on `main` in at least one sub-case, 6 are fixed, and 7 are not this kind of bug: a lint, two false errors, wording, and three plans. The 38 group as follows:

| Class | Reports | rustc codes | What the front end lacks |
|---|---|---|---|
| Aliasing | 2, and every generator failure | `E0499`, `E0502`, `E0505` | Loans. A borrow held in a local, a field or a closure is not tracked, so a change or a move of its place while the borrow lives passes. |
| Escaping borrows | 3 | `E0515` | `L0510` follows a returned borrow through some forms only: not through a struct bound by `local`, a struct in a returned `vec`, `assoc`, or a call whose result shares the argument's lifetime. Its promotion rule is wider than rustc's, so `(ref (/ 7 base))` with `(def base 0)` passes. |
| Closure signatures | 9 | `E0597`, `E0308`, `E0373`, and the uncoded "lifetime may not live long enough" | A closure bound by `local` cannot declare a lifetime, so the emitter gives a borrowed parameter `'static` or leaves it elided. `L0512` guards the shapes it traces, and misses a borrow carried in a struct, through a `match` binding, through a field, or from the enclosing function's own reference parameter. |
| Signature lifetimes | 9 | `E0106`, `E0195`, `E0496`, `E0621`, and the uncoded message | The emitter places every lifetime by rules that no stage checks against the body. Each report is a case a rule does not cover: a call cycle, a trait method, a by-value `self`, two swapped parameters, a closure's `for<'a>` that shadows the function's `'a`. |
| Closure kinds | 6 | `E0276`, `E0308` | `Type::Function` does not tell a `fn` pointer from a closure, so a capturing closure meets a `fn` pointer field, binding or nested type. A trait's default body and its impls infer a function parameter's kind separately. |
| Reference depth | 5 | `E0277`, `E0308` | The type stage's type of a value holds fewer `&` layers than the emitted Rust: a `match` binder matched through a reference, a trait method held in a local, a `rust` template over `(ref (ref i64))`, a field compared with a `(ref (ref str))`. |
| Trait bounds | 2 | `E0277`, `E0046` | A runtime trait without methods, such as `signed`, has no impl row to check, and a `Sized` requirement through a chain of generic functions is not checked. |
| One-off | 4 | `E0507`, `E0308`, a parse error, a span at `1:1` | The def-move check and the emitter's `const` rule disagree. A field passed to a `mut` parameter does not make its root parameter `mut`. The emitter omits a `;` before a `move` closure. A rustc span maps to `1:1`. |

Lifetimes and aliasing hold 23 of the 38 and every generator failure. The other classes are places where one stage decides a fact and a later stage, or rustc, decides it again differently.

**A loan check in Knave matches rustc on the generated programs.** A loan model of 90 lines over the generated statements gives each local that holds a borrow the set of locals it borrows from, through helpers, `longer`, and captures. A borrow lives from its `local` to its last use, and a `mut` use or a move of a borrowed local while one of its borrows lives is a conflict. On the 651 accepted programs the model flags all 161 that rustc rejects and none of the 490 that rustc compiles. The model is the rule in Option A below, cut down to straight-line code.

## Options

Every option keeps the decided scope below: the rule covers programs built from Knave code and prelude forms.

### A: Check lifetimes and aliasing in Knave

The ownership stage gains two facts it does not hold today, and every later stage reads them instead of deciding again.

1. **Loans.** Every `ref`, every adjustment that adds a borrow, every field read that borrows, every call whose result borrows an argument, and every closure that borrows a capture makes a loan of a place. A loan lives from where it is made to the last use of any value that holds it: a local, a field, a struct, a vector, a closure, or a result. While a loan lives, its place cannot be changed, moved, or lent as `mut`, and a `mut` loan excludes every other loan of its place. A loan of a local cannot live past the local's scope. This is Rust's non-lexical rule, stated on Knave's typed IR.
2. **Regions.** The ownership stage, not the emitter, decides which parameters each result and each stored value may borrow from, for every function, method and closure, through call cycles by a fixed point. The emitter writes the lifetimes this record names and decides none. A trait's record is the meet of its default body and every impl, as closure kinds and slices already are. A closure bound by `local` whose result borrows a parameter emits through the helper the owner chose for it on 2026-11-06, so its signature can name the tie:

```lisp
(defstruct car :make string :year i64)

(defn main ()
  (local leaf (->car :make (str "Leaf") :year 2020))
  (local make-of (fn ((c (ref car))) (:make c)))
  (println (make-of (ref leaf))))
```

`main` emits `let make_of = |c: &'static Car| &c.make;` today, and rustc reports `E0597` at `(ref leaf)`. Under this option it emits:

```rust
fn tied<F>(f: F) -> F
where
    F: for<'a> Fn(&'a Car) -> &'a String,
{
    f
}

fn main() {
    let leaf = Car { make: "Leaf".to_string(), year: 2020 };
    let make_of = tied(|c: &Car| &c.make);
    println!("{}", make_of(&leaf))
}
```

A call site then checks the tie by rule 1, as it checks a `defn`'s.

The diagnostics are new ownership codes in the shape of `L0501`, with a label at the loan, the conflicting use, and the later use that keeps the loan alive:

```text
error[L0517]: `v` changes while `r` still borrows it
 --> src/main.knv:4:10
  |
3 |   (local r (at v 0))
  |            -------- `r` borrows `v` here
4 |   (push! v 4)
  |          ^ `v` changes here
5 |   (println r)
  |            - `r` is used here, after the change
  |
help: read what you need from `v` before changing it, or copy the element with `(clone (at v 0))`
```

`L0517` is a change or a `mut` loan of a place that a live loan borrows, and `L0518` is a move of such a place. `L0510` and `L0512` become cases of rule 1 and keep their codes. PR 421's `L0516`, two `mut` arguments of one place, is a case of `L0517` at one call and keeps its code.

**Programs it rejects that rustc accepts.** On the generated programs, none. In general, Knave's rule is rustc's rule on the Rust that Knave emits, so a difference is a bug in one of them. Two places stay narrower by choice: a borrow of a temporary is promoted to `'static` only when the temporary is a literal or a constructor of literals, where rustc also promotes some arithmetic over `const` items, and two-phase borrows are not modelled, so a call that lends a place as `mut` and reads it in a later argument hoists that argument, as the emitter already does for locals.

**Compile speed.** The loan rule is a liveness pass over each body, linear in its size times the number of places it borrows. rustc's own borrow check is a small share of rustc's time. The prototype adds nothing measurable to a 23 ms check, but it is Python over straight-line code, so the real cost needs a measurement on the corpus, against the budget the corpus entries already set: each entry's emission stays under one second.

### B: Restrict the language so the errors cannot arise

Each class gets a restriction instead of a check. On aliasing, the choices measured on the 490 generated programs that rustc compiles:

| Restriction | Accepted programs it rejects |
|---|---|
| No local holds a borrow: a borrow lives only inside the form that makes it | 460 (94%) |
| A local that holds a borrow freezes the places it borrows until its scope ends | 187 (38%) |

On closure signatures, a closure bound by `local` may not return a borrow of a parameter, and the program writes a `defn` instead. On closure kinds, every function type is a `fn` pointer, so no closure captures. On signature lifetimes, a function may return a borrow of one parameter only.

**Programs it rejects that rustc accepts.** The table shows the cost on aliasing alone: a third of ordinary programs at best. The closure-kind restriction removes every capturing callback, such as `(for-each cars (fn (c) (push! names (:make c))))`.

**Compile speed.** Faster than A: each restriction is a syntactic check.

### C: Ask rustc during `check`

`check` emits the Rust and runs rustc's analysis without code generation, then maps each error back to the source.

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.knv:4:3
```

**Programs it rejects that rustc accepts.** None, by construction.

**Compile speed.** About 270 ms of rustc per check on a 14-line program, against 23 ms for the front end, and the interpreter could not run without rustc. The errors keep rustc's words and Rust's names, and a mapped span lands on the form that emitted the Rust, not always on the form at fault.

## Recommendation

Take Option A for lifetimes and aliasing, and for every other class make one stage decide each fact and every later stage read it. The classes then close by these rules:

| Class | Rule |
|---|---|
| Aliasing | A loan lives from where it is made to the last use of any value that holds it. While it lives, its place does not change, move, or lend as `mut`. |
| Escaping borrows | A loan of a local does not outlive the local. A borrow of a temporary is promoted only when the temporary is a literal or a constructor of literals. |
| Closure signatures | A closure's result ties to the parameters it borrows by the same record as a `defn`'s, emitted through the owner's helper. |
| Signature lifetimes | The ownership stage decides every lifetime by a fixed point over the call graph, and the emitter writes only what it decided. |
| Closure kinds | The type stage tells a `fn` pointer from a closure. A capturing closure that meets a `fn` pointer is `L0511`, wherever it meets one. A trait method's function parameter takes one kind across its default body and every impl. |
| Reference depth | The typed IR holds every reference layer the emitted Rust has, with each adjustment as a node. The emitter writes no `&` and no `*` that the typed IR does not hold, and a debug assertion compares the two. |
| Trait bounds | Every bound the emitted Rust requires is one the type stage proved. A runtime trait without methods lists its implementors, and `Sized` is a bound the type stage tracks for type parameters. |
| One-off | The def-move check and the emitter ask one `const` rule. A field passed to a `mut` parameter is a `mut` use of its root. The others are emitter bugs that the generator finds. |

Option A rejects nothing on the generated programs that rustc accepts, where the cheapest restriction rejects 38%. It keeps the language's model, Rust's ownership with Knave's inference, and it gives the interpreter a front end that has decided every fact the Rust holds, which the interpreter needs to walk the typed IR without guessing.

### The scope: Knave code and prelude forms

The project owner decided the scope on 2026-11-09. The rule covers every program built from Knave code and prelude forms. An error inside a user's own Rust text, a `(rust ...)` template or a crate call, stays a mapped rustc error, because Knave cannot see inside it. A prelude `rust` form is covered: an error in it is a bug in the prelude.

How the interpreter runs a Rust leaf is not this RFC's subject. Two designs hold it: the interpreter design gives each prelude `rust` form a native implementation in the interpreter, and the compiled crate bindings design compiles the crate-backed forms of a program into a `cdylib` that the interpreter loads.

### Testing the claim

Fixtures cover only the shapes someone thought of. The generator above found the largest class with no thought at all. The harness:

1. **A generator of well-typed programs.** It builds programs typed by construction, from a seed, over a grammar of the language's forms: functions with inferred modes, structs, enums, `match`, vectors, maps, closures that capture, `set!`, `push!`, borrows held in locals and fields, calls through function values, traits with default bodies, and generic functions. Each program prints the values it computes.
2. **The oracle.** A program that `check` accepts and rustc rejects is a failure. A program that `check` rejects with an ownership code, and whose Rust, emitted without the ownership checks, rustc compiles, is a false error. A hidden `emit` flag skips the checks for the harness only.
3. **A reducer.** A failing program shrinks to the smallest program that still fails, by dropping and simplifying forms, and becomes a fixture under `tests/fixtures/`.
4. **Runs.** Each pull request runs a fixed set of seeds: 200 programs of today's generator took 15 seconds on four cores. A scheduled job runs fresh seeds and files what it finds.

The generator grows with the language: a form it cannot build is a form the claim does not cover yet, and the RFC that adds a form adds it to the generator.

### One harness for checks and agreement

The interpreter's agreement tests compare interpreted and compiled output. The harness above already compiles every program it generates, so it runs each compiled program and the interpreter on the same program and compares standard output, standard error and the exit code. Every run fixture and every test runs both ways as well. One harness then answers both questions: does rustc accept what `check` accepts, and does the interpreter compute what the compiled program computes.

The two backends could disagree on these behaviors. Each needs a language rule that both follow:

| Behavior | Rule |
|---|---|
| Evaluation order | Every form evaluates its parts once, left to right. A struct update evaluates its field values before its base when the base is a name or a field read. The emitter's hoisting of a `mut` argument keeps this order, as [Call order](../docs/language.md#ownership) states. |
| Integer overflow | Today a debug build panics and `build --release` wraps: `(+ x 1)` on `9223372036854775807` prints `-9223372036854775808` in a release build, because the release profile leaves overflow checks off. Proposed: overflow panics in every backend and profile, and a program that wants wrapping says so with a wrapping operation. |
| Division by zero, and `MIN / -1` | Panics in both. |
| Floats | `f32` arithmetic rounds to `f32` at every step, and a float prints as Rust's `Display` prints it. |
| `hash-map` order | Not fixed today: Rust seeds the hasher at random, so two compiled runs can differ. Proposed: both backends use one fixed-seed hasher, so iteration order is the same on every run and in both backends. Printing already sorts. |
| Drop order | A value drops at the end of its scope, in reverse order of binding, and a temporary at the end of its statement, as in Rust. Knave writes no destructors, so the order shows only through crate values and the runtime's buffered output, which both backends flush at exit. |
| Closure captures | A `move` closure copies a Copy capture when the closure is made, so a later change of the local does not reach it. The interpreter captures the same way. |
| Sharing | The interpreter may share values that Rust would copy or move, because the ownership checks prove the difference unobservable. That proof is this RFC: a program that rustc would reject for aliasing could show a change through a shared value when interpreted. |
| Panics and exit codes | A panic prints its message and exits with code 101 in both. A program's own `exit` code passes through. |
| Stack depth | Deep recursion that is not a tail call overflows at a depth that differs between backends. A stack overflow is a crash in both, and agreement tests compare only that both crash. |

### Order of work

Before the interpreter:

1. The harness, with today's generator grown to the forms above. It measures every later step.
2. The loan rule, rule 1 of Option A, which closes the aliasing class and the generator's failures and takes in `L0510` and `L0512`.
3. Regions in the ownership stage, which close the signature and closure-signature classes.
4. Reference depth in the typed IR, because the interpreter walks the typed IR and must see every layer the compiled program has.

These can follow, before 1.0: closure kinds, trait bounds, the one-off bugs, and the rules for overflow and `hash-map` order, which must be decided before the agreement tests run in CI.

## Decision

The project owner decided the scope on 2026-11-09: the rule covers every program built from Knave code and prelude forms, and an error in a user's own Rust text, a `(rust ...)` template or a crate call, stays a mapped rustc error. The rest awaits the owner's decision.

## Rejected options and why

- **B, restrict the language:** the cheapest aliasing restriction rejects 38% of generated programs that rustc compiles, and the closure-kind restriction removes capturing callbacks. It trades the language's model for the checker's convenience.
- **C, ask rustc during `check`:** it costs a rustc run per check, more than ten times the front end, keeps rustc in the interpreter's loop, and leaves the diagnostics in rustc's words.

## Rule

[docs/language.md: Ownership](../docs/language.md#ownership)
