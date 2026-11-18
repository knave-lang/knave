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

### A. A field bound, emitted as one Rust trait per field name (recommended, and the owner's model)

The checker gives an unknown owner a field bound, `(a (has :name r))`: `a` is a struct with a field `name`, and `r` is the type a read of that field gives. The bound behaves as a trait bound does. It generalizes, it is checked at each call, and pinning removes it when every call passes one struct. The emitter writes one Rust trait per field name that a reached generic read needs, in the simplest of six shapes that the program's structs allow (Rule 2.4), and one impl per struct that a reached call passes.

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

Every struct that a call passes to `hello` reads `:name` as `(ref str)`, so the read settles to that type, and `knave types` prints `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`.

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

The read types now differ, one a value and two stored borrows, so the trait reads through a borrow of the struct, and each reader settles its own read, by refinement 12. After the build:

```rust
trait HasName {
    type Name: ?Sized;
    fn name(&self) -> &Self::Name;
}

impl<'a> HasName for User<'a> {
    type Name = &'a str;
    fn name(&self) -> &&'a str {
        &self.name
    }
}

impl<'a> HasName for Animal<'a> {
    type Name = &'a str;
    fn name(&self) -> &&'a str {
        &self.name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Product {
    name: i32,
}

impl HasName for Product {
    type Name = i32;
    fn name(&self) -> &i32 {
        &self.name
    }
}

fn uppercase_name<'a, A: HasName<Name = &'a str>>(x: &A) -> String {
    (*x.name()).to_uppercase()
}

fn describe<A: HasName>(x: &A) -> String
where
    <A as HasName>::Name: Copy + std::fmt::Display,
{
    format!("name: {}", *x.name())
}
```

`uppercase-name` takes only `user` and `animal`, whose reads are one Copy value, the stored `(ref str)`, so it copies the read out of the getter and keeps the struct's lifetime: `(type uppercase-name (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`, the same text as with two structs. `describe` takes `user` and `product`, whose reads differ but are each a value or a stored borrow, so it copies its read out too, by refinement 13, and uses it only through `display`: `(where ((a (has :name r)) (r display)) (fn ((ref a)) string))`. Each call takes its own struct's type, so `describe` reads a `(ref str)` from `user` and an `i32` from `product`.

### B. `defstruct` writes a trait and an impl per field

The prelude macro would write `(deftrait has-name (type field-type) (get-name ...))` and an `impl` for each field of each struct, and a keyword read would expand to `get-name`. Two structs that share a field would both define `has-name`, which is the clash that made keyword reads an expander feature in [RFC 0038](0038-field-keywords.md). Every field of every struct would emit a trait and an impl, so every program's Rust would change. A macro runs before the checker, so it cannot tell a read whose struct is known, which should stay `x.name`, from one whose struct is not.

### C. One copy of the function per struct

The emitter would write `hello_user` and `hello_animal`. The Rust would hold one function per struct for every generic reader, calls would name a struct the source never wrote, and a function value such as `(map users hello)` would need the copy picked by its element type. Rust's own answer to "any type with this capability" is a trait.

### D. A written accessor parameter

The program would pass the reader as a function, `(hello :name x)`. This is possible today and changes the signature of every reader. Decision 1 rules it out: the unannotated `hello` must compile.

## Recommendation

Option A, with the shapes of Rule 2.4. It is the owner's model, Rust programmers write each shape by hand, and it adds a bound kind to the checker in place of a kernel form. Every program whose reads have zero or one candidate struct, or whose calls pin each reader to one struct, keeps its Rust byte for byte.

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

## Rejected options and why

- **B: a trait per field from `defstruct`:** two structs that share a field clash, every program's Rust changes, and a macro cannot see whether a read's struct is known.
- **C: one copy per struct:** the Rust grows by one function per struct and reader, calls name a struct the source never wrote, and function values need a choice the source never states.
- **D: a written accessor:** decision 1 requires the unannotated function to compile.
- **A field bound on every unknown owner, one candidate included:** a reader of the only struct with `:name` would print a `where` and emit a trait for one implementor, where a concrete type says the same thing and reads better. Rule 5 keeps one candidate concrete.
- **One associated type name, `FieldType`, for every field trait:** an owner with two field bounds makes `A::FieldType` ambiguous, `E0221`. Refinement 5 names it after the field.
- **A getter that returns `&Self::FieldType` for every field:** a stored borrow's read would end at the struct's borrow, and a Copy field would read as a borrow, unlike the known read. Refinement 6 replaces it.
- **A generic associated type for the read, `type Name<'s> where Self: 's`:** it serves every kind of field in one shape, but a `mut` parameter that also reads the field is `E0502`, a chained read is `E0597`, the `for<'s>` bound that would repair them implies `'static`, and a fixed read `HasName<Name<'a> = &'a str>` merges the stored borrow's lifetime with the parameter's, which is `E0716` for `(defn trimmed (x) (trim (:name x)))` on a temporary. Refinement 8 replaces it.
- **The shape taken from every struct that declares the field:** it keeps a reader's shape fixed when its calls change, but an unused struct, or one in a library that never reads the field, changes the reader's types, as refinement 11 shows. Refinement 11 replaces it.
- **Per-struct copies of a reader with an instantiation whose read borrows the struct:** `name_of_user` and `name_of_book` would keep the stored borrow's lifetime where the mixed shape borrows the parameter. This is Option C for one shape, with its costs, so the owner rejected it for now and may revisit it if programs reach the cost that Rule 3 accepts.
- **Candidates limited to the structs the reading module sees:** the checker has no field visibility rule today. A known read of a private struct's field from another module compiles on `main`, so a limit only for generic reads would be an exception. It would also break a library reader: `(defn hello (x) (str "hi " (:name x)))` in a library compiles on `main` for its caller's `user`, and a library could not hold a generic reader of its callers' structs.

## Rule

