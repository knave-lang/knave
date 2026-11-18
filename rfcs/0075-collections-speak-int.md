# RFC 0075: Collections speak `int`

**Status:** Accepted
**Date:** 2026-11-17

Item 487 of the project's plan. The owner approved the direction: the prelude's collection forms speak `int`, as in Go and Swift. `len` and `count` return `int`, index arguments take `int`, the emitted Rust converts at the boundary, and `usize` stays for Rust interop and precise control. The design builds on RFC 0074, accepted and released in v0.17.0, which makes `int` the shown name of `i64`: Knave source, diagnostics and docs below say `int`, and the emitted Rust says `i64`. Code references are against `origin/main` at `b00494c5f`. Every "today" output was produced with a build of `b00494c5f`. Rust-only claims were checked with rustc 1.95 from the dev shell, at `-C opt-level=3`, edition 2024.

## Problem

An index, a length and a count are `usize` today, and the default integer is `int`. The two meet in almost every loop, and the user pays at each meeting. Probed on `b00494c5f`:

```lisp
(type pick (fn ((ref (vec int)) int) int))
(defn pick (items n)
  (deref (at items n)))
```

```text
error[L0401]: mismatched types
 --> mix.knv:3:20
  |
3 |   (deref (at items n)))
  |                    ^ expected `usize`, found `int`
  |
  = note: run `knave doc L0401` for an explanation
```

The error has no help: no diagnostic in `src/` or `expand/src/` suggests a conversion for an integer mismatch. The prelude has no conversion function either: `prelude/number.knv` exports arithmetic, comparisons and traits only. A user who needs one writes a `rust` cast template, as three corpus entries do for other pairs: `(rust "{0} as f64" (fn (usize) f64))` (`matrix_geometry.knv:186`), `(rust "{0} as i64" (fn (f64) i64))` (`generic_types.knv:501`) and `(rust "{0} as u32" (fn (i64) u32))` (`warehouse/book.knv:136`). Seven prelude docstrings warn "`index` is a [usize], so an [int] is an error" (`assoc.knv:30`, `:201`, `vec.knv:238`, `:287`, `:300`, `iterator.knv:379`, `place.knv:46`).

The cost spreads by inference. A literal that reaches an index is pinned to `usize` (`src/types/unifier.rs:432-459`), so `(let (i 1) (at items i))` emits `let i: usize = 1;`, and the `usize` flows into every function that `i` reaches. The `evaluator`, `warehouse`, `matrix_geometry` and `generic_types` corpus programs write `usize` for positions, line numbers, sizes and counts only because `at`, `get`, `take`, `len` and `count` force it.

Go and Swift settled this the same way: `len` returns `int`, an index is an `int`, and a negative index is a runtime error. Rust chose `usize` because its integers are the machine's. Knave's users write a language whose default integer is `int`, and the boundary belongs in the emitted code, not in the user's program.

## Every form that touches a length, a count or an index

Two kinds of code type a user's index: prelude `rust` templates, and five kernel-form sites in the type checker. The compiler has no special case for any prelude form, apart from two emitter paths that recognize `at` by its template text (see "The template text of `at`").

### Prelude forms (exported)

| Form | Where | Today | After (option A) |
|---|---|---|---|
| `len` | `prelude/sequence.knv:31-34` | `{0}.len()` → `usize` | `crate::rt::length({0}.len())` → `int`, checked |
| `count`, one collection: `(count (iter c))` | `prelude/iterator.knv:123-130`, expands to `len` | `c.len()` | `crate::rt::length(c.len())` |
| `count`, any other call (`count-items`) | `iterator.knv:97-99` | `{0}.count()` → `usize` | `crate::rt::count({0})` → `int`, checked |
| `count` as a value (`count-function`) | `iterator.knv:101-103` | `Iterator::count` | `crate::rt::count` |
| `enumerate` | `iterator.knv:67-71` | `{0}.enumerate()` → `(pair usize a)` | `(0_i64..).zip({0})` → `(pair int a)` |
| `take` | `iterator.knv:190-193` | `{0}.take({1})`, limit `usize` | `{0}.take({index 1})`, limit `int` |
| `at` | `prelude/vec.knv:161-164` | `&{0}[{1}]`, index `usize` | `&{0}[{index 1}]`, index `int` |
| `windows` | `vec.knv:155-159` | `{0}.windows({1})`, size `usize` | `{0}.windows({index 1})`, size `int` |
| `remove-at!` | `vec.knv:127-129` | `{0}.remove({1})`, index `usize` | `{0}.remove({index 1})`, index `int` |
| `get` as a value (`slice-get`) | `prelude/map.knv:135-139` | `{0}.get({1})`, index `usize` | `{0}.get({index? 1})`, index `int`: a negative index gives `none` |
| `subs` | `prelude/text.knv:802-804` | `&{0}[{1}..{2}]`, both `usize` | `&{0}[{index 1}..{index 2}]`, both `int` |
| `set!` of an `at` place | `prelude/place.knv`, expands to `set-at` | kernel row 3 | kernel row 3 |
| `assoc`, `assoc-in` on a vector | `prelude/assoc.knv`, expand to `assoc-at` | kernel row 5 | kernel row 5 |
| `range` | `iterator.knv:29-32` | generic over `integer step` | unchanged: `(range 0 (len items))` becomes an `int` range with no annotation |

`{index n}` is a new template placeholder conversion, defined under option A.

### Kernel forms typed in the checker

| # | Site | Form | Template | Change |
|---|---|---|---|---|
| 1 | `src/types/collection.rs:39` | `Collection::of` for a vector or a slice | none | the sequence key becomes `I64`. Every `.key()` read sits behind a map filter today, so nothing reads it for a sequence; the sites below should read the key type from here so it is stated once. |
| 2 | `src/types/infer.rs:2830-2846` (`infer_slice_get`, `Usize` at `:2843`) | `(get items i)` on a vector, a slice or an unknown collection | `{0}.get({1})` | parameter `I64`, template `{0}.get({index? 1})`: a negative index gives `none` (decision 1(b)) |
| 3 | `src/types/infer.rs:3154-3178` (`infer_set_at_vector`, `Usize` at `:3169`, template at `:3170-3174`) | `set-at` on a vector, any depth | `{0}[{1}][{2}]... = {n+1}` | each key `I64`, each `[{k}]` becomes `[{index k}]` |
| 4 | `src/types/infer/open_collections.rs:126-155`, template `:148`, `expect_index` `:215-221` (`Usize` at `:217`) | `set-at` on a collection whose type is still open | `{0}[{1}] = {2}` | key `I64`, template `{0}[{index 1}] = {2}`. The branch at `:191-195` forces an integer literal key to the index type when the collection stays unknown. After the switch that type is `I64`, the literal default, so the branch does nothing and is deleted. |
| 5 | `src/types/infer.rs:3313-3325` (`assoc_keys`, `Usize` at `:3318`), from `infer_vector_assoc` `:2712-2724`, emitted by `assoc_assignment` at `src/emit/expressions.rs:1409-1415` | `assoc-at` on a vector | `{ let mut value = v; value[k1]... = x; value }` | each key `I64`. No template: the type stage runs the negative-literal check on each key, and `assoc_assignment` writes each key by the shapes of option A, a bare literal or `crate::rt::index(k)` |

The `usize` primitive row (`expand/src/core/primitive.rs:114-118`), its layout (`src/types/size.rs:47`) and its runtime trait impls (`src/emit/runtime.rs:312`, `prelude/runtime/integer.rs:11`, `show.rs:81`) stay: `usize` stays a type.

### Prelude code that stays `usize`

These are not user-facing positions:

- `task-command-line`, `task-value`, `task-value-or`, `task-switch` (`prelude/task.knv:614-738`): private plumbing of `deftask`, called only with literals its macro writes. The literals keep pinning to `usize`, and the emitted task code (`cli__emitted_task_program`) does not change.
- `note-untested` (`prelude/test.knv:841-844`): private, called by `run-tests` with a count it computes at expansion.
- The implementor lists of `step`, `number`, `integer`, `partial-eq`, `partial-ord`, `ord`, `hash`, `clone`, `copy`, `display`, `debug`, `show`, `from-str` and `borrow`: `usize` keeps every trait it has.
- `prelude/runtime/sequence.rs` and `map_sequence.rs`: the `Sequence` trait's `len` stays `usize`, mirroring Rust's own `len`, because the template converts after the call.
- `prelude/runtime/signal.rs`: Rust's `signal` binding.

The macro-time `len` and `nth` (`(use code len nth ...)` in `assoc.knv` and `iterator.knv`) are the interpreter's list functions. They emit no Rust. Out of scope.

### The template text of `at`

Two emitter paths recognize `at` by its template text, `&{0}[{1}]`:

| Site | What it does |
|---|---|
| `src/emit/expressions.rs:542-553` (`borrowed_index_template`), via `ParsedTemplate::is_single_index_place` (`expand/src/core/template.rs:335-345`) | emits `(deref (at v i))` and an `at` place as `v[i]`, not `*&v[i]` |
| `src/emit/expressions/compound_assignment.rs:22` (`ELEMENT_READ = "&{0}[{1}]"`), `:140-146` (`reads_element`), with `is_steady` at `:120-134` | turns `(set! (at v i) (+ (deref (at v i)) 1))` into `v[i] += 1`, when each index is steady: a local, a literal, or a pure read of steady values |

`is_single_index_place` already matches a placeholder of any kind, so it keeps matching `&{0}[{index 1}]`. `ELEMENT_READ` compares text byte for byte, so it becomes `&{0}[{index 1}]`. Both paths keep working only if the index stays the template's own argument, a local `i`, and not a call such as `(index i)` in its place, which `is_steady` rejects. That constraint decides between the options below.

## Options

Each option is shown on the same program:

```lisp
(defn bump (items i)
  (set! (at items i) (+ (deref (at items i)) 1))
  (deref (at items 0)))
```

### A. A template conversion, `{index n}`, applied by the compiler (recommended)

