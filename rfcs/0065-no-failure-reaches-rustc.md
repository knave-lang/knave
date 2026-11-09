# RFC 0065: No failure reaches rustc

**Status:** Proposed
**Date:** 2026-11-09

RFC numbers 0063 and 0064 are reserved for the program interpreter and for compiled crate bindings, whose designs are not yet in this folder.

## Problem

The project owner ruled that no failure reaches rustc before 1.0. Every program the checker accepts compiles, and every rejection is a Knave diagnostic at the user's source. Today [docs/language.md: Ownership](../docs/language.md#ownership) says the opposite: "Any lifetime or aliasing error that slips past these checks comes from rustc, mapped back to the source and the user's names." [RFC 0060](0060-set-infers-mut.md) accepted rustc's `E0499` for two `mut` arguments of one collection on the same terms.

The program interpreter depends on the rule. It walks the typed, ownership-checked IR and trusts `check`, so a program that rustc rejects would run when interpreted and fail when compiled. An agent that tests with the interpreter would ship a program that CI cannot build.

The probes below ran on a release build of `origin/main` at `021f9eaeb`, with rustc 1.95.0 on Linux.

### What the probes found

**The ownership stage does not track loans, and the simplest shape gets through.** The ownership stage tracks moves of names. A borrow held in a local is invisible to it:

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

**A first generator found this class at once.** It wrote straight-line `main` bodies over one struct, vectors, fixed borrowing helpers such as `(defn first-make (cs) (:make (at cs 0)))`, fixed `mut` helpers such as `(defn grow! (cs c) (push! cs c))`, and closures that capture a vector. It produced 1,000 programs of 14 statements each:

| Outcome | Programs |
|---|---|
| `check` rejects (`L0501`) | 349 |
| `check` accepts, rustc compiles | 490 |
| `check` accepts, rustc rejects (`E0502` 156, `E0505` 5) | 161 |

So 24.7% of the accepted programs of that grammar fail in rustc.

**The open bug reports fall into a few classes.** 51 open reports mention rustc or an `E0xxx` code. 38 still fail on `main` in at least one sub-case, 6 are fixed, and 7 are not this kind of bug: a lint, a false error, a readability item, wording, and three plans. The 38 group as follows. The column sums to 40, because reports 177 and 185 each fall in two classes.

| Class | Reports | rustc codes | What the front end lacks |
|---|---|---|---|
| Aliasing | 2, and every generator failure | `E0499`, `E0502`, `E0505`, `E0506` | Loans. A borrow held in a local, a field or a closure is not tracked, so a change or a move of its place while the borrow lives passes. |
| Escaping borrows | 3 | `E0515` | `L0510` follows a returned borrow through some forms only: not through a struct bound by `local`, a struct in a returned `vec`, `assoc`, or a call whose result shares the argument's lifetime. Its constant rule has holes: `(ref (/ 7 base))` with `(def base 0)` passes. Report 266 has 12 sub-cases, and 7 of them are user `rust` templates, which the decided scope below leaves to rustc. The other 5 count here. |
| Closure signatures | 9 | `E0597`, `E0308`, `E0373`, and the uncoded "lifetime may not live long enough" | A closure bound by `local` cannot declare a lifetime, so the emitter gives a borrowed parameter `'static` or leaves it elided. `L0512` guards the shapes it traces, and misses a borrow carried in a struct, through a `match` binding, through a field, or from the enclosing function's own reference parameter. |
| Signature lifetimes | 9 | `E0106`, `E0195`, `E0496`, `E0621`, and the uncoded message | The emitter places every lifetime by rules that no stage checks against the body. Each report is a case a rule does not cover: a call cycle, a trait method, a by-value `self`, two swapped parameters, a closure's `for<'a>` that shadows the function's `'a`. |
| Closure kinds | 6 | `E0276`, `E0308` | `Type::Function` does not tell a `fn` pointer from a closure, so a capturing closure meets a `fn` pointer field, binding or nested type. A trait's default body and its impls infer a function parameter's kind separately. |
| Reference depth | 5 | `E0277`, `E0308` | The type stage's type of a value holds fewer `&` layers than the emitted Rust: a `match` binder matched through a reference, a trait method held in a local, a `rust` template over `(ref (ref i64))`, a field compared with a `(ref (ref str))`. |
| Trait bounds | 2 | `E0277`, `E0046` | A runtime trait without methods, such as `signed`, has no impl row to check, and a `Sized` requirement through a chain of generic functions is not checked. |
| One-off | 4 | `E0507`, `E0308`, a parse error, a span at `1:1` | The def-move check and the emitter's `const` rule disagree. A field passed to a `mut` parameter does not make its root parameter `mut`. The emitter omits a `;` before a `move` closure. A rustc span maps to `1:1`. |