The build states these rules in [docs/language.md: Field keywords](../docs/language.md#field-keywords), [Types](../docs/language.md#types), [Ownership](../docs/language.md#ownership), [Names in Rust](../docs/language.md#names-in-rust) and [10. User-defined traits](../docs/language.md#10-user-defined-traits), in the explanation of `L0407`, and in new explanations of `L0467` and `L0468`. It replaces the two sentences of Field keywords that say a function needs a signature when two structs share a field name, and it rewrites the paragraph of `expand/src/diagnostic/explanations/L0407.md` that says a field read or update finds its struct by the field's name: a read with no candidate is `L0408`, a read with several gives a field bound, and only an update with several, or a bound nothing fixes, stays `L0407`.

### 1. A call whose field type fails the body's use

A bound has two parts. Its type is always the field's type of Rule 2.4, and a body that fixes the read adds a requirement on the read, which the bound prints as a third element: `(a (has :p t (ref spot)))` reads "`a` has a `:p` of type `t`, which reads as `(ref spot)`". A body that uses the read through a trait, as `str` uses `display`, gives `t` that trait bound. A struct meets a fixed read `X` when its known read becomes `X` by the adjustments a known read gets at that use, which [Ownership: Adjustment](../docs/language.md#ownership) lists: one added borrow, removed extra borrows, a `string` viewed as `str`, a `vec` viewed as a slice, and a copy out. So `(defn r (x) (show-ref (:age x)))`, with `show-ref` taking `(ref int)`, takes `cat` and `dog`, as `(show-ref (:age c))` does for a known `cat`, and emits `show_ref(&x.age())`. And `(defn show-p (x) (describe-spot (:p x)))`, with `describe-spot` taking `(ref spot)`, takes an `a1` that stores `:p (ref spot)`, an `a2` that holds `:p spot` and an `a3` that holds a Copy `:p pt` under a fixed `(ref pt)`.

The requirement prints only while `t` is open. When the structs' field types agree, `t` is that type and the requirement holds as it does for a known read, so decision 1's `uppercase-name` prints `(has :name (ref str))`. When they differ, `knave types` prints both parts, as `(type show-p (where ((a (has :p t (ref spot)))) (fn ((ref a)) string)))`, and the text pastes back. A written bound's type is the field's type, so a written `(has :age (ref int))` is `L0467` for `cat`, whose `:age` is `int`, while `(has :age int (ref int))` holds.

When the field types differ under one fixed read, the read converts through one helper trait, which the emitter writes once at the crate root beside the field traits when a program needs it:

```rust
trait ReadAs<S: ?Sized> {
    fn read_as(&self) -> &S;
}

impl<T: ?Sized> ReadAs<T> for T { fn read_as(&self) -> &T { self } }
impl<T: ?Sized> ReadAs<T> for &T { fn read_as(&self) -> &T { self } }
impl<T: ?Sized> ReadAs<T> for &&T { fn read_as(&self) -> &T { self } }
impl ReadAs<str> for String { fn read_as(&self) -> &str { self } }
impl ReadAs<str> for &String { fn read_as(&self) -> &str { self } }
impl<T> ReadAs<[T]> for Vec<T> { fn read_as(&self) -> &[T] { self } }
impl<T> ReadAs<[T]> for &Vec<T> { fn read_as(&self) -> &[T] { self } }
```

Each impl is one dropped borrow layer, the view, or both, so it covers every storage form whose known read meets the fixed read: an owned `S`, a stored `&S` or `&&S`, a `String` or a stored `&String` for `str`, and a `Vec<T>` or a stored `&Vec<T>` for `[T]`. `std::borrow::Borrow` covers no `&String`, `&Vec<T>` or `&&S`. The emitter writes only the impls the program's fixed reads use, and `show-p` emits `fn show_p<A: HasP>(x: &A) -> String where A::P: ReadAs<Spot> { describe_spot(x.p().read_as()) }`. A module that calls `read_as()` gets `use crate::ReadAs;`, as it gets the field trait's `use`, and the names yield as Rule 2.5's do, `ReadAs2` and `read_as_2()`, beside a program item of the same name. When the field types agree, the read needs no conversion and emits as before.

A call checks the bound against the struct it passes, and a struct whose read fails the field type, the fixed read or the trait is `L0467` at that call, a new code of the type stage. The title names the struct's known read. `L0465` is the last code on `main`, PR 676 takes `L0466`, and no open pull request takes `L0467` or `L0468`.

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

The message never names `HasName`, `Name` or `name()`, so rustc's `E0271`, which the prototype hits when it calls `uppercase_name(&Product { .. })`, never reaches the user.

A call with a value that has no such field reports the existing codes at the call, with a secondary label on the read and the same note:

| Argument | Code | Title |
| --- | --- | --- |
| a struct without the field, `(hello (->point 1.0 2.0))` | `L0408` | `` `point` has no field `:name` `` |
| an enum, `(hello shape/point)` | `L0410` | `` `shape` has no fields `` |
| any other type, `(hello 5)` | `L0410` | `` `int` has no fields `` |

### 2. Readable output

1. **A trait emits only for a field that an emitted function reads from a generic owner.** A read whose owner is a known struct, from the start or after pinning, emits `x.name` as it does today. A program with no generic read after pinning emits no trait and no impl, so its Rust stays byte for byte what `main` emits.
2. **An impl emits only for a struct that a reached instantiation of a reader takes for the bound, a call or a function value, and those structs are the ones that choose the shape of Rule 2.4,** directly or through another generic function's bound, as the structural trait impls of [8.2 Struct traits](../docs/language.md#82-struct-traits) do. A struct no instantiation takes gets no impl. A reader that changes the field needs a second trait, Rule 3.2.
3. **Each struct's read of the field has a kind and a type, from the Fields bullet of [Ownership](../docs/language.md#ownership):**

   | Kind | Field | Read on `main` | Rust read |
   | --- | --- | --- | --- |
   | value | a function type, or a Copy type that the size rule of Ownership: Fields copies | the value | `i64`, `fn(&str) -> bool` |
   | stored borrow | `str`, `(slice a)`, `(ref t)` | the stored borrow, with the struct's lifetime | `&'a str`, `&'a [A]`, `&'a T` |
   | borrow of the struct | any other type, and a generic struct's parameter field whose argument is still open | a borrow of the field | `&String`, `&A` |

   On `main`, a known read tests the size rule on the instantiated field type at the read (the `reads_field_by_copy` call at `src/types/infer.rs:3535`, the method at `:3548`), so a generic struct's field reads by what is known there: `(boxed int)` known at the read gives `fn get(b: &Boxed<i64>) -> i64 { b.name }`, a parameter with a `number` bound gives `fn twice<A: Number>(b: Boxed<A>) -> A { b.age + b.age }`, and an argument still unknown gives `(ref a)`, as `fn get<A>(b: &Boxed<A>) -> &A` and `&b.name + 1` show. In a bound, each instantiation reads by what is known at it, as a known read does: `(boxed int)` reads `int`, a value, `(boxed string)` reads `(ref string)`, and `(boxed a)` with `a` still open reads `(ref a)`. One impl still serves every instantiation: `impl<A> HasAge for Boxed<A>` with `type Age = A`, and `A: Copy` on the impl in the value shape. A field whose declared type does not hold the parameter, such as `:count int` in `(struct (pair a) (count int) (val a))`, reads as any other field does.
4. **The trait's shape is the simplest that the passed structs allow,** by refinements 7, 8, 10 and 11. The passed structs are those that some reached instantiation of a generic reader of the field takes, a call or a function value, directly or through another generic function's bound. A struct that declares the field and reaches no reader changes nothing, and an instantiation that takes a new struct can change the shape of every reader of that field:

   | The structs' reads | Trait | Bound's type, and the read |
   | --- | --- | --- |
   | one type, value | `trait HasAge { fn age(&self) -> i64; }`, or `trait HasOwner<'u> { fn owner(&self) -> User<'u>; }` when the value holds the struct's lifetime | `int`, `user` |
   | one type, borrow of the struct | `trait HasTitle { fn title(&self) -> &String; }` | `string`, read as `(ref string)` |
   | one type, stored borrow | `trait HasName<'u> { fn name(&self) -> &'u str; }` | `(ref str)` |
   | differing, every one a value | `trait HasAge { type Age; fn age(&self) -> Self::Age; }` | `r` |
   | differing, every one a stored borrow | `trait HasTag<'u> { type Tag: ?Sized; fn tag(&self) -> &'u Self::Tag; }` | `(ref b)`, with the struct's lifetime |
   | differing otherwise, mixed kinds and generic structs included | `trait HasItem { type Item: ?Sized; fn item(&self) -> &Self::Item; }` | `r`, read by refinements 12 and 13 as `r`, copied out as `*x.item()`, when every instantiation's read is a value or a stored borrow, and as `(ref r)`, a borrow of the parameter, for a reader with an instantiation whose read borrows the struct |

   A generic struct's read of a field whose type holds the struct's parameter never agrees with another struct's, so that field always takes an associated type. `(pair a)`'s `:count int` agrees with `cat`'s `:count int` and keeps the plain shape. In the last shape, every struct's getter returns `&self.item`. `Item` is the read itself for a value and for a stored borrow, `i32` or `&'a str`, so a reader whose instantiations give only such reads copies its read out with `*x.item()` under `Item: Copy`, and a stored borrow keeps the struct's lifetime there. One such read settles the reader's read type, and several keep it open as `r`. `Item` is the field's type for any other field, `String` or `A`, whose read is already a borrow of the struct. So `Item` is always the bound's type.

   **A bound names the field's type, the same in every shape and in both passes.** `(a (has :age t))` holds for a struct whose field has type `t` at that instantiation: `int` for `cat` and for `(boxed int)`, `(ref str)` for a `user` that stores `:name str`, and `string` for `book`. For a value and a stored borrow, `t` is the known read. For a read that borrows the struct, `t` is the type under that borrow. A reader reads `t` when every instantiation's read is a value or a stored borrow, and `(ref t)` otherwise, for every struct it takes. So a reader of `book` and `film` alone reads `(ref string)`, the known read, and a borrowing `age-of` of `cat`, `book` and `user` reads `(ref int)`, `(ref string)` and `(ref (ref str))`, the `&i64`, `&String` and `&&'a str` its getter returns. A body that fixes the read keeps this type and adds the read requirement of Rule 1, which a struct meets by its known read, so both `a1`'s stored `(ref spot)` and `a2`'s `spot` meet a fixed `(ref spot)`. The mode follows only the function's own instantiations, by refinements 12 and 13 and the caller rule below, so whether a struct meets a bound depends neither on another reader's calls nor on the field's shape. A written `(has :age (ref int))` for `cat` is `L0467` in every program, as Rule 4 says.

   **When the shape settles.** The checker checks a definition before the definitions that call it, and the members of a recursive group together. So a generic read keeps its variable `r` through inference, each instantiation settles `r` from the struct it takes, and the shape settles when every definition is checked.

   **A reader's instantiations** are the structs that reach it through every path the shape counts: its own calls and function values, the instantiations of every generic caller that passes its owner on, through that caller's bound, and the instantiations of every other member of its recursive group. So `(defn age-of (x) (:age x))`, called directly only with `dog`, also takes `user` and `cat` when `(defn show-age (x) (str "age " (age-of x)))` is called with them, and it settles from all three. A reader's set always holds its generic callers' sets, so a reader is never more settled than a caller that passes its owner on.

   **A recursive group** settles in one of two ways, as every Hindley-Milner group does. When an instantiation inside the group passes a struct, it fixes the owner for the whole group, the members are monomorphic, and an instantiation from outside with another struct is `L0401`. When no instantiation inside the group fixes the owner, the group generalizes, as on `main`, where mutually recursive `f` and `g` called as `(f 1 2)` and `(g "a" 2)` print `1` and `a`. Every member of such a group settles from the group's pooled set: with `a-age` and `b-age` calling each other on `x`, `a-age` called with `cat` and `dog` and `b-age` with `user`, both read with mixed kinds, and both copy out by refinement 13, because `int` is a value and `str` a stored borrow. A `book` with `:age string` passed to either makes both borrow.

   **Each reader settles its own `r`** from its instantiations, by refinement 12. When they give one read type, the checker writes that type into the reader and into the scheme of every generic caller that holds a copy of `r` through the reader's bound, so the caller reads as the reader does: in a plain shape every reader of the field settles so, which is refinement 7's read typed at once, and in the mixed shape a reader whose instantiations give one Copy read settles so too and copies it out of the getter. A reader whose instantiations give several read types keeps `r` generic, which in the value and stored-borrow shapes needs no change. In the mixed shape such a reader copies out under `Item: Copy` when every one of those reads is a value or a stored borrow, by refinement 13. The `Copy` bound follows from the instantiations, so it prints no part of its own and a pasted `(has :name r)` settles to it again, as a changing reader's `mut` does in Rule 3.2. The bound joins the Rust of every generic caller that passes its owner on, as the changing trait does, so `(defn via (x) (age-of x))` emits `fn via<A: HasAge>(x: &A) -> A::Age where A::Age: Copy { age_of(x) }`. Without it rustc rejects `via` as `E0277`.

   **Every function settles its own result.** A generic caller takes its mode from its own instantiations too, never from its callee's. A call whose instantiations, the caller's own for a generic caller or the struct a known call passes, give only values and stored borrows copies a borrowing callee's read out at the call. So `(defn g (x) (age-of x))`, called with `cat` and `dog`, emits `fn g<A: HasAge>(x: &A) -> A::Age where A::Age: Copy { *age_of(x) }` and returns `int` whether or not `show-age` passes `age-of` a `book`, and `(map (iter cs) age-of)` over a `(vec cat)` emits `.map(|c| *age_of(c))`. A copy out applies where the callee's result is the read itself, and where it holds the read in a type that copies out without an allocation: an `option`, as `age_opt(c).copied()`, an `iterator`, as `.copied()`, and a `pair`, as `let (age, n) = age_pair(x); (*age, n)`. A callee whose result holds the read inside an owning collection or a struct, as `(defn age-list (x) (vec (:age x)))` does, cannot be copied out without an allocation. A caller without a written type takes such a callee's mode with its type. A written type that needs the copy, a signature or an annotation of the caller or of any function whose result reaches it, fixes the callee's mode instead, so the callee takes only structs whose read is a value or a stored borrow, and a call that passes it another struct is `L0467` at that call. With a library's `(type ages (fn ((ref cat)) (vec int)))` and `(defn ages (c) (age-list c))`, a program's `(age-list (->book (str "old")))` gets `` `age-list` returns `:age` by value for `ages`, but `book`'s `:age` reads as `(ref string)` ``, with a label on `ages`'s signature, and the library's lines keep their verdict.

   **A read that meets a literal** follows the existing Literals rule, which a function's open literal type already follows: the literal type is not generalized, and the calls settle it. `(defn next-age (x) (+ (:age x) 1))` called with `cat`, whose `:age` is `int`, prints `(type next-age (where ((a (has :age int))) (fn ((ref a)) int)))`, as `(defn inc (x) (+ x 1))` prints `(fn (u8) u8)` when called with a `u8`. Called with `cat` and with `drone`, whose `:age` is `u8`, it is `L0401` at the second call, with the note `` `x` defaulted to `int`, because `next-age` is called with `cat` and with `drone` ``, the same note `inc` gets for two number types. A body that leaves the read generic, as `(str (:age x))` does, takes both.

   **The second check of the mixed shape.** In the mixed shape, a reader with an instantiation whose read borrows the struct, and another whose read is a value or a stored borrow, reads `(ref t)` for every struct it takes, and a caller whose result type comes from that read, as `(defn f (c) (age-of c))`, changes type with it. Every other reader keeps the read type its instantiations gave it, by refinements 12 and 13: a reader whose reads are all values and stored borrows copies them out, and a reader whose reads all borrow the struct already reads the borrow. So a second check runs only when some reader of a mixed-shape field takes both a struct whose read borrows it and one whose read does not. A reader of `user` (`str`) and `product` (`i32`) no longer needs it. It checks the whole program again with the settled read types, as `TypeChecker::check` (`src/types/checker.rs:2196`) already runs `check_program` again with a fresh checker when it finds new vector bases. A first pass with errors stops there and reports them, as that loop's `?` does, so the user sees one pass's errors, never both.

   **The first pass never rejects a program that the second accepts.** Both passes check every bound against the same field types, and the second differs from the first only in the borrow layer it adds to a borrowing reader's value and stored-borrow reads. Inside the reader, the adjustments that a known read gets cover that layer: the copy out of a borrow at a Copy use, and the borrow at a `ref` parameter. So a body that the first pass rejects fails the second too, and the layer shows only in the reader's result and in every type that the body unifies with the read, as a parameter `d` in `(if c (:age x) d)`. Those are the second pass's own errors. `(defn next-age (x) (+ (:age x) 1))` with `cat` and `(boxed int)` reads `int` from both in either pass. With a `book` passed to another reader, the field turns mixed and `next-age` still copies, `*x.age() + 1`, so it prints `4` and `8`. The settled read types join the state that `configured` carries into every later iteration of that loop, as `vector_bases` does, so a field check that turns a read into a borrow of a `vec` field, and so finds new vector bases, keeps its reads in the iterations that follow. One extra pass is enough: such a reader's read only gains a borrow layer, and a field bound looks through borrows (`field_owner`, `src/types/infer.rs:3564`), so no field's set of passed structs, and no reader's set of instantiations, changes in the second pass. On the two largest corpus entries, `deep_nesting.knv` and `generic_types.knv`, the type stage is about 0.11 to 0.13 seconds of a debug `knave check`, so the second pass adds about 25 to 30 percent to `check`, and at most one iteration beyond the vector-base loop's own. Only a program with such a reader pays it, and every such program fails on `main` with `L0407`.

   **An error of the second pass names its cause.** A reader's mode follows its instantiations, which any module can add, so an error of the second pass can land on a line that names neither the new struct nor the call that brought it. Every error of the second pass whose types hold a read that the second pass turned into a borrow carries a note that names the reader, every instantiation and why. It also carries a secondary label at the reader's first instantiation, in source order, whose read borrows the struct, as the Literals rule's `L0401` names the calls that defaulted a literal. When that instantiation reaches the reader through a generic caller, the label sits on the call of that caller, and a second note names the path, as Rule 1's chain does: `` `show` passes `x` to `age-of` ``. With `cat`, `dog`, `(defn age-list (x) (vec (:age x)))` and an unwritten `(defn ages (c) (age-list c))` in a library, and in the program `(defstruct book :age string)`, `(age-list (->book (str "old")))` and `(type older (fn ((vec int)) (vec int)))` with `(older (ages c))`:

   ```text
   error[L0401]: mismatched types
    --> crossmod.knv:6:19
     |
   6 |   (println (older (ages c)))
     |                   ^^^^^^^^ expected `(vec int)`, found `(vec (ref int))`
     |
    ::: crossmod.knv:4:12
     |
   4 |   (println (age-list (->book (str "old"))))
     |            ------------------------------- this call gives `age-list` a `book`, whose `:age` reads as `(ref string)`
     |
     = note: `age-list` reads `:age` through a borrow, because it is called with `cat`, whose `:age` is `int`, and with `book`, whose `:age` borrows the `book`
   ```

   A check fixture, `check/field_read_turned_borrow.knv`, with the library in a subdirectory, holds this error.

   **The note reaches the later stages.** An ownership error can also come from a read that the second pass turned into a borrow. A library `(defn oldest () (local c (->cat 9)) (age-list c))` returns a `(vec int)` while `age-list` copies, and a vector of borrows of its local `c` once the program passes `age-list` a `book`. So the type stage records each reader that the second pass turned borrowing, with its instantiations and its first borrowing instantiation, in the typed program beside the parameter modes that the ownership stage already reads. Every error of a later stage whose span holds a value typed from such a reader's read carries the same note, label and path note. The ownership stage does not yet report that returned borrow, as Rule 3 says, so `oldest` joins the fixture with its `L0510` once tracked item 550 lands.

   **A reader that no instantiation reaches** has no passed struct, so its read keeps its variable, and `knave types` prints `(type hello (where ((a (has :name r))) (fn ((ref a)) r)))`. The emitter writes no Rust for it, as it writes none for any unreached function on `main`, which still checks and prints it (`DependencyGraph::unchecked`, `src/types/graph.rs:234`). The same rule covers:
   - a library checked alone, whose doc page prints the open signature, while a program that uses the library prints the reader's type from that program's instantiations,
   - a definition at a `repl` or `eval` prompt, which prints the open signature when it is defined and takes its shape from the later prompts' instantiations, so a later `types` of that definition can print a settled type,
   - a reader instantiated only from a compile-time body, which the checker skips, so it takes no struct.
