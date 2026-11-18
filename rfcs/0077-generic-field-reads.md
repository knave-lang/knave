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

The checker gives an unknown owner a field bound, `(a (has :name b))`: `a` is a struct with a field `name`, and `b` is the type a read of that field gives. The bound behaves as a trait bound does. It generalizes, it is checked at each call, and pinning removes it when every call passes one struct. The emitter writes one Rust trait per field name that a reached generic read needs, and one impl per struct that a reached call passes. Rule 2 gives the trait's shape. Its associated type takes the field's name, as `type Name` in `HasName`.

Decision 1, after the build:

```rust
trait HasName {
    type Name<'s>
    where
        Self: 's;
    fn get_name(&self) -> Self::Name<'_>;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct User<'a> {
    name: &'a str,
}

impl<'a> HasName for User<'a> {
    type Name<'s>
        = &'a str
    where
        Self: 's;
    fn get_name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Animal<'a> {
    name: &'a str,
}

impl<'a> HasName for Animal<'a> {
    type Name<'s>
        = &'a str
    where
        Self: 's;
    fn get_name(&self) -> &'a str {
        self.name
    }
}

fn make_user(name: &str) -> User<'_> {
    User { name }
}

fn make_animal(name: &str) -> Animal<'_> {
    Animal { name }
}

fn hello<'a, A: HasName>(x: &'a A) -> String
where
    A::Name<'a>: std::fmt::Display,
{
    format!("hello {}", x.get_name())
}

fn main() {
    crate::rt::restore_default_sigpipe();
    println!("{}", hello(&make_user("ana")));
    println!("{}", hello(&make_animal("rex")))
}
```

`knave types` prints `(type hello (where ((a (has :name b)) (b display)) (fn ((ref a)) string)))`.

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

After the build, beside the trait and the impls above, with `Product`'s impl:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Product {
    name: i32,
}

impl HasName for Product {
    type Name<'s> = i32;
    fn get_name(&self) -> i32 {
        self.name
    }
}

fn uppercase_name<'a, A: HasName<Name<'a> = &'a str>>(x: &'a A) -> String {
    x.get_name().to_uppercase()
}

