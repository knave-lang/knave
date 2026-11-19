# RFC 0077: Field reads generic over every struct with the field

**Status:** Proposed
**Date:** 2026-11-18

Code references are against `origin/main` at `0c579cb4a`. Each knave output marked "on `main`" comes from the release binary of that commit. Each emitted Rust block marked "after the build" is the shape this RFC asks for, and the prototype compiled and ran it with rustc 2024.

## Problem

A field read names its struct only through the value's type ([docs/language.md: Field keywords](../docs/language.md#field-keywords)). When the value's type is still unknown at the read, the checker looks the field up by name across every struct of the program (`unique_owner`, `src/types/infer.rs:3585`). One struct with the field fixes the type. Any other count is `L0407`:

```lisp
(defstruct user :name str)
(defstruct animal :name str)

(defn hello (x) (str "hello " (:name x)))

(defn main ()
  (println (hello (->user "ana")))
  (println (hello (->animal "rex"))))
```

On `main`:

```text
error[L0407]: type annotations needed
 --> one.knv:4:32
  |
4 | (defn hello (x) (str "hello " (:name x)))
  |                                ^^^^^ cannot tell which struct's `name` this is
  |
  = help: name the struct in a signature, as in `(fn ((ref point)) _)`
```

The help is no fix here: a signature that names `user` rejects `animal`. No written type says "any struct with a `name`", so the program needs two functions with one body.

The same lookup gives two smaller faults:

- **No struct with the field is `L0407` too.** `(defn size-of (x) (:size x))` with no `size` field anywhere reads "cannot tell which struct's `size` this is", where the real fault is that no struct declares `:size`.
- **The read decides at its position.** The lookup runs when the checker reaches the read, so a later form in the same body that fixes the value's type comes too late.

## Options

### A. A field bound, emitted as a plain trait when the reads agree and as one function per struct otherwise (recommended, and the owner's model)

The checker gives an unknown owner a field bound, `(a (has :name r))`: `a` is a struct with a field `name`, and `r` is the type a read of that field gives. The bound behaves as a trait bound does. It generalizes, it is checked at each call, and pinning removes it when every call passes one struct. Every read has exactly the type that a read of its struct gives on `main`. The emitter writes a plain Rust trait for a field when every struct that reaches its readers gives one read, and otherwise one ordinary function per struct, each a known read (Rule 2.4).

Decision 1, after the build. Both structs store `:name` as a borrow of one type, so the trait is plain, with the stored borrow's lifetime as its parameter:

```rust
trait HasName<'u> {
    fn name(&self) -> &'u str;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct User<'a> {
    name: &'a str,
}

impl<'a> HasName<'a> for User<'a> {
    fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Animal<'a> {
    name: &'a str,
}

impl<'a> HasName<'a> for Animal<'a> {
    fn name(&self) -> &'a str {
        self.name
    }
}

fn make_user(name: &str) -> User<'_> {
    User { name }
}

fn make_animal(name: &str) -> Animal<'_> {
    Animal { name }
}

fn hello<'a, A: HasName<'a>>(x: &A) -> String {
    format!("hello {}", x.name())
}

fn main() {
    crate::rt::restore_default_sigpipe();
    println!("{}", hello(&make_user("ana")));
    println!("{}", hello(&make_animal("rex")))
}
```

`knave types` prints `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`.

Decision 2 adds a struct whose `name` is an `i32`:

```lisp
(defstruct user :name str)
(defstruct animal :name str)
(defstruct product :name i32)

(defn uppercase-name (x) (upper-case (:name x)))
(defn describe (x) (str "name: " (:name x)))

(defn main ()
  (println (uppercase-name (->user "ana")))
  (println (uppercase-name (->animal "rex")))
  (println (describe (->user "ana")))
  (println (describe (->product 7))))
```

`uppercase-name` receives `user` and `animal`, whose reads agree, so it keeps the trait above. `describe` receives `user` and `product`, whose reads are `(ref str)` and `i32`, so it emits one function per struct. After the build:

```rust
fn uppercase_name<'a, A: HasName<'a>>(x: &A) -> String {
    x.name().to_uppercase()
}

fn describe_user(x: &User<'_>) -> String {
    format!("name: {}", x.name)
}

fn describe_product(x: &Product) -> String {
    format!("name: {}", x.name)
}

fn main() {
    crate::rt::restore_default_sigpipe();
    println!("{}", uppercase_name(&make_user("ana")));
    println!("{}", uppercase_name(&make_animal("rex")));
    println!("{}", describe_user(&make_user("ana")));
    println!("{}", describe_product(&make_product(7)))
}
```

`Product` gets no impl, because no trait reader receives it. Each copy is `describe`'s first-pass types instantiated at that struct, so it takes a borrow as the generic `describe` would. `knave types` prints `(type describe (where ((a (has :name r)) (r display)) (fn ((ref a)) string)))`, and each call takes its own struct's read.

### B. `defstruct` writes a trait and an impl per field

The prelude macro would write `(deftrait has-name (type field-type) (get-name ...))` and an `impl` for each field of each struct, and a keyword read would expand to `get-name`. Two structs that share a field would both define `has-name`, which is the clash that made keyword reads an expander feature in [RFC 0038](0038-field-keywords.md). Every field of every struct would emit a trait and an impl, so every program's Rust would change. A macro runs before the checker, so it cannot tell a read whose struct is known, which should stay `x.name`, from one whose struct is not.

### C. One copy of the function per struct

The emitter would write `hello_user` and `hello_animal` for every generic reader. Calls would name a struct the source never wrote, and a function value such as `(map users hello)` would need the copy picked by its element type. Option A takes this emission for a reader whose structs give different reads, and keeps a trait where one trait serves.

### D. A written accessor parameter

The program would pass the reader as a function, `(hello :name x)`. This is possible today and changes the signature of every reader. Decision 1 rules it out: the unannotated `hello` must compile.

## Recommendation

Option A, by refinement 14. It is the owner's model. A Rust programmer writes a plain trait for a capability that every type gives in one shape, and separate functions for types that differ. Every read keeps its known type at every call, so adding code changes no call's types, only the Rust's shape and, by the pin rule of Rule 2.4, the bound a reader prints. It adds a bound kind to the checker in place of a kernel form. Every program whose reads have zero or one candidate struct, or whose calls pin each reader to one struct, keeps its Rust byte for byte.

## Decision

The project owner decided on 2026-11-08:

1. `(defn hello (x) (str "hello " (:name x)))` with no annotation compiles and accepts every struct with a `name` field, such as `(defstruct user :name str)` and `(defstruct animal :name str)`.
2. Fields of one name may have different types, such as `(defstruct user :name str)` and `(defstruct product :name i32)`, so `(defn uppercase-name (x) (uppercase (:name x)))` is generic over the field's type.
3. The model: one Rust trait per field name, such as `HasName` with an associated `FieldType` and `fn get_name(&self) -> &Self::FieldType`, and `impl HasName for Animal { type FieldType = String; ... }` for each struct.
4. Before 1.0 the owner is the only user, so the change needs no migration aid.

Decision 2 spells the prelude function `uppercase`, which is `upper-case` in the prelude (`prelude/text.knv:789`). This RFC reads the decision with that name.

The project owner refined decision 3 on 2026-11-18:

5. **An associated type takes the field's name.** `HasName` declares `type Name`, not `FieldType`. A shared name would name two items on an owner with two field bounds, which rustc rejects as `E0221`, "ambiguous associated type".
6. **The getter returns the type a known read gives:** the stored borrow for a field the struct stores as a borrow, the value for a small Copy field, and a borrow of the field otherwise. A getter that always returns `&Self::FieldType` would end a stored borrow's lifetime at the struct's borrow: `(let (n (name-of (->user "ana" 3))) (println n))` prints `ana` on `main`, and becomes rustc's `E0716` once a second struct declares `:name str`. It would also read a Copy `int` field as `(ref int)`, where `(infer (:age u))` is `"int"` on `main`.
7. **The simplest trait the program needs.** When every struct with the field gives the same read type, the trait is plain, with no associated type, and the read takes that type at once. The trait takes a lifetime parameter only when those structs store the field as a borrow, as `trait HasName<'u> { fn name(&self) -> &'u str; }`, because a plain `fn name(&self) -> &str` ties the stored borrow to the struct's borrow, which is `E0716` for a name read from a temporary. Owned and Copy fields keep the plain form, as `trait HasAge { fn age(&self) -> i64; }`.
8. **No generic associated type.** When the read types differ, the trait takes an associated type: `type Age` with `fn age(&self) -> Self::Age` when every read is a value, `trait HasTag<'u>` with `type Tag: ?Sized` and `fn tag(&self) -> &'u Self::Tag` when every struct stores the field as a borrow, and `fn item(&self) -> &Self::Item` otherwise, mixed kinds and generic structs included. A generic associated type, `type Name<'s>`, gives `E0502` for a `mut` parameter that also reads the field, `E0597` for chained reads and for its `for<'s>` repair, which implies `'static`, and `E0716` for a fixed read. The owner accepts that a reader called with mixed kinds borrows its parameter for a stored-borrow read, a cost that only programs failing on `main` with `L0407` can reach, as Rule 3 says.
9. **Getters are named after the field,** `name()` and `name_mut()`, as the Rust API guidelines name getters, not `get_name`.