Lifetimes and aliasing hold 23 of the 38 and every generator failure. The other classes are places where one stage decides a fact and a later stage, or rustc, decides it again differently.

### What a loan model predicts

A second generator tests rule 1 of Option A below on a wider grammar. Each program has four generated helpers and a `main` body:

- **Helpers.** Their bodies are drawn from templates: a choice between two parameters' elements under `if`, a field of one parameter under a condition on another, a call of an earlier helper with its arguments swapped, a borrow held in a local and returned, a `mut` helper that returns an element after a `push!` on one branch, with or without a held borrow, a helper that stores a parameter's field into a vector parameter, a `loop` that returns an element, and `mut` helpers that hold a borrow across a `push!` inside an `if` or a `loop`, with the read before or after the `push!`.
- **`main`.** It binds structs, vectors and integers; borrows of fields and elements; borrows held in a struct field, `(defstruct tag :label (ref str))`, and in a vector; closures that capture a vector, shared or as `mut`; and results of the helpers. It reads, changes, moves and pushes through them, at the top level and inside `if` branches and `loop` bodies. A block, including a bare `(do ...)`, may bind a local and read it, and a later `local` may bind a name again to a new borrow, at the top level or inside a bare `do`. `main` also pushes an element of an integer vector onto the same vector, both read in place, `(push! v (at v 0))`, and through a local that holds the element's borrow, `(push! v x)`.

The model reads each program's syntax tree, the same tree the generator printed. It infers each helper's regions from its body: which parameters its result borrows, which parameters are `mut`, and which parameter's borrow the body stores into another. It does not read the generator's choices. Each `local` starts a new value, so a name bound again, or bound on each pass through a loop, holds only the loans that flow into it after that binding. It checks only the functions `main` reaches, because the emitter writes only those, so rustc sees only those. The generator and the model are under `target/scratch/gen2/` of the work tree that produced this RFC. The runs below are `run7` and `run8` there: `run7.txt` and `run8.txt` hold rustc's verdicts, `run7.model` and `run8.model` the model's, and `run7.<variant>` and `run8.<variant>` the variants'. `fixtures.py` runs the model on the programs review wrote to break it. `run1` to `run6` come from earlier versions of the generator.

| Run | Programs | `check` rejects (`L0501`) | Accepted, rustc compiles | Accepted, rustc rejects | Model agrees |
|---|---|---|---|---|---|
| Seeds 1 to 1,000, 14 statements | 1,000 | 256 | 461 | 283 | 744 of 744 |
| Seeds 1,001 to 2,000, 20 statements | 1,000 | 346 | 272 | 382 | 654 of 654 |

Over both runs, the rustc rejections carry `E0502` in 614 programs, `E0499` in 153, `E0506` in 52 and `E0505` in 11, and a program can carry several. In the first run, the model places 94 of the 283 rejections in a helper's body, and 65 involve a `mut` loan. Of that run's 744 accepted programs, 526 have a branch in `main`, 478 a loop, 527 a bare `do`, 577 bind a local inside a branch or a loop, 581 bind a name twice, 312 push a vector's own element read in place, 86 push a held element borrow onto its own vector, 204 hold a borrow in a struct field, 184 a borrow in a vector, 272 call a helper with a branch or a loop, and 224 hold a helper's returned borrow.

Seeds 1 to 40 of an earlier version of this generator were used to write and correct the model. Review then found that the model kept one value per name, which gave a false conflict on a local bound inside a loop and on a name bound twice, two forms that version never built. The model now starts a new value at each binding, and the generator builds both forms. Review's third round found two more forms the generator never built: `(push! v (at v 0))`, which compiles and which the model rejected, and a shadowing `local` inside a bare `do`, which rustc rejects and which the model accepted. The model now ends a copied read's loan at the read and scopes a bare `do`, the generator builds both forms. Review's fourth round found the same copy through a local, `(local x (at v 0)) (push! v x)`, which compiles and which the model rejected. The model now ends the loan at any copy out of a borrow, the generator pushes a held element borrow onto its own vector, and both runs above are new. Review ran three earlier versions on 6,000 fresh seeds and this version on 2,000 more, and the model agreed on all 5,544 accepted programs.