fn describe<'a, A: HasName>(x: &'a A) -> String
where
    A::Name<'a>: std::fmt::Display,
{
    format!("name: {}", x.get_name())
}
```

`upper-case` takes `(ref str)`, so the body fixes the read's type: `(type uppercase-name (where ((a (has :name (ref str)))) (fn ((ref a)) string)))`. `describe` uses the read only through `display`, so it keeps the read's type open: `(where ((a (has :name b)) (b display)) (fn ((ref a)) string))`. Each call takes its own struct's read type, so `describe` reads a `(ref str)` from `user` and an `i32` from `product`.

### B. `defstruct` writes a trait and an impl per field

The prelude macro would write `(deftrait has-name (type field-type) (get-name ...))` and an `impl` for each field of each struct, and a keyword read would expand to `get-name`. Two structs that share a field would both define `has-name`, which is the clash that made keyword reads an expander feature in [RFC 0038](0038-field-keywords.md). Every field of every struct would emit a trait and an impl, so every program's Rust would change. A macro runs before the checker, so it cannot tell a read whose struct is known, which should stay `x.name`, from one whose struct is not.

### C. One copy of the function per struct

The emitter would write `hello_user` and `hello_animal`. The Rust would hold one function per struct for every generic reader, calls would name a struct the source never wrote, and a function value such as `(map users hello)` would need the copy picked by its element type. Rust's own answer to "any type with this capability" is a trait.

### D. A written accessor parameter

The program would pass the reader as a function, `(hello :name x)`. This is possible today and changes the signature of every reader. Decision 1 rules it out: the unannotated `hello` must compile.

## Recommendation

Option A. It is the owner's model, Rust programmers write it by hand, and it adds a bound kind to the checker in place of a kernel form. Every program whose reads have zero or one candidate struct, or whose calls pin each reader to one struct, keeps its Rust byte for byte.

## Decision

The project owner decided on 2026-11-08:

1. `(defn hello (x) (str "hello " (:name x)))` with no annotation compiles and accepts every struct with a `name` field, such as `(defstruct user :name str)` and `(defstruct animal :name str)`.
2. Fields of one name may have different types, such as `(defstruct user :name str)` and `(defstruct product :name i32)`, so `(defn uppercase-name (x) (uppercase (:name x)))` is generic over the field's type.
3. The model: one Rust trait per field name, such as `HasName` with an associated `FieldType` and `fn get_name(&self) -> &Self::FieldType`, and `impl HasName for Animal { type FieldType = String; ... }` for each struct.
4. Before 1.0 the owner is the only user, so the change needs no migration aid.

Decision 2 spells the prelude function `uppercase`, which is `upper-case` in the prelude (`prelude/text.knv:789`). This RFC reads the decision with that name.

The project owner refined decision 3 on 2026-11-18:

5. Each field trait's associated type takes the field's name: `HasName` declares `type Name`, so a bound reads `A::Name: Display`. A shared name such as `FieldType` would name two items on an owner with two field bounds, which rustc rejects as `E0221`, "ambiguous associated type", and the emitter would have to write `<A as HasName>::FieldType` there.

Decision 3's getter, `fn get_name(&self) -> &Self::FieldType`, holds for a field that a read borrows, such as a `string` field: `type Name<'s> = &'s String` returns the same `&String`. It cannot hold for the two other kinds of field, so Rule 2.3 makes the associated type the type of a read and the getter return it by value:

- **A field the struct stores as a borrow.** A `&Self::Name` result borrows the struct, not the stored borrow. On `main`, `(defn name-of (x) (:name x))` with `(let (n (name-of (->user "ana" 3))) (println n))` prints `ana`. With `&Self::Name`, a second struct with `:name str` makes the same program rustc's `E0716`, "temporary value dropped while borrowed", because `n` borrows the temporary `user`.
- **A Copy field.** On `main`, `(infer (:age u))` is `"int"`. A getter that returns `&i64` makes the generic read `(ref int)` and the known read `int`, so pinning cannot give the known read's Rust.

## Rejected options and why

- **B: a trait per field from `defstruct`:** two structs that share a field clash, every program's Rust changes, and a macro cannot see whether a read's struct is known.
- **C: one copy per struct:** the Rust grows by one function per struct and reader, calls name a struct the source never wrote, and function values need a choice the source never states.
- **D: a written accessor:** decision 1 requires the unannotated function to compile.
- **A field bound on every unknown owner, one candidate included:** a reader of the only struct with `:name` would print a `where` and emit a trait for one implementor, where a concrete type says the same thing and reads better. Rule 5 keeps one candidate concrete.
- **A getter that returns `&Self::Name` for every field:** a generic read of a stored borrow would end at the struct's borrow, which turns a working program into `E0716`, and a Copy field would read as a borrow, unlike the known read.
- **One associated type name, `FieldType`, for every field trait:** an owner with two field bounds makes `A::FieldType` ambiguous, `E0221`, so the emitter would write `<A as HasName>::FieldType` there and the plain path elsewhere. The owner chose the field's name on 2026-11-18.
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

The message never names `HasName`, `Name` or `get_name`, so rustc's `E0271`, which the prototype hits when it calls `uppercase_name(&Product { .. })`, never reaches the user.

A call with a value that has no such field reports the existing codes at the call, with a secondary label on the read and the same note:

| Argument | Code | Title |
| --- | --- | --- |
| a struct without the field, `(hello (->point 1.0 2.0))` | `L0408` | `` `point` has no field `:name` `` |
| an enum, `(hello shape/point)` | `L0410` | `` `shape` has no fields `` |
| any other type, `(hello 5)` | `L0410` | `` `int` has no fields `` |

### 2. Readable output

1. **A trait emits only for a field that an emitted function reads from a generic owner.** A read whose owner is a known struct, from the start or after pinning, emits `x.name` as it does today. A program with no generic read after pinning emits no trait and no impl, so its Rust stays byte for byte what `main` emits.
2. **An impl emits only for a struct that a reached call passes for the bound,** directly or through another generic function's bound, as the structural trait impls of [8.2 Struct traits](../docs/language.md#82-struct-traits) do. A struct no call passes gets no impl. A trait declares only the methods a reached use needs.
3. **The trait is `Has` and the field's Rust name in Pascal case, with a generic associated type for the read and a getter that returns it by value:**

   ```rust
   trait HasName {
       type Name<'s>
       where
           Self: 's;
       fn get_name(&self) -> Self::Name<'_>;
   }
   ```

   Each impl's read type is the type a known read of the field gives on `main`, by the Fields bullet of [Ownership](../docs/language.md#ownership):

   | Field | Read on `main` | `type Name<'s>` | Getter body |
   | --- | --- | --- | --- |
   | stored as a borrow: `str`, `(slice a)`, `(ref t)` | the stored borrow | `&'a str`, `&'a [A]`, `&'a T`, with the struct's lifetime | `self.name` |
   | a function type, or a Copy type of 16 bytes or less | the value | `i32`, `fn(&str) -> bool` | `self.name` |
   | any other type | a borrow of the field | `&'s String`, `&'s Vec<i64>` | `&self.name` |

   `admin?` gives `HasIsAdmin`, `type IsAdmin<'s>` and `get_is_admin`. A trait name that a written item takes yields as an introduced item does: `HasName2` beside a program's own `has-name` struct.