A template writes `{index 1}` where Rust needs a `usize` and the form's parameter is `int`. The compiler writes the argument by its shape:

| Argument | Emitted |
|---|---|
| a non-negative integer literal | the bare literal, `items[0]`: Rust infers `usize` from the position |
| a negative integer literal | nothing: the type stage reports a new code, `` the value `-1` is negative ``, labeled "an index or a size is at least 0" (decision 7(a)) |
| anything else | `crate::rt::index(i)` |

Kernel sites 2 to 4 use the same conversion, through `{index? 1}` and `[{index k}]` in the templates they build. Site 5, `assoc-at` on a vector, builds no template: `infer_vector_assoc` returns `TypedExprKind::Assoc`, and `assoc_assignment` writes `[`, each key and `]` as fragments. So the type stage runs the same negative-literal check on each key of a vector `assoc`, with the same code and wording, and `assoc_assignment` writes each key by the shapes above: a non-negative literal bare, anything else as `crate::rt::index(k)`. `(assoc v -1 9)` and `(assoc-in grid (0 -1) 7)` report the new code, as `L0406` reports them today. No other kernel site writes a key or an index without a template: the five sites of the table are the only ones that type an index as `Usize` today, and `TypedExprKind::Assoc` is the only typed form whose emitter writes a key itself.

```rust
fn bump(items: &mut Vec<i64>, i: i64) -> i64 {
    items[crate::rt::index(i)] += 1;
    items[0]
}
```

**A second conversion for `get`, `{index? n}`.** The owner chose decision 1(b): `get` never panics, so a negative index gives `none`, while `at` panics. `{index? n}` is the conversion for a position where an index past the end gives `none`. It converts the argument alone, in its place, and maps a negative value to `usize::MAX`, an index past the end of every collection. It writes the argument by its shape:

| Argument | Emitted |
|---|---|
| a non-negative integer literal | the bare literal: `items.get(5)` |
| anything else, a negative literal included | `usize::try_from(i).unwrap_or(usize::MAX)`: `items.get(usize::try_from(i).unwrap_or(usize::MAX))` |

**The argument is an open position.** `usize::try_from` is generic over its input, so rustc does not take the argument's type from it. The emitter writes the argument of `{index? n}` as it writes an argument at a generic position: every literal in it carries its type suffix, `-4294967295_i64`, a literal in a branch of an `if` or at the end of a block included (`docs/language.md:187`). Checked with rustc 1.95, `items.get(usize::try_from(-4294967295_i64).unwrap_or(usize::MAX))` and the same with `if flag() { -4294967295_i64 } else { 0_i64 }` both give `None`. Without the suffix, `usize::try_from(-4294967295)` would make rustc fall back to `i32`, and because Knave builds with `--cap-lints allow` (`src/build/build_plan.rs:9`) the literal would wrap to `1` and `get` would return item 1. `{index n}` needs no such rule, because `crate::rt::index` takes an `i64`.

`items.get(usize::MAX)` is `None` for every slice: `get` checks `index < len`, and no length exceeds `usize::MAX`, so the compare fails even for a vector of zero-sized items of length `usize::MAX`. `{index? n}` has no negative-literal error: `(get items -1)` is valid and gives `none`, as Clojure's `(get [1 2] -1)` gives `nil`. It needs no runtime file, no closure and no binder.

**The collection stays where the template puts it.** Only the index changes, so `{0}` is evaluated first and always, as today, and a collection that is a call or a temporary keeps working. Checked with rustc 1.95:

```lisp
(defn make-items () (vec 1 2 3))
(defn main ()
  (let (i 1
        items (vec 1 2))
    (prn (get (make-items) i))
    (let (x (get (make-items) i))
      (prn x))
    (prn (get (do (println "x") items) -1))))
```

```rust
println!("{}", crate::rt::Shown(&make_items().get(usize::try_from(i).unwrap_or(usize::MAX))));
{
    let owner = make_items();
    let x = owner.get(usize::try_from(i).unwrap_or(usize::MAX));
    println!("{}", crate::rt::Shown(&x))
}
println!(
    "{}",
    crate::rt::Shown(
        &({
            println!("x");
            items
        })
        .get(usize::try_from(-1_i64).unwrap_or(usize::MAX))
    )
)
```

rustc compiles it, and with `{:?}` in place of `crate::rt::Shown` it prints `Some(2)`, `Some(2)`, `x` and `None`. The second `get` binds its owner before the `let`, as the lent-temporary rule (`docs/language.md:1221`) does for a template with no block, closure, branch, loop or `;`, which `{0}.get({index? 1})` still is. A conversion that wraps the whole template, `usize::try_from(i).ok().and_then(|index| make_items().get(index))`, moves `{0}` into a closure: rustc refuses it (`E0515`, cannot return value referencing temporary value), the owner of the second `get` can no longer be bound outside, and the print of the third runs only for an index of 0 or more.

**Cost.** With rustc 1.95 at `-C opt-level=3` over `&[u64]`, `items.get(usize::try_from(i).unwrap_or(usize::MAX))` adds three instructions before the bounds compare of `items.get(i)` with `i: usize`, and no branch:

```text
get_position:  testq %rdx, %rdx; movq $-1, %rcx; cmovnsq %rdx, %rcx; xorl %eax, %eax; cmpq %rsi, %rcx; leaq (%rdi,%rcx,8), %rcx; cmovbq %rcx, %rax
get_plain:                                                         xorl %eax, %eax; cmpq %rsi, %rdx; leaq (%rdi,%rdx,8), %rcx; cmovbq %rcx, %rax
```

**Where it is allowed.** The contract of `{index? n}` is a position where the index `usize::MAX` gives `none`. The prelude's and the kernel's only use, `{0}.get({index? 1})` on a slice, keeps it, because no slice has an item at `usize::MAX`. The type stage cannot check the contract itself, so it applies a partial guard: it rejects `{index? n}` in a template whose result is not an `option`, at the template, as a malformed placeholder. That catches the plain misuse, `&{0}[{index? 1}]`, which would panic with Rust's bounds message, `the index is 18446744073709551615`, where `{index n}` names the negative value. A user template can still pass the guard and break the contract, as `Some(&{0}[{index? 1}])` does, or `{0}.get(&{index? 1})` on a map that holds the key `usize::MAX`. The placeholders row of `docs/language.md:1078` states the contract in these words.

A second conversion is a second rule. It is the owner's choice of `get`'s contract that makes it necessary: one conversion cannot both panic and give `none`. `{index? n}` keeps it general, a property of any position where an index past the end gives `none`, not a case for `get`.

**The parameter must be `int`.** `{index n}` and `{index? n}` convert from `int`: `crate::rt::index` takes an `i64`, and `{index? n}`'s literal suffix is `_i64`. The type stage rejects either placeholder at a parameter whose type is not `int`, written `int` or `i64`, such as a `usize`, a `u8` or a type variable such as `(n integer)`, at the template, as a malformed placeholder. Without the check, a literal argument would compile, because it emits bare, and a `usize` variable would emit `crate::rt::index(i)` and reach rustc as `E0308`. The placeholders row of `docs/language.md:1078` states the rule.

**Representation.** The conversion is not an ownership access, so it is not an `Access` variant beside `{owned n}`. `Piece::Placeholder` (`expand/src/core/template.rs:11`) gains a separate field, `conversion: Option<Conversion>`, read only by the emitter and by the type stage: its negative-literal check, which also runs on each key of site 5, the `option` guard of `{index? n}`, and the `int` parameter check. The ownership pass reads `access` as today, and `{index 1}` borrows nothing.

**Values.** A template used as a value becomes a closure (`docs/language.md`, `rust` templates). `at` passed as a value emits `|argument_0: &[i64], argument_1: i64| &argument_0[crate::rt::index(argument_1)]`: a closure parameter is never a literal, so it always takes the runtime conversion.

**Constants.** Only an integer literal is elided or checked at compile time. A `def` constant is "anything else": `(def k 1)` and `(at items k)` emit `items[crate::rt::index(K)]`, which rustc folds to the same code as `items[K]`, and `(def k -1)` panics at run time, not at compile time. Extending the rule to constants would need the type stage to read a `def`'s value, for a case no fixture shows.

**The runtime file.** `prelude/runtime/index.rs`, emitted only into a program that calls it, through a new `RuntimePart` row and a `NAMED` entry (`src/emit/runtime.rs:5-40`, `:52`):

```rust
#[inline]
#[track_caller]
pub fn index(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(index) => index,
        Err(_) => negative(value),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn negative(value: i64) -> ! {
    panic!("attempt to use a negative value as an index or a size: {value}")
}
```

`take` and `windows` use the same placeholder and the same function. Their argument is a count, not a position, so the message names both roles, as the owner chose in decision 6(a). A second placeholder would exist only to change the message: no `at`-shape path matches `take` or `windows`, so its other job, eliding a literal, is the one `{index n}` already does.

**What the compiler states.** The emitter writes the Rust call `crate::rt::index(...)`, a name `prelude/runtime/index.rs` defines. The precedent for the emitter writing a runtime call is `START_UP_CALL = "crate::rt::restore_default_sigpipe();"` (`src/emit/runtime.rs:265`). The `RuntimePart::NAMED` rows only detect runtime names in emitted text, so the new row there is detection, not the statement. The compiler states no Knave name of the prelude.

**Why a compiler change.** "Small compiler, big prelude" asks first whether the prelude can express this. Option B is that attempt, and it falls short in three places that the compiler has to cover anyway:

1. **The kernel sites cannot be reached from the prelude.** `get`, `assoc`, `assoc-in` and `update` expand to `map-at`, `assoc-at` and `set-at`, which accept a vector or a map. Whether a key is an index is known only once the checker has typed the collection, after expansion. `(get scores -1)` on a map with `int` keys is valid and must not be touched. So the checker must type and convert vector keys itself.
2. **`v[i] += 1` needs the index to stay a plain argument.** Under B, the index is the call `(index i)`. `is_steady` (`compound_assignment.rs:120-134`) accepts a call only when it is a pure read it can recognize, and it recognizes `at` and operators by their templates. `ELEMENT_READ` (`compound_assignment.rs:22`) already matches `at`'s template text byte for byte, an existing exception to "never make the compiler depend on how the prelude implements a form", and A keeps it, edited to `&{0}[{index 1}]`. Teaching `is_steady` that `index` is pure would add a second dependency of that kind, on `index`'s template text, or need a new general "pure template" marker, a compiler feature larger than A.
3. **The literal case sees the expanded argument.** B's macro sees the unexpanded argument, so a literal that another macro expands to, as in `(at items (first-slot))` with `(defmacro first-slot () 0)`, reaches it as a list and takes the runtime call. A's rule runs after expansion, on the typed argument. Neither sees through a `def` constant (see "Constants" above).

The kernel sites need the conversion in the compiler regardless, so A gives the prelude forms the same mechanism rather than a second one. A is one mechanism for every index, in the prelude and in the kernel: `{index n}`, and `{index? n}` for `get`, whose negative index gives `none` by the owner's decision 1(b).

### B. A prelude `index` function and macro wrappers

```lisp
(def index (rust "crate::rt::index({0})" (fn (int) usize)))

(def at-usize (rust "&{0}[{1}]" (where (a) (fn ((ref (slice a)) usize) (ref a)))))

(def at
  (macro (form)
    (if (number? (nth form 2))
      (if (number-lt (nth form 2) 0)
        (fail "an index is at least 0" (nth form 2) "write an index of 0 or more")
        (list (quote at-usize) (nth form 1) (nth form 2)))
      (list (quote at-usize) (nth form 1) (list (quote index) (nth form 2))))))
```

`take`, `windows`, `remove-at!`, `subs` and `get` as a value follow the same pattern, and `set!` of an `at` place wraps its keys. Each form needs `defrule-with-value` or a separate value definition, because a macro is not a value.

```rust
fn bump(items: &mut Vec<i64>, i: i64) -> i64 {
    items[crate::rt::index(i)] = items[crate::rt::index(i)] + 1;
    items[0]
}
```

What B does well: the negative-literal message and its help live in the prelude, and the compiler gains nothing for the prelude forms. What it costs:

- the three gaps listed under A: the kernel sites, the lost `+=`, and literals that another macro produces;
- two definitions per form, a macro and a `usize` function, and a value definition beside each macro;
- expansion work per call. Measured with the v0.15.0 binary under callgrind on a scratch program of 800 `at` calls, half with a literal index and half with a variable: a raw `(macro (form) ...)` wrapper as above adds about 20,000 instructions per call (`expand` total 62.0 M without the wrapper, 78.3 M with it, +26%). The same wrapper written with `defmacro` adds about 122,000 per call (159.7 M). On the corpus this is small, because few index forms are written in source: `matrix_geometry` has 9 such calls, about +0.12% of its 152 M `check` total, and `warehouse` 2. A program that indexes in a loop body written many times pays more.

### C. `as usize` with no check

```rust
fn bump(items: &mut Vec<i64>, i: i64) -> i64 {
    items[i as usize] += 1;
    items[0]
}
```

No new mechanism, no runtime file, the fastest code. `at`, `remove-at!` and `subs` still panic on a negative index through Rust's bounds check, but with the wrapped value: `index out of bounds: the len is 3 but the index is 18446744073709551615`. `take` and `windows` silently treat `-1` as 2^64 - 1 and take every item or none. A literal index would still need the compiler for the bare literal and the negative-literal check, so C saves only the runtime file.

### D. An `index` trait over `int` and `usize`

```lisp
(def at (rust "&{0}[{1}.index()]" (where (a (n index)) (fn ((ref (slice a)) n) (ref a)))))
```

```rust
fn bump(items: &mut Vec<i64>, i: i64) -> i64 {
    items[i.index()] += 1;
    items[0]
}
```

Every index form becomes generic. A literal index needs a default through the trait, diagnostics print a type variable where they printed `usize`, and the user still has two index types to keep apart. The owner's direction is one index type. The template text also changes `at`'s shape, so both emitter paths need work.

### E. The conversion written into the template text

```lisp
(def at (rust "&{0}[crate::rt::index({1})]" (where (a) (fn ((ref (slice a)) int) (ref a)))))
```

```rust
fn bump(items: &mut Vec<i64>, i: i64) -> i64 {
    items[crate::rt::index(i)] = items[crate::rt::index(i)] + 1;
    items[crate::rt::index(0)]
}
```

No compiler change for the prelude forms. It breaks both emitter paths that match `at` by its shape: element reads emit `*&items[...]`, and the compound assignment is lost. It wraps every literal index in a runtime call, the most common index of all. The kernel sites still need the compiler.

## The conversions

### A length to `int`

By the owner's decision 3(b), every length conversion is checked. `len` emits `crate::rt::length({0}.len())`, from a new runtime file, `prelude/runtime/length.rs`:

```rust
#[inline]
#[track_caller]
pub fn length(value: usize) -> i64 {
    match i64::try_from(value) {
        Ok(length) => length,
        Err(_) => too_long(value),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn too_long(value: usize) -> ! {
    panic!("length does not fit in int: {value}")
}
```

The message names `int`, the type's shown name. `len`'s docs carry no caveat: no length wraps. The check matters for one case only:

- **Rust caps an allocation at `isize::MAX` bytes.** `Layout` rejects a larger size, and `Vec`, `String`, `HashMap` and `BTreeMap` allocate through it, so a collection of items of at least one byte holds at most `isize::MAX` items, which is `i64::MAX`. Every supported target is 64-bit (`flake.nix:17-19` lists `x86_64-linux`, `aarch64-linux` and `aarch64-darwin`, and `knave build` compiles for the host), so this RFC considers no 32-bit case.
- **A string's `len` counts bytes**, under the same cap.
- **A vector of a zero-sized type,** such as a vector of `unit`, allocates nothing, so its length can reach `usize::MAX`, and the prelude reaches it: `collect` over a `TrustedLen` iterator of zero-sized items is folded by LLVM, so `(len (collect (map (range (the u64 0) 18446744073709551615) (fn (x) (do)))))` returns `18446744073709551615` at once under `knave run --release` on `b00494c5f`. After the switch it panics `length does not fit in int: 18446744073709551615`, where an unchecked `as i64` would give `-1`. `(count (iter c))`, which expands to `len`, gets the same check.

**Cost: free where Rust's cap already proves the length fits, one compare elsewhere.** With rustc 1.95 at `-C opt-level=3`, `crate::rt::length` of:

| Length of | Machine code |
|---|---|
| `&[u64]` | `movq %rsi, %rax; retq`: LLVM knows a slice of sized items is shorter than `2^63`, and drops the check |
| `&Vec<u64>` | `movq 16(%rdi), %rax; retq`, the same |
| `&str` | `testq %rsi, %rsi; js .too_long; movq %rsi, %rax` |
| `&HashMap<u64, u64>` | `movq 24(%rdi), %rax; testq %rax, %rax; js .too_long` |
| `&[()]` | `testq %rsi, %rsi; js .too_long; movq %rsi, %rax` |

A string, a map and a zero-sized slice pay a never-taken `test; js`, which macro-fuses, per `len` call, not per item. A counted loop is unchanged: `for i in 0..rt::length(items.len()) { total += items[rt::index(i)] }` still compiles to the same symbol as the `usize` loop (`sum_plain = sum_checked`).

### A count to `int`

**`count` is not lossless.** Rust's `Range::count` is O(1), and LLVM folds the count of a `map` or a `zip` over a range with a pure closure to O(1) too. Measured:

| Rust | Time | `count` | `as i64` |
|---|---|---|---|
| `(0u64..u64::MAX).count()` | 87 ns | 18446744073709551615 | -1 |
| `(i64::MIN..i64::MAX).count()` | O(1) | 18446744073709551615 | -1 |
| `(0u64..u64::MAX).map(\|x\| x + 1).count()` | 42 ns | 18446744073709551615 | -1 |
| `(0u64..u64::MAX).zip(0u64..u64::MAX).count()` | 42 ns | 18446744073709551615 | -1 |
| `(0i128..1 << 100).count()` | O(1) | panics in Rust: `count overflowed usize` | |

The prelude reaches each row with no Rust of its own: `(count (range a b))` is not an `(iter c)` call, so it expands to `count-items`. The sources of a count above `i64::MAX`:

- **`range`** over `int` or `isize` that spans more than `i64::MAX`, such as `(range (- 0 9223372036854775807) 9223372036854775807)`, and over `u64`, `usize` or `u128` above `i64::MAX` items. `i128` and `u128` ranges beyond `usize::MAX` already panic in Rust.
- **`map`, `zip`, `filter`, `take-while`** and `enumerate` over such a range: O(1) when LLVM folds the closure, otherwise a loop that runs for centuries, whose count is still wrong if it ends.
- **Not** `take`, whose limit is an `int`; not `windows`, `chars`, `lines`, `split` or `iter` of a collection, which the allocation cap bounds.

So `count-items` and `count-function` both become one checked runtime function:

```rust
#[inline]
#[track_caller]
pub fn count(items: impl Iterator) -> i64 {
    let counted = items.count();
    match i64::try_from(counted) {
        Ok(count) => count,
        Err(_) => count_overflow(counted),
    }
}
```

with a cold path that panics `attempt to count with overflow: the count is 18446744073709551615`. The check is one compare per call, not per item. `(count (iter c))` keeps emitting `len`, through `crate::rt::length`. The same function serves `count` as a value, so `Iterator::count` becomes `crate::rt::count`. By the owner's decision 4(a), counts are checked this way.

`enumerate`'s index, `(0_i64..)`, has the same bound: past `i64::MAX` items `RangeFrom` panics in a debug build, as `+` does, and wraps in a release build. LLVM folds such a loop too: `(0i64..).zip(0u64..n).reduce(|_, b| b)` with `n = black_box(u64::MAX)` returns `Some((-2, 18446744073709551614))` in 100 ns at `opt-level=3`, and the prelude reaches it as `(reduce (enumerate (range (the u64 0) n)) step)`. Today the index is `usize` and does not wrap there. This is a stated limit of `enumerate`: in a release build, the index of an item past `i64::MAX` wraps. Checking it would add a compare per item, which a counted loop does not pay today.