5. **Names.** The trait is `Has` and the field's Rust name in Pascal case, the associated type is the field's Rust name in Pascal case, and the getters are the field's Rust name and that name with `_mut`, by refinements 5 and 9: `admin?` gives `HasIsAdmin`, `type IsAdmin` and `is_admin()`, and the changing trait of Rule 3.2 is `HasIsAdminMut` with `is_admin_mut()`. A name that a written item takes yields as an introduced item does: `HasName2` beside a program's own `has-name` struct, and `name_2()` beside a program trait that gives the same struct a method `name`.
6. **The trait emits at the crate root, and each impl beside its struct.** A struct in another module emits its impl in that module's `mod` as `impl<'a> crate::HasName<'a> for Animal<'a>`, and a module that calls `name()` gets a Rust `use` of the trait, as a user trait's method call does. Every impl names its struct's lifetime and parameters, as `impl<'a> HasName<'a> for User<'a>` and `impl<A> HasItem for Boxed<A>`, the spelling that [10. User-defined traits](../docs/language.md#10-user-defined-traits) uses for `impl<'a> Finder for Snippet<'a>`.
7. **Projections emit as every projection does today.** The emitter writes `<A as HasName>::Name` (`src/emit/types.rs:508`), so a field named like another bound's associated type, such as `:item` beside a trait with `(type item)`, is never ambiguous. A fixed read emits as the trait's binding, as `HasName<Name = &'a str>` in the mixed shape, where `Name` is the stored borrow, or `HasTag<'a, Tag = str>` in the stored-borrow shape. A stored borrow's lifetime is the trait's own parameter, never the parameter's borrow, so `(defn trimmed (x) (trim (:name x)))` emits `fn trimmed<'a, A: HasName<'a>>(x: &A) -> &'a str`, and its result outlives a temporary argument.
8. **A read whose type carries a field bound of its own emits its owner as a type parameter.** A field bound looks through borrows, as a known read strips its owner's borrows (`field_owner`, `src/types/infer.rs:3564`): `(b (has :x f))` holds for a struct `b` with `:x` and for any borrow of one. So `(defn start-x (l) (+ (:x (:start l)) 1.0))`, with two line structs whose `:start` differ and two point structs with `:x float`, gives `(a (has :start b))` and `(b (has :x float))`, and emits `fn start_x<A: HasStart<Start = B>, B: HasX>(l: &A) -> f64 { l.start().x() + 1.0 }`. A borrowed intermediate works as a by-value one does: `A: HasHome<Home = B>, B: HasLabel` reads `x.home().label()` through `&Self::Home`.

   **An intermediate that is a stored borrow.** In the mixed shape, `Item` is the read itself for a stored borrow, so a struct that stores `:start (ref spot)` gives `B = &'a Spot`, and `B: HasX` needs an impl for a borrow. The emitter writes one blanket impl beside the inner field's trait exactly when a chained read's intermediate can be a stored borrow, that is when the outer field has the mixed shape and some passed struct's read of it is a stored borrow, a declared `(ref spot)` or a `(ref a)` of a generic struct's parameter, as `(struct (holder a) (start (ref a)))` at `(holder spot)` or with `a` open:

   ```rust
   impl<T: HasX + ?Sized> HasX for &T {
       fn x(&self) -> f64 {
           (**self).x()
       }
   }
   ```

   A trait with an associated type passes it through, as `type X = T::X`, and a trait with a lifetime parameter takes it, as `impl<'u, T: HasLabel<'u> + ?Sized> HasLabel<'u> for &T`. A program without such a read emits no blanket impl, so its Rust is unchanged. When every passed struct stores the outer field as a borrow, the field has the stored-borrow shape, `B` is the struct under the borrow, and no blanket impl is needed. That shape's chained read takes an outlives bound, because `B` sits under the trait's lifetime: `fn start_x<'a, A: HasStart<'a, Start = B>, B: HasX + ?Sized + 'a>(l: &A) -> f64`. Without `B: 'a`, rustc rejects it as `E0309`. With `line` storing `:start point` and `span` storing `:start (ref spot)`, `start-x` prints `2` and `3`. Without the impl, rustc rejects it as `E0277`, `` the trait `HasX` is not implemented for `&Spot` ``.