4. **The trait emits at the crate root, and each impl beside its struct.** A struct in another module emits its impl in that module's `mod` as `impl<'a> crate::HasName for Animal<'a>`, and a module that calls `get_name` gets a Rust `use` of the trait, as a user trait's method call does.
5. **Every impl names its struct's lifetime and parameters,** as `impl<'a> HasName for User<'a>` and `impl<A> HasName for Boxed<A>`, the spelling that [10. User-defined traits](../docs/language.md#10-user-defined-traits) uses for `impl<'a> Finder for Snippet<'a>`. The read type of a stored borrow needs the name `'a`.
6. **A read type that only the bound names emits as the trait's projection at the parameter's lifetime.** The parameter's borrow takes the shared lifetime `'a`, by the existing naming rule, and its traits go to a `where` clause. A fixed read type emits as the trait's binding:

   | Reader | Rust |
   | --- | --- |
   | one field, `hello` | `A::Name<'a>: Display` |
   | two fields, `(str (:name x) (:age x))` | `A::Name<'a>: Display` and `A::Age<'a>: Display` |
   | a fixed read, `uppercase-name` | `HasName<Name<'a> = &'a str>` |

   A projection is never ambiguous, because two fields of one struct never share a name, so the emitter never writes a qualified path.
7. **A read type that carries a field bound of its own emits as a type parameter of its own.** `(:name (:owner x))`, with two structs that declare `:owner` and two that declare `:name`, gives `(a (has :owner (ref b)))` and `(b (has :name c))`, and emits `fn owner_name<'a, A: HasOwner<Owner<'a> = &'a B>, B: HasName + 'a>(x: &'a A)`. The inner read's owner is the referent `B`, as a known read strips its owner's borrows.
8. **A pinned function emits what it emits with the struct written in its signature.** With only `(hello (->user "ana"))` in `main`, `hello` emits `fn hello(x: User<'_>) -> String` and `x.name`, the Rust that `main` emits today when `user` is the only struct with `name`. Rule 3 makes this hold.

### 3. Ownership

**A read has the type that a known read of the struct's field gives, the read type of Rule 2.3.** A read whose owner is unknown has a fresh variable `b` for that type. When the owner becomes known, in the body, at pinning or at a call, `b` takes the field's read type, so the read of a Copy `int` field is `int`, the read of a `str` field is `(ref str)` with the struct's lifetime, and the read of a `string` field is `(ref string)`. The read never takes a borrow that a later fact turns into a copy, and every adjustment runs on the settled type, as it does for a known read. A pinned read therefore has the type and the Rust of the known read, which is what Rule 2.8 needs.

The same rule covers a known struct whose field type holds a type parameter of the struct. On `main`, `(struct (boxed a) (name a))` with `(defn hello (x) (:name x))` called on `(new boxed (name 3))` is `L0401`, ``expected `(ref int)`, found `int` ``, at the read, because the read took a borrow before pinning made `a` a Copy `int`. After the build, the read of `name` has a read-type variable until `a` is known, and the program compiles to `fn hello(x: Boxed<i64>) -> i64`, which copies the Copy `boxed` as a known read does.