Five variants of the model measure the rules that review questioned:

| Variant | Accepted programs it gets wrong, first run | Second run |
|---|---|---|
| A name holds one value for the whole body, with no new value at a binding | 154 that rustc compiles | 102 that rustc compiles |
| A place is a whole local, not a path | 11 that rustc compiles, each a change of one field while another field is borrowed | 13 that rustc compiles |
| A loan lives to the last use of a value that holds it, with no rule for the function's result | 26 that rustc rejects | 23 that rustc rejects |
| A call's `mut` loan starts before its other arguments are read, as in `(push! v (len v))` | 291 that rustc compiles | 205 that rustc compiles |
| A copy out of a borrow keeps its loan, as in `(push! v (at v 0))` or `(push! v x)` | 149 that rustc compiles | 128 that rustc compiles |

**What these runs do not show.** No program has a `match`, an enum, a map, a recursive function, a call cycle, a trait, a generic function, a closure with parameters, a closure that returns a borrow, a crate call or a `rust` template. No program reassigns a whole struct or vector with `set!` while a field of it is borrowed, binds a local to another local that holds a borrow, has more than four helpers, or passes one place to a helper in two positions. Only 9 of the 744 call one helper from another, so the region rule through calls has little evidence, and the fixed point over cycles has none. `E0597`, `E0515` and `E0716` never arise, because the grammar never lets a borrow outlive its local. The share of programs that fail measures these grammars, not the programs people write.

## Options

Every option keeps the decided scope below: the rule covers programs built from Knave code and prelude forms.

### A: Check lifetimes and aliasing in Knave

The ownership stage gains two facts it does not hold today, and every later stage reads them instead of deciding again.

1. **Loans.**
   - **Places.** A place is a path: a local, then fields, dereferences and indexes. Two places conflict when one is a prefix of the other. An index lends the whole collection, so `(at v 0)` lends `v`. A call whose result borrows an argument lends each whole argument that its region record names. So `(local m (:make c))` followed by `(set! (:year c) 2)` is allowed, as rustc allows `let m = &c.make; c.year = 2;`.
   - **Making a loan.** Every `ref`, every adjustment that adds a borrow, every field read that borrows, every call whose result borrows an argument, and every closure that borrows a capture makes a loan of a place. Wherever the compiler copies a Copy value out of a borrow, at a field or element read or through a local that holds the borrow, the copy holds no loan, and the borrow is used only at that copy. On a vector of integers, `(push! v (at v 0))` emits `v.push(v[0])`, and `(local x (at v 0)) (push! v x)` emits `let x = &v[0]; v.push(*x);`, and both compile.
   - **Liveness.** A loan is live at a point when the point is reachable from the loan and some value whose region holds the loan is live there. Each binding starts a new value: a `local` inside a loop body is a new value on each pass, and a `local` that binds a name again is a different value from the one it shadows. A value's region is one set for the whole body, as in rustc's non-lexical lifetimes: a loan that flows into a value on any path after its binding is in that value's region at every point where the value is live. A loan that reaches the function's result is live at every point from the loan to every exit. A value whose type has a `Drop` impl uses its loans at the end of its scope. This is rustc's region liveness, not Polonius's, and Knave must not accept what rustc rejects, even where rustc is less precise than it could be. In this program `r` has no later use on the `else` path, but its loan reaches the result, so the `push!` conflicts and rustc reports `E0502`:

     ```lisp
     (defn pick! (v)
       (local r (at v 0))
       (if (> (deref r) 0) r (do (push! v 1) (at v 0))))
     ```

   - **Conflicts.** While a loan lives, its place cannot be changed, moved, or lent as `mut`, and a `mut` loan excludes every other loan of its place. A loan of a local cannot live past the local's scope.
   - **Call order.** A call's `mut` loan begins after its other arguments are evaluated, as [Call order](../docs/language.md#ownership) states. A loan that another argument holds is still live at the call, because the call receives it. Where an argument is a copy out of a borrow, the call receives the copy, which holds no loan. So `(push! v (len v))` is allowed, and `(relabel! c (:make c))` is a conflict. The emitter makes this order true in Rust: rustc's two-phase borrows give it for an autoref method call such as `v.push(v.len())`, and for every other call the emitter hoists the other arguments, as it does today for a user function's `mut` argument. A debug assertion checks that each emitted call with a `mut` argument is one of the two.