9. **A pinned function emits what it emits with the struct written in its signature.** With only `(hello (->user "ana"))` in `main`, `hello` emits `fn hello(x: User<'_>) -> String` and `x.name`, the Rust that `main` emits today when `user` is the only struct with `name`. A pinned read writes the known read, and an adjustment the checker recorded on a mixed-shape `(ref r)`, such as the dereference of a Copy value, folds into it, as `*&x.age` is `x.age`. A reader pinned to `(boxed int)` reads `int`, as a known read does. The single-struct program that is `L0401` on `main`, ``expected `(ref int)`, found `int` ``, keeps that error, because a pinned function is the function that `main` checks, and the fault lies in pinning, outside this RFC.

### 3. Ownership

**A read has the type that its trait's shape gives, Rule 2.4.** In the plain shapes and the value shape that is the known read's type: the read of a Copy `int` field is `int`, the read of a `str` field is `(ref str)` with the struct's lifetime, and the read of a `string` field is `(ref string)`. In the stored-borrow shape it is a borrow with the struct's lifetime. In the mixed shape a reader whose instantiations give only values and stored borrows has its read, with the struct's lifetime for a stored borrow, and a reader with an instantiation whose read borrows the struct has a borrow of the parameter, `(ref t)` by Rule 2.4, so the emitter treats a returned read as a borrow of the argument, as it does for a `string` field on `main`: `(let (n (name-of (->owner (str "a")))) (println n))` emits `let owner = make_owner("a".to_string()); let n = name_of(&owner);`. Every adjustment runs on that type, as it does for a known read.