The project owner refined decision 3 again on 2026-11-18, after the third review:

10. **A field trait takes its lifetime parameter whenever the read type carries the struct's lifetime,** a stored borrow or a value that holds one, as `trait HasOwner<'u> { fn owner(&self) -> User<'u>; }` for `(defstruct car :owner user)` with `(defstruct user :name str)`. This refines refinement 7. On `main`, `(let (o (owner-of (->car (->user "ana")))) (println (:name o)))` prints `ana` through `fn owner_of(x: Car<'_>) -> User<'_>`. A plain `fn owner(&self) -> User<'_>` ties the result to the borrow of the car, which is `E0716` for that program once a second struct declares `:owner user`.
11. **A field's shape comes only from the structs that some instantiation of a generic reader of that field takes,** a call or a function value such as `(map (iter cs) age-of)`, the same structs that get an impl, so a struct that no reader receives changes nothing. With the shape taken from every struct that declares the field, `(defstruct user :age str)`, declared and never passed, would turn a plain `:age int` reader into the mixed shape, change `(infer (age-of c))` from `int` to `(ref int)`, and make `(collect (map (iter cs) age-of))` under a `(vec int)` signature `L0401`.

The project owner refined decision 3 again on 2026-11-18, after the fourth review:

12. **Each reader settles its read type from its own instantiations,** calls and function values. The trait's shape stays per field, by refinement 11, but a reader whose instantiations all give one Copy value reads through the shared getter and copies it out, as `fn age_of<A: HasAge<Age = i64>>(x: &A) -> i64 { *x.age() }`, so it keeps `(has :age int)`. Only a reader that itself takes mixed kinds borrows its parameter. Under refinement 11 alone, `(defn show-age (x) (str "age " (:age x)))` called with a `user` whose `:age` is `str` would turn `:age` mixed for every reader, so `age-of`, called only with `cat` and `dog`, would return `(ref int)`, and `(collect (map (iter cs) age-of))` under a `(vec int)` signature would be `L0401`.

The project owner refined decision 3 again on 2026-11-18, after the fifth review:

13. **A reader copies its read out whenever every instantiation's read is a value or a stored borrow,** under a `Copy` bound on the associated type, as `fn name_of<A: HasName>(x: &A) -> A::Name where A::Name: Copy { *x.name() }`. Only a reader with an instantiation whose read borrows the struct, an owned field that the size rule does not copy, such as a `string`, a `vec` or a large Copy struct, or a generic struct's parameter field whose argument is open or reads so, borrows its parameter. This extends refinement 12 to a reader whose instantiations give several Copy reads, which refinement 12 left without a rule, and narrows refinement 8's accepted cost from a reader that takes mixed kinds to a reader with an instantiation whose read borrows the struct. Borrowing such a reader would break Rule 3.2's compound form: `(defn grow! (x) (set! (:age x) (+ (:age x) (:age x))))` with `cat` (`int`) and `drone` (`u8`), and a `book` with `:age string` that makes the field mixed, would bind `let value = x.age();`, a borrow that the `age_mut` call then conflicts with, rustc's `E0502`. Copied out, it emits `let value = *x.age(); *x.age_mut() += value;` and compiles.

The project owner decided the direction on 2026-11-18, after the ninth review:

14. **The trait emits only when one read serves every struct, and every other reader emits one function per struct.** A field's readers use a plain trait when every struct that reaches them gives the same read, in type and kind, in the plain shapes of refinements 7 and 10, with a lifetime parameter for a stored borrow or for a value that holds one. Otherwise a reader emits one ordinary function per struct it reaches, each a known read, transitively through generic callers. Every read has exactly its known-read type in both, so no type changes when code is added, and only the Rust's shape switches. This replaces the shapes with an associated type, the value shape, the stored-borrow shape and the mixed shape, together with `ReadAs`, the copy-out rules, the per-function settling of modes and the second pass. It supersedes refinement 5, since no trait has an associated type, and refinements 8, 12 and 13. It narrows decision 3's one trait per field to the fields whose reads agree. Refinements 6, 7, 9, 10 and 11 stand. Refinement 8's reason for no generic associated type still holds, and its accepted cost goes away: each copy keeps its struct's lifetimes.

The project owner refined refinement 14 on 2026-11-18, after the tenth review:

15. **Inside a reader, a generic struct's parameter field always reads as a borrow, `(ref a)`,** at every instantiation, on the sole-candidate path and on the bound path alike, as `main` reads it today. So `get` over `(boxed int)` reads `(ref int)` whether `boxed` is the only struct with `:title` or a `film` that no call passes to `get` declares it too, no program that compiles on `main` changes type, and an unused struct changes nothing. This narrows Rule 2.3 for generic structs. A known read outside a reader keeps `main`'s rule, so `(:title (->boxed 5))` in `main` still reads `int`.

## Rejected options and why

- **B: a trait per field from `defstruct`:** two structs that share a field clash, every program's Rust changes, and a macro cannot see whether a read's struct is known.
- **C alone, one copy per struct for every reader:** a reader whose structs agree would grow by one function per struct where one plain trait serves. Refinement 14 keeps the trait there.
- **D: a written accessor:** decision 1 requires the unannotated function to compile.
- **A field bound on every unknown owner, one candidate included:** a reader of the only struct with `:name` would print a `where` and emit a trait for one implementor, where a concrete type says the same thing and reads better. Rule 5 keeps one candidate concrete.
- **One associated type name, `FieldType`, for every field trait:** an owner with two field bounds makes `A::FieldType` ambiguous, `E0221`. Refinement 5 named it after the field, and refinement 14 leaves no associated type.
- **A getter that returns `&Self::FieldType` for every field:** a stored borrow's read would end at the struct's borrow, and a Copy field would read as a borrow, unlike the known read. Refinement 6 replaces it.
- **A generic associated type for the read, `type Name<'s> where Self: 's`:** a `mut` parameter that also reads the field is `E0502`, a chained read is `E0597`, the `for<'s>` bound that would repair them implies `'static`, and a fixed read merges the stored borrow's lifetime with the parameter's, which is `E0716` for `(defn trimmed (x) (trim (:name x)))` on a temporary.
- **Traits with an associated type for differing reads (refinements 8, 12 and 13):** one getter that served a value, a stored borrow and a borrow of the struct gave a read whose mode followed other calls. Each of nine reviews found blockers in it: a borrow layer that some readers saw and others did not, a second check to settle it, copy-out rules for each result type, a conversion trait for each storage depth, and outlives bounds per shape. Refinement 14 replaces it.
- **The shape taken from every struct that declares the field:** an unused struct, or one in a library that never reads the field, would change the reader's Rust. Refinement 11 counts only the structs that reach a reader.
- **Candidates limited to the structs the reading module sees:** the checker has no field visibility rule today. A known read of a private struct's field from another module compiles on `main`, so a limit only for generic reads would be an exception. It would also break a library reader: `(defn hello (x) (str "hi " (:name x)))` in a library compiles on `main` for its caller's `user`.

## Rule

The build states these rules in [docs/language.md: Field keywords](../docs/language.md#field-keywords), [Types](../docs/language.md#types), [Ownership](../docs/language.md#ownership), [Names in Rust](../docs/language.md#names-in-rust) and [10. User-defined traits](../docs/language.md#10-user-defined-traits), in the explanation of `L0407`, and in a new explanation of `L0467`. It replaces the two sentences of Field keywords that say a function needs a signature when two structs share a field name, and it rewrites the paragraph of `expand/src/diagnostic/explanations/L0407.md` that says a field read or update finds its struct by the field's name: a read with no candidate is `L0408`, a read with several gives a field bound, and only an update with several, or a bound nothing fixes, stays `L0407`.

### 1. A call whose struct fails the body's use

The bound's type `r` is the read's type, and each struct gives there the read that Rule 2.3 gives a read whose owner is open, which for a struct without parameters is its known read: `int` for `cat`'s `:age int`, `(ref str)` for `user`'s `:name str`, `(ref string)` for `book`'s `:title string`. A body that uses the read through a trait, as `str` uses `display`, gives `r` that trait bound. A body that fixes the read, as `upper-case` fixes it to `(ref str)`, adds a requirement, which a struct meets when its known read becomes the fixed type by the adjustments a known read gets at that use, which [Ownership: Adjustment](../docs/language.md#ownership) lists: one added borrow, removed extra borrows, a `string` viewed as `str`, a `vec` viewed as a slice, and a copy out. A field read's owner position does not fix `r` either. When the reads of `(:p x)` differ, the inner read `(:tag (:p x))` gives `r` a field bound of its own, `(r (has :tag (ref string)))`, as Rule 2.10's chained reads do, even when `:tag` has one candidate, and a field bound looks through borrows, so an `a1` that stores `:p (ref spot)` and an `a3` that stores `:p (ref (ref spot))` both meet it.

So `(defn r (x) (show-ref (:age x)))`, with `show-ref` taking `(ref int)`, takes `cat` and `dog`, as `(show-ref (:age c))` does for a known `cat`. `(defn show-p (x) (describe-spot (:p x)))`, with `describe-spot` taking `(ref spot)`, takes an `a1` that stores `:p (ref spot)` and an `a2` that holds `:p spot`. `uppercase-name` takes `user`, a `person` with `:name string` and a `holder` with `:name (ref string)`. Each copy of Rule 2.4 is a known read, so it needs no conversion: `describe_spot(x.p)` for `a1` and `describe_spot(&x.p)` for `a2`.

When `r` agrees across the structs and holds no open type variable, the bound prints that type, as `(has :name (ref str))`. A read that holds a caller's open variable, as `get` called from a generic `both` reads `(ref v)`, prints as `r`, as differing reads do, so the text never names a variable of another function. When it differs, the bound prints `r` and one more element per fixed read, and the `where` lists `r` as it lists every variable that only a bound names, as `(infer =)` lists `b`: `(type show-p (where ((a (has :p r (ref spot))) r) (fn ((ref a)) string)))`. A body that fixes one field's read twice, as `(str (upper-case (:name x)) (as-len (:name x)))` with `as-len` taking `(ref string)`, prints both, `(has :name r (ref str) (ref string))`, and a struct must meet each. `tag-of` over `a1` and `a3` prints `(where ((a (has :p r)) (r (has :tag (ref string)))) (fn ((ref a)) string))`, where `r`'s own entry declares it. A `has` bound's type is determined by its owner, as an associated type is, so the rule that a variable named only in a bound is `L0407` (`check/cannot_infer_bound_argument.knv`) does not apply to it. The text pastes back. A written bound's type is the read's type, so a written `(has :age (ref int))` is `L0467` for `cat`, whose `:age` reads as `int`, while `(has :age int (ref int))` holds.

A call checks the bound against the struct it passes, and a struct whose read fails the type, a fixed read or a trait is `L0467` at that call, a new code of the type stage. `L0465` is the last code at `0c579cb4a`, `L0466` is on `origin/main` at `6702dbfae`, and no open pull request names `L0467`.

```lisp
(defn main ()
  (println (uppercase-name (->product 7))))