| Use | Bound | Emitted read |
| --- | --- | --- |
| returned, `(defn name-of (x) (:name x))` | `(a (has :name b))` | `fn name_of<'a, A: HasName>(x: &'a A) -> A::Name<'a> { x.get_name() }`, and `user`'s `&'a str` outlives the temporary `user` |
| a Copy field in arithmetic, `(+ (:age x) 1)` | `(a (has :age int))` | `x.get_age() + 1` |
| owned where the read type is open, `(set! total (:age x))` | `(b copy)`, inferred as for a generic parameter | `total = x.get_age()` |
| a clone, `(clone (:title x))` with `:title string` | `(a (has :title (ref string)))` | `x.get_title().clone()` |
| a move, `(keep (:title x))` with `keep` taking an owned `string` | none | `L0503` at the read, as for a known struct today, with the help `(clone ...)` |
| a change, `(set! (:age x) 5)` | `(a (has :age int))`, and `x` takes `mut` | `*x.get_age_mut() = 5`, by open detail 1 |

1. **A read never moves the field,** as on `main`, where only the base of a collection or struct update moves one. A use that needs to own a field that is not Copy is `L0503` today and stays `L0503`.
2. **A `set!` of a generic field, or a `mut` borrow of one, adds a stored type and a `mut` getter to the trait** when a reached function needs them: `type StoredAge; fn get_age_mut(&mut self) -> &mut Self::StoredAge;`, where `StoredAge` is the field's Rust type as the struct stores it, `i64` or `&'a str`. The parameter takes `mut` by the existing Modes rule, so `(defn birthday! (x) (set! (:age x) (+ (:age x) 1)))` emits `fn birthday<A: HasAge<StoredAge = i64>>(x: &mut A) { *x.get_age_mut() += 1; }`. Every struct has a stored type, a view field included, so every impl can write the method. A call whose struct the same `set!` on a known value would reject is `L0467` at the call, with the known `set!`'s reason: on `main`, `(set! (:name u) "bob")` on a `str` field is `L0401`, so a generic `set!` of a `str` field with that value rejects `user` at the call. Open detail 1 asks the owner to confirm the method.
3. **A struct update or a collection update on a generic owner stays an error.** `(assoc p :x 2)` builds a new value of the owner's struct, and a field trait has no constructor. Rule 4 gives the error its new text.
4. **A generic owner is never Copy.** A parameter whose type only a field bound knows takes `ref` or `mut`, never copy, unless the body infers `copy` for it. That keeps decision 1's `hello` at `x: &'a A`.

### 4. Diagnostics with several candidates

A read on an unknown owner prints as the bound everywhere a type prints, as a `deftrait` with an associated type prints `(c (container item))` in [10. User-defined traits](../docs/language.md#10-user-defined-traits):

| Where | Text |
| --- | --- |
| `knave types` and a doc page's signature | `(type hello (where ((a (has :name b)) (b display)) (fn ((ref a)) string)))` |
| `(infer (:name x))` in `hello` | `"b"`, the read type's variable, as `(infer x)` is `"a"`. In a function pinned to `user` it is `"(ref str)"`, as on `main` |
| `(infer hello)` | `"(where ((a (has :name b)) (b display)) (fn (a) string))"`, with modes left out by the existing rule |
| a type error that names the owner | `a`, with a note that `a` is any struct with `:name`: `user` or `animal` |

The text pastes into a `type` signature, so a written signature can state the bound: `(type hello (where ((a (has :name b)) (b display)) (fn ((ref a)) string)))`. A bound may hold several fields, `(a (has :name b) (has :age int))`, and a read type may carry its own bound, as Rule 2.7 shows.

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

### 6. Interplay

1. **Structs across modules.** A candidate is any struct of the program, as on `main`. The checker has no field visibility rule: a known read of a private struct's field from another module compiles on `main`, and a generic read checks no visibility either. So a library can hold a generic reader for its callers' structs, and the trait and impls of Rule 2.4 span the modules.
2. **Generic structs.** A generic struct is a candidate like any other, and its read type takes the struct's arguments at the call: `(boxed int)` meets `(has :name int)`. Its impl carries the struct's parameters, as Rule 2.5 says.
3. **An enum's case.** An enum is never a candidate. A case's fields exist only inside that case, so a getter cannot exist for the enum. A program reads a case's field through `match`, as on `main`, and a generic reader called with an enum is `L0410`.
4. **A written type annotation.** A written signature is exact. One that names a struct emits today's Rust. One that names a type variable without the field bound is `L0440`, with a help that writes the bound, as `(a (has :name b))`, as a missing `copy` gets `(a copy)` today. A binder annotation, as `(x (ref user))`, fixes the owner.
5. **A closure.** A `local` closure stays monomorphic, as on `main`, so a field bound on its own parameter must settle by the end of checking. `(let (f (fn (y) (:name y))) (f (->user "ana")) (f (->animal "rex")))` is `L0401`, ``expected `user`, found `animal` ``, at the second argument, as `(let (f (fn (y) y)) (f 1) (f "a"))` is today. A closure that no call fixes is `L0407` at the read, `` cannot tell which struct's `name` this is ``, with the note of Rule 4 that lists the candidates. A read inside a closure whose owner is the enclosing function's parameter gives the function the bound, so it generalizes with the function.
6. **A field the prelude reads.** The prelude declares no struct and no function that reads a field. Its macros, `defstruct`'s constructors, `assoc`, `assoc-in`, `update` and the keyword closure, expand to `field` and `new` in the user's module, so their reads follow these rules there. A keyword closure, as `(map (iter xs) :name)` in an unannotated function, gives the function the bound through its parameter's element type. `update` reads and then builds, so a generic owner is the update error of Rule 3.3.

### 7. Prelude first

The prelude cannot express this, and the build adds no kernel form:

- **The choice is the checker's.** Whether `(:name x)` emits `x.name` or `x.get_name()` depends on the type of `x` after inference and pinning. A macro runs before the checker and sees only the form.
- **The struct set is the program's.** The candidates span modules, and each impl depends on the calls that reach it. Only the compiler sees every struct, every call and every pin.
- **A trait per field from a macro clashes,** as Option B says, and [RFC 0038](0038-field-keywords.md) moved keyword reads into the expander for the same reason.

The change lives in the type rule of the existing kernel form `field`, and in one kernel name of a new kind: the bound `has`, which a `where` writes beside traits. A kernel name is needed because the prelude cannot define a family of bounds keyed by a field name, and the printed bound must paste into a signature. `has` is exported by the built-in `kernel` module and re-exported by the prelude like every kernel name. The build adds it as a row of the table in [Types](../docs/language.md#types), `(where ((a (has :name b))) (fn ((ref a)) b))` beside `fn<A: HasName>(&A) -> A::Name<'_>`, and records why it is a kernel name in [Kernel forms](../docs/language.md#kernel-forms), beside the reasons of the other kernel names. `prelude/prelude.knv` documents `has` with a `document` form, as it documents `struct`. The Rust names `HasName`, `Name` and `get_name` are no fact the prelude states, so the compiler derives them from the field, as it derives `make_point` from `->point`.

