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

Every struct with `:name` reads it as `(ref str)`, so the read takes that type at once, and `knave types` prints `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`.

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

The read types now differ, one a value and two stored borrows, so every read goes through a borrow of the struct. After the build:

```rust
trait HasName {
    type Name: ?Sized;
    fn name(&self) -> &Self::Name;
}

impl<'a> HasName for User<'a> {
    type Name = str;
    fn name(&self) -> &str {
        self.name
    }
}

impl<'a> HasName for Animal<'a> {
    type Name = str;
    fn name(&self) -> &str {
        self.name
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

fn uppercase_name<A: HasName<Name = str>>(x: &A) -> String {
    x.name().to_uppercase()
}

fn describe<A: HasName>(x: &A) -> String
where
    <A as HasName>::Name: std::fmt::Display,
{
    format!("name: {}", x.name())
}
```

`upper-case` takes `(ref str)`, so the body fixes the read: `(type uppercase-name (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`, the same text as with two structs. `describe` uses the read only through `display`: `(where ((a (has :name (ref b))) (b display)) (fn ((ref a)) string))`. Each call takes its own struct's type, so `describe` reads a `str` from `user` and an `i32` from `product`.

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

## Rejected options and why

- **B: a trait per field from `defstruct`:** two structs that share a field clash, every program's Rust changes, and a macro cannot see whether a read's struct is known.
- **C: one copy per struct:** the Rust grows by one function per struct and reader, calls name a struct the source never wrote, and function values need a choice the source never states.
- **D: a written accessor:** decision 1 requires the unannotated function to compile.
- **A field bound on every unknown owner, one candidate included:** a reader of the only struct with `:name` would print a `where` and emit a trait for one implementor, where a concrete type says the same thing and reads better. Rule 5 keeps one candidate concrete.
- **One associated type name, `FieldType`, for every field trait:** an owner with two field bounds makes `A::FieldType` ambiguous, `E0221`. Refinement 5 names it after the field.
- **A getter that returns `&Self::FieldType` for every field:** a stored borrow's read would end at the struct's borrow, and a Copy field would read as a borrow, unlike the known read. Refinement 6 replaces it.
- **A generic associated type for the read, `type Name<'s> where Self: 's`:** it serves every kind of field in one shape, but a `mut` parameter that also reads the field is `E0502`, a chained read is `E0597`, the `for<'s>` bound that would repair them implies `'static`, and a fixed read `HasName<Name<'a> = &'a str>` merges the stored borrow's lifetime with the parameter's, which is `E0716` for `(defn trimmed (x) (trim (:name x)))` on a temporary. Refinement 8 replaces it.
- **Per-struct copies of a reader called with mixed kinds:** `describe_user` and `describe_product` would keep each stored borrow's lifetime where the mixed shape borrows the parameter. This is Option C for one shape, with its costs, so the owner rejected it for now and may revisit it if programs reach the mixed shape's `L0510`.
- **Candidates limited to the structs the reading module sees:** the checker has no field visibility rule today. A known read of a private struct's field from another module compiles on `main`, so a limit only for generic reads would be an exception. It would also break a library reader: `(defn hello (x) (str "hi " (:name x)))` in a library compiles on `main` for its caller's `user`, and a library could not hold a generic reader of its callers' structs.

## Rule