The owner's accepted cost, narrowed by refinement 13: a reader with an instantiation whose read borrows the struct cannot return a stored borrow's read past its argument. `(defn pick () (local u (->user "ana")) (name-of u))` returns `&'static str` when `name-of` reads a known `user`, when it takes only stored borrows of `str`, and when it takes `user` and `product`, whose `str` and `i32` reads it copies out. When `name-of` itself takes `user` and a `book` with `:name string`, the result borrows the local `u`. On `main` the ownership stage does not report a call's result that borrows a local argument: `(defstruct book :age string)`, `(defn age-of (x) (:age x))` and `(defn pick () (local u (->book (str "a"))) (age-of u))` pass `knave check` and fail in rustc with `E0515`, as a direct `(:age u)` does. Tracked item 550 covers that leak, beside issues #507 and #510, which cover two others. This RFC inherits it: such a `pick` is rustc's `E0515` until item 550 makes it `L0510`, and no fixture holds it before then. A reader takes mixed kinds only in a program that fails on `main` with `L0407`.

| Use | Bound | Emitted read |
| --- | --- | --- |
| returned, `(defn name-of (x) (:name x))`, `:name str` on two structs | `(a (has :name (ref str)))` | `fn name_of<'a, A: HasName<'a>>(x: &A) -> &'a str { x.name() }`, and the result outlives a temporary argument |
| a Copy field in arithmetic, `(+ (:age x) 1)`, `:age int` on two structs | `(a (has :age int))` | `x.age() + 1` |
| a `mut` parameter that reads too, `(set! (:age x) (* (:age x) 2))` | `(a (has :age int))`, and `x` takes `mut` | `*x.age_mut() *= 2`, by the Assignment bullet of Ownership, as `main` emits `x.age *= 2` |
| owned where the read type is open, `(set! total (:level x))`, `:level` an `int` and a `u8` | `(a (has :level r))`, `(r copy)` | `total = x.level()` |
| a clone, `(clone (:title x))`, `:title string` on two structs | `(a (has :title string))` | `x.title().clone()` |
| a move, `(keep (:title x))` with `keep` taking an owned `string` | none | `L0503` at the read, as for a known struct today, with the help `(clone ...)` |