## Open details

1. **The `mut` getter.** Rule 3.2 adds a stored type, as `StoredAge`, and `get_<field>_mut` to the trait when a `set!` or a `mut` borrow needs them. The alternative is an error at such a `set!`, with help to write a signature. The recommendation is the method, because Rust programmers write it and the uniform `set!` rule then holds for generic owners.
2. **The bound's spelling.** This RFC writes `(has :name b)`. A keyword names the field as a read and a `defstruct` do. `field` would read as the kernel form, and a name ending in `?` reads as a predicate. The owner may prefer another name.
3. **An owned text or vector field read through its view.** On `main`, a known read of a `string` field is `(ref string)`, and it meets `(ref str)` by the `str` adjustment. A generic read whose body fixes `(ref str)`, as `uppercase-name` does, would reject a struct whose `:name` is `string` under an exact rule, while the same read of the known struct passes. That is worse output and an exception to the adjustment rule, and it would grow under the one text type of issue #589, where the user writes only `str` and the compiler picks `&str` or `String` per use: two structs that both write `:name str` could then disagree in a field bound. The recommendation is the view rule. In a field bound, a field that the struct stores as `String` reads as `(ref str)`, and one stored as `Vec<T>` reads as `(ref (slice t))`, by the existing shrinking adjustment, so the impl writes `type Name<'s> = &'s str` and `&self.name`, which Rust's deref coercion accepts, and needs no `AsRef<str>`. The prototype ran `uppercase_name` on such a struct. A known read keeps its type, so no program on `main` changes. After issue #589 every text field reads as `(ref str)` in a bound, whatever Rust type it emits as, so the rule needs no change then. The cost is that `(clone (:name x))` in a generic reader clones a `(ref str)`, which is `L0404` as for a `str` field today.

## Prototype

`target/scratch/proto/` and `target/scratch/proto2/` held the prototype, which is not part of this pull request. rustc 2024 compiled and ran the Rust of Option A, Rule 2 and Rule 3: `d1.rs` printed `hello ana` and `hello rex`, and `d2.rs` printed `ANA`, `REX`, `name: ana` and `name: 7`, then ran the `name-of` program of the Decision section, which printed `ana` where a `&Self::Name` getter gives `E0716`, and `uppercase_name` on a struct whose `name` is a `String`, which printed `BO`. Further files checked two field bounds on one owner, which a shared `FieldType` name makes `E0221`, a chained read, a generic struct's impl, a Copy read and the `mut` getter. Calling `uppercase_name(&Product { .. })` gave rustc's `E0271`, which Rule 1 replaces with `L0467`. The instrumented `main` build of Rule 5 changed only two `eprintln!` lines in `src/types/infer.rs`.