```

After the build:

```text
error[L0467]: `uppercase-name` needs a `:name` that reads as `(ref str)`, but `product`'s `:name` reads as `i32`
 --> two.knv:9:28
  |
9 |   (println (uppercase-name (->product 7))))
  |                            ^^^^^^^^^^^^^ this `product` has a `:name` of type `i32`
  |
 ::: two.knv:5:27
  |
5 | (defn uppercase-name (x) (upper-case (:name x)))
  |                           ---------- `upper-case` takes `(ref str)`, so `:name` must read as `(ref str)`
  |
  = note: `uppercase-name` accepts every struct whose `:name` reads as `(ref str)`: `user` and `animal`
  = help: pass a struct whose `:name` reads as `(ref str)`
```

A read type that lacks a trait the body needs takes the same code, with the trait in the title:

```text
error[L0467]: `describe` needs a `:name` that implements `display`, but `badge`'s `:name` reads as `(fn () int)`
```

The secondary label points at the form that brought the type or the trait, as `upper-case` and `str` here. The note lists the structs that meet the bound, in source order, and a struct from another module prints as two types of one name print in a diagnostic today.

**One diagnostic for a chain of calls.** When `(defn shout (y) (uppercase-name y))` passes its own generic owner on and `main` calls `(shout (->product 7))`, the bound reaches `main` through `shout`'s generalized bound. `L0467` labels the call in `main`, the first call that passes a struct. The secondary label stays on the form that fixed the type, inside `uppercase-name`, and a second note names the path: `` `shout` passes `y` to `uppercase-name` ``. No other call reports.

The message never names `HasName` or `name()`, so rustc's `E0277`, which a call with `Product` would give, never reaches the user. A struct that meets the bound reads as its known read at every use, so neither the trait reader nor a copy can fail in rustc.

A call with a value that has no such field reports the existing codes at the call, with a secondary label on the read and the same note:

| Argument | Code | Title |
| --- | --- | --- |
| a struct without the field, `(hello (->point 1.0 2.0))` | `L0408` | `` `point` has no field `:name` `` |
| an enum, `(hello shape/point)` | `L0410` | `` `shape` has no fields `` |
| any other type, `(hello 5)` | `L0410` | `` `int` has no fields `` |

### 2. Readable output

1. **Rust changes only for a function that reads a field from a generic owner.** A read whose owner is a known struct, from the start or after pinning, emits `x.name` as it does today. A program with no generic read after pinning emits no trait, no impl and no copy, so its Rust stays byte for byte what `main` emits.
2. **A reader's structs are the structs that reach it,** through its own calls and function values, through every generic caller that passes its owner on, and through every other member of its recursive group. So `(defn age-of (x) (:age x))`, called directly only with `dog`, also receives `user` and `cat` when `(defn show-age (x) (str "age " (age-of x)))` is called with them. A struct that declares the field and reaches no reader changes nothing, by refinement 11. When every struct that reaches a function whose owner reaches a field read, a reader or a generic caller that passes its owner on, is one struct, at any arguments, as `get` over `(boxed int)` and `(boxed string)`, its owner is that struct with its parameters generalized, which is what `main`'s sole-candidate path gives, so the reader prints, emits and runs as on `main` whatever other structs declare the field: `(type get (where (a) (fn ((ref (boxed a))) (ref a))))`, one `fn get<A>(b: &Boxed<A>) -> &A`, and an in-body `(infer (:title b))` of `"(ref a)"`. It takes no trait and no copy, and it takes `main`'s diagnostics too: the struct of a call that fails the body's use still counts as reaching it, so `(defn f (b) (show-ref (:title b)))` over `(boxed 5)` and `(boxed (str "x"))` is `main`'s `L0401` at the second call, ``expected `(boxed int)`, found `(boxed string)` ``, beside an unpassed `film` too. A generic caller `(defn wrap (b) (get b))` over the same calls keeps `main`'s `fn wrap<A>(b: &Boxed<A>) -> &A` and its in-body `(infer (get b))` of `"(ref a)"`, even when a `film` call makes `get` copy: `wrap` then calls the open copy `get_boxed_a`.
3. **Each struct's read of the field has a kind and a type, from the Fields bullet of [Ownership](../docs/language.md#ownership):**

   | Kind | Field | Read on `main` | Rust read |
   | --- | --- | --- | --- |
   | value | a function type, or a Copy type that the size rule of Ownership: Fields copies | the value | `i64`, `fn(&str) -> bool` |
   | stored borrow | `str`, a `(slice t)` that holds no parameter, `(ref t)` | the stored borrow, with the struct's lifetime | `&'a str`, `&'a [i64]`, `&'a T` |
   | borrow of the struct | any other type | a borrow of the field | `&String`, `&A` |

   `main` tests the size rule on the field's declared type at the read (`infer_field`, the `reads_field_by_copy` call at `src/types/infer.rs:3535`, the method at `:3548`), so a read whose owner is known reads `(:title (->boxed 5))` as `int`. Refinement 15 reads a generic struct's parameter field "as `main` reads it today". Rule 2.3 takes its "parameter field" as a field whose declared type is the bare parameter, which reads as `(ref a)`, and its "as `main` reads it today" as governing every other type that holds a parameter, so the two parts do not conflict. `main`'s first pass reads a field whose owner is open by the Fields bullet applied to its declared type with the struct's parameters open: a function type is a value, a `(ref ...)` is a stored borrow, and every other type that holds a parameter is a borrow of the struct unless the size rule copies it with the parameters open, as it copies `(option (ref a))` and `(boxed (ref a))`, which hold the parameter only under a borrow. That read applies wherever the owner is open when the checker reaches the read, on both paths: in a reader, in each of its copies, and in a function whose owner a later form fixes. A read whose owner is known keeps the known read. In every instantiation:

   | Declared field | Read with the owner open | The read at `int` |
   | --- | --- | --- |
   | `a`, `(option a)`, `(slice a)`, `(vec a)`, `(boxed a)`, `(option (fn (a) int))` | a borrow of the struct | `(ref int)`, `(ref (option int))`, `(ref (slice int))`, `(ref (vec int))`, `(ref (boxed int))`, `(ref (option (fn (int) int)))` |
   | `(fn (a) int)`, `(fn ((ref a)) int)` | the value | `(fn (int) int)`, `(fn ((ref int)) int)` |
   | `(option (ref a))`, `(boxed (ref a))`, which hold the parameter only under a borrow, so the size rule copies them with the parameter open | the value | `(option (ref int))`, `(boxed (ref int))` |
   | `(ref a)` | the stored borrow | `(ref int)` |
   | `int`, `(boxed int)`, which hold no parameter | as any other field | `int`, `(boxed int)` |