If item 454 of the project's plan lands, `len` takes iterators and replaces `count`: its iterator path uses `crate::rt::count`, and its collection path `crate::rt::length`.

### An `int` to an index

`crate::rt::index`, shown under option A. On a 64-bit target, `usize::try_from` compiles to the sign test alone:

```text
at_checked:  testq %rdx, %rdx; js .negative; cmpq %rsi, %rdx; jae .bounds; movq (%rdi,%rdx,8), %rax
at_cast:                                       cmpq %rsi, %rdx; jae .bounds; movq (%rdi,%rdx,8), %rax
```

**The message** follows the debug overflow panic, `attempt to add with overflow`, and adds the value as Rust's bounds panic does. `#[track_caller]` reports the line of the emitted call, as Rust's own bounds check does.

**The check is always on**, in debug and release builds, as Rust's bounds check is, and unlike Rust's overflow check. A release-only `as usize` is option C's silent `take`. Decision 2.

**`get` of a negative index.** By the owner's decision 1(b), `get` returns `none` for a negative index, as for an index past the end, and never panics; `at` panics. `get` uses `{index? n}`, described under option A.

### The emitted Rust of each form

Probed today on `b00494c5f`, and option A's output after:

```lisp
(defn main ()
  (let (items (vec 10 20 30)
        i 1
        a (deref (at items i))
        b (deref (at items 0)))
    (set! (at items 2) 7)
    (for-each (enumerate (iter items)) (fn (entry) (println (pr-str entry))))
    (println (len items))
    (prn (get items 5))
    (println (subs "hello" 1 3))
    (let (w (count (windows items 2)))
      (println (remove-at! items 0))
      (let (zs (assoc (vec 1 2) 0 9))
        (println (count (take (iter items) 2)))))))
```

| Form | Today | After |
|---|---|---|
| `i 1` in a `let`, used as an index | `let i: usize = 1;` | `let i: i64 = 1;` |
| `(at items i)` | `items[i]` | `items[crate::rt::index(i)]` |
| `(at items 0)` | `items[0]` | `items[0]` |
| `(set! (at items 2) 7)` | `items[2] = 7;` | `items[2] = 7;` |
| `(set! (at items i) (+ (deref (at items i)) 1))` | `items[i] += 1;` | `items[crate::rt::index(i)] += 1;` |
| `(enumerate (iter items))` | `items.iter().enumerate()`, items `(usize, &i64)` | `(0_i64..).zip(items.iter())`, items `(i64, &i64)` |
| `(len items)` | `items.len()` | `crate::rt::length(items.len())` |
| `(+ (len items) 1)` | `items.len() + 1` | `crate::rt::length(items.len()) + 1` |
| `(get items 5)` | `items.get(5)` | `items.get(5)` |
| `(get items i)` | `items.get(i)` | `items.get(usize::try_from(i).unwrap_or(usize::MAX))` |
| `(get (make-items) i)` | `make_items().get(i)` | `make_items().get(usize::try_from(i).unwrap_or(usize::MAX))` |
| `(let (x (get (make-items) i)) ...)` | `let owner = make_items(); let x = owner.get(i);` | `let owner = make_items(); let x = owner.get(usize::try_from(i).unwrap_or(usize::MAX));` |
| `(get items -1)` | `L0406`, `` the literal `-1` does not fit in `usize` `` | `items.get(usize::try_from(-1_i64).unwrap_or(usize::MAX))`, which gives `None` |
| `(subs "hello" 1 3)` | `&"hello"[1_usize..3_usize]` | `&"hello"[1..3]` |
| `(count (windows items 2))` | `items.windows(2).count()` | `crate::rt::count(items.windows(2))` |
| `(remove-at! items 0)` | `items.remove(0)` | `items.remove(0)` |
| `(assoc (vec 1 2) 0 9)` | `{ let mut value = vec![1_i64, 2_i64]; value[0] = 9; value }` | the same |
| `(count (take (iter items) 2))` | `items.iter().take(2).count()` | `crate::rt::count(items.iter().take(2))` |
| `(take (iter items) n)` | `items.iter().take(n)` | `items.iter().take(crate::rt::index(n))` |
| `count` as a value | `Iterator::count` | `crate::rt::count` |
| `at` as a value | `\|argument_0: &[i64], argument_1: usize\| &argument_0[argument_1]` | `\|argument_0: &[i64], argument_1: i64\| &argument_0[crate::rt::index(argument_1)]` |
| `(set! (at years (- (len years) 1)) year)`, the last index (`emit/vector_parameters_as_slices.knv:9`, 4 sites in fixtures) | `years[years.len() - 1] = year` | `years[crate::rt::index(crate::rt::length(years.len()) - 1)] = year` |

None adds a clone or an allocation. `subs` reads better than today, because its literals lose the `_usize` suffix. The added noise is `crate::rt::index(i)` around a variable index, `crate::rt::length(...)` around a length and `crate::rt::count(...)` around a counted iterator, at the boundary the owner chose. The noisiest case is the last index, a length converted to `i64` and back, shown in the table's last row.

**Precedence.** A length is a function call, `crate::rt::length(items.len())`, so it needs no parentheses in any operator, and the switch adds no `as` cast to any template. `b00494c5f` parenthesizes a user's cast template inside every binary operator, `(items.len() as i64) + 1`, more than Rust needs, but this RFC no longer creates such casts, so it leaves that rule alone.

`enumerate` emits `(0_i64..).zip(items)`, the hand-written way to count in `i64` in Rust, in place of `.enumerate().map(|(index, item)| (index as i64, item))`, which adds a closure. The two compile to the same vectorized loop (63 and 65 instructions for a weighted sum). `Zip<RangeFrom<i64>, I>` is neither `ExactSizeIterator` nor `DoubleEndedIterator`, where `Enumerate<I>` is both when `I` is. No prelude form needs either today. If item 454 builds `len` of an iterator on `ExactSizeIterator`, an enumerated iterator falls to its `count` path; `.enumerate().map(...)` keeps both traits and is the fallback if that matters.

## Literal inference

**The rule does not change:** an integer literal takes the type its use expects, and defaults to `int` (`src/types/unifier.rs:20-25`, `:432-459`; `src/types/finalize.rs:211`). What changes is what an index expects.

- **An index literal is an `int`.** `(at items 1)` pins its literal to `int` and emits `items[1]` as today. A binding that only an index reads, `i` in `(let (i 1) (at items i))`, becomes `int`, and so do the functions it flows into. This is the change's purpose.
- **A negative index literal is still a compile-time error.** Today it is `L0406`, `` the literal `-1` does not fit in `usize` ``, from the literal's range check. After the switch, `-1` fits the `int` parameter, and the `{index n}` check reports it instead, on the typed argument. By the owner's decision 7(a), it reports a new code with its own explanation page: `` the value `-1` is negative ``, labeled "an index or a size is at least 0". PR 1's fixtures cover `(let (x (deref (at items -1))) ...)`, `(+ 1 (deref (at items -1)))` and `(println (deref (at items -1)))` through a test template shaped like `at`. PR 3's cover `(set! (at v -1) 9)`, `(assoc v -1 9)` and `(assoc-in grid (0 -1) 7)`, each reporting the new code.
- **A literal passed to a `usize` parameter still becomes `usize`.** A user `rust` template or crate signature that takes `usize` pins its literal as today, and so does the private task plumbing.
- **A user who writes `usize` explicitly** keeps a `usize`, and where it meets an index form gets `L0401` with the conversion help (see "Rust interop"):

```text
error[L0401]: mismatched types
 --> pick.knv:3:20
  |
3 |   (deref (at items n)))
  |                    ^ expected `int`, found `usize`
  |
  = help: convert it with `(convert n)`, which panics when the value does not fit
  = note: run `knave doc L0401` for an explanation
```
- **`(the usize 1)` as an index** is that same error. The `usize` page's example, `(deref (at (vec 5 6) (the usize 1)))` (`prelude/prelude.knv:652`), and its sentence (`:650`) "A vector or a slice is indexed by a `usize`" are rewritten: `usize` is Rust's size type, written for Rust interop.
- **`(infer (len (vec)))`** is `"int"`, not `"usize"` (`docs/language.md:208`), and `(infer len)` is `"(where ((a sequence)) (fn ((ref a)) int))"` (`prelude/types.knv:157`).

`literal_inference.knv`'s `index-link-*` chain writes `(type keep-index (fn (usize) usize))` (`:1367`), so its literals keep pinning to `usize` and its output does not change.

## Rust interop

`usize` stays a type. It still comes from a user `rust` template whose signature names it, from a crate function's written signature, and from a struct field or function signature the user wrote. Each place where it meets an `int` form now needs a conversion, and Knave has none.

### `convert`, in the prelude

```lisp
(def convert
  (rust "crate::rt::convert::<{to}, _>({0})"
    (where ((from integer) (to integer))
      (fn (from) to))))
```

The target comes from the use, as `parse`'s does (`prelude/text.knv:260-261`): `(at items (convert n))` converts a `usize` to `int`, and `(the usize (convert i))` an `int` to `usize`. The turbofish is a template type placeholder, `{to}`, which emits the Rust type the binder takes at the call, as `{target}` does for `parse` (`docs/language.md:1001`). The emitter needs no change. A use that fixes no target is `L0407`, with the existing `the` help.

**The runtime makes the Knave bound sufficient.** The signature promises that any two `integer` types convert, so the Rust must compile for any two, including in generic code, where `fn f<A: Integer, B: Integer>(x: A) -> B` calls `crate::rt::convert::<B, _>(x)`. `std`'s `TryFrom` covers every pair of concrete integer types, but `B: TryFrom<A>` does not follow from `B: Integer` for a generic `A`. So `Integer` (`prelude/runtime/integer.rs:1`, a marker `pub trait Integer: Number {}` today) gains supertraits that let every value widen to one of two 128-bit types and every type convert back from them:

```rust
pub trait Integer:
    Number + std::fmt::Display + TryFrom<i128> + TryFrom<u128> + TryInto<i128> + TryInto<u128>
{
}
```

`convert` lives in its own runtime part, `prelude/runtime/convert.rs`:

```rust
#[inline]
#[track_caller]
pub fn convert<To: Integer, From: Integer>(value: From) -> To {
    let signed = From::try_from(-1_i128).is_ok();
    let converted = if signed {
        TryInto::<i128>::try_into(value).ok().and_then(|wide| To::try_from(wide).ok())
    } else {
        TryInto::<u128>::try_into(value).ok().and_then(|wide| To::try_from(wide).ok())
    };
    match converted {
        Some(converted) => converted,
        None => conversion_overflow::<To, From>(value),
    }
}
```

A signed value widens to `i128` and an unsigned one to `u128`. `From::try_from(-1_i128).is_ok()` tells the two apart, and rustc folds it to a constant in each instance, so each conversion takes one branch. `conversion_overflow` is cold and panics `` attempt to convert 300 to `u8` with overflow ``, naming the type through `std::any::type_name::<To>()`. Every integer type implements `Display`, `TryFrom<i128>`, `TryFrom<u128>`, `TryInto<i128>` and `TryInto<u128>` in `std` (the identity cases through the blanket `From` impl), so the supertraits add no impl.

**No unused code in a program that does not convert.** The emitted-lints job builds emitted programs with `-D warnings`, so a runtime part may hold no item that a program carries without using. The `Integer` part grows by supertraits only, which declare no item, so a program bounded by `integer` that never calls `convert` carries no dead code. `convert` and `conversion_overflow` sit in the `convert` part, which a `RuntimePart` row emits only into a program that writes `crate::rt::convert`, and a `REQUIRED` row brings the `Integer` part with it, since `convert`'s signature names `Integer`. The `Integer` declaration still changes in every program that names `crate::rt::Integer`: today that is `stages__emit_fixtures@range_over_generic_integer.knv.snap`, the only snapshot with `pub trait Integer`, and PR 2 updates it. Probed with rustc 1.95: the `Integer` part alone, with a generic caller, compiles under `-D warnings`; `fn generic<A: Integer, B: Integer>(x: A) -> B { rt::convert::<B, _>(x) }` compiles; `convert::<u128, _>(u128::MAX)`, `generic::<i64, i8>(-5)` and `generic::<u128, i8>(100)` give the right value; and `convert::<u8, _>(300)` and `generic::<i64, usize>(-1)` panic with the message above. The concrete calls compile to what a hand-written `from` or `try_from` does, except a signed narrowing, which keeps the 128-bit compare that any conversion through a 128-bit type pays:

| Call | Machine code |
|---|---|
| `convert::<usize, _>(x: i64)` | `testq %rdi, %rdi; js .overflow; movq %rdi, %rax` |
| `convert::<i64, _>(x: usize)` | `testq %rdi, %rdi; js .overflow; movq %rdi, %rax` |
| `convert::<u8, _>(x: i64)` | `cmpq $256, %rdi; jae .overflow; movl %edi, %eax` |
| `convert::<u128, _>(x: u128)` | `movq %rsi, %rdx; movq %rdi, %rax` |
| `generic::<u16, i32>(x)` | `movzwl %di, %eax` |
| `convert::<i64, _>(x: i32)` | `movslq %edi, %rax`, as `i64::from(x)` |
| `convert::<i32, _>(x: i64)` | `movq; sarq $63; movq; addq; adcq; movabsq; cmpq; movq; sbbq; jae .overflow; movl %edi, %eax`, where `i32::try_from(x).unwrap()` is `movslq; cmpq; jne` |
| `usize::try_from(x: i64).unwrap()`, for reference | `testq %rdi, %rdi; js ...; movq %rdi, %rax` |

Emitted: `let n: i64 = crate::rt::convert::<i64, _>(count);`.

**The emitter's copy of the supertraits.** `RUNTIME_SUPERTRAITS` (`src/emit/types.rs:149-168`) restates each runtime trait's supertraits, so the emitter can drop a bound that another bound covers. Its `crate::rt::Integer` row lists `crate::rt::Number`, `Copy`, `PartialOrd` and `PartialEq`. PR 2 extends the row with `std::fmt::Display`, the Rust path of the prelude's `display` (`prelude/text.knv:40`), so a parameter bounded by `integer` and `display` emits `A: crate::rt::Integer` and not a redundant `+ std::fmt::Display`. The `TryFrom` and `TryInto` supertraits match no Knave bound, so they need no row entry. PR 2 extends the table rather than replacing it: deriving it from the runtime source would need the emitter to parse Rust, and the table's rows already restate the other runtime traits the same way. A unit test that every listed supertrait appears in the trait's declaration in `prelude/runtime/integer.rs` would keep the two in step.

By the owner's decision 5(a), `convert` ships in this RFC, under that name: integers only, target from context, checked. It is integer-only under a general name, and floats will want the same name. RFC 0070's `foreign-names` entries for Rust's `as`, `into` and `try_into` and Clojure's `int` and `long` go beside it once that form lands.

### The help on a mismatch, stated in the prelude

The help is prelude text, through RFC 0070's `mismatch-help` (`rfcs/0070-foreign-names.md:375-381`), which states a help for an `L0401` whose expected type is a row's first type and whose found type is its second, with `{value}` for the labeled value as the user wrote it. This RFC adds one row, in the form RFC 0070 specifies, and extends `mismatch-help` with one rule, the precedence of "One help per error" below, which applies to every row. The extension adds a case RFC 0070 does not address, an `L0401` that already carries a help, and leaves its rule for every other `L0401` as it stands, so RFC 0070 stays `Accepted`, not `Superseded by 0075`: `rfcs/README.md` marks an earlier RFC superseded when a later one changes its rule. `mismatch-help` is not built yet, and from PR 2 `docs/language.md` states the rule with the extension:

```lisp
(mismatch-help int usize
  "convert it with `(convert {value})`, which panics when the value does not fit")
```

It sits beside `convert` in `prelude/number.knv`. No Rust names `convert` or the help text.