The build states these rules in [docs/language.md: Field keywords](../docs/language.md#field-keywords), [Types](../docs/language.md#types), [Ownership](../docs/language.md#ownership), [Names in Rust](../docs/language.md#names-in-rust) and [10. User-defined traits](../docs/language.md#10-user-defined-traits), in the explanation of `L0407`, and in a new explanation of `L0467`. It replaces the two sentences of Field keywords that say a function needs a signature when two structs share a field name, and it rewrites the paragraph of `expand/src/diagnostic/explanations/L0407.md` that says a field read or update finds its struct by the field's name: a read with no candidate is `L0408`, a read with several gives a field bound, and only an update with several, or a bound nothing fixes, stays `L0407`.

### 1. A call whose field type fails the body's use

The bound's read type is a type like any other. A body that fixes it, as `upper-case` fixes it to `(ref str)`, gives the bound that type. A body that uses it through a trait, as `str` uses `display`, gives the read type that trait bound. A call checks the bound against the struct it passes, and a struct whose read fails either is `L0467` at that call, a new code of the type stage. `L0465` is the last code on `main`, and PR 676 takes `L0466`.

```lisp
(defn main ()
  (println (uppercase-name (->product 7))))
```

After the build:

```text
error[L0467]: `uppercase-name` needs a `:name` that reads as `(ref str)`, but `product`'s `:name` is `i32`
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
error[L0467]: `describe` needs a `:name` that implements `display`, but `badge`'s `:name` is `(fn () int)`
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
2. **An impl emits only for a struct that a reached call passes for the bound,** directly or through another generic function's bound, as the structural trait impls of [8.2 Struct traits](../docs/language.md#82-struct-traits) do. A struct no call passes gets no impl. A reader that changes the field needs a second trait, Rule 3.2.
3. **Each struct's read of the field has a kind and a type, from the Fields bullet of [Ownership](../docs/language.md#ownership):**

   | Kind | Field | Read on `main` | Rust read |
   | --- | --- | --- | --- |
   | value | a function type, or a Copy type that the size rule of Ownership: Fields copies | the value | `i64`, `fn(&str) -> bool` |
   | stored borrow | `str`, `(slice a)`, `(ref t)` | the stored borrow, with the struct's lifetime | `&'a str`, `&'a [A]`, `&'a T` |
   | borrow of the struct | any other type, and in a bound, any field other than a stored borrow whose declared type holds a type parameter of its struct | a borrow of the field | `&String`, `&A` |

   On `main`, a known read tests the size rule on the instantiated field type at the read (`src/types/infer.rs:3530`), so a generic struct's field reads by what is known there: `(boxed int)` known at the read gives `fn get(b: &Boxed<i64>) -> i64 { b.name }`, a parameter with a `number` bound gives `fn twice<A: Number>(b: Boxed<A>) -> A { b.age + b.age }`, and an argument still unknown gives `(ref a)`, as `fn get<A>(b: &Boxed<A>) -> &A` and `&b.name + 1` show. In a bound, the read takes the last of these for every argument, so one impl serves every instantiation of a generic struct. A field whose declared type does not hold the parameter, such as `:count int` in `(struct (pair a) (count int) (val a))`, reads as any other field does.
4. **The trait's shape is the simplest that every struct declaring the field allows,** by refinements 7 and 8. The candidates are the structs of Rule 5, not only the ones a call passes, so a reader's shape does not change with its calls:

   | The structs' reads | Trait | Generic read |
   | --- | --- | --- |
   | one type, value | `trait HasAge { fn age(&self) -> i64; }` | `int`, at once |
   | one type, borrow of the struct | `trait HasTitle { fn title(&self) -> &String; }` | `(ref string)`, at once |
   | one type, stored borrow | `trait HasName<'u> { fn name(&self) -> &'u str; }` | `(ref str)`, at once |
   | differing, every one a value | `trait HasAge { type Age; fn age(&self) -> Self::Age; }` | `r` |
   | differing, every one a stored borrow | `trait HasTag<'u> { type Tag: ?Sized; fn tag(&self) -> &'u Self::Tag; }` | `(ref b)`, with the struct's lifetime |
   | differing otherwise, mixed kinds and generic structs included | `trait HasItem { type Item: ?Sized; fn item(&self) -> &Self::Item; }` | `(ref b)`, a borrow of the parameter |

   A generic struct's read of a field whose type holds the struct's parameter never agrees with another struct's, so that field always takes an associated type. `(pair a)`'s `:count int` agrees with `cat`'s `:count int` and keeps the plain shape. In the last shape, a value read goes through `&self.item`, and a stored borrow reborrows as `self.item` at the struct's borrow.
5. **Names.** The trait is `Has` and the field's Rust name in Pascal case, the associated type is the field's Rust name in Pascal case, and the getters are the field's Rust name and that name with `_mut`, by refinements 5 and 9: `admin?` gives `HasIsAdmin`, `type IsAdmin` and `is_admin()`, and the changing trait of Rule 3.2 is `HasIsAdminMut` with `is_admin_mut()`. A name that a written item takes yields as an introduced item does: `HasName2` beside a program's own `has-name` struct, and `name_2()` beside a program trait that gives the same struct a method `name`.
6. **The trait emits at the crate root, and each impl beside its struct.** A struct in another module emits its impl in that module's `mod` as `impl<'a> crate::HasName<'a> for Animal<'a>`, and a module that calls `name()` gets a Rust `use` of the trait, as a user trait's method call does. Every impl names its struct's lifetime and parameters, as `impl<'a> HasName<'a> for User<'a>` and `impl<A> HasItem for Boxed<A>`, the spelling that [10. User-defined traits](../docs/language.md#10-user-defined-traits) uses for `impl<'a> Finder for Snippet<'a>`.
7. **Projections emit as every projection does today.** The emitter writes `<A as HasName>::Name` (`src/emit/types.rs:508`), so a field named like another bound's associated type, such as `:item` beside a trait with `(type item)`, is never ambiguous. A fixed read emits as the trait's binding, as `HasName<Name = str>` or `HasTag<'a, Tag = str>`. A stored borrow's lifetime is the trait's own parameter, never the parameter's borrow, so `(defn trimmed (x) (trim (:name x)))` emits `fn trimmed<'a, A: HasName<'a>>(x: &A) -> &'a str`, and its result outlives a temporary argument.
8. **A read whose type carries a field bound of its own emits its owner as a type parameter.** A field bound looks through borrows, as a known read strips its owner's borrows (`field_owner`, `src/types/infer.rs:3564`): `(b (has :x f))` holds for a struct `b` with `:x` and for any borrow of one. So `(defn start-x (l) (+ (:x (:start l)) 1.0))`, with two line structs whose `:start` differ and two point structs with `:x float`, gives `(a (has :start b))` and `(b (has :x float))`, and emits `fn start_x<A: HasStart<Start = B>, B: HasX>(l: &A) -> f64 { l.start().x() + 1.0 }`. A borrowed intermediate works as a by-value one does: `A: HasHome<Home = B>, B: HasLabel` reads `x.home().label()` through `&Self::Home`.
9. **A pinned function emits what it emits with the struct written in its signature.** With only `(hello (->user "ana"))` in `main`, `hello` emits `fn hello(x: User<'_>) -> String` and `x.name`, the Rust that `main` emits today when `user` is the only struct with `name`. A pinned read writes the known read, and an adjustment the checker recorded on a mixed-shape `(ref b)`, such as the dereference of a Copy value, folds into it, as `*&x.age` is `x.age`. A reader of a generic struct's parameter field is the exception: its bound reads `(ref a)`, so a reader pinned to a Copy instantiation keeps the borrow, and the single-struct program that is `L0401` on `main`, ``expected `(ref int)`, found `int` ``, keeps that error, a pinning fault outside this RFC.

### 3. Ownership

**A read has the type that its trait's shape gives, Rule 2.4.** In the plain shapes and the value shape that is the known read's type: the read of a Copy `int` field is `int`, the read of a `str` field is `(ref str)` with the struct's lifetime, and the read of a `string` field is `(ref string)`. In the stored-borrow shape it is a borrow with the struct's lifetime. In the mixed shape it is a borrow of the parameter, `(ref b)`, so the ownership pass treats a returned read as a borrow of the argument, as it does for a `string` field on `main`: `(let (n (name-of (->owner (str "a")))) (println n))` emits `let owner = make_owner("a".to_string()); let n = name_of(&owner);`. Every adjustment runs on that type, as it does for a known read.

The owner's accepted cost: in the mixed shape, a stored borrow's read cannot outlive its argument. `(defn pick () (local u (->user "ana")) (name-of u))` returns `&'static str` when `name-of` reads a known `user`, and is `L0510` when `name-of` is a mixed-shape reader, because the result borrows the local `u`. A reader is mixed-shape only when it is called with two structs, so the program fails on `main` with `L0407`.

| Use | Bound | Emitted read |
| --- | --- | --- |
| returned, `(defn name-of (x) (:name x))`, `:name str` on two structs | `(a (has :name (ref str)))` | `fn name_of<'a, A: HasName<'a>>(x: &A) -> &'a str { x.name() }`, and the result outlives a temporary argument |
| a Copy field in arithmetic, `(+ (:age x) 1)`, `:age int` on two structs | `(a (has :age int))` | `x.age() + 1` |
| a `mut` parameter that reads too, `(set! (:age x) (* (:age x) 2))` | `(a (has :age int))`, and `x` takes `mut` | `*x.age_mut() *= 2`, by the Assignment bullet of Ownership, as `main` emits `x.age *= 2` |
| owned where the read type is open, `(set! total (:level x))`, `:level` an `int` and a `u8` | `(a (has :level r))`, `(r copy)` | `total = x.level()` |
| a clone, `(clone (:title x))`, `:title string` on two structs | `(a (has :title (ref string)))` | `x.title().clone()` |
| a move, `(keep (:title x))` with `keep` taking an owned `string` | none | `L0503` at the read, as for a known struct today, with the help `(clone ...)` |

1. **A read never moves the field,** as on `main`, where only the base of a collection or struct update moves one. A use that needs to own a field that is not Copy is `L0503` today and stays `L0503`.
2. **A `set!` of a generic field, or a `mut` borrow of one, needs a second trait, `Has<Field>Mut`, whose supertrait is the field's trait:**

   ```rust
   trait HasNameMut: HasName {
       fn name_mut(&mut self) -> &mut Self::Name;
   }
   ```

   The trait emits only when a reached function changes the field, and a struct implements it only when a reached changing reader receives it, so a struct that only read-only readers receive never needs it. A Rust trait's methods are all or nothing, so one trait with both getters would make every struct of the mixed shape write `name_mut`, and a struct that stores `:name str` cannot give `&mut str` from `&'a str`, which rustc rejects as `E0046`. The changing getter returns `&mut` of the stored type, which the read gives in every shape where the method exists: `fn age_mut(&mut self) -> &mut i64` in the plain value shape, `&mut String` in the plain owned shape, `&mut Self::Age` and `&mut Self::Item` with an associated type. The bound prints no part of its own for a change: the checker reads the need from the body's `set!`, as it reads the parameter's `mut` mode, so a reader's text still pastes into a signature. `(defn birthday! (x) (set! (:age x) (+ (:age x) 1)))` emits `fn birthday<A: HasAgeMut>(x: &mut A) { *x.age_mut() += 1; }`.

   A struct that stores the field as a borrow never implements the changing trait, because a `set!` of that field on a known value is `L0401` on `main`, as `(set! (:name u) "bob")` on a `str` field is. A call that passes such a struct to a reader that changes the field is `L0467` at the call, with the known `set!`'s reason, and no other error reaches the user. Open detail 1 asks the owner to confirm the trait.
3. **A struct update or a collection update on a generic owner stays an error.** `(assoc p :x 2)` builds a new value of the owner's struct, and a field trait has no constructor. Rule 4 gives the error its new text.
4. **A generic owner is never Copy.** A parameter whose type only a field bound knows takes `ref` or `mut`, never copy, unless the body infers `copy` for it. That keeps decision 1's `hello` at `x: &A`.

### 4. Diagnostics with several candidates

A read on an unknown owner prints as the bound everywhere a type prints, as a `deftrait` with an associated type prints `(c (container item))` in [10. User-defined traits](../docs/language.md#10-user-defined-traits):

| Where | Text |
| --- | --- |
| `knave types` and a doc page's signature | `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))` |
| `(infer (:name x))` in `hello` | `"(ref str)"` in a plain shape. `"r"` or `"(ref b)"`, the read's variable, in a shape with an associated type, as `(infer x)` is `"a"` |
| `(infer hello)` | `"(where ((a (has :name (ref str)))) (fn (a) string))"`, with modes left out by the existing rule |
| a type error that names the owner | `a`, with a note that `a` is any struct with `:name`: `user` or `animal` |

The text pastes into a `type` signature, so a written signature can state the bound: `(type hello (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`. A bound may hold several fields, `(a (has :name (ref str)) (has :age int))`, and a read type may carry its own bound, as Rule 2.8 shows.

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

**Fixtures for the shapes.** Each fixture below adds one struct to the one above it and switches the trait's shape, while the reader's `(println (infer ...))` line, which the emitted Rust holds as a string literal, stays the same:

| Fixture | Structs | Trait | `infer` text of the reader |
| --- | --- | --- | --- |
| `emit/field_trait_plain_value.knv` | `cat`, `dog` with `:age int` | `trait HasAge { fn age(&self) -> i64; }` | `next-age`: `(where ((a (has :age int))) (fn (a) int))` |
| `emit/field_trait_value_types.knv` | the same, and `drone` with `:age u8` | `trait HasAge { type Age; ... }`, `HasAge<Age = i64>` | the same |
| `emit/field_trait_plain_borrow.knv` | `user`, `animal` with `:name str` | `trait HasName<'u> { fn name(&self) -> &'u str; }` | `uppercase-name`: `(where ((a (has :name (ref str)))) (fn (a) string))` |
| `emit/field_trait_stored_borrows.knv` | the same, and `poll` with `:name (slice int)` | `trait HasName<'u> { type Name: ?Sized; ... }`, `HasName<'a, Name = str>` | the same |
| `emit/field_trait_mixed.knv` | the same, and `product` with `:name i32` | `trait HasName { type Name: ?Sized; ... }`, `HasName<Name = str>` | the same |
| `emit/field_trait_plain_owned.knv` | `book`, `film` with `:title string`, and `(defn shout (x) (loud (:title x)))` with `(type loud (fn ((ref string)) string))` | `trait HasTitle { fn title(&self) -> &String; }` | `shout`: `(where ((a (has :title (ref string)))) (fn (a) string))` |
| `emit/field_trait_generic_struct.knv` | the same, and `(boxed a)` with `:title a` | `trait HasTitle { type Title: ?Sized; ... }`, `impl<A> HasTitle for Boxed<A>` | the same |

Run fixtures cover the programs the reviews raised: a name read from a temporary in the plain and stored-borrow shapes, `trimmed`, `double-age!`, `start-x` through a by-value point, a chained read through a Copy struct that holds a borrow, a two-field reader, `(boxed a)` read at `string` and `int` by one generic `get`, and a field `:item` beside a trait with `(type item)`. A check fixture holds `pick`'s `L0510` in the mixed shape.

### 6. Interplay

1. **Structs across modules.** A candidate is any struct of the program, as on `main`. The checker has no field visibility rule: a known read of a private struct's field from another module compiles on `main`, and a generic read checks no visibility either. So a library can hold a generic reader for its callers' structs, and the trait and impls of Rule 2.6 span the modules.
2. **Generic structs.** A generic struct is a candidate like any other. Its parameter's field reads as a borrow, by Rule 2.3, so `(boxed int)` meets `(has :name (ref int))` and `(boxed string)` meets `(has :name (ref string))`. One `impl<A> HasName for Boxed<A>` with `type Name = A` serves both, and a generic caller that passes `(boxed t)` needs no second impl. `(defn get (b) (:name b))` called on a `(boxed string)` and a `(boxed int)` compiles as on `main`. The `L0401` that `main` reports for the single-struct `boxed` program, ``expected `(ref int)`, found `int` ``, is a pinning fault outside this RFC.
3. **An enum's case.** An enum is never a candidate. A case's fields exist only inside that case, so a getter cannot exist for the enum. A program reads a case's field through `match`, as on `main`, and a generic reader called with an enum is `L0410`.
4. **A written type annotation.** A written signature is exact. One that names a struct emits today's Rust. One that names a type variable without the field bound is `L0440`, with a help that writes the bound, as `(a (has :name (ref str)))`, as a missing `copy` gets `(a copy)` today. A binder annotation, as `(x (ref user))`, fixes the owner.
5. **A closure.** A `local` closure stays monomorphic, as on `main`, so a field bound on its own parameter must settle by the end of checking. `(let (f (fn (y) (:name y))) (f (->user "ana")) (f (->animal "rex")))` is `L0401`, ``expected `user`, found `animal` ``, at the second argument, as `(let (f (fn (y) y)) (f 1) (f "a"))` is today. A closure that no call fixes is `L0407` at the read, `` cannot tell which struct's `name` this is ``, with the note of Rule 4 that lists the candidates. A read inside a closure whose owner is the enclosing function's parameter gives the function the bound, so it generalizes with the function.
6. **A field the prelude reads.** The prelude declares no struct and no function that reads a field. Its macros, `defstruct`'s constructors, `assoc`, `assoc-in`, `update` and the keyword closure, expand to `field` and `new` in the user's module, so their reads follow these rules there. A keyword closure, as `(map (iter xs) :name)` in an unannotated function, gives the function the bound through its parameter's element type. `update` reads and then builds, so a generic owner is the update error of Rule 3.3.

### 7. Prelude first

The prelude cannot express this, and the build adds no kernel form:

- **The choice is the checker's.** Whether `(:name x)` emits `x.name` or `x.name()` depends on the type of `x` after inference and pinning. A macro runs before the checker and sees only the form.
- **The struct set is the program's.** The candidates span modules, a trait's shape depends on every struct that declares the field, and each impl depends on the calls that reach it. Only the compiler sees every struct, every call and every pin.
- **A trait per field from a macro clashes,** as Option B says, and [RFC 0038](0038-field-keywords.md) moved keyword reads into the expander for the same reason.

The change lives in the type rule of the existing kernel form `field`, and in one kernel name of a new kind: the bound `has`, which a `where` writes beside traits. A kernel name is needed because the prelude cannot define a family of bounds keyed by a field name, and the printed bound must paste into a signature. `has` is exported by the built-in `kernel` module and re-exported by the prelude like every kernel name. The build adds it as a row of the table in [Types](../docs/language.md#types), `(where ((a (has :age int))) (fn ((ref a)) int))` beside `fn<A: HasAge>(&A) -> i64`, and records why it is a kernel name in [Kernel forms](../docs/language.md#kernel-forms), beside the reasons of the other kernel names, and adds it to that section's list of the kernel names that type positions hold (`docs/language.md:85`), beside `fn`, `ref`, `where` and `with`. `prelude/prelude.knv` documents `has` with a `document` form, as it documents `struct`. The Rust names `HasName`, `Name` and `name()` are no fact the prelude states, so the compiler derives them from the field, as it derives `make_point` from `->point`.

## Open details

1. **The changing getter.** Rule 3.2 puts `<field>_mut` in a second trait, `Has<Field>Mut: Has<Field>`, implemented only by the structs that a changing reader receives, so a struct that stores the field as a borrow never has to write it. The alternative is an error at a `set!` of a generic field, with help to write a signature. The recommendation is the second trait, because Rust programmers write it, `L0467` at the call stays the only error, and the uniform `set!` rule then holds for generic owners.
2. **The bound's spelling.** This RFC writes `(has :name r)`. A keyword names the field as a read and a `defstruct` do. `field` would read as the kernel form, and a name ending in `?` reads as a predicate. The owner may prefer another name.
3. **An owned text or vector field read through its view.** On `main`, a known read of a `string` field is `(ref string)`, and it meets `(ref str)` by the `str` adjustment. A generic read whose body fixes `(ref str)`, as `uppercase-name` does, rejects a struct whose `:name` is `string` under the rules above, with `L0467`, while the same read of the known struct passes. That is worse output and an exception to the adjustment rule, and it grows under the one text type of issue #589, where the user writes only `str` and the compiler picks `&str` or `String` per use: two structs that both write `:name str` could then disagree in a field bound. The recommendation is the view rule. In a field bound, a field that the struct stores as `String` reads as `(ref str)`, and one stored as `Vec<T>` reads as `(ref (slice t))`, by the existing shrinking adjustment, so the impl writes `fn name(&self) -> &str { &self.name }`, which Rust's deref coercion accepts, and needs no `AsRef<str>`. The prototype ran `uppercase_name` on such a struct. After issue #589 every text field reads as `(ref str)` in a bound, whatever Rust type it emits as, so the rule needs no change then. If the owner takes the view rule, the build changes:
   - the "borrow of the struct" row of Rule 2.3, whose `&String` becomes `&str` and whose `&Vec<T>` becomes `&[T]`,
   - the plain owned row of Rule 2.4, `fn title(&self) -> &str`, and its fixture `emit/field_trait_plain_owned.knv`,
   - the first paragraph of Rule 3, where a `string` field's read in a bound is `(ref str)`,
   - Rule 3.2, where the read no longer gives the stored type of a `string` or `vec` field: `title_mut` would return `&mut str`, which no `set!` can use, so the changing trait takes the stored type from the field's declaration, `&mut String`,
   - the `clone` row of Rule 3, where `(clone (:title x))` clones a `(ref str)`, which is `L0404` as for a `str` field today,
   - refinement 6's "a borrow of the field otherwise", which then reads "a borrow of the field's view".

   The view rule also makes one new rejection: a body that fixes `(ref string)`, such as `(as-str (:name x))` with `as-str` taking `(ref string)` (`prelude/text.knv:258`), rejects a struct whose `:name` is `string`, with `L0467`, while the same read of a known struct passes. `as-str` goes away with issue #589, so this lasts until then.

## Prototype

`target/scratch/proto3/` and `target/scratch/proto4/` held the prototype, which is not part of this pull request. rustc 2024 compiled and ran the Rust of Option A, Rule 2 and Rule 3:

- `proto4/d1.rs`, decision 1 in the plain stored-borrow shape, printed `hello ana` and `hello rex`, then a name and a trimmed name read from temporaries.
- `proto4/d2.rs`, decision 2 in the mixed shape, printed `ANA`, `REX`, `name: ana` and `name: 7`. A name read from a temporary there is `E0716` in Rust, which the ownership pass's binding of the temporary prevents, as Rule 3 says.
- `proto4/values.rs`, `values2.rs` and `stored2.rs` held the fixture pairs of Rule 5.
- `proto5/mixedmut.rs` held the changing trait of Rule 3.2 in the mixed and plain value shapes, with `user` implementing only `HasName`, and the plain owned shape with `&String`, read by `shout` through `loud`.
- `proto3/shapes.rs` held every shape with the probes of the first two reviews, rebuilt from their text, with the plain owned shape written in the view rule's `&str`: `double-age!` and `double-level!` with a `mut` parameter that also reads, a name and a trimmed name from temporaries, a two-field reader, a stored borrow of a local, a generic `get` over `Boxed<i64>`, `Boxed<String>` and a generic caller, the mixed shape, `start-x` through a by-value point, a chained read through a Copy struct that holds a borrow, a chained read through a borrow, and `:item` beside a trait with `type Item`.

Calling `uppercase_name(&Product { .. })` gave rustc's `E0271`, which Rule 1 replaces with `L0467`. The instrumented `main` build of Rule 5 changed only two `eprintln!` lines in `src/types/infer.rs`.