4. **The trait or the copies.** The readers of a field whose structs each give one read, in type and kind, use the field's trait when they all give the same read. Each other reader of the field emits one function per struct it receives. Two readers whose structs agree within each reader but not with each other, as `age-of` over `cat` and `dog` (`int`) and `label-of` over `user` and `admin` (`(ref str)`), both emit copies, because one trait has one getter. Every reader whose structs agree within it counts toward a field's agreement, whether or not it copies for another reason, so the choice never depends on itself. A reader emits one way: when any field it reads from a generic owner needs copies, it copies for every field, as `(defn card (x) (str (:name x) (:age x)))` over a `cat` and a `drone` whose `:name` is `i32` in both and whose `:age` is `int` and `u8` emits `card_cat` and `card_drone`. Its `i32` read of `:name` still counts, so `hello` over `user` and `animal` beside it copies too. A read whose type holds an open type variable, as `(boxed v)` and `(crate v)` reading `(ref v)` inside a generic `both`, counts as differing, because a plain trait cannot name `v`, and a trait with a type parameter would be a shape of its own. Such a reader copies, and each copy stays generic over the variable, as `get_boxed_a<A>` and `get_crate_a<A>`. The trait is plain, by refinements 7 and 10:

   | The reads | Trait | Bound |
   | --- | --- | --- |
   | value | `trait HasAge { fn age(&self) -> i64; }` | `(has :age int)` |
   | value that holds the struct's lifetime | `trait HasOwner<'u> { fn owner(&self) -> User<'u>; }` | `(has :owner user)` |
   | borrow of the struct | `trait HasTitle { fn title(&self) -> &String; }` | `(has :title (ref string))` |
   | stored borrow | `trait HasName<'u> { fn name(&self) -> &'u str; }` | `(has :name (ref str))` |

   An impl emits for each struct that a trait reader receives, as `impl HasTitle for Boxed<String>` for a `(boxed string)`, beside the structural trait impls of [8.2 Struct traits](../docs/language.md#82-struct-traits). A reader that copies needs no impl. The switch changes only the Rust: `(infer (age-of c))` is `int` whether `age-of` uses the trait or copies, and a reader that gains a struct with another read keeps the types at its calls and starts emitting copies. What the reader prints follows the pin rule: `r` prints as the agreed read while the structs agree, as a literal type prints after the calls on `main`, and as `r` once they differ. So adding a `product` call to `describe` changes its printed bound and an `(infer (:name x))` in its body from `"(ref str)"` to `"r"`, and refinement 14's "no type changes" means the type at each call.
5. **A copy is the reader's first-pass types instantiated at one struct.** It reads each field as Rule 2.3 reads a field whose owner is open, takes the parameter mode the reader's first pass gives, which Rule 3.4 limits for a generic owner, and gets no second check, so `describe_user` takes `x: &User<'_>` and reads `x.name`, and `get` over `(boxed int)` and a `film` copies as `get_boxed_int`, which takes `b: &Boxed<i64>`, reads `&b.title` and returns `&i64`. Only a function that every call pins gets `main`'s second check of Rule 2.9. A generic caller whose owner is still open after Rule 2.2 and that passes it to any reader that copies, for any field, copies too, one function per struct it receives, as `g` calling `label` over `cat` and `book` emits `g_cat` calling `label_cat` and `g_book` calling `label_book`. A generic caller that passes its owner only to trait readers stays generic, and one whose owner Rule 2.2 fixes to one struct passes an open instantiation, which takes that instantiation's open copy. A copy of an instantiation that holds open variables takes its generics from the reader's first-pass types instantiated at that struct: each distinct open variable becomes one type parameter, and the copy carries the bounds the reader's first pass puts on those variables. So `(defn same (d) (= (:x d) (:y d)))` over a `(duo v v)` from a generic `both` and over a `pair` copies as `fn same_duo_a_a<A: PartialEq>(d: &Duo<A, A>) -> bool` and `same_pair`, because the `=` ties `:x` and `:y` together, and a copy over `Duo<A, B>` would be rustc's `E0308`. Instantiations that spell alike by Rule 2.6 share one copy, as `(boxed v)` from `both` and `(boxed b)` from `peek` share `fn get_boxed_a<A>(x: &Boxed<A>) -> &A`.
6. **Names.** The trait is `Has` and the field's Rust name in Pascal case, and the getters are the field's Rust name and that name with `_mut`, by refinement 9: `admin?` gives `HasIsAdmin` and `is_admin()`, and the changing trait of Rule 3.2 is `HasIsAdminMut` with `is_admin_mut()`. A copy is the reader's Rust name, an underscore and the spelling of each owner's type, in parameter order. A type's spelling is its knave name in snake case followed by the spelling of each argument, so a primitive spells by its knave name, and the distinct open variables spell as `a`, `b`, `c` and on, in the order of their first appearance in the owner's type, matching the copy's type parameters `A`, `B`, `C`: `user`, `boxed_int`, `boxed_ref_str` for `(boxed (ref str))`, `boxed_vec_int` for `(boxed (vec int))`, `boxed_a` for `(boxed t)` with `t` open, `boxed_vec_a` for `(boxed (vec t))`, `boxed_boxed_a` for `(boxed (boxed t))`, `duo_a_a` for `(duo v v)` and `duo_a_b` for `(duo t u)`. So `describe` over `user` gives `describe_user`, `size` over `(boxed int)` gives `size_boxed_int`, and `same-name` over a `user` and an `animal` gives `same_name_user_animal`.

   Different types can spell alike: two `user` structs in two modules, `(boxed int)` beside a struct named `boxed-int`, or `hello-user` over `x` beside `hello` over `user-x`. `(duo int t)` and `(duo t int)` spell apart, as `duo_int_a` and `duo_a_int`. Every name of such a group takes a number, `hello_user_1` and `hello_user_2`, in the order of the owners' declarations: the declaring module's path, then the position in that module, then the arguments' spellings, and for two readers, the readers' declarations first. A call or a struct renumbers every group whose names its new names take: a call that passes a `user-2` to `hello` adds the unique copy `hello_user_2`, so the copies of two `user` structs become `hello_user_1` and `hello_user_3`. Only the Rust changes.

   **Every introduced name yields by one rule.** The names this build introduces, field traits, changing traits, getters and copies, join the names `main` already introduces, such as the constructor `make_user` of `->user`. In each module, they take their names in a fixed order across kinds: constructors, then field traits, then changing traits, then getters on each struct, then copies whose spelling is unique, then numbered copies. A name that a written item or an earlier introduced item takes gets the next free number, as Names in Rust numbers an introduced name against a written one today. So a reader `make` over `user` and `product` emits `make_user_2` and `make_product_2` beside the constructors, a changing trait for `:name` beside a field `:name-mut`'s field trait `HasNameMut` is `HasNameMut2` with `name_mut_2()`, and two `user` structs beside a `user-2`, whose copy `hello_user_2` is unique, number as `hello_user_1` and `hello_user_3`. A name that a written item takes yields as before: `HasName2` beside a program's own `has-name` struct, `name_2()` beside a program trait that gives the same struct a method `name`, and `hello_user_2` beside a program's own `hello-user`.
7. **The trait emits at the crate root, each impl beside its struct, and each copy beside its reader.** A struct in another module emits its impl in that module's `mod` as `impl<'a> crate::HasName<'a> for Animal<'a>`, and a module that calls `name()` gets a Rust `use` of the trait, as a user trait's method call does. Every impl names its struct's lifetime and parameters, as `impl<'a> HasName<'a> for User<'a>`. A stored borrow's lifetime is the trait's own parameter, never the parameter's borrow, so `(defn trimmed (x) (trim (:name x)))` emits `fn trimmed<'a, A: HasName<'a>>(x: &A) -> &'a str`, and its result outlives a temporary argument. A trait reader that takes its owner by value and returns a read that borrows the struct names a fresh lifetime `'a` for the result and bounds the owner by it, `A: HasTitle + 'a`, by Rule 3.4.
8. **A function value picks its copy by the struct it takes.** `(map (iter cs) age-of)` over a `(vec cat)` emits `age_of_cat` where the pinned reader's name would stand when `age-of` copies, and `.map(age_of)` when it uses the trait. When that struct is the enclosing function's generic owner, the enclosing function copies too, so the struct is known in each copy.
9. **A pinned function is the function that `main` checks.** With only `(hello (->user "ana"))` in `main`, `hello` emits `fn hello(x: User<'_>) -> String` and `x.name`, the Rust that `main` emits today when `user` is the only struct with `name`. For a field that is no generic struct's parameter field, that is the Rust of the struct written in its signature. `main` checks a pinned body twice (`prepare_member`, `src/types/checker.rs:1019`): the first pass reads the field with the owner open, and the second reads it with the owner pinned. So the single-struct `boxed` program is `L0401` at the read inside `get`, ``expected `(ref int)`, found `int` ``: `(ref int)` is the first pass's result type, and `int` is the second pass's read. A reader pinned to `(boxed int)` on the bound path gets the same two passes, so a `film` beside it keeps that error. A copy is no pinned function and gets only the first pass, by Rule 2.5.
10. **A chained read.** `(defn start-x (l) (+ (:x (:start l)) 1.0))` prints `(a (has :start b))` and `(b (has :x float))` when the line structs' `:start` reads differ. Then `start-x` copies, and each copy reads `:x` from a known struct: `start_x_line` reads `l.start.x` and `start_x_span` reads it through the stored `&Spot`. When every line struct gives one `:start` read, `b` is that struct, and the trait reader reads `l.start().x`. An inner read on an open owner never needs a trait, because an open intermediate means the outer reads differ.
11. **A recursive group** copies as one. When an instantiation inside the group passes a struct, it fixes the owner for the whole group, the members are monomorphic, and an instantiation from outside with another struct is `L0401`. When no instantiation inside the group fixes the owner, the group generalizes, as on `main`, where mutually recursive `f` and `g` called as `(f 1 2)` and `(g "a" 2)` print `1` and `a`. Every member then receives the group's pooled structs, so `a-age` and `b-age` calling each other on `x`, with `a-age` called with `cat` and `b-age` with `user`, emit `a_age_cat` calling `b_age_cat` and `a_age_user` calling `b_age_user`.
12. **A read that meets a literal** follows the existing Literals rule: the literal type is not generalized, and the calls settle it. `(defn next-age (x) (+ (:age x) 1))` called with `cat`, whose `:age` is `int`, prints `(type next-age (where ((a (has :age int))) (fn ((ref a)) int)))`, as `(defn inc (x) (+ x 1))` prints `(fn (u8) u8)` when called with a `u8`. Called with `cat` and with `drone`, whose `:age` is `u8`, it is `L0401` at the second call, with the note `` `x` defaulted to `int`, because `next-age` is called with `cat` and with `drone` ``, the same note `inc` gets for two number types. A body that leaves the read generic, as `(str (:age x))` does, takes both, and copies.
13. **A reader that no instantiation reaches** has no struct, so its read keeps its variable, and `knave types` prints `(type hello (where ((a (has :name r)) (r display)) (fn ((ref a)) string)))`. The emitter writes no Rust for it, as it writes none for any unreached function on `main`, which still checks and prints it (`DependencyGraph::unchecked`, `src/types/graph.rs:234`). Its printed type follows the candidates: with `user` the only struct with `:name`, an unreached `hello` prints `(type hello (fn (user) string))`, and an `animal` with `:name str` that nothing uses turns it into the bound above, though no call's type changes and no Rust is emitted, as the pin rule of Rule 2.4 allows. The same rule covers a library checked alone, whose doc page prints the open signature, a definition at a `repl` or `eval` prompt, which takes its Rust from the later prompts' instantiations, and a reader instantiated only from a compile-time body, which the checker skips.

### 3. Ownership

**A read has its known-read type, in a trait reader and in a copy.** The read of a Copy `int` field is `int`, the read of a `str` field is `(ref str)` with the struct's lifetime, and the read of a `string` field is `(ref string)`. Every adjustment runs on that type, as it does for a known read. A copy keeps each struct's lifetimes, so `(defn pick () (local u (->user "ana")) (name-of u))` returns `&'static str` whether `name-of` uses the trait or copies over `user` and `product`.

A call's result that borrows a local argument passes `knave check` on `main` and fails in rustc with `E0515`: `(defstruct book :age string)`, `(defn age-of (x) (:age x))` and `(defn pick () (local u (->book (str "a"))) (age-of u))`, as a direct `(:age u)` does. Tracked item 550 covers that leak, beside issues #507 and #510, which cover two others. This RFC inherits it, and no fixture holds it before item 550 makes it `L0510`.

| Use | Bound | Emitted read |
| --- | --- | --- |
| returned, `(defn name-of (x) (:name x))`, `:name str` on two structs | `(a (has :name (ref str)))` | `fn name_of<'a, A: HasName<'a>>(x: &A) -> &'a str { x.name() }`, and the result outlives a temporary argument |
| a Copy field in arithmetic, `(+ (:age x) 1)`, `:age int` on two structs | `(a (has :age int))` | `x.age() + 1` |
| a `mut` parameter that reads too, `(set! (:age x) (* (:age x) 2))` | `(a (has :age int))`, and `x` takes `mut` | `*x.age_mut() *= 2`, by the Assignment bullet of Ownership, as `main` emits `x.age *= 2` |
| a clone, `(clone (:title x))`, `:title string` on two structs | `(a (has :title (ref string)))` | `x.title().clone()` |
| a move, `(keep (:title x))` with `keep` taking an owned `string` | none | `L0503` at the read, as for a known struct today, with the help `(clone ...)` |

A copy writes the known read, as `x.age + 1`.

1. **A read never moves the field,** as on `main`, where only the base of a collection or struct update moves one. A use that needs to own a field that is not Copy is `L0503` today and stays `L0503`.
2. **A `set!` of a generic field in a trait reader, or a `mut` borrow of one, needs a second trait, `Has<Field>Mut`, whose supertrait is the field's trait:**

   ```rust
   trait HasAgeMut: HasAge {
       fn age_mut(&mut self) -> &mut i64;
   }
   ```

   The trait emits only when a reached trait reader changes the field, and a struct implements it only when such a reader receives it. The changing getter returns `&mut` of the stored type: `&mut i64` for a value, `&mut String` for a borrow of the struct. The changing trait takes the field trait's lifetime parameter under refinement 10, as `trait HasOwnerMut<'u>: HasOwner<'u> { fn owner_mut(&mut self) -> &mut User<'u>; }`, so `(set! (:owner c) (->user "bob"))` on `car` and `boat` emits `*c.owner_mut() = make_user("bob")`. The bound prints no part of its own for a change: the checker reads the need from the body's `set!`, as it reads the parameter's `mut` mode. `(defn birthday! (x) (set! (:age x) (+ (:age x) (:bonus x))))` over `cat` and `dog` emits `fn birthday<A: HasAgeMut + HasBonus>(x: &mut A) { *x.age_mut() += x.bonus(); }`, which Rust evaluates right side first, because both operands are primitive. A copy changes the field directly, as `grow_drone` writes `x.age += x.age`.

   **An argument that reads the owner binds first.** A `_mut` getter borrows the whole owner before a method's arguments run, where a known read borrows only the field, which Rust's two-phase borrow allows. So in a changing trait reader, a method call on the changing getter whose arguments read the owner binds each such argument first, under a fresh temporary named as the ones that `call_with_temporaries` gives a later argument of a plain call whose earlier argument is `&mut` (`call_argument_1`, `src/emit/expressions.rs:1781`). That mechanism stops at a template callee (`:1656`), and `push!` is the template `{0}.push({1})`, so the build extends it to a template call on a changing getter: `(defn fill! (b) (push! (:items b) (len (:items b))))` over `bag` and `sack` emits `let call_argument_1 = b.items().len(); b.items_mut().push(call_argument_1);`. Without the binding, `b.items_mut().push(b.items().len())` is rustc's `E0502`, as is `b.items_mut().push(b.count())`. Binding serves an argument whose value is a value, a stored borrow or owned. An argument whose bound value still borrows the owner, a closure that reads it, as `(sort-by! (:items b) (fn (n) (- (deref n) (:pivot b))))`, or a read that borrows the struct, as `(:label b)` of a `string`, keeps the whole owner borrowed while the `_mut` getter runs, which is `E0502` bound or not. Such a reader copies, and each copy reads disjoint fields, as `main` emits `b.items.sort_by_key(|n: &i64| *n - b.pivot)`.

   A struct that stores the field as a borrow never implements the changing trait, because a `set!` of that field on a known value is `L0401` on `main`, as `(set! (:name u) "bob")` on a `str` field is. A call that passes such a struct to a reader that changes the field is `L0467` at the call, with the known `set!`'s reason. Open detail 1 asks the owner to confirm the trait.
3. **A struct update or a collection update on a generic owner stays an error.** `(assoc p :x 2)` builds a new value of the owner's struct, and a field trait has no constructor. Rule 4 gives the error its new text.
4. **A generic owner is Copy only when every struct that reaches the function is Copy,** whether the function is a reader or a generic caller that passes its owner on, each with its own structs. Then a by-value use copies it, as `main` copies a known Copy struct, and otherwise it follows `main`'s rule for a value that is not Copy, in a trait reader and in each copy alike, so each struct keeps the verdict that `main` gives it known. A struct without parameters counts as Copy when its emitted Rust is Copy. A generic struct with a type parameter is never Copy in a reader, at any instantiation, `(boxed int)` included, because `main`'s Copy impls require the parameter to be Copy and its sole-candidate path tests the owner with the parameter open. So `(defn get (b) (local c b) (local d b) (:title b))` is `L0501` over `(boxed 5)` alone, as on `main`, and stays `L0501` when a `tag` call joins it.

   - **Every struct is Copy.** A body that only reads the owner takes `ref`, which keeps decision 1's `hello` at `x: &A`. A body that uses the owner by value anywhere makes it Copy: its bound prints `copy` first, and the other bounds in the order the body finds them, as `main` prints a variable's bounds, so `(a copy (has :name (ref str)) partial-eq)` for a body that reads `:name` and then compares; a trait reader takes it by value under `A: Copy`; and a copy takes its struct by value, as `main` does for that struct known. This is what `main` does for a known Copy struct, and the Rust always compiles, since a Copy value never moves. So `(defn hello (x) (local y x) (str "hello " (:name x) " " (:name y)))` over `user` and `animal` emits `fn hello<'a, A: HasName<'a> + Copy>(x: A) -> String { let y = x; ... }`. `(defn f (b) (local c b) (local d b) (:age b))` over `cat` and `dog` emits `fn f<A: HasAge + Copy>(b: A) -> i64`, and over `cat` and a `drone` with `:age u8` emits `fn f_cat(b: Cat) -> i64` and `fn f_drone(b: Drone) -> u8`. `(defn f (b) (local xs (collect (map (iter (vec 1 2)) (fn (n) (keep b))))) (len xs))` over `cat` and `dog`, where `keep` takes its owner by value, emits `fn f<A: HasAge + Copy>(b: A) -> usize`, as do a move while a borrowing read of the owner is alive and a closure bound to a local and called twice.
   - **Some struct is not Copy.** A body that only reads the owner takes `ref`, a `set!` takes `mut`, a by-value use, as `(local c b)`, moves it and takes `own`, and every use that `main` rejects for a known struct that is not Copy takes `main`'s code and text wherever it falls: a use after the move is `L0501`, ``use of moved value `b` ``, a by-value use inside a closure that a parameter may call more than once is `L0509`, with `FnMut` or `Fn` in its title as the parameter accepts, and a closure bound to a local and called twice is `main`'s `L0501`. So `get` with two `local`s over `book` and `film` is `L0501` at the second `local`. A move while a borrowing read of the owner is alive passes `main`'s check and fails in rustc with `E0505`, as it does for a known struct that is not Copy, so the build inherits that `E0505`, with no note, as Rule 3 inherits `E0515`, until the tracked issue family of item 414 closes it.

     **Such a diagnostic names its cause** when the body compiles for some struct that reaches the function, since the user may not have touched the body. The build reports the diagnostics that `main` reports for that body over the known struct, no more, so a body with an `L0509` and an `L0501`, which `main` gives both, gives both. Each carries `main`'s text, a note written in `OwnershipProblems` that names the first struct in source order that is not Copy, and a secondary label on the first call in source order that passes it. When that struct reaches the function through a generic caller, the label sits on the call in `main` that passes it to the caller, and a second note names the path, as Rule 1's chain does: `` `g` passes `x` to `f` ``. A generic struct's note says why a Copy instantiation still counts: `` `b` is not Copy because `f` is called with `boxed`, which is not Copy for every `a` ``. A function that one struct reaches takes `main`'s diagnostic unchanged, by Rule 2.2, so `get` over `(boxed 5)` alone is `main`'s `L0501` with no note. With `cat` and `pet` both passed to `f`, and `pet` holding a `vec`:

     ```text
     error[L0501]: use of moved value `b`
       --> moved.knv:6:12
        |
      5 |   (local c b)
        |            - value moved here
      6 |   (local d b)
        |            ^ value used here after it moved
        |
       ::: moved.knv:11:12
        |
     11 |   (println (f (->pet 2 (vec 1)))))
        |            --------------------- this call passes a `pet`
        |
     help: clone the value if both places need it
        |
      5 -   (local c b)
      5 +   (local c (clone b))
        |
        = note: `b` is not Copy because `f` is called with `pet`
        = note: run `knave doc L0501` for an explanation
     ```

     An `L0509` for the same cause carries the same note and label beside `main`'s own labels. On the `L0501` that `main` gives a closure bound to a local and called twice, whose title names the closure, as ``use of moved value `g` ``, the note names the captured owner, `` `b` is not Copy because `f` is called with `pet` ``. `main`'s help, `(local c (clone b))`, is a fix in the generic case too: taking it gives the owner a `clone` bound, so a trait reader takes `A: HasAge + Clone`, which `cat` and `pet` both meet, as `main` derives `Clone` for both.

   A reader that takes its owner by value, copied or moved, and returns a read that borrows the struct, returns a borrow of its own parameter. Its Rust names the lifetime as `main` does for a known `book`, and a trait reader in that case names a fresh lifetime and bounds the owner's type parameter by it, so rustc gives the one error that `main` gives that body over a known struct, a struct that stores a borrow included. `(defn get (b) (local c b) (:title c))` over `(boxed int)` and `film` copies, as `fn get_boxed_int(b: Boxed<i64>) -> &'static i64` and `fn get_film(b: Film) -> &'static String`. Over `(boxed string)` and `film`, whose reads agree, it is a trait reader, `fn get<'a, A: HasTitle + 'a>(b: A) -> &'a String`. With `'static`, a `film` whose `:by` borrows a local would add `E0597` at the call, and with no bound on `A`, `E0309`. Each is the rustc `E0515` that `main` gives that body over a known struct, which the build inherits as Rule 3 does, until tracked item 550 makes it `L0510`.

### 4. Diagnostics with several candidates

A read on an unknown owner prints as the bound everywhere a type prints, as a `deftrait` with an associated type prints `(c (container item))` in [10. User-defined traits](../docs/language.md#10-user-defined-traits):

| Where | Text |
| --- | --- |
| `knave types` and a doc page's signature | `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))` |
| `(infer (:name x))` in `hello` | `"(ref str)"` when the reads agree, and `"r"`, the read's variable, when they differ, as `(infer x)` is `"a"` |
| `(infer hello)` | `"(where ((a (has :name (ref str)))) (fn (a) string))"`, with modes left out by the existing rule |
| a type error that names the owner | `a`, with a note that `a` is any struct with `:name`: `user` or `animal` |

The printed text is the same whether the reader uses the trait or copies, and it pastes into a `type` signature. A written signature is exact (Rule 6.4): a written result type checks against the reads as any written signature does, and a call that passes a struct whose read fails it is `L0467` at that call. A bound may hold several fields, `(a (has :name (ref str)) (has :age int))`, and a read type may carry its own bound, as Rule 2.10 shows.

The errors without an owner name the candidates:

```text
error[L0408]: no struct has a field `:size`
 --> zero.knv:1:20
  |