**Why one pair, not every integer pair.** A row over every pair of integer types would also match an `L0401` whose found type is a literal's default, and there `convert` is the wrong fix. In `(defn port () 8080)`, used where a `u16` and an `i32` are both expected, `port`'s result defaults to `int`, and the error reads `` expected `u16`, found `int` ``. The right fix is a `type` signature on `port`; `port`'s type is written nowhere, so a help that says to convert or to respell a written type names a fix that does not apply. Telling those errors apart needs a mechanism that sees the default at the error: the found expression is a call whose callee's result is a literal generic that the defaulting run set to `int` (`TypedProgram::literal_generics`, `src/types/typed.rs:384`, and `DefaultingRule`'s `literal_defaults` and `default_literal_parameters`, passed in at `src/check.rs:215-224`). The row would have to read the callee's generic at the error and look it up in both tables, for a help on pairs this RFC does not create. `Pinning::literal_pins` does not serve: it holds the bindings that took one use type, which are exactly the ones that did not default.

The `int` and `usize` pair is safe without it. A literal defaults only to `int` (`src/types/unifier.rs:20-25`), so a found `usize` is never a literal default: it comes from a written `usize` (a signature, a field, a `rust` template or `the`), or from a literal that a `usize` use pinned. Both are fixed by `convert`. The reverse pair, expected `usize` and found `int`, is left without a row: its found `int` can be a literal default, as `port`'s is.

**One help per error.** A row applies only to an `L0401` that the checker gives no help of its own. The literal-default help (`explain_literal_default`, `src/types/infer.rs:2144-2160`, worded at `src/types/problems.rs:275-282` and `:308-319`) wins: when a function's parameter defaulted to `int` because its uses disagree, the checker adds a note and a help that suggests a `type` signature. That names the cause, and `convert` at one call would hide it.

The 8 snapshots that report an `L0401` between two integer types today, and what PR 2 does to each:

| Snapshot | Today | After PR 2 |
|---|---|---|
| `check/literal_default_calls` | `` expected `int`, found `usize` ``, with the literal-default note and help | unchanged: the pair matches, and the literal-default help wins |
| `run/function_value_literal_items_called_directly` | `` expected `u8`, found `int` ``, with the literal-default note and help | unchanged: no row |
| `run/function_value_literal_items_two_types` | `` expected `u8`, found `int` ``, with the literal-default note and help | unchanged: no row |
| `run/function_value_literal_items_written_call` | `` expected `int`, found `u16` ``, with the literal-default note and help | unchanged: no row |
| `run/function_value_bounded_literal` | `` expected `i16`, found `int` ``, with the literal-default note and help | unchanged: no row |
| `check/literal_result_two_uses` | `` expected `u16`, found `int` ``, no help; the `int` is `port`'s literal default | unchanged: no row |
| `check/literal_result_chain_two_uses` | the same, through `other` | unchanged: no row |
| `check/infer_operand_literal_use` | the same, `port` passed to `listen` | unchanged: no row |

So PR 2 changes none of the 8, by the row's pair alone, with no new mechanism. Its help shows in its own fixtures: a written `usize` passed where `int` is expected gets it, a written `int` passed where `u8` is expected gets none, and the help is absent beside the literal-default help. The switch, PR 3, then adds the help to the index mismatches of "What breaks". A help for the three `port` snapshots, which today have none, is outside this RFC; it needs the mechanism above.

`mismatch-help` is not built yet: RFC 0070's first build ships it with its `(str name)` entry. If it has not landed when this RFC's PR 2 starts, PR 2 builds it as RFC 0070 specifies, with the precedence above.

**The help serves new code.** It fires where a `usize` reaches an index form, which in new code means a `usize` from Rust: a `rust` template, a crate signature, a field that holds a Rust size. There `convert` is the fix. The help names no other fix: a help that tells the user to respell a `usize` they wrote would serve only code written before the switch, which the owner's standing rule excludes (see "Migration aids removed"). The reverse pair gets no help (decision 10(c)).

### Relayed rustc errors

A user `rust` template that passes an `int` where its Rust code wants a `usize` is a rustc error, relayed as today (`src/build/rustc_errors.rs`). This RFC changes no user template.

## What breaks

The owner is the only user: no migration aid, `release:minor` on the PR that switches the forms.

A program breaks where a value typed `usize` meets an `int`. That value is either written as `usize`, or produced by `len`, `count` or `enumerate` and then passed to a written `usize`. A literal index keeps compiling. `range` is generic, so `(range 0 n)` follows `n`.

**Sources.** `examples/` and `prelude/tests/` write no `usize`. 100 fixture files do:

| Group | Unchanged | Stops compiling, or a check fixture gains an error |
|---|---|---|
| check | 8 | 10 |
| corpus, top-level files | 2 | 2 |
| corpus, submodules | 0 | 7 |
| emit | 7 | 18 |
| run | 8 | 28 |
| expand, format | 10 | 0 |
| **Total** | **35** | **65** |

The 65 share a few patterns:

- a written result of `usize` whose body is `len` or `count`: `check/match_moves_in_every_arm.knv:1`, `check/clone_meaning.knv:18`, `check/int_names_the_kernel_type_in_an_impl.knv`, `emit/borrowed_items.knv:8`, `emit/split_on_any_pattern.knv` and its `run` twin, `corpus/generic_types.knv:476`;
- `count` passed where a `(fn (...) usize)` is written: `check/iterator_in_function_type.knv:2-25`, `check/iterator_in_trait_signature.knv:9-24`;
- a written `usize` position passed to `at`, `get`, `take` or `subs`: `corpus/evaluator/{lexer,parser,machine}.knv`, `corpus/warehouse/{parsing,reports}.knv`, `corpus/matrix_geometry/linear.knv:28-122`, `emit/set_compound_assignment.knv:6`, `run/compound_assignment_with_field_index.knv:2`, `run/returned_closure_holds_borrow_beside_changed_copy.knv:2`, `run/calculator/*`, `run/stack_machine/*`;
- a struct field written `usize` that holds a `len`: `corpus/warehouse/model.knv:13` (built at `warehouse/book.knv:36`), `run/assoc_base_read_by_moved_field.knv:3`.

What the user sees for each pattern after the switch, probed on `b00494c5f` through a function typed `(fn ((ref (vec int))) int)` standing in for the switched `len`:

| Pattern | Error after the switch | Help |
|---|---|---|
| a written `usize` result whose body is `len` or `count` | `` expected `usize`, found `int` `` | none |
| `count` passed where a `(fn (...) usize)` is written | `` expected `(fn (...) usize)`, found `(fn (...) int)` `` | none |
| a written `usize` position passed to `at`, `get`, `take` or `subs` | `` expected `int`, found `usize` `` | the `convert` row's help |
| a struct field written `usize` that holds a `len` | `` expected `usize`, found `int` `` | none |

Only the third pattern matches the row. In the first, second and fourth, the written `usize` is the *expected* type and the found `int` comes from `len` or `count`, the reverse pair, which has no row because a found `int` can be a literal's default (see "Why one pair, not every integer pair"). Those breaks show a bare `L0401`, as the Problem section's example does in the other direction, and keep it: by decision 10(c), the reverse pair gets no help.

Each of these is fixed by respelling `usize` as `int`. One is not: `emit/vector_parameter_chains.knv:38-41` wraps Rust's `{0}.capacity()`, whose Rust type is `usize`, and `:61` compares it with `len`. Respelling the Knave type alone gives a rustc error. It takes `convert` around the template's call, or a template that converts, `{0}.capacity() as i64`. Its `run` twin is the same.

The corpus programs that break are `evaluator` (`evaluator.knv:23` and three submodules), `warehouse` (three submodules), `matrix_geometry` (`matrix_geometry/linear.knv`) and `generic_types` (`:476`). `literal_inference.knv:1367` and `matrix_geometry.knv:161`, `:186` keep compiling, because nothing an index form produces reaches them.

**Output that changes without a source change.**

- **Emit snapshots:** 431 of 448 contain `usize`. Most hold it only in runtime boilerplate (`signal`, `DEFAULT_HANDLER`, the numbers trait list) and do not change. 56 have `usize` in the program's own code, before `mod rt {` or in a snapshot with no runtime, and change, plus every emit snapshot of a fixture that calls `len`, `count`, `enumerate`, `take` or indexes with a variable, whether or not it writes `usize`. The PR author reads each hunk against the table of emitted forms.
- **Agreement fixtures:** the 36 `stages__agree_fixtures` snapshots record rustc's verdict on emitted Rust, and index emission changes in many of them. Run `--test stages agree_fixtures` beside the emit diff, and read each changed verdict.
- **Check snapshots whose wording changes:** `generic_function_value_no_adjustment` (`` expected `(fn ((ref (vec int))) usize)` ``), `iterator_items_need_borrow_signature_two` (a help's suggested signature), `literal_default_calls` (`` found `usize` ``, its note and help; the error moves to the `i32` call), `test_equal_mismatch` (`` expected `usize`, found `str` ``), `mutual_recursion_modes` (printed inferred types).
- **Run output:** `run/infer_unresolved_operand` prints `(infer (len (vec)))`, `"usize"` today and `"int"` after.
- **Doc-page snapshots** that quote the prelude prose: `doc_assoc_place_pages`, `doc_iterator_pages`, `doc_prelude_undocumented`, `doc_sequence_pages`, `doc_vec_pages`.
- **Prelude doc tests:** `prelude/prelude.knv:652` (`(the usize 1)` as an index) fails, and with it the CLI snapshot `runs_the_prelude_tests_and_lists_the_untested_prelude_names:75`. `prelude/types.knv:157` changes its expected text.
- **Unit tests:** `src/emit/lent_temporaries.rs`, whose `LiftFixture::SOURCE` writes `usize` at `:153-321` and feeds it `len` results. No other unit test or `tests/cli.rs` expectation asserts `usize` as an index type.
- **`L0458`'s closure-receiver arm:** `MethodReceivers::receives` (`src/types/method_receivers.rs:48-55`) flags a closure parameter only when a template places it before a `.`. `crate::rt::count({0})` and `(0_i64..).zip({0})` are function calls with the parameter inside, so `(local tally (fn (it) (count it)))` no longer reports `L0458`, and rustc accepts `|it: _| crate::rt::count(it)`. That helps users, but `check/closure_iterator_receiver.knv` uses only `count`: all four of its errors vanish, and it is the only snapshot that reaches `UnnameablePlace::ClosureReceiver` (`src/types/opaque_positions.rs:45`). PR 3 rewrites it around another method template, such as `max` of an iterator, whose template is `{0}.max()` (`largest-item`, `iterator.knv:132-135`), so the arm stays covered. `docs/language.md:283`, whose example is `(let (tally (fn (it) (count it))) ...)` emitting `|it: _| it.count()`, and `L0458.md:9-11`, which names `count` as the method, switch to the same template.
- **Explanation pages:** the fixes of `L0413.md` (`:30`, `:41`), `L0415.md` (`:31`, `:43`) and `L0444.md` (`:34`, `:46`) type a `size` as `usize` with a `len` body, so `each_explanation_fix_builds_and_runs` (`tests/stages.rs:370`) fails until they say `int`. Their erroneous examples, `L0413.md:15-19`, `L0415.md:17-21` and `L0444.md:19-23`, write the same `usize` result over a `len` body, and would report the new `L0401` before their own code, which `each_erroneous_explanation_example_reports_its_code_first` (`tests/stages.rs:352`) rejects; they are respelled with the fixes. `L0221.md` and `L0226.md` use a `size-of` template that returns `usize` and stay. `L0406.md` does not change: the negative index literal gets its own code and page (decision 7(a)).
- **`docs/language.md`:** `:97` (`get` reads "by a `usize` index", and its value type), `:174` (the `is_alive(grid, x: usize, y: usize)` example, from `emit/one_type_helpers`), `:208` (`(infer (len (vec)))`), `:282` and `:334`, where an example pairs `usize` with `count`, and `:283` (see `L0458` above), all in PR 3. `:85`, `:114`, `:183`, `:1318` and `:1499` stay. The placeholders row of the template rule table (`:1078`) gains `{index n}` and `{index? n}` in PR 1.
- **Prelude prose that becomes false:** `place.knv:46`, `assoc.knv:30`, `:201`, `iterator.knv:265`, `:281`, `:379`, `vec.knv:238`, `:287`, `:300`, `sequence.knv:62`, `prelude.knv:650`; `text.knv:1155`, where `subs` "emits `&text[start..end]`". `map.knv:126`, where `get` returns `none` "for a missing index", stays true under decision 1(b), and gains a sentence that a negative index gives `none` too. `vec.knv:301`, where `get` "returns an [option] in place of the panic", stays true.
- **`xtask/agree`:** the generator writes no `usize`. It indexes with literals (`reading.rs:12`, `value_kind.rs:16`, `helper_template.rs:72-166`), which emit as today, and prints or returns `len` (`value_kind.rs:11`, `:20`, `reading.rs:25`), whose emitted Rust gains `crate::rt::length(...)`. Run `cargo xtask agree --seeds 0..200` and update `pull-request-findings.txt` if the findings move.

**What a user sees.** A program that passes a value it typed `usize` to an index form gets `L0401` with the `convert` help. A program that wrote `usize` as the type of a `len` or `count` result, a struct field that holds one, or a function type that `count` is passed to, gets a bare `L0401`: `` expected `usize`, found `int` ``, or the function-type mismatch. A program that never wrote `usize` keeps compiling, and its emitted Rust gains `crate::rt::length(...)` around each length, `crate::rt::count(...)` around each counted iterator, and `crate::rt::index(...)` around each variable index. Run output changes only where a program prints `infer` text.

## Cost

**Target: no front-end stage rises more than 1% in instructions on any corpus entry.** Option A adds no pass and no hot-path work:

- **Expansion:** the prelude templates change text. `{index 1}` parses in the placeholder parser once per template, when `build.rs` builds the prelude state. The saved state grows by the template text, `convert`, `index`'s and `count`'s runtime files, and the `mismatch-help` row. That growth reaches every program, `empty_program` included, so the allocation counters are the ones to read.
- **Type stage:** the five kernel sites swap `Usize` for `I64`, the same work. The negative-literal check is one sign test per literal at an `{index n}` or a key of a vector `assoc`. The `mismatch-help` lookup runs only on an `L0401`.
- **Ownership:** unchanged: `{index n}` borrows nothing, and its conversion is not an access.
- **Emission:** a shape test of the argument at each `{index n}`, and four new runtime parts to scan for: `index`, `length`, `count` and `convert`. Rough size: `evaluator` emits 6 `len`, 9 `get`, 1 `enumerate` and 3 bracketed indexes or slices today; `matrix_geometry` 6 `len`, 2 `enumerate`, 9 indexes; `warehouse` 8 `len`, 5 `get`, 2 `take`, no index. Each becomes a few more tokens.

Option B instead adds 20,000 to 122,000 expansion instructions per written index call (see option B): about +0.12% on `matrix_geometry`, more on index-heavy programs.

**The corpus entries change source.** The four programs under "What breaks" fail to compile after the switch, so the switch PR respells their `usize` as `int` in the same commit. A respelling keeps every token count, so the front end does the same work on the new source, and the comparison of the base binary on the old source with the branch on the new measures the compiler, not the program.

**Measuring.** Before the PR is ready: `cargo xtask bench measure --workspace <base checkout> --save base.json` and the same for the branch, then `cargo xtask bench compare`. Once ready, read the `front-end instruction counts` comment, and report any stage above 1% on any program to the owner as a blocker. If the allocation gate trips, stop and report: no `Counters-Accepted:` trailer without the owner. Base emitted Rust comes from the base commit's probe binary, as `docs/pull-requests.md` "Probe binaries" says.

**The emitted Rust's run time.**

- A counted loop, `for i in 0..rt::length(items.len()) { total += items[rt::index(i)] }`, compiles to **the same machine code** as `for i in 0..items.len() { total += items[i] }`: rustc folds the two functions into one symbol (`sum_plain = sum_checked`). The range proves `i` non-negative and in bounds, so LLVM removes both checks and vectorizes.
- A random access, `items[rt::index(i)]`, adds `test; js` before the bounds check, which macro-fuse and never branch, and a stack adjustment for the cold call. On a loop that does nothing but random reads of a 4096-item vector, the best of 12 runs was 125.3 ms against 121.6 ms for `items[i as usize]`, about 3%, on a machine under load (load average 9 on 18 cores).
- `crate::rt::length` is free for a slice or a vector of sized items, and one fused `test; js` per call for a string, a map or a zero-sized slice (see "A length to `int`"). `crate::rt::count` and `take(rt::index(n))` add one compare per call, not per item.

## Order of work as pull requests

RFC 0074 has landed, so the help texts, docs and pages say `int` from the start. Each PR runs the narrowest working commands, regenerates and reads its snapshots, and checks the counters. Each PR updates, in the same commit, every statement of `AGENTS.md`'s reference files, `docs/language.md` included, that its change makes false.

1. **The `{index n}` conversion and `prelude/runtime/index.rs`, with no form using it. This PR is the prototype: it is built first, before the owner accepts the RFC, as was chosen for RFC 0074.** The `conversion` field on `Piece::Placeholder`, the three argument shapes in the emitter, the closure for a template used as a value, the negative-literal check with its new code, wording and explanation page (decision 7(a)), the `index` `RuntimePart` row and `NAMED` entry, `{index? n}` with its `option` check, the `int` parameter check of both placeholders, and `{index n}` and `{index? n}` in the placeholders row of `docs/language.md:1078`. Unit tests on the parser and emitter, and check and emit fixtures through a test-only `rust` template: a variable index, a literal, a negative literal in `(let (x (deref (at items -1))) ...)`, `(+ 1 (deref (at items -1)))` and `(println (deref (at items -1)))`, a value use, `v[i] += 1` through a test template shaped like `at`, and through a test template shaped like `get`, a literal, a variable and a negative literal giving `none`, a collection that is a call result, a collection argument that prints, which must print whatever the index, a literal outside `i32`'s range, `(get items -4294967295)`, and an `if` whose branches are such literals, both giving `none`, `{index? n}` in a template that returns no `option` rejected, and `{index n}` at a `usize` parameter and at a type variable rejected. Every one of them runs. It settles the design's compiler risks before the owner decides: that both `at`-shape paths keep matching with `{index n}`, and that the negative-literal check reaches every literal. No output change, no label.
2. **`convert` and its help.** `Integer`'s supertraits, with the `range_over_generic_integer` emit snapshot they change, the `RUNTIME_SUPERTRAITS` row, `prelude/runtime/convert.rs` with its `RuntimePart` row, its `NAMED` entry and a `REQUIRED` row on `Integer`, because a concrete call writes `crate::rt::convert::<i64, _>(count)` and never names `crate::rt::Integer`, `convert`, the `convert` page, check and run fixtures for widening, narrowing, overflow and a generic caller, a run fixture whose only runtime name is a concrete `convert`, and the `mismatch-help` row for expected `int` and found `usize`, with the precedence of "One help per error", recorded in `docs/language.md` beside the other `mismatch-help` text, building `mismatch-help` itself if RFC 0070's build has not. Its fixtures cover a written `usize` passed as `i64`, which gets the help, an `i64` passed as `u8`, which gets none, and the literal-default help winning. None of the 8 existing integer-pair snapshots changes (see the table under "Rust interop"). Adds behavior and breaks nothing, so no label. It lands before the switch so the switch can use `convert` in `vector_parameter_chains`.
3. **The switch.** In the prelude: `len`, `enumerate`, `take`, `at`, `windows`, `remove-at!`, `slice-get` and `subs`, and `count-items` and `count-function` through `crate::rt::count`, with its `RuntimePart` row. In the checker: the five kernel sites, and the deleted branch at `open_collections.rs:191-195`. `prelude/runtime/length.rs` with its `RuntimePart` row. In the emitter: `ELEMENT_READ`. Respell the 65 fixture sources as `int`, the corpus programs, the `lent_temporaries` unit-test program, and the `L0413`, `L0415` and `L0444` fixes and erroneous examples; convert in `vector_parameter_chains`; rewrite `check/closure_iterator_receiver.knv` around `max`, whose template is `{0}.max()`. Add check fixtures for `(set! (at v -1) 9)`, `(assoc v -1 9)` and `(assoc-in grid (0 -1) 7)`, each reporting the new negative-literal code. Rewrite the prelude prose listed under "What breaks" and the `usize` page to state the current rule only (see "Migration aids removed"), with every new or changed example binding through `let`, not `local`, `L0458.md:9-11`, and `docs/language.md` `:97`, `:174`, `:208`, `:282`, `:283` and `:334`. Run the emit diff of every corpus entry against the base commit's probe binary, `--test stages agree_fixtures`, and `cargo xtask agree --seeds 0..200`, and read each hunk. `release:minor`.

Item 454 of the project's plan ("one `len` for collections and iterators, remove `count`"): if it lands before PR 3, PR 3 applies `crate::rt::length` to `len`'s collection path and `crate::rt::count` to its iterator path, and drops the `count` rows. If it lands after, it carries both. The `enumerate` traits note under "The emitted Rust of each form" applies to it.

## Decisions for the project owner

1. **Does `get` with a negative index panic or return `none`?**
   - (a) Panic, as `at` does, through the same `{index n}`. One conversion for every index, in Go's and Swift's model, where a negative index is always a bug; `get`'s `none` keeps meaning "no item there yet", past the end.
   - (b) `none`. **Decided by the owner.** Knave takes `get` from Clojure, where `(get [1 2] -1)` is `nil` and `nth` throws, and `get`'s contract becomes "never panics on an index". It needs a second conversion at `get`, `{index? n}`, which emits `items.get(usize::try_from(i).unwrap_or(usize::MAX))`, and a bug that computes `-1` reads as a missing item. Under (b), `(get items -1)` is valid, so it gets no compile-time error either: decision 7's check applies to `{index n}` only.
2. **When does the negative check run?**
   - (a) Always, as Rust's bounds check does. **Decided by the owner.** Release builds of `take` and `windows` would otherwise take a negative count as a huge one.
   - (b) Debug only, as Rust's overflow check does: the release build emits `i as usize`, option C.
3. **How does a length become `int`?**
   - (a) `as i64`, with a stated limit. Free, what Rust programmers write, and lossless for every collection of items of at least one byte. A vector of zero-sized items longer than `i64::MAX`, which `collect` over a folded range can build at once, gives a negative length; `len` and `(count (iter c))` state the wrap as a limit.
   - (b) A checked runtime call around every `len`, panicking `length does not fit in int: <n>`, with no doc caveat. **Decided by the owner.** Free for a slice or a vector of sized items, one fused compare for a string, a map or a zero-sized slice, and every `len` reads as a runtime call.
4. **How does a count become `int`?** Unlike a length, a count of a range-derived iterator exceeds `i64::MAX` instantly (see "A count to `int`").
   - (a) Checked, through `crate::rt::count`, which panics with "attempt to count with overflow". **Decided by the owner.** One compare per call, and no silently negative count.
   - (b) `as i64`, with the wrap stated as a limit of `count`.
5. **`convert`: in this RFC, and under which name?**
   - (a) `convert`, integers only, target from context, checked, as designed above. **Decided by the owner.** This RFC creates the need, and floats can join the name later through the same trait-bound signature.
   - (b) Not in this RFC: a user writes a `rust` cast until a conversion RFC. The help then has nothing to name.
   - (c) Another name: `as`, which in Rust truncates silently and would teach the wrong meaning; or `into`, which in Rust is infallible.
6. **The panic message for a negative index or size.**
   - (a) One message for both roles: `attempt to use a negative value as an index or a size: -1`. **Decided by the owner.** One placeholder, one function, one message.
   - (b) `attempt to index with a negative value: -1` for every form, though `take` and `windows` take a count.
   - (c) Two placeholders, `{index n}` and `{count n}`, with a message each. Two rules for one conversion.
7. **How is a negative index literal reported?**
   - (a) A new code, worded for both roles as decision 6's panic is: `` the value `-1` is negative ``, label "an index or a size is at least 0", with its explanation page. **Decided by the owner.** `-1` fits `int`, so the literal is not out of range; the rule is about indexes and sizes, and `{index n}` reports it for `(take items -1)` and `(windows items -1)` too.
   - (b) Reuse `L0406`, reworded for this case, and extend `L0406.md`.
8. **Does the mismatch also point at the written type?** This applies where the row fires: a value whose `usize` type is written in a signature, a field or a `the` reaches an index form, so the found `usize` comes from a written annotation, and the right fix is often there.
   - (a) The help, and nothing more. When the owner decided, the help named two fixes, `convert` and respelling the written type. **Decided by the owner**, then narrowed by the owner's standing rule against migration aids: the help names only `convert` (see "Migration aids removed").
   - (b) Also add a secondary label "`usize` is written here" to every `L0401` whose found type comes from a written annotation. A general compiler change to `L0401`, worth its own item.
9. **Does the corpus keep a `usize` program?** The switch respells `usize` as `int` in four corpus programs. `literal_inference` keeps its chain on purpose.
   - (a) Respell all four. **Decided by the owner.** The corpus measures the language as users write it.
   - (b) Keep one on `usize` with `convert` at each index, to measure interop code. Fixtures already cover `convert`.
10. **A help for the reverse pair, expected `usize` and found `int`, as later work?** Three of the four breaking patterns are this pair or a function type over it: a written `usize` result or field fed by `len` or `count`, and a `(fn ... usize)` fed by `count`. Today they get a bare `L0401`. A row for the pair cannot ship in this RFC, because a found `int` can be a literal's default, as `(defn port () 8080)` used where a `usize` and an `i32` are expected, and there `convert` and respelling are both wrong fixes. A help needs the mechanism named under "Why one pair, not every integer pair": at the error, tell whether the found expression is a call whose callee's result is a literal generic that defaulting set to `int` (`TypedProgram::literal_generics`, `src/types/typed.rs:384`, and `DefaultingRule`'s tables, `src/check.rs:223-224`), and skip the row when it is. The function-type pattern also needs a row that matches inside a function type, or a help keyed on the result position. Its cost is a lookup per `L0401` of an integer pair, on the error path only.
   - (a) Later work: record it as its own item, built after this RFC with that mechanism, which would also let the row cover every integer pair. This RFC ships without it; the switch's breaks in these patterns land on the owner, the only user, and each is a respelling the error's label points at.
   - (b) In this RFC, in PR 2: the mechanism and a reverse row, which grows PR 2 with checker work on the error path.
   - (c) Never: the bare `L0401` names both types, which is enough. **Decided by the owner.**

## Prototype

PR 1 is the prototype. It is built first, before the owner accepts the RFC, as was chosen for RFC 0074. PR 1 starts from `main`, which carries RFC 0074. It settles the design's compiler risks: that both `at`-shape paths, `borrowed_index_template` and `ELEMENT_READ`, keep matching with `{index n}`, and that the negative-literal check reaches every literal at an `{index n}`, under `deref` and inside `println` included.

The evidence so far: today's emitted Rust and diagnostics come from a build of `b00494c5f`. The machine code, timings and the generic `convert` come from Rust outside the repository. Option B's expansion cost comes from callgrind runs of the v0.15.0 binary on scratch programs. The `{to}` turbofish was probed on `b00494c5f`: `(the int (convert x))` with `x` a `usize` emits `crate::rt::convert::<i64, _>(x)`.

## Recommendation

Option A: every user-facing index and size in the prelude and the kernel forms becomes `int`, through a template conversion, `{index n}`, that emits a bare literal, reports a negative literal at compile time, or calls an always-on (decision 2(a)) `crate::rt::index`; `get` uses `{index? n}`, so a negative index gives `none` (decision 1(b)). `len` converts through a checked `crate::rt::length` (decision 3(b)), and `count` through a checked `crate::rt::count`. `convert`, written in the prelude with a runtime that compiles for any two integer types, serves the `usize` that Rust interop still brings, and its help is a prelude `mismatch-help` row. The emitted Rust gains no clone or allocation, a counted loop compiles to the same machine code, and the common literal index reads as today.

## Decision

The project owner decided:

- **Decision 1: (b).** `get` with a negative index returns `none` and never panics; `at` panics. `get` uses the `{index? n}` conversion, which emits `items.get(usize::try_from(i).unwrap_or(usize::MAX))` for a variable index.
- **Decision 2: (a).** The negative check of `{index n}` runs always, in debug and release builds.
- **Decision 3: (b).** Every length conversion is checked: `len` and `(count (iter c))` emit `crate::rt::length(items.len())`, which panics `length does not fit in int: <n>`. `len`'s docs carry no caveat.

- **Decision 4: (a).** Counts are checked: `count` of an iterator, and `count` as a value, go through `crate::rt::count`, which panics `attempt to count with overflow: the count is <n>` when the count does not fit in `int`.

- **Decision 5: (a).** `convert` ships in this RFC, in PR 2: integers only, its target type from context, checked, panicking `` attempt to convert <n> to `<type>` with overflow ``.

- **Decision 6: (a).** One panic message for every `{index n}`, for an index and a size alike: `attempt to use a negative value as an index or a size: -1`.

- **Decision 7: (a).** A negative integer literal at an `{index n}` reports a new code, `` the value `-1` is negative ``, labeled "an index or a size is at least 0", with its own explanation page. `L0406` and its page do not change.

- **Decision 8: (a).** No secondary label points at the written type. The help first named two fixes; the owner's standing rule against migration aids removed the respelling one, so it names only `convert`.

- **Decision 9: (a).** The switch PR respells `usize` as `int` in all four corpus programs that break, `evaluator`, `warehouse`, `matrix_geometry` and `generic_types`. `literal_inference` keeps its `usize` chain.

- **Decision 10: (c).** No help for the reverse pair, expected `usize` and found `int`, now or later.

The owner restated a standing rule with decision 10: no migration aid of any kind, because the owner is the only user. "Migration aids removed" below records what the audit took out.

## Migration aids removed

The RFC keeps what helps new code and removes what only eases moving code written against `usize`:

- **The respelling clause of the `convert` help.** The help read "...or write `int` where `usize` is written for it". That clause served only a program that typed a position `usize` before the switch. The help now reads "convert it with `(convert {value})`, which panics when the value does not fit", which serves new code that gets a `usize` from Rust. The `mismatch-help` row stays for that reason.
- **The reverse-pair help, decision 10.** Its patterns, a `usize` result, field or function type fed by `len` or `count`, arise only in code written before the switch. Decided (c): never.
- **Prose that names the old type.** The rewritten docstrings, the `usize` page, the explanation pages and `docs/language.md` state the current rule only. None says that an index "used to be" a `usize`, that `usize` "is no longer" the index type, or how to move code. The `usize` page says what `usize` is for now: Rust's size type, for Rust interop.

Kept, because they serve new code: `convert` and its help, the new negative-index code and its page, the checked `length`, `count` and `index` panics, and `{index? n}` for `get`. The respelling of fixtures, corpus programs, unit tests and explanation examples is not an aid: it keeps the repository's own tests compiling, and ships no text to users.

## Rejected options and why

- **A `mismatch-help` row over every pair of integer types:** it would tell a user whose found `int` is a literal's default to convert or to respell a type that is written nowhere. Telling those errors apart needs the defaulting tables at the error, for pairs this RFC does not create.
- **B, a prelude `index` function with macro wrappers:** cannot reach the kernel sites, loses `v[i] += 1`, misses literals that another macro produces, doubles each form's definitions, and adds 20,000 or more expansion instructions per index call.
- **C, `as usize` with no check:** `take` and `windows` would silently treat `-1` as 2^64 - 1, and `at`'s panic would read `the index is 18446744073709551615`.
- **D, an `index` trait over `int` and `usize`:** two index types for the user, generic index forms, and a type variable in diagnostics. The owner's direction is one index type.
- **E, the conversion written into the template text:** breaks both emitter paths that match `at` by its shape and wraps every literal index in a runtime call.
- **`as i64` for `count`:** a range-derived count wraps negative instantly.
- **`.enumerate().map(|(index, item)| (index as i64, item))`:** adds a closure the source does not write, for the same machine code as `(0_i64..).zip(items)`.
- **A `convert` with `To: TryFrom<From>` bounds:** fails to compile in generic Knave code, because the Knave `integer` bound does not imply it.
- **An emitter turbofish for `convert`:** a template type placeholder already writes it.
- **The conversion help worded in `TypeProblems`:** it would name a prelude function in Rust.
- **`{index? n}` as a closure around the template, `usize::try_from(i).ok().and_then(|index| items.get(index))`:** moves the collection into the closure, so `get` on a call result or a temporary is a rustc error, a lent temporary's owner cannot be bound outside, and the collection runs only for an index of 0 or more.
- **A runtime function for `{index? n}`, such as `crate::rt::position(i)`:** costs the same as `usize::try_from(i).unwrap_or(usize::MAX)` but adds a runtime part, and a reader must look up what it returns for a negative value.
- **Keeping `usize` and adding only the conversion help:** leaves the problem, and the inference that spreads `usize` through programs.

## Rule

[docs/language.md: Types](../docs/language.md#types) and the collection forms: an index, a size, a length and a count are `int`; a negative index or size panics, always, and a negative literal there is a compile-time error, except in `get`, where a negative index gives `none`; a length or a count above `int`'s range panics; an `enumerate` index past `int`'s range wraps in a release build, a stated limit; `usize` is for Rust interop, through `convert`. The placeholders row of the `rust` template rule table (`docs/language.md:1078`) gains `{index n}` and `{index? n}`.