1. **A read never moves the field,** as on `main`, where only the base of a collection or struct update moves one. A use that needs to own a field that is not Copy is `L0503` today and stays `L0503`.
2. **A `set!` of a generic field, or a `mut` borrow of one, needs a second trait, `Has<Field>Mut`, whose supertrait is the field's trait:**

   ```rust
   trait HasNameMut: HasName {
       fn name_mut(&mut self) -> &mut Self::Name;
   }
   ```

   The trait emits only when a reached function changes the field, and a struct implements it only when a reached changing reader receives it, so a struct that only read-only readers receive never needs it. A Rust trait's methods are all or nothing, so one trait with both getters would make every struct of the mixed shape write `name_mut`, and a struct that stores `:name str` cannot give `&mut str` from `&'a str`, which rustc rejects as `E0046`. The changing getter returns `&mut` of the stored type, which the read gives in every shape where the method exists: `fn age_mut(&mut self) -> &mut i64` in the plain value shape, `&mut String` in the plain owned shape, `&mut Self::Age` and `&mut Self::Item` with an associated type. The bound prints no part of its own for a change: the checker reads the need from the body's `set!`, as it reads the parameter's `mut` mode, so a reader's text still pastes into a signature. `(defn birthday! (x) (set! (:age x) (+ (:age x) 1)))` emits `fn birthday<A: HasAgeMut>(x: &mut A) { *x.age_mut() += 1; }`.

   The changing trait takes the field trait's lifetime parameter under refinement 10, as `trait HasOwnerMut<'u>: HasOwner<'u> { fn owner_mut(&mut self) -> &mut User<'u>; }`, so `(set! (:owner c) (->user "bob"))` on `car` and `boat` emits `*c.owner_mut() = make_user("bob")`.

   **A compound assignment whose right side reads the same owner binds that side first** when the read type is not a known primitive. Rust evaluates a compound assignment's right operand first only when both operands are primitive types, so with an open read, as in the value shape where `cat` gives `int` and `drone` gives `u8`, `*x.age_mut() += x.bonus()` keeps the `mut` borrow live during the read, which rustc rejects as `E0502`. `(set! (:age x) (+ (:age x) (:bonus x)))` then emits `let value = x.bonus(); *x.age_mut() += value;`, and `(set! (:age x) (+ (:age x) (:age x)))` emits `let value = x.age(); *x.age_mut() += value;`, and in the mixed shape, where the getter returns `&Self::Age`, the copying reader of refinement 13 writes `let value = *x.age();`, as the Assignment bullet of Ownership already reads the operands into locals when the right side can change the place. A known primitive read keeps the compound form, with each right-side read copied out where its getter returns a borrow: with `HasAgeMut<Age = i64>`, `*x.age_mut() += x.bonus()` compiles in the plain and value shapes, as `x.age += x.bonus` does on `main`, and a mixed `:bonus` writes `*x.age_mut() += *x.bonus()`, because `i64 += &i64` is an overloaded operator whose place Rust evaluates first.

   **A trait method cannot change a field through a `has` bound.** A `deftrait` method's signature may hold a `has` bound for reads, but its Rust signature has no body to read a change from, so it cannot carry the changing trait, and an impl body that changes the field would be rustc's `E0276`, "impl has stricter requirements than trait". Such a `set!` is `L0468` at the `set!`, a new code of the type stage: `` `rename!`'s signature in `named` lets `x` read `:name`, not change it ``, with a help that moves the change into a function outside the trait.

   A struct that stores the field as a borrow never implements the changing trait, because a `set!` of that field on a known value is `L0401` on `main`, as `(set! (:name u) "bob")` on a `str` field is. A call that passes such a struct to a reader that changes the field is `L0467` at the call, with the known `set!`'s reason, and no other error reaches the user. Open detail 1 asks the owner to confirm the trait.
3. **A struct update or a collection update on a generic owner stays an error.** `(assoc p :x 2)` builds a new value of the owner's struct, and a field trait has no constructor. Rule 4 gives the error its new text.
4. **A generic owner is never Copy.** A parameter whose type only a field bound knows takes `ref` or `mut`, never copy, unless the body infers `copy` for it. That keeps decision 1's `hello` at `x: &A`.

### 4. Diagnostics with several candidates

A read on an unknown owner prints as the bound everywhere a type prints, as a `deftrait` with an associated type prints `(c (container item))` in [10. User-defined traits](../docs/language.md#10-user-defined-traits):

| Where | Text |
| --- | --- |
| `knave types` and a doc page's signature | `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))` |
| `(infer (:name x))` in `hello` | `"(ref str)"` in a plain shape. `"r"` or `"(ref r)"`, by Rule 2.4, in a shape with an associated type, as `(infer x)` is `"a"` |
| `(infer hello)` | `"(where ((a (has :name (ref str)))) (fn (a) string))"`, with modes left out by the existing rule |
| a type error that names the owner | `a`, with a note that `a` is any struct with `:name`: `user` or `animal` |

The text pastes into a `type` signature. A written bound's type is the field's type of Rule 2.4 in every shape, and a third element is the read requirement of Rule 1, so `(has :age int)` holds for `cat` whatever other readers take, a written `(has :age (ref int))` is `L0467` at every call that passes `cat`, and `(has :age int (ref int))` holds. The mode shows only in the function's result type, `(fn ((ref a)) int)` or `(fn ((ref a)) (ref int))`, which follows the function's own instantiations, so a generic caller's printed text never depends on calls it does not make. A written result type fixes the mode, as a written signature is exact (Rule 6.4). A written copying result, as a library's `(type age-of (where ((a (has :age t))) (fn ((ref a)) t)))`, takes only structs whose read is a value or a stored borrow, and a program that passes it a `book`, directly or through a generic caller, gets `L0467` at that call: `` `age-of` returns `:age` by value, but `book`'s `:age` reads as `(ref string)` ``. A written borrowing result, `(fn ((ref a)) (ref t))`, makes the field take the mixed shape, as a struct whose read borrows would, so the reader keeps its written type whichever structs reach it. So a written signature can state the bound: `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`. A bound may hold several fields, `(a (has :name (ref str)) (has :age int))`, and a read type may carry its own bound, as Rule 2.8 shows.

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

After the build, the lookup still counts every struct of the program, as `main` does. One candidate still fixes the owner at the read, so every program that compiles on `main` keeps its types. Two or more give the owner the field bound, and a later form of the body that fixes the owner checks the bound then, so the read no longer decides at its position. None is `L0408` at the read, as Rule 4 shows.

**An open literal type wins over the lookup.** On `main`, `(:x n)` after `(let (n 5) ...)` is `L0407`, as `L0407.md` documents. A read whose owner is an open integer or float literal type is `L0410` at the read, `` `int` has no fields ``, naming the literal's default type, before any candidate counts, because no struct is a number.

| Program | On `main` | After the build |
| --- | --- | --- |
| decision 1, `user` and `animal` | `L0407` at `(:name x)` | generic, as Option A shows |
| decision 2, `user`, `animal` and `product` | `L0407` at each `(:name x)` | generic, as Option A shows |
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

**Fixtures for the shapes.** The fixtures come in groups. Each fixture of a group adds one struct to the fixture above it, with a second reader that takes the new struct, and switches the trait's shape, while the first reader's `(println (infer ...))` line, which the emitted Rust holds as a string literal, stays the same. The first reader never takes the new struct, because its body fixes a read the new struct fails: `next-age` with `drone` would be `L0401` with the Literals rule's note, and `uppercase-name` with `poll` or `product`, and `shout` with `(boxed int)`, would be `L0467` at the call. The last three fixtures stand alone.

| Group | Fixture | Structs | First reader | Second reader | Trait |
| --- | --- | --- | --- | --- | --- |
| value | `emit/field_trait_plain_value.knv` | `cat`, `dog` with `:age int` | `next-age`, `(+ (:age x) 1)`: `(where ((a (has :age int))) (fn (a) int))` | none | `trait HasAge { fn age(&self) -> i64; }` |
| value | `emit/field_trait_value_types.knv` | the same, and `drone` with `:age u8` | the same | `show-age`, `(str "age " (:age x))`, with `cat` and `drone` | `trait HasAge { type Age; ... }`, `HasAge<Age = i64>` |
| borrow | `emit/field_trait_plain_borrow.knv` | `user`, `animal` with `:name str` | `uppercase-name`: `(where ((a (has :name (ref str)))) (fn (a) string))` | none | `trait HasName<'u> { fn name(&self) -> &'u str; }` |
| borrow | `emit/field_trait_stored_borrows.knv` | the same, and `poll` with `:name (slice int)` | the same | `count-name`, `(len (:name x))`, with `user` and `poll` | `trait HasName<'u> { type Name: ?Sized; ... }`, `HasName<'a, Name = str>` |
| borrow | `emit/field_trait_mixed.knv` | the same, and `product` with `:name i32` | the same | `describe`, `(str "name: " (:name x))`, with `user` and `product` | `trait HasName { type Name: ?Sized; ... }`, `HasName<Name = &'a str>` |
| owned | `emit/field_trait_plain_owned.knv` | `book`, `film` with `:title string` | `shout`, `(loud (:title x))` with `(type loud (fn ((ref string)) string))`: `(where ((a (has :title string))) (fn (a) string))`, whose fixed `(ref string)` holds for `string` | none | `trait HasTitle { fn title(&self) -> &String; }` |
| owned | `emit/field_trait_generic_struct.knv` | the same, and `(boxed a)` with `:title a` | the same, with `book` and `(boxed string)` | `size`, `(:title x)` returned, with `film` and `(boxed int)` | `trait HasTitle { type Title: ?Sized; ... }`, `impl<A> HasTitle for Boxed<A>` |
| alone | `emit/field_trait_mixed_value_reader.knv` | `cat`, `dog` with `:age int`, and `book` with `:age string` | `age-of`, with `cat` and `dog`, and `ages`, `(collect (map (iter cs) age-of))` under `(vec int)`: `(where ((a (has :age int))) (fn (a) int))` | `show-age`, `(str "age " (:age x))`, with `book` and `cat`, through a borrow | `trait HasAge { type Age: ?Sized; fn age(&self) -> &Self::Age; }`, `HasAge<Age = i64>` and `*x.age()` for `age-of` |
| alone | `emit/field_trait_unpassed_struct.knv` | `cat`, `dog` with `:age int`, and `user` with `:age str` that no reader takes | `age-of`: `(where ((a (has :age int))) (fn (a) int))`, as with `cat` and `dog` alone | none | `trait HasAge { fn age(&self) -> i64; }`, no impl for `User` |
| alone | `emit/field_trait_plain_value_lifetime.knv` | `user` with `:name str`, and `car`, `boat` with `:owner user` | `owner-of`: `(where ((a (has :owner user))) (fn (a) user))` | none | `trait HasOwner<'u> { fn owner(&self) -> User<'u>; }` |


Run fixtures cover the programs the reviews raised: a name read from a temporary in the plain and stored-borrow shapes, `trimmed`, `double-age!`, `start-x` through a by-value point, a chained read through a Copy struct that holds a borrow, a two-field reader, `(boxed a)` read at `string` and `int` by one generic `get`, and a field `:item` beside a trait with `(type item)`. A run fixture holds `pick` and `grow!` in the mixed shape, with `name-of` taking `user` and `product`, `grow!` taking `cat` and `drone`, and a `book` with `:name string` and `:age string` passed to a third reader so that both fields are mixed. Both readers copy out. Another holds `next-age` on `cat` and `(boxed int)` beside a `book`, printing `4` and `8`, and `start-x` on `line` and on a `span` that stores `:start (ref spot)`, printing `2` and `3`.

### 6. Interplay

1. **Structs across modules.** A candidate is any struct of the program, as on `main`. The checker has no field visibility rule: a known read of a private struct's field from another module compiles on `main`, and a generic read checks no visibility either. So a library can hold a generic reader for its callers' structs, and the trait and impls of Rule 2.6 span the modules.
2. **Generic structs.** A generic struct is a candidate like any other. Its parameter's field reads by what is known at each instantiation, by Rule 2.3, so `(boxed int)` meets `(has :name int)` and `(boxed string)` meets `(has :name string)`. One `impl<A> HasName for Boxed<A>` with `type Name = A` serves both, and a generic caller that passes `(boxed t)` needs no second impl. `(defn get (b) (:name b))` called on a `(boxed string)` and a `(boxed int)` compiles as on `main`. The `L0401` that `main` reports for the single-struct `boxed` program, ``expected `(ref int)`, found `int` ``, is a pinning fault outside this RFC.
3. **An enum's case.** An enum is never a candidate. A case's fields exist only inside that case, so a getter cannot exist for the enum. A program reads a case's field through `match`, as on `main`, and a generic reader called with an enum is `L0410`.
4. **A written type annotation.** A written signature is exact. One that names a struct emits today's Rust. One that names a type variable without the field bound is `L0440`, with a help that writes the bound, as `(a (has :name (ref str)))`, as a missing `copy` gets `(a copy)` today. A binder annotation, as `(x (ref user))`, fixes the owner.
5. **A closure.** A `local` closure stays monomorphic, as on `main`, so a field bound on its own parameter must settle by the end of checking. `(let (f (fn (y) (:name y))) (f (->user "ana")) (f (->animal "rex")))` is `L0401`, ``expected `user`, found `animal` ``, at the second argument, as `(let (f (fn (y) y)) (f 1) (f "a"))` is today. A closure that no call fixes is `L0407` at the read, `` cannot tell which struct's `name` this is ``, with the note of Rule 4 that lists the candidates. A read inside a closure whose owner is the enclosing function's parameter gives the function the bound, so it generalizes with the function.
6. **A field the prelude reads.** The prelude declares no struct and no function that reads a field. Its macros, `defstruct`'s constructors, `assoc`, `assoc-in`, `update` and the keyword closure, expand to `field` and `new` in the user's module, so their reads follow these rules there. A keyword closure, as `(map (iter xs) :name)` in an unannotated function, gives the function the bound through its parameter's element type. `update` reads and then builds, so a generic owner is the update error of Rule 3.3.

### 7. Prelude first

The prelude cannot express this, and the build adds no kernel form:

- **The choice is the checker's.** Whether `(:name x)` emits `x.name` or `x.name()` depends on the type of `x` after inference and pinning. A macro runs before the checker and sees only the form.
- **The struct set is the program's.** The candidates span modules, and a trait's shape and its impls depend on the calls that reach its readers. Only the compiler sees every struct, every call and every pin.
- **A trait per field from a macro clashes,** as Option B says, and [RFC 0038](0038-field-keywords.md) moved keyword reads into the expander for the same reason.

The change lives in the type rule of the existing kernel form `field`, and in one kernel name of a new kind: the bound `has`, which a `where` writes beside traits. A kernel name is needed because the prelude cannot define a family of bounds keyed by a field name, and the printed bound must paste into a signature. `has` is exported by the built-in `kernel` module and re-exported by the prelude like every kernel name. The build adds it as a row of the table in [Types](../docs/language.md#types), `(where ((a (has :age int))) (fn ((ref a)) int))` beside `fn<A: HasAge>(&A) -> i64`, and records why it is a kernel name in [Kernel forms](../docs/language.md#kernel-forms), beside the reasons of the other kernel names, and adds it to that section's list of the kernel names that type positions hold (`docs/language.md:85`), beside `fn`, `ref`, `where` and `with`. `prelude/prelude.knv` documents `has` with a `document` form, as it documents `struct`. The Rust names `HasName`, `Name` and `name()` are no fact the prelude states, so the compiler derives them from the field, as it derives `make_point` from `->point`.

## Open details

1. **The changing getter.** Rule 3.2 puts `<field>_mut` in a second trait, `Has<Field>Mut: Has<Field>`, implemented only by the structs that a changing reader receives, so a struct that stores the field as a borrow never has to write it. The alternative is an error at a `set!` of a generic field, with help to write a signature. The recommendation is the second trait, because Rust programmers write it, `L0467` at the call stays the only error, and the uniform `set!` rule then holds for generic owners.
2. **The bound's spelling.** This RFC writes `(has :name r)`, and `(has :name r (ref str))` with a fixed read. A keyword names the field as a read and a `defstruct` do. `field` would read as the kernel form, and a name ending in `?` reads as a predicate. The owner may prefer another name.
3. **An owned text or vector field read through its view.** On `main`, a known read of a `string` field is `(ref string)`, and it meets `(ref str)` by the `str` adjustment. Rule 1 now checks a fixed read type by the struct's known read and the adjustments of a call argument, so `uppercase-name` takes a `person` with `:name string` beside `user`, through `A::Name: ReadAs<str>`, and `as-str` with its `(ref string)` takes it too. The view rule, under which the bound would read a `String` field as `(ref str)`, is no longer needed for that, and the RFC no longer recommends it: it would change the plain owned shape, the changing getter and the `clone` row for no case that Rule 1 rejects. After issue #589, two structs that both write `:name str` and emit `&str` and `String` have different field types, `(ref str)` and `str`, so an open read takes both as `r`, and a fixed `(ref str)` takes both by Rule 1. The owner may still prefer the view rule for its shorter getters.

## Prototype

`target/scratch/proto3/` to `target/scratch/proto13/` held the prototype, which is not part of this pull request. The eighth review's `rs8/borrowref.rs`, `rs8/twolevel.rs`, `rs8/refint.rs` and `rs8/optcopy.rs`, the seventh review's `rs7/refspot.rs`, `rs7/view.rs`, `rs7/callerflip.rs` and `rs7/chainshape.rs`, the sixth review's `rs6/refine13.rs`, `rs6/boxcat.rs` and `rs6/chain.rs`, and the fifth review's `rs5/copyout.rs`, `rs5/mixedcmpd.rs`, `rs5/transitive.rs`, `rs5/group.rs` and `rs5/tempborrow.rs` cover the same claims for a reader who cannot rerun these files. rustc 2024 compiled and ran the Rust of Option A, Rule 2 and Rule 3:

- `proto4/d1.rs`, decision 1 in the plain stored-borrow shape, printed `hello ana` and `hello rex`, then a name and a trimmed name read from temporaries.
- `proto4/d2.rs`, decision 2 in the mixed shape before refinement 12, printed `ANA`, `REX`, `name: ana` and `name: 7`. A name read from a temporary there is `E0716` in Rust, which the ownership pass's binding of the temporary prevents for a reader that takes mixed kinds, as Rule 3 says.
- `proto4/values.rs`, `values2.rs` and `stored2.rs` held the fixture pairs of Rule 5.
- `proto5/mixedmut.rs` held the changing trait of Rule 3.2 in the mixed and plain value shapes, with `user` implementing only `HasName`, and the plain owned shape with `&String`, read by `shout` through `loud`.
- `proto6/ownerval.rs` held refinement 10: the reviewer's `owner-of` on a temporary `car` and a `boat`, an `(option (ref str))` field, and a value shape whose `type Pin = User<'a>` keeps the lifetime. `proto6/ages.rs` held refinement 11: `ages` collects `int` reads of `cat` while a `user` with `:age str` reaches no reader and gets no impl.
- `proto13/round8.rs` held the eighth review's fixes: `ReadAs` over `user`, `person` and a `holder` that stores `&String`, printing `ANA BOB CY`, over a `Vec<i64>`, a `&Vec<i64>` and a `&[i64]` for a slice, printing `3 3 3`, and over `Spot`, `&Spot` and `&&Spot`, printing `a b a`. It also held a stored `&Pt` beside a Copy `Pt` under `(ref pt)`, printing `1 2`, `r` passing `&x.age()` to `show_ref` for `cat` and `dog`, printing `age 3 age 4`, and copy outs of an `option`, a `pair` and an `iterator`, printing `Some(3) (4, 1) [5]`.
- `proto12/round7.rs`, before round 8, held the seventh review's fixes: `uppercase-name` on a `user` that stores `&str` and a `person` that owns a `String` through `Borrow<str>`, printing `ANA BOB`, and a borrowing `age-of` beside `g`, `g2`, `ages` and a known call that copy its read out by their own instantiations, printing `age old 3 4 [5] [6] 7`. A written copying reader called with a `book` is `E0277` there, which Rule 4 reports as `L0467`.
- `proto11/round6.rs` held the sixth review's fixes: `next-age` on `cat` and `(boxed int)` in the value shape, with `impl<A: Copy> HasAge for Boxed<A>`, and in the mixed shape beside a `book`, printing `4 8` in each. It also held a borrowing `age-of` that returns `&i64`, `&String` and `&&str`, and the blanket impls of Rule 2.8 for a plain trait and for one with a lifetime parameter, printing `2 3` and `lo`. `proto11/assoc.rs` held the blanket impl with an associated type and printed `x 1 x far`.
- `proto10/copyout.rs` held refinement 13 in the mixed shape, with a `book` whose `:name` and `:age` are `string`: `name-of` on `user` and `product`, `describe`, `pick`'s `&'static str`, `grow!` on `cat` and `drone` through `let value = *x.age();`, and a borrowing reader of `book` and `user`. It printed `ana rex 7 ana`, `name: ana name: 7`, `4 6`, `b eve age old` and `age 4`.
- `proto9/transitive.rs`, before refinement 13, held a reader's instantiations through a generic caller, `age-of` taking `user` and `cat` through `show-age`, and a recursive group that settles from its pooled set, and printed `4 age old age 9 1 5 old`.
- `proto8/perreader.rs`, before refinement 13, held refinement 12 on the fourth review's program: `age-of` copies `*x.age()` and stays `int`, so `ages` collects a `Vec<i64>`, `show-age` takes `user` and `cat` through a borrow, and `age_text`, which takes only `user`, copies the stored `&'a str` out with the struct's lifetime past a temporary. `proto8/d2.rs` held decision 2 under refinement 12.
- `proto7/cmpd.rs` held the compound assignment of Rule 3.2, `let value = x.bonus(); *x.age_mut() += value;` over `cat` and `drone`, with the known-primitive form beside it and `E0502` for the unbound open form, and `HasOwnerMut<'u>: HasOwner<'u>` on `car` and `boat`.
- `proto3/shapes.rs` held every shape with the probes of the first two reviews, rebuilt from their text, with the plain owned shape written in the view rule's `&str`: `double-age!` and `double-level!` with a `mut` parameter that also reads, a name and a trimmed name from temporaries, a two-field reader, a stored borrow of a local, a generic `get` over `Boxed<i64>`, `Boxed<String>` and a generic caller, the mixed shape, `start-x` through a by-value point, a chained read through a Copy struct that holds a borrow, a chained read through a borrow, and `:item` beside a trait with `type Item`.

Calling `uppercase_name(&Product { .. })` gave rustc's `E0271`, which Rule 1 replaces with `L0467`. The instrumented `main` build of Rule 5 changed only two `eprintln!` lines in `src/types/infer.rs`.