2. **Regions.** The ownership stage, not the emitter, decides which parameters each result and each stored value may borrow from, for every function, method and closure, through call cycles by a fixed point. The emitter writes the lifetimes this record names and decides none. A trait's record is the meet of its default body and every impl, as closure kinds and slices already are. A closure bound by `local` whose result borrows a parameter emits through the `tied` helper, which names the tie:

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
error[L0518]: `v` changes while `r` still borrows it
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

`L0518` is a change or a `mut` loan of a place that a live loan borrows, and `L0519` is a move of such a place. PR 421's unpushed work takes `L0517`, for a returned closure that changes a parameter. If PR 421 lands without it, `L0517` stays free and these codes leave a gap. `L0510` and `L0512` become cases of rule 1 and keep their codes. PR 421's `L0516`, two `mut` arguments of one place, is a case of `L0518` at one call and keeps its code.

**Programs it rejects that rustc accepts.** On the 1,398 accepted generated programs, none. Knave's rule is rustc's rule on the Rust that Knave emits, so a difference is a bug in one of them. A borrow of a temporary keeps today's constant rule, which [docs/language.md: Ownership](../docs/language.md#ownership) states and which promotes `(ref (+ 1 2))` and `(ref (pair "Leaf" 2019))` as rustc does. This RFC closes only the rule's known holes from report 266: a `def` resolves to its value before the rule reads it, a divisor of zero is not constant, a divisor of `-1` is constant only when the dividend is a constant other than its type's minimum, `&&` and `||` are never constant, `==` on operands that are not primitive is not constant, and a `ref` of a `do` or an `if` borrows a temporary even when each branch is a borrowed parameter. Each of these emits Rust that rustc rejects today, so no program that compiles today is rejected. `check` checks every function, and rustc sees only the functions `main` reaches, so a conflict in a function that nothing calls becomes an error that `build` does not report today.

**Compile speed.** Three costs, each with its own growth:

| Cost | Complexity |
|---|---|
| Loan liveness, per body | A backward dataflow over the body's control-flow graph with one bit per local that holds a loan, then one check of each access against the loans live there. Each pass is linear in the number of nodes times the number of such locals. A loop adds passes, bounded by its nesting depth plus two. Straight-line code needs one pass. |
| Regions, per cycle of the call graph | A function outside a cycle is read once, in callee order. A cycle iterates until no region grows. Each iteration reads every body in the cycle, and the number of iterations is bounded by the number of borrowed parameters in the cycle, because each iteration adds at least one parameter to some region. |
| Adjustment nodes in the typed IR | One node per adjustment that adds or removes a borrow. Every later stage visits them, so the cost is linear in the number of adjustments, at every stage. |

The prototype model is Python over the syntax tree, so it says nothing about the compiler's speed. Today `knave check` with a release build takes 550 ms on `many_modules`, 382 ms on `call_chain`, 364 ms on `deep_nesting` and 275 ms on `macro_heavy`. Against the corpus rule that an entry's emission stays under one second, `many_modules` already uses more than half. `call_chain` is the entry where a region fixed point over long chains of calls would show first, and `many_modules` the one where per-body passes add up. The budget is in the Decision section.

### B: Restrict the language so the errors cannot arise

Each class gets a restriction instead of a check. On aliasing, the choices measured on the 490 programs of the first generator that rustc compiles:

| Restriction | Accepted programs it rejects |
|---|---|
| No local holds a borrow: a borrow lives only inside the form that makes it | 449 (92%) |
| A local that holds a borrow freezes the places it borrows until its scope ends | 186 (38%) |

`target/scratch/gen/restrict.py` measures both, over the statement model of the first generator's prototype. An earlier count of 460 and 187 came from a script that was not kept, and review's separate reconstruction gives 449 and 186 as this script does. Five of that grammar's 16 statement choices make a held borrow, so these shares measure the grammar, not the programs people write. They show that each restriction rejects programs that rustc accepts often enough for a generator to find one every few programs.

On closure signatures, a closure bound by `local` may not return a borrow of a parameter, and the program writes a `defn` instead. On closure kinds, every function type is a `fn` pointer, so no closure captures. On signature lifetimes, a function may return a borrow of one parameter only.