1 | (defn size-of (x) (:size x))
  |                    ^^^^^ unknown field
  |
  = help: a field with a similar name exists: `:side`
```

The help appears only when a field of a struct is within the distance of the existing closest-name rule.

```text
error[L0407]: type annotations needed
 --> assoc_ambiguous.knv:7:25
  |
7 | (defn bump (p) (assoc p :x 2))
  |                         ^^ cannot tell which struct this update builds
  |
  = note: `point` and `spot` have a field `:x`
  = help: name the struct in a signature, as in `(fn (point) point)`
```

### 5. What decides the parameter's type

On `main`, `field_owner` (`src/types/infer.rs:3564`) strips the value's borrows. A known struct is the owner. A type variable goes to `unique_owner`, which takes every struct of the program that declares the field (`StructTable::with_field`, `src/types/structs.rs:101`), whatever module declares it and whether or not the reading module imports it. One struct fixes the variable on the spot. Zero or several are `L0407`. A struct update uses the same lookup through `update_owner_from_fields` (`src/types/infer.rs:3390`), keeping the structs that declare every field it names.

After the build, the lookup still counts every struct of the program, as `main` does. One candidate still fixes the owner at the read, so every program that compiles on `main` keeps its types. Two or more give the owner the field bound, and a later form of the body that fixes the owner checks the bound then, so the read no longer decides at its position. The read keeps the type that Rule 2.3 gives it while its owner was open: `(let (t (:title b)) ...)` followed by an `if` that fixes `b` to `(boxed bool)` reads `t` as `(ref bool)`, on both paths, as `main` does. None is `L0408` at the read, as Rule 4 shows.

**An open literal type wins over the lookup.** On `main`, `(:x n)` after `(let (n 5) ...)` is `L0407`, as `L0407.md` documents. A read whose owner is an open integer or float literal type is `L0410` at the read, `` `int` has no fields ``, naming the literal's default type, before any candidate counts, because no struct is a number.

| Program | On `main` | After the build |
| --- | --- | --- |
| decision 1, `user` and `animal` | `L0407` at `(:name x)` | a trait reader, as Option A shows |
| decision 2, `user`, `animal` and `product` | `L0407` at each `(:name x)` | a trait reader and copies, as Option A shows |
| decision 1 with `user` alone | `fn hello(x: User<'_>) -> String`, single-struct inference | the same, byte for byte |
| decision 1 with `(type hello (fn ((ref user)) string))` | `fn hello(x: &User<'_>) -> String` | the same, byte for byte |
| a library's `hello`, called by a module that declares `user` | `hi ana` | the same, byte for byte |

A throwaway build of `main` that logs each zero-or-several lookup ran `knave check` on each of the 2,702 `.knv` files under `prelude/`, `examples/` and `tests/fixtures/`, leaving out `read/` and `format/`. Three files reach the lookup, and only their snapshots change:

| File | Lookup | Snapshot after the build |
| --- | --- | --- |
| `tests/fixtures/check/type_shape_errors.knv` | read of `x`, two structs | loses its first error: `read-x` compiles as a generic reader |
| `tests/fixtures/check/assoc_ambiguous.knv` | update of `x`, two structs | keeps `L0407`, with the label and note of Rule 4 |
| `tests/fixtures/run/doc_examples_bare_keyword.knv` | read of `size`, no struct | `L0408`, ``no struct has a field `:size` `` |

No prelude or example file reaches the lookup, so no other output changes.

**Fixtures for the switch.** Each fixture of a group adds one struct to the fixture above it, with a second reader that receives the new struct and so emits copies, while the first reader keeps the trait and its `(println (infer ...))` line, which the emitted Rust holds as a string literal, stays the same. The first reader never receives the new struct: `next-age` with `drone` would be `L0401` with the Literals rule's note, and `uppercase-name` with `poll` or `product`, and `shout` with `(boxed int)`, would be `L0467` at the call.

| Group | Fixture | Structs | First reader | Second reader | Rust |
| --- | --- | --- | --- | --- | --- |
| value | `emit/field_trait_plain_value.knv` | `cat`, `dog` with `:age int` | `next-age`, `(+ (:age x) 1)`: `(where ((a (has :age int))) (fn (a) int))` | none | `trait HasAge { fn age(&self) -> i64; }` |
| value | `emit/field_copies_value.knv` | the same, and `drone` with `:age u8` | the same | `show-age`, `(str "age " (:age x))`, with `cat` and `drone` | the trait for `next-age`, and `show_age_cat` and `show_age_drone` |
| borrow | `emit/field_trait_plain_borrow.knv` | `user`, `animal` with `:name str` | `uppercase-name`: `(where ((a (has :name (ref str)))) (fn (a) string))` | none | `trait HasName<'u> { fn name(&self) -> &'u str; }` |
| borrow | `emit/field_copies_stored_borrows.knv` | the same, and `poll` with `:name (slice int)` | the same | `count-name`, `(len (:name x))`, with `user` and `poll` | the trait, and `count_name_user` and `count_name_poll` |
| borrow | `emit/field_copies_mixed.knv` | the same, and `product` with `:name i32` | the same | `describe`, `(str "name: " (:name x))`, with `user` and `product` | the trait, and `describe_user` and `describe_product` |
| owned | `emit/field_trait_plain_owned.knv` | `book`, `film` with `:title string` | `shout`, `(loud (:title x))` with `(type loud (fn ((ref string)) string))`: `(where ((a (has :title (ref string)))) (fn (a) string))` | none | `trait HasTitle { fn title(&self) -> &String; }` |
| owned | `emit/field_copies_generic_struct.knv` | the same, and `(boxed a)` with `:title a` | the same, with `book` and `(boxed string)` | `size`, `(:title x)` returned, with `film` and `(boxed int)` | `impl HasTitle for Boxed<String>`, and `size_film` and `size_boxed_int` |
| alone | `emit/field_copies_beside_trait.knv` | `cat`, `dog` with `:age int`, and `book` with `:age string` | `age-of`, with `cat` and `dog`, and `ages`, `(collect (map (iter cs) age-of))` under `(vec int)` | `show-age` with `book` and `cat`, and `g`, which passes its owner to `show-age`, with `cat` | the trait for `age-of` and `.map(age_of)`, and `show_age_book`, `show_age_cat` and `g_cat` |
| alone | `emit/field_copies_disagreeing_readers.knv` | `cat`, `dog` with `:age int`, and `user`, `admin` with `:age str` | `age-of` with `cat` and `dog` | `label-of` with `user` and `admin` | no trait: `age_of_cat`, `age_of_dog`, `label_of_user` and `label_of_admin` |
| alone | `emit/field_trait_unpassed_struct.knv` | `cat`, `dog` with `:age int`, and `user` with `:age str` that no reader takes | `age-of`: `(where ((a (has :age int))) (fn (a) int))`, as with `cat` and `dog` alone | none | `trait HasAge { fn age(&self) -> i64; }`, no impl for `User` |
| alone | `emit/field_trait_plain_value_lifetime.knv` | `user` with `:name str`, and `car`, `boat` with `:owner user` | `owner-of`: `(where ((a (has :owner user))) (fn (a) user))` | none | `trait HasOwner<'u> { fn owner(&self) -> User<'u>; }` |

A check fixture, `check/field_owner_moved.knv`, holds `L0501` with its note and label for an `f` with two `local`s over `cat` and a `pet` that is not Copy, and for `(boxed 5)` beside a `tag`, `L0509` with its note and label for the passed closure `f` over `cat` and `pet`, and `L0501` with its note and label for a closure bound to a `local` and called twice over `cat` and `pet`. A run fixture holds the same bodies' copies over `user` and `animal` and over `cat` and `dog`, through `A: Copy`, over `cat` and `drone`, as copies that take each struct by value, and, over `cat` and `dog` through `A: Copy`, the passed closure, a single by-value use, a move while a borrowing read is alive, and a closure bound to a `local` and called twice. Run fixtures cover the programs the reviews raised: a name read from a temporary through the trait and through a copy, `trimmed`, `birthday!` with `:bonus`, `grow!` on `cat` and `drone`, `pick` over `user` and `product`, `start-x` over `line` and `span`, `tag-of` over `a1` and `a2`, `show-p` over `a1`, `a2` and a deeper stored borrow, `uppercase-name` over `user`, `person` and `holder`, a function value that picks its copy, a recursive group over `cat` and `user`, two generic levels of callers, and `(boxed a)` read at `string` and `int` by one generic `get`.

### 6. Interplay

1. **Structs across modules.** A candidate is any struct of the program, as on `main`. The checker has no field visibility rule: a known read of a private struct's field from another module compiles on `main`, and a generic read checks no visibility either. So a library can hold a generic reader for its callers' structs: its trait and impls span the modules, and its copies emit beside the reader in the library's module.
2. **Generic structs.** A generic struct is a candidate like any other, and a read whose owner is open reads its field as `main`'s first pass does, by refinement 15 and Rule 2.3's open-read table. A trait reader over `book` and `(boxed string)` gets `impl HasTitle for Boxed<String>`, since both read `(ref string)`. `(defn get (b) (:title b))` called only on a `(boxed string)` and a `(boxed int)` receives one struct, so by Rule 2.2 it is `main`'s generic `get<A>` whatever other structs declare `:title`. Called also on a `film`, it copies, as `get_film` and `fn get_boxed_int(b: &Boxed<i64>) -> &i64`, and its calls keep the type the sole-candidate path gives on `main`. Beside a `film`, a generic caller that passes `(boxed t)` with `t` open gets `get_boxed_a<A>`, and the copies of Rule 2.4's open-variable rule, `get_boxed_a<A>` and `get_crate_a<A>`, read `&x.title` as before. A function-typed `:f (fn (a) int)` reads as the value, a `:p (ref a)` as the stored borrow, and a `:v (option (ref a))` as the value, so `get-f` over `(hook int)`, `(hook bool)` and a `plain` copies with `get_f_hook_int` returning `fn(i64) -> i64`, and `get-v` over `(wrap int)` and an `other` copies with `get_v_wrap_int` returning `Option<&'a i64>`. `next-age` over `cat` and `(boxed int)` reads `int`, a value, and `(ref int)`, a borrow, so it copies too.
3. **An enum's case.** An enum is never a candidate. A case's fields exist only inside that case, so a getter cannot exist for the enum. A program reads a case's field through `match`, as on `main`, and a generic reader called with an enum is `L0410`.
4. **A written type annotation.** A written signature is exact. One that names a struct emits today's Rust. One that names a type variable without the field bound is `L0440`, with a help that writes the bound, as `(a (has :name (ref str)))`, as a missing `copy` gets `(a copy)` today. A binder annotation, as `(x (ref user))`, fixes the owner.
5. **A closure.** A `local` closure stays monomorphic, as on `main`, so a field bound on its own parameter must settle by the end of checking. `(let (f (fn (y) (:name y))) (f (->user "ana")) (f (->animal "rex")))` is `L0401`, ``expected `user`, found `animal` ``, at the second argument, as `(let (f (fn (y) y)) (f 1) (f "a"))` is today. A closure that no call fixes is `L0407` at the read, `` cannot tell which struct's `name` this is ``, with the note of Rule 4 that lists the candidates. A read inside a closure whose owner is the enclosing function's parameter gives the function the bound, so it generalizes with the function, and copies with it.
6. **A field the prelude reads.** The prelude declares no struct and no function that reads a field. Its macros, `defstruct`'s constructors, `assoc`, `assoc-in`, `update` and the keyword closure, expand to `field` and `new` in the user's module, so their reads follow these rules there. A keyword closure, as `(map (iter xs) :name)` in an unannotated function, gives the function the bound through its parameter's element type. `update` reads and then builds, so a generic owner is the update error of Rule 3.3.

### 7. Prelude first

The prelude cannot express this, and the build adds no kernel form:

- **The choice is the checker's.** Whether `(:name x)` emits `x.name`, `x.name()` or a copy depends on the type of `x` after inference and pinning, and on the structs that reach the reader. A macro runs before the checker and sees only the form.
- **The struct set is the program's.** The candidates span modules, and whether a reader uses the trait or copies depends on the calls that reach it. Only the compiler sees every struct, every call and every pin.
- **A trait per field from a macro clashes,** as Option B says, and [RFC 0038](0038-field-keywords.md) moved keyword reads into the expander for the same reason.

The change lives in the type rule of the existing kernel form `field`, and in one kernel name of a new kind: the bound `has`, which a `where` writes beside traits. A kernel name is needed because the prelude cannot define a family of bounds keyed by a field name, and the printed bound must paste into a signature. `has` is exported by the built-in `kernel` module and re-exported by the prelude like every kernel name. The build adds it as a row of the table in [Types](../docs/language.md#types), `(where ((a (has :age int))) (fn ((ref a)) int))` beside `fn<A: HasAge>(&A) -> i64`, and records why it is a kernel name in [Kernel forms](../docs/language.md#kernel-forms), beside the reasons of the other kernel names, and adds it to that section's list of the kernel names that type positions hold (`docs/language.md:85`), beside `fn`, `ref`, `where` and `with`. `prelude/prelude.knv` documents `has` with a `document` form, as it documents `struct`. The Rust names `HasName`, `name()` and `hello_user` are no fact the prelude states, so the compiler derives them from the field, the reader and the struct, as it derives `make_point` from `->point`.

## Open details

1. **The changing getter.** Rule 3.2 puts `<field>_mut` in a second trait, `Has<Field>Mut: Has<Field>`, implemented only by the structs that a changing trait reader receives, so a struct that stores the field as a borrow never has to write it. The alternative is an error at a `set!` of a generic field, with help to write a signature. The recommendation is the second trait, because Rust programmers write it, `L0467` at the call stays the only error, and the uniform `set!` rule then holds for generic owners.
2. **The bound's spelling.** This RFC writes `(has :name r)`, and `(has :name r (ref str))` with a fixed read. A keyword names the field as a read and a `defstruct` do. `field` would read as the kernel form, and a name ending in `?` reads as a predicate. The owner may prefer another name.
3. **An owned text or vector field read through its view.** On `main`, a known read of a `string` field is `(ref string)`, and it meets `(ref str)` by the `str` adjustment. Rule 1 checks a fixed read by the struct's known read and those adjustments, and a reader whose structs give different reads copies, so `uppercase-name` takes a `person` with `:name string` beside `user` with no view rule. The view rule, under which a `String` field would read as `(ref str)`, would change the plain owned trait to `fn title(&self) -> &str`, the changing getter and the `clone` row for no case that Rule 1 rejects, so the RFC does not recommend it. The owner may still prefer it for shorter getters.

## Prototype

`target/scratch/proto14/mix.rs`, `target/scratch/proto15/round10.rs`, `target/scratch/proto16/solebox.rs`, `target/scratch/proto17/round11.rs`, `target/scratch/proto18/round12.rs`, `target/scratch/proto19/round13.rs`, `target/scratch/proto20/round15.rs`, `target/scratch/proto21/ownread.rs` and `target/scratch/proto22/closure.rs` held the prototype, which is not part of this pull request. rustc 2024 compiled and ran it. `proto14/mix.rs` emits, under refinement 14, one module per probe program of the first nine reviews, and `proto15` to `proto22` hold the programs of the tenth to the seventeenth. `proto14/mix.rs` printed one line per module:

- decision 1, with `hello`, a name read from a temporary and `trimmed` through `HasName<'u>`: `hello ana hello rex ana rex`
- decision 2, with `uppercase-name` through the trait and `describe_user` and `describe_product`: `ANA REX name: ana name: 7`
- the value group, with `next-age` and `r` through `HasAge`, `birthday!` through `HasAgeMut + HasBonus`, and `show-age` and `grow!` copied over `cat` and `drone`: `4 6 3 age 6 age 6 6 6 age 4`
- the owned group, with `shout` over `book` and `(boxed string)`, `size_film`, `size_boxed_int`, a generic `get_boxed_a` and `clone`: `DUNE S jaws 4 k jaws`
- refinement 10, with `owner-of` on a temporary `car` and a `set!` through `HasOwnerMut<'u>`: `ana bob`
- the readers of rounds 4 to 8, with `age-of` through the trait beside `show-age` copies, `ages` collecting `Vec<i64>`, `g` copying through `label`, a function value, a recursive group over `cat`, `user` and `book`, and results inside a `vec`, an `option` and a `result`
- `pick` over `user` and `product`: `ana rex 7`
- chained reads, with `start-x` over `line` and `span`, `tag-of` over `a1` and `a2`, `bump!` and `move-start!`: `2 3 a b 12 10`
- fixed reads, with `show-p` over `a1`, `a2` and a three-layer stored borrow, a stored `&Pt` beside a Copy `Pt`, `uppercase-name` over `user`, `person`, `holder` and `deep`, two fixed reads of one field, and a slice reader over `Vec`, `&Vec` and `&[i64]`: `a b a 1 2 ANA BOB CY CY BOB3 CY2 3 3 3`
- stored borrows of two types and two uniform readers that disagree: `3 2 3 4 old new`
- two generic levels of callers: `old 3 4 [5] 7`

`proto22/closure.rs` held the closure `f` over `cat` and `dog` through `A: HasAge + Copy`, printing `2 2`, a single by-value use, which Rule 3.4 now also gives `A: Copy`, printing `1 2`, and a move while a borrowing read is alive and a closure bound to a local and called twice, printing `2 4`. `proto21/ownread.rs` held the own-mode trait reader over `(boxed string)` and a `film` that stores a borrow, under `A: HasTitle + 'a` with a named `'a`, which rustc rejects with `E0515` alone. `proto20/round15.rs` held Rule 3.4's Copy owner on the fifteenth review's probes: `hello` with a `local` alias over `user` and `animal` through `HasName<'a> + Copy`, printing `hello ana ana | hello rex rex`, `f` with two `local`s over `cat` and `dog` through `HasAge + Copy`, printing `1 2`, and over `cat` and `drone` as by-value copies, printing `1 3`.

`proto19/round13.rs` held the thirteenth review's fixes: `get_boxed_int` and `get_tag` taking a borrow although their bodies copy the owner, printing `5 3`, which Rule 3.4 now replaces with `L0501`, since `(boxed int)` is not Copy in a reader, and `wrap<A>` calling the open copy `get_boxed_a` while a `film` call makes `get` copy, printing `5 x f`.

`proto18/round12.rs` held the twelfth review's fixes: `get_v_wrap_int` returning `Option<&i64>` beside an `other`, printing `1`, `get_boxed_int` taking `&Boxed<i64>` beside `get_film`, and `describe_user` taking `&User`, printing `5 f name: ana`, `same_duo_a_a<A: PartialEq>` and `same_pair`, printing `true true true`, and `get<A>` with an in-body read typed `&A` for `(boxed int)` and `(boxed string)` beside an unpassed `film`, printing `5 x`.

`proto17/round11.rs` held the eleventh review's fixes: `get-f` copies over `(hook int)` and `(hook bool)` that return the function values, printing `10 1`, `get-p` copies over `(holder int)` and `(holder string)` that return the stored borrows, printing `1 x`, the `readfix` read that keeps `&b.title` after a later form fixes `b`, printing `false`, `order!` copied over `board` and `plank` with the closure reading `b.pivot`, the numbered names `make_user_2`, `HasNameMut2` with `name_mut_2()` and `hello_user_1` to `hello_user_3`, and `get_boxed_a<A>` shared by two generic callers, printing `1 k`.

`proto16/solebox.rs` held refinement 15 on the tenth review's `solebox` probes: the sole-candidate `get<A>`, which Rule 2.2 now also emits beside an unpassed `film`, and copies `get_boxed_int` and `get_boxed_string` for a `get` that also receives another struct, both print `6 5 x x`, reading `&i64` and `&String`, and a trait reader over `book` and `(boxed string)` prints `DUNE S`, and `next-age` copies over `cat` and `(boxed int)`, printing `4 8`. In `proto14/mix.rs`, `size_boxed_int` returns `&i64`.

`proto15/round10.rs` held the tenth review's fixes: `fill!` and a `tally!` that pushes `count` through `let value = ...; b.items_mut().push(value)`, printing `[7, 8, 2, 4] [9, 1, 5]`, numbered copies over two `user` structs and over `(boxed int)` beside `boxed-int`, printing `hi ana hi bob 5 x s`, `tag-of` over `a1` and `a3`, printing `a a`, `get_boxed<T>` and `get_crate<T>` from a generic `both`, printing `1 1 a a`, and `card` with `hello` beside it, all copied, printing `12 34 hello ana hello rex`.

`proto14/mix.rs` passes a Copy struct to a copy by value, the mode before the twelfth review; Rule 2.5 now gives the first pass's borrow, as `proto18/round12.rs` shows. The rounds' own Rust probes under `rs/` to `rs9/` hold the associated-type designs that refinement 14 replaces. Calling a trait reader with a struct that has no impl gives rustc's `E0277`, which Rule 1 replaces with `L0467`. The instrumented `main` build of Rule 5 changed only two `eprintln!` lines in `src/types/infer.rs`.