**Programs it rejects that rustc accepts.** The table above, and every capturing callback, such as `(for-each cars (fn (c) (push! names (:make c))))`, under the closure-kind restriction.

**Compile speed.** Faster than A: each restriction is a syntactic check.

### C: Ask rustc during `check`

`check` emits the Rust and runs rustc's analysis without code generation, then maps each error back to the source.

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.knv:4:3
```

**Programs it rejects that rustc accepts.** None, by construction.

**Compile speed.** On a generated program that compiles, `rustc --emit=metadata` takes about 90 ms, five times the front end's 16 to 21 ms. The interpreter could not run without rustc. The errors keep rustc's words and Rust's names, and a mapped span lands on the form that emitted the Rust, not always on the form at fault.

## Recommendation

Take Option A for lifetimes and aliasing, and for every other class make one stage decide each fact and every later stage read it. The classes then close by these rules:

| Class | Rule |
|---|---|
| Aliasing | A loan of a place, a path, lives where rustc's region liveness says: wherever a value whose region holds it is live, and from the loan to every exit when it reaches the result. While it lives, its place does not change, move, or lend as `mut`. A call's `mut` loan begins after its other arguments are evaluated. |
| Escaping borrows | A loan of a local does not outlive the local. A borrow of a temporary keeps today's constant rule, with the holes of report 266 closed. |
| Closure signatures | A closure's result ties to the parameters it borrows by the same record as a `defn`'s, emitted through the `tied` helper. |
| Signature lifetimes | The ownership stage decides every lifetime by a fixed point over the call graph, and the emitter writes only what it decided. |
| Closure kinds | The type stage tells a `fn` pointer from a closure. A capturing closure that meets a `fn` pointer is `L0511`, wherever it meets one. A trait method's function parameter takes one kind across its default body and every impl. |
| Reference depth | The typed IR holds every reference layer the emitted Rust has, with each adjustment as a node. The emitter writes no `&` and no `*` that the typed IR does not hold, and a debug assertion compares the two. |
| Trait bounds | Every bound the emitted Rust requires is one the type stage proved. A runtime trait without methods lists its implementors, and `Sized` is a bound the type stage tracks for type parameters. |
| One-off | The def-move check and the emitter ask one `const` rule. A field passed to a `mut` parameter is a `mut` use of its root. The others are emitter bugs that the generator finds. |

On 1,398 accepted generated programs with branches, loops, bare `do` blocks, locals bound inside them, names bound twice, borrows held in fields and vectors, and helper bodies whose regions it inferred, the loan rule agreed with rustc on every one. Option A keeps the language's model, Rust's ownership with Knave's inference, and it gives the interpreter a front end that has decided every fact the Rust holds, which the interpreter needs to walk the typed IR without guessing.

### The scope: Knave code and prelude forms

The rule covers every program built from Knave code and prelude forms. An error inside a user's own Rust text, a `(rust ...)` template or a crate call, stays a mapped rustc error, because Knave cannot see inside it. A prelude `rust` form is covered: an error in it is a bug in the prelude.

How the interpreter runs a Rust leaf is not this RFC's subject. Two designs hold it: the interpreter design gives each prelude `rust` form a native implementation in the interpreter, and the compiled crate bindings design compiles the crate-backed forms of a program into a `cdylib` that the interpreter loads.

### Crate calls whose borrows Knave cannot determine

A crate call's result can hold a borrow of an argument, and Knave code can then change the argument while the result lives. The conflict is in Knave code, so the loan rule must see the loan where it can.

- **What counts as undetermined.** Decided: a crate call's borrows are settled when Rust's elision rules decide them from the Knave type declared for the binding, including any written lifetime. Knave does not read the crate's own Rust signature. A binding whose declared type elision does not settle is undetermined. Every other crate call is determined, and its loans enter the loan rule as a `defn`'s do. A result whose type has a `Drop` impl, such as a lock guard, keeps its loans until the end of its scope, by the liveness rule.
- **What an undetermined result borrows.** Decided: nothing, so `check` never rejects a valid program for it. If the program does change an argument while the result lives, rustc reports it, mapped to the source.
- **Under the interpreter design.** Decided: an input that reaches a crate is not interpreted. It compiles against prebuilt dependencies, so rustc sees the whole of it and reports the same error locally as in CI.
- **Under the compiled crate bindings design.** rustc sees only the wrappers, and a borrow does not outlive the call, so a crate result that keeps a borrow cannot be held in a handle. The owner has not chosen between two outcomes: the wrapper fails to build, or the input takes the compile path as a backup. This RFC recommends the compile path, because a wrapper's build error lands in generated code, not in the user's text, while the compile path reports rustc's error at the user's source, as CI does. The compiled crate bindings RFC makes the choice.

With either outcome, the interpreter does not run a program that rustc would reject.

### Testing the claim

Fixtures cover only the shapes someone thought of. The two generators above found the largest class with no thought at all. The harness:

1. **A generator of well-typed programs.** It builds programs typed by construction, from a seed, over a grammar of the language's forms. It starts from the second generator above, and grows to the forms that grammar lacks: `match`, enums, maps, recursive and mutually recursive functions, traits with default bodies, generic functions, calls through function values, and closures that return borrows. Each program prints the values it computes.
2. **The oracle.** A program that `check` accepts and rustc rejects is a failure. A program that `check` rejects with an ownership code, and whose Rust, emitted without the ownership checks, rustc compiles, is a false error. A hidden `emit` flag skips the checks for the harness only. The oracle compares only functions that `main` reaches, because rustc sees only those.
3. **Fixtures for known shapes.** The harness keeps review's probes as fixtures: `pick!` above, a borrow read inside a `loop` after a `push!` of its vector, a change of one field while another is borrowed, `(push! v (len v))`, a user `mut` helper called with `(len v)`, `(ref (+ 1 2))`, `(defn quotient () (ref (/ 7 -1)))`, `(push! v (at v 0))`, and `(local x (at v 0)) (push! v x)` on a vector of integers, which compile, the first two programs below, which compile and which a model with one value per name rejects, and the third, where a bare `do` shadows `r` and rustc reports `E0502` at the `push!`:

   ```lisp
   (defn main ()
     (local v (vec 1 2))
     (loop (n 0) (if (< n 2) (do (push! v 9) (local r (at v 0)) (println r) (recur (+ n 1))) (do))))

   (defn main ()
     (local v (vec 1 2))
     (local w (vec 5))
     (local r (at v 0))
     (println r)
     (push! v 9)
     (local r (at w 0))
     (println r))

   (defn main ()
     (local v (vec 1 2))
     (local w (vec 5))
     (local r (at v 0))
     (do (local r (at w 0)) (println r))
     (push! v 9)
     (println r))
   ```

4. **A reducer.** A failing program shrinks to the smallest program that still fails, by dropping and simplifying forms, and becomes a fixture under `tests/fixtures/`.
5. **Runs.** Each pull request runs a fixed set of seeds. 1,000 programs of the second generator took 51 to 72 seconds on four cores at `nice 19` on the machine that wrote this RFC, so 200 take about 14 seconds there. Review measured 21 seconds for 200 programs of the first generator on another machine. A scheduled job runs fresh seeds and files what it finds.

The generator grows with the language: a form it cannot build is a form the claim does not cover yet, and the RFC that adds a form adds it to the generator.

### One harness for checks and agreement

The interpreter's agreement tests compare interpreted and compiled output. The harness above already compiles every program it generates, so it runs each compiled program and the interpreter on the same program and compares standard output, standard error and the exit code. Every run fixture and every test runs both ways as well. One harness then answers both questions: does rustc accept what `check` accepts, and does the interpreter compute what the compiled program computes. Each program adds one interpreted run, whose cost is not measured, because the interpreter does not exist yet.

The two backends could disagree on these behaviors. Each needs a language rule that both follow:

| Behavior | Rule |
|---|---|
| Evaluation order | Every form evaluates its parts once, left to right. A struct update evaluates its field values before its base when the base is a name or a field read. The emitter's hoisting of a `mut` argument keeps this order, as [Call order](../docs/language.md#ownership) states. |
| Integer overflow | Decided: Rust's model. A debug build panics and `build --release` wraps. The interpreter, `knave test`, `eval` and `repl` follow debug builds, and agreement tests compare against debug builds only. The language reference says, next to the interpreter's documentation, that `build --release` wraps where the interpreter panics. |
| Division by zero, and `MIN / -1` | Panics in both, in every profile. |
| Floats | `f32` arithmetic rounds to `f32` at every step, and a float prints as Rust's `Display` prints it. |
| `hash-map` order | Decided: a `hash-map` stays Rust's `HashMap`, with unspecified iteration order. Printing a map sorts its entries, but `iter`, `keys` and `vals` yield an order that differs on each compiled run. The interpreter varies its order on each run, so a test that depends on that order fails locally as it can in CI. It draws the order from a seed and prints the seed when a run fails, so the run can be repeated. A fixture whose output depends on `hash-map` order is a fixture bug. The generator never lets `hash-map` iteration order reach output. A program that needs a stable order uses `sorted-map`, which iterates in key order in both backends. |
| Drop order | A value drops at the end of its scope, in reverse order of binding, and a temporary at the end of its statement, as in Rust. Knave writes no destructors, so the order shows only through crate values with a `Drop` impl. `println` writes through Rust's standard output, which flushes each line. |
| Closure captures | A `move` closure copies a Copy capture when the closure is made, so a later change of the local does not reach it. The interpreter captures the same way. |
| Sharing | The interpreter may share values that Rust would copy or move, because the ownership checks prove the difference unobservable. That proof is this RFC: a program that rustc would reject for aliasing could show a change through a shared value when interpreted. |
| Panics and exit codes | A panic exits with code 101 in both, and a program's own `exit` code passes through. A compiled panic's first line names the thread by an id that changes on each run and a location in the emitted Rust, which the interpreter does not have. The harness compares the panic message and the exit code, and ignores that line. A panic hook that prints the Knave source span in both backends is a separate design. |
| Stack depth | Deep recursion that is not a tail call overflows at a depth that differs between backends. A stack overflow is a crash in both, and agreement tests compare only that both crash. |

### Order of work

Before the interpreter:

1. The harness, with the second generator grown to the forms above. It measures every later step.
2. The loan rule, rule 1 of Option A, which closes the aliasing class and the generators' failures and takes in `L0510` and `L0512`.
3. Regions in the ownership stage, which close the signature and closure-signature classes.
4. Reference depth in the typed IR, because the interpreter walks the typed IR and must see every layer the compiled program has.

These can follow, before 1.0: closure kinds, trait bounds, and the one-off bugs.

## Decision

The project owner decided on 2026-11-09:

- **Option A.** Knave checks lifetimes and aliasing.
- **Scope.** The rule covers every program built from Knave code and prelude forms. An error in a user's own Rust text, a `(rust ...)` template or a crate call, stays a mapped rustc error.
- **Crate borrows.** A crate call's borrows are settled when Rust's elision rules decide them from the Knave type declared for the binding, including any written lifetime, because Knave does not read the crate's own Rust signature. A result whose borrow Knave cannot determine counts as borrowing nothing, so a valid program is never rejected.
- **Crate calls under the interpreter design.** An input that reaches a crate is not interpreted: it compiles against prebuilt dependencies, so rustc reports the same error locally as in CI.
- **Overflow.** Overflow keeps Rust's model. The interpreter, `knave test`, `eval` and `repl` follow debug builds, and agreement tests compare against debug builds only. The language reference notes, next to the interpreter's documentation, that `build --release` wraps.
- **Compile speed budget.** The baseline is the commit before step 1 of the order of work. Across the steps of Option A, the front end's total instruction count over the corpus grows by at most 10% against that baseline, and no corpus entry's emission goes over one second. Each step's pull request body reports the comparison of its head with the baseline. CI's instruction count job never fails on a rise, so a breach stops the work and goes to the owner. The job's 1% gate on allocated and peak live bytes applies as usual.
- **`hash-map` order.** A `hash-map` stays Rust's `HashMap`, with unspecified iteration order. The interpreter varies the order on each run from a seed that it prints when a run fails, the generator never lets the order reach output, a fixture whose output depends on it is a fixture bug, and a program that needs a stable order uses `sorted-map`.

The project owner chose the `tied` helper for closures bound by `local` on 2026-11-06.

Left to the compiled crate bindings RFC: whether a crate result that keeps a borrow fails its wrapper's build or takes the compile path. This RFC recommends the compile path.

## Rejected options and why

- **B, restrict the language:** on the first generator's grammar, the cheapest aliasing restriction rejects 38% of the programs that rustc compiles, and the closure-kind restriction removes capturing callbacks. It trades the language's model for the checker's convenience.
- **C, ask rustc during `check`:** it costs about 90 ms of rustc per check, five times the front end, keeps rustc in the interpreter's loop, and leaves the diagnostics in rustc's words.

## Rule

[docs/language.md: Ownership](../docs/language.md#ownership)
