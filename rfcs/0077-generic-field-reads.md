# RFC 0077: Field reads generic over every struct with the field

**Status:** Proposed
**Date:** 2026-11-18

Code references are against `origin/main` at `0c579cb4a`. Each knave output marked "on `main`" comes from the release binary of that commit. Each emitted Rust block marked "after the build" is the shape this RFC asks for, and the prototype compiled and ran it with rustc.

## Problem

A field read names its struct only through the value's type ([docs/language.md: Field keywords](../docs/language.md#field-keywords)). When the value's type is still unknown at the read, the checker looks the field up by name across the program's structs (`unique_owner`, `src/types/infer.rs:3585`). One struct with the field fixes the type. Any other count is `L0407`:

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
 --> one.knv:3:32
  |
3 | (defn hello (x) (str "hello " (:name x)))
  |                                ^^^^^ cannot tell which struct's `name` this is
  |
  = help: name the struct in a signature, as in `(fn ((ref point)) _)`
```

The help is no fix here: a signature that names `user` rejects `animal`. No written type says "any struct with a `name`", so the program needs two functions with one body.

The same lookup gives three smaller faults:

- **No struct with the field is `L0407` too.** `(defn size-of (x) (:size x))` with no `size` field anywhere reads "cannot tell which struct's `size` this is", where the real fault is that no struct declares `:size`.
- **The count ignores visibility.** A struct that another module neither exports nor lets the reader import still counts. A module that declares `user` beside a `(use pets2 greet)` whose `pets2` declares a private `animal :name str` reports `L0407` on `(:name x)`.
- **The read decides at its position.** The lookup runs when the checker reaches the read, so a later form in the same body that fixes the value's type comes too late.

## Options

### A. A field bound, emitted as one Rust trait per field name (recommended, and the owner's model)

The checker gives an unknown owner a field bound, `(a (has :name b))`: `a` is a struct with a field `name`, and `b` is that field's type. The bound behaves as a trait bound does. It generalizes, it is checked at each call, and pinning removes it when every call passes one struct. The emitter writes one Rust trait per field name that a reached generic read needs, and one impl per struct that a reached call passes.

Decision 1, after the build:

```rust
trait HasName {
    type FieldType: ?Sized;
    fn get_name(&self) -> &Self::FieldType;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct User<'a> {
    name: &'a str,
}

impl<'a> HasName for User<'a> {
    type FieldType = str;
    fn get_name(&self) -> &str {
        self.name
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Animal<'a> {
    name: &'a str,
}

impl<'a> HasName for Animal<'a> {
    type FieldType = str;
    fn get_name(&self) -> &str {
        self.name
    }
}

fn make_user(name: &str) -> User<'_> {
    User { name }
}

fn make_animal(name: &str) -> Animal<'_> {
    Animal { name }
}

fn hello<A: HasName>(x: &A) -> String
where
    A::FieldType: std::fmt::Display,
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
impl HasName for Product {
    type FieldType = i32;
    fn get_name(&self) -> &i32 {
        &self.name
    }
}

fn uppercase_name<A: HasName<FieldType = str>>(x: &A) -> String {
    x.get_name().to_uppercase()
}

fn describe<A: HasName>(x: &A) -> String
where
    A::FieldType: std::fmt::Display,
{
    format!("name: {}", x.get_name())
}
```

`upper-case` takes `(ref str)`, so the body fixes the field type: `(type uppercase-name (where ((a (has :name str))) (fn ((ref a)) string)))`. `describe` uses the field only through `display`, so it keeps the field type open: `(where ((a (has :name b)) (b display)) (fn ((ref a)) string))`. Each call takes its own struct's field type, so `describe` reads a `str` from `user` and an `i32` from `product`.

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

Decision 2 spells the prelude function `uppercase`, which is `upper-case` in the prelude (`prelude/text.knv:789`). This RFC reads the decision with that name. Decision 3's `type FieldType = String` is the impl for a `string` field. A `str` field gives `type FieldType = str`, which needs the trait's `?Sized`, as Rule 2 says.

## Rejected options and why

- **B: a trait per field from `defstruct`:** two structs that share a field clash, every program's Rust changes, and a macro cannot see whether a read's struct is known.
- **C: one copy per struct:** the Rust grows by one function per struct and reader, calls name a struct the source never wrote, and function values need a choice the source never states.
- **D: a written accessor:** decision 1 requires the unannotated function to compile.
- **A field bound on every unknown owner, one candidate included:** a library function that reads the only struct with `:name` would print a `where` and emit a trait for one implementor, where a concrete type says the same thing and reads better. Rule 5 keeps one candidate concrete.
- **A separate Rust type parameter for the field type, as `fn hello<A: HasName<FieldType = B>, B: Display + ?Sized>`:** it is valid Rust, but `A::FieldType` is the form a Rust programmer writes, and it keeps one generic parameter per owner.

## Rule

The build states these rules in [docs/language.md: Field keywords](../docs/language.md#field-keywords), [Types](../docs/language.md#types), [Ownership](../docs/language.md#ownership), [Names in Rust](../docs/language.md#names-in-rust) and [10. User-defined traits](../docs/language.md#10-user-defined-traits), in the explanation of `L0407` (`expand/src/diagnostic/explanations/L0407.md`), and in a new explanation of `L0466`. It replaces the two sentences of Field keywords that say a function needs a signature when two structs share a field name.

### 1. A call whose field type fails the body's use

The bound's field type is a type like any other. A body that fixes it, as `upper-case` fixes it to `str`, gives the bound that type. A body that uses it through a trait, as `str` uses `display`, gives the field type that trait bound. A call checks the bound against the struct it passes, and a struct whose field fails either is `L0466` at that call, a new code of the type stage. On `main`, `L0465` is the last code of `L04xx`.

```lisp
(defn main ()
  (println (uppercase-name (->product 7))))
```

After the build:

```text
error[L0466]: `uppercase-name` needs a `:name` of type `str`, but `product`'s `:name` is `i32`
 --> two.knv:9:28
  |
9 |   (println (uppercase-name (->product 7))))
  |                            ^^^^^^^^^^^^^ this `product` has a `:name` of type `i32`
  |
 ::: two.knv:5:27
  |
5 | (defn uppercase-name (x) (upper-case (:name x)))
  |                           ---------- `upper-case` takes `(ref str)`, so `:name` must be `str`
  |
  = note: `uppercase-name` accepts every struct whose `:name` is `str`: `user` and `animal`
  = help: pass a struct whose `:name` is `str`, or write a `type` signature for `uppercase-name` that names `product`
```

A field type that lacks a trait the body needs takes the same code, with the trait in the title:

```text
error[L0466]: `describe` needs a `:name` that implements `display`, but `badge`'s `:name` is `(fn () int)`
```

The secondary label points at the form that brought the trait, as `str` here. The note lists the structs that meet the bound, in source order, and the label of a struct from another module prints as two types of one name print in a diagnostic today.

The message never names `HasName`, `FieldType` or `get_name`, so rustc's `E0271`, which the prototype hits when it calls `uppercase_name(&Product { .. })`, never reaches the user.

A call with a value that has no such field reports the existing codes at the call, with a secondary label on the read and the same note:

| Argument | Code | Title |
| --- | --- | --- |
| a struct without the field, `(hello (->point 1.0 2.0))` | `L0408` | `` `point` has no field `:name` `` |
| an enum, `(hello shape/point)` | `L0410` | `` `shape` has no fields `` |
| any other type, `(hello 5)` | `L0410` | `` `int` has no fields `` |

### 2. Readable output

1. **A trait emits only for a field that an emitted function reads from a generic owner.** A read whose owner is a known struct, from the start or after pinning, emits `x.name` as it does today. A program with no generic read after pinning emits no trait and no impl, so its Rust stays byte for byte what `main` emits.
2. **An impl emits only for a struct that a reached call passes for the bound,** directly or through another generic function's bound, as the structural trait impls of [8.2 Struct traits](../docs/language.md#82-struct-traits) do. A struct no call passes gets no impl.
3. **The trait is `Has` and the field's Rust name in Pascal case, with `type FieldType: ?Sized` and `fn get_<field>(&self) -> &Self::FieldType`.** `admin?` gives `HasIsAdmin` and `get_is_admin`. `?Sized` lets a `str` or `(slice a)` field, which a struct stores as a borrow, give its view as the field type, so `get_name` returns `&str` and no read writes `&&str`. A trait name that a written item takes yields as an introduced item does: `HasName2` beside a program's own `has-name` struct.
4. **The trait emits at the crate root, and each impl beside its struct.** A struct in another module emits `impl crate::HasName for Animal<'_>` in that module's `mod`, and a module that calls `get_name` gets a Rust `use` of the trait, as a user trait's method call does.
5. **An impl for a struct that holds a borrow names its lifetime,** as `impl<'a> HasName for User<'a>`, as [10. User-defined traits](../docs/language.md#10-user-defined-traits) writes `impl<'a> Finder for Snippet<'a>`. A generic struct's impl carries its parameters, as `impl<A> HasName for Boxed<A> { type FieldType = A; ... }`.
6. **A field type that only the bound names emits as the projection, `A::FieldType`.** Its traits go to a `where` clause, as `A::FieldType: std::fmt::Display`. A fixed field type emits as the trait's binding, as `HasName<FieldType = str>`.
7. **A pinned function emits what it emits with the struct written in its signature.** With only `(hello (->user "ana"))` in `main`, `hello` emits `fn hello(x: User<'_>) -> String` and `x.name`, the Rust that `main` emits today when `user` is the only struct with `name`.

### 3. Ownership

A generic read borrows. The getter returns `&Self::FieldType`, and the read's type is `(ref b)`. A Copy value leaves the borrow only where a use needs a value, by the existing Adjustment rule, so the read never decides ownership from a type it does not know yet. On `main`, a generic struct shows why: `(struct (boxed a) (name a))` with `(defn hello (x) (:name x))` called on `(new boxed (name 3))` is `L0401`, ``expected `(ref int)`, found `int` ``, at the read, because the read took a borrow before pinning made `a` a Copy `int`. The build fixes that report for both kinds of read.

| Use | Bound | Emitted read |
| --- | --- | --- |
| a borrow, `(defn name-of (x) (:name x))` | `(a (has :name b))` | `fn name_of<A: HasName>(x: &A) -> &A::FieldType { x.get_name() }` |
| a copy, `(defn next-age (x) (+ (:age x) 1))` with `:age int` | `(a (has :age int))` | `*x.get_age() + 1` |
| a copy of a field type the body leaves open, as `(set! total (:age x))` | `(b copy)`, inferred as for a generic parameter | `total = *x.get_age()` |
| a clone, `(clone (:name x))` | `(b clone)` | `x.get_name().clone()` |
| a move, `(keep (:name x))` with `keep` taking an owned `string` | none | `L0503` at the read, as for a known struct today, with the help `(clone ...)` |
| a change, `(set! (:age x) 5)` | `(a (has :age int))`, and `x` takes `mut` | `*x.get_age_mut() = 5` |

1. **A read never moves the field,** as on `main`, where only the base of a collection or struct update moves one. A use that needs to own a field that is not Copy is `L0503` today and stays `L0503`.
2. **A `set!` of a generic field, or a `mut` borrow of one, adds `fn get_<field>_mut(&mut self) -> &mut Self::FieldType` to the trait.** The trait gains the method only when a reached function needs it. The parameter takes `mut` by the existing Modes rule, so `(defn birthday! (x) (set! (:age x) (+ (:age x) 1)))` emits `fn birthday<A: HasAge<FieldType = i64>>(x: &mut A) { *x.get_age_mut() += 1; }`. Open detail 1 asks the owner to confirm the method.
3. **A struct update or a collection update on a generic owner stays an error.** `(assoc p :x 2)` builds a new value of the owner's struct, and a field trait has no constructor. Rule 4 gives the error its new text.
4. **A generic owner is never Copy.** A parameter whose type only a field bound knows takes `ref` or `mut`, never copy, unless the body infers `copy` for it. That keeps decision 1's `hello` at `x: &A`.

### 4. Diagnostics with several candidates

A read on an unknown owner prints as the bound everywhere a type prints, as a `rust-trait` with an associated type prints `(c (container item))`:

| Where | Text |
| --- | --- |
| `knave types` and a doc page's signature | `(type hello (where ((a (has :name b)) (b display)) (fn ((ref a)) string)))` |
| `(infer (:name x))` in `hello` | `"b"`, the field type's variable, as `(infer x)` is `"a"` |
| `(infer hello)` | `"(where ((a (has :name b)) (b display)) (fn (a) string))"`, with modes left out by the existing rule |
| a type error that names the owner | `a`, with a note that `a` is any struct with `:name`: `user` or `animal` |

The text pastes into a `type` signature, so a written signature can state the bound: `(type hello (where ((a (has :name b)) (b display)) (fn ((ref a)) string)))`. `has` is a kernel bound that the prelude re-exports and documents, as it does every kernel name. A bound may hold several fields, `(a (has :name b) (has :age int))`, and a field type may carry its own bound, so `(:city (:address x))` gives `(a (has :address b))` and `(b (has :city c))` when two structs declare `:city`.

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

The help appears only when a field of a visible struct is within the distance of the existing closest-name rule.

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

### 5. What decides the parameter's type today

On `main`, `field_owner` (`src/types/infer.rs:3564`) strips the value's borrows. A known struct is the owner. A type variable goes to `unique_owner`, which takes every struct of the program that declares the field (`StructTable::with_field`, `src/types/structs.rs:101`). One struct fixes the variable on the spot. Zero or several are `L0407`. A struct update uses the same lookup through `update_owner_from_fields` (`src/types/infer.rs:3390`), keeping the structs that declare every field it names.

After the build, the lookup counts the candidates of Rule 6.1. One candidate still fixes the owner at the read, so every program that compiles on `main` keeps its types. Two or more give the owner the field bound, and a later form of the body that fixes the owner checks the bound then, so the read no longer decides at its position. None is `L0408` at the read, as Rule 4 shows.

| Program | On `main` | After the build |
| --- | --- | --- |
| decision 1, `user` and `animal` | `L0407` at `(:name x)` | generic, as Option A shows |
| decision 2, `user`, `animal` and `product` | `L0407` at each `(:name x)` | generic, as Option A shows |
| decision 1 with `user` alone | `fn hello(x: User<'_>) -> String`, single-struct inference | the same, byte for byte |
| decision 1 with `(type hello (fn ((ref user)) string))` | `fn hello(x: &User<'_>) -> String` | the same, byte for byte |

A throwaway build of `main` that logs each zero-or-several lookup ran `knave check` on each of the 2,702 `.knv` files under `prelude/`, `examples/` and `tests/fixtures/`, leaving out `read/` and `format/`. Three files reach the lookup, and only their snapshots change:

| File | Lookup | Snapshot after the build |
| --- | --- | --- |
| `tests/fixtures/check/type_shape_errors.knv` | read of `x`, two structs | loses its first error: `read-x` compiles as a generic reader |
| `tests/fixtures/check/assoc_ambiguous.knv` | update of `x`, two structs | keeps `L0407`, with the label and note of Rule 4 |
| `tests/fixtures/run/doc_examples_bare_keyword.knv` | read of `size`, no struct | `L0408`, ``no struct has a field `:size` `` |

No prelude or example file reaches the lookup, so no other output changes.

### 6. Interplay

1. **Structs across modules.** A candidate is a struct whose fields the reading module may read, by the Visibility rule of [RFC 0038](0038-field-keywords.md): one of its own, or one it imports. A private struct of another module no longer counts, so the `pets2` program of the Problem compiles with `user` as the only candidate. A call that passes a struct whose fields the reader may not read is `L0408` at the call, with a note that the struct's fields are private to its module.
2. **Generic structs.** A generic struct is a candidate like any other, and its field type takes the struct's arguments at the call: `(boxed int)` meets `(has :name int)`. Its impl carries the struct's parameters, as Rule 2 says.
3. **An enum's case.** An enum is never a candidate. A case's fields exist only inside that case, so a getter of `&Self::FieldType` cannot exist for the enum. A program reads a case's field through `match`, as on `main`, and a generic reader called with an enum is `L0410`.
4. **A written type annotation.** A written signature is exact. One that names a struct emits today's Rust. One that names a type variable without the field bound is `L0440`, with a help that writes the bound, as `(a (has :name b))`, as a missing `copy` gets `(a copy)` today. A binder annotation, as `(x (ref user))`, fixes the owner.
5. **A field the prelude reads.** The prelude declares no struct and no function that reads a field. Its macros, `defstruct`'s constructors, `assoc`, `assoc-in`, `update` and the keyword closure, expand to `field` and `new` in the user's module, so their reads follow these rules there. A keyword closure, as `(map (iter xs) :name)` in an unannotated function, gives the function the bound through its parameter's element type: `fn names<A: HasName>(xs: &[A]) -> impl Iterator<Item = &A::FieldType>`. `update` reads and then builds, so a generic owner is the update error of Rule 3.3.

### 7. Prelude first

The prelude cannot express this, and the build adds no kernel form:

- **The choice is the checker's.** Whether `(:name x)` emits `x.name` or `x.get_name()` depends on the type of `x` after inference and pinning. A macro runs before the checker and sees only the form.
- **The struct set is the program's.** The candidates span modules, and each impl depends on the calls that reach it. Only the compiler sees every struct, every call and every pin.
- **A trait per field from a macro clashes,** as Option B says, and [RFC 0038](0038-field-keywords.md) moved keyword reads into the expander for the same reason.

The change lives in the type rule of the existing kernel form `field`, and in one new kernel bound, `has`, which a `where` writes beside traits. `docs/language.md` records why `has` is a kernel name: a bound that names a field of any struct needs the struct table, which only the compiler holds. The prelude keeps every macro unchanged, and it documents `has` on its own page, as it documents every kernel name it re-exports. The Rust names `HasName`, `FieldType` and `get_name` are no fact the prelude states, so the compiler derives them from the field, as it derives `make_point` from `->point`.

## Open details

1. **The `mut` getter.** Rule 3.2 adds `get_<field>_mut` to the trait when a `set!` or a `mut` borrow needs it. The alternative is an error at such a `set!`, with help to write a signature. The recommendation is the method, because Rust programmers write it and the uniform `set!` rule then holds for generic owners.
2. **The bound's spelling.** This RFC writes `(has :name b)`. A keyword names the field as a read and a `defstruct` do. `field` would read as the kernel form, and a name ending in `?` reads as a predicate. The owner may prefer another name.
3. **A `string` field read as `str`.** On `main`, a read of a known `string` field meets `(ref str)` by the `str` adjustment. A generic read whose body fixes `str`, as `uppercase-name` does, rejects a struct whose `:name` is `string`, with `L0466`, because `String` and `str` are two field types. The alternative is a field bound that meets the use through a view, emitted as `A::FieldType: AsRef<str>` and `x.get_name().as_ref()`. The recommendation is the exact rule for this build, and the view rule as a later RFC if programs need it.

## Prototype

`target/scratch/proto/` held the prototype, which is not part of this pull request. rustc 2024 compiled and ran the Rust of Option A, Rule 2 and Rule 3: `d1.rs` printed `hello ana` and `hello rex`, `d2final.rs` printed `ANA`, `REX`, `name: ana` and `name: 7`, and `d3final.rs` checked the copy, clone, borrow and `get_age_mut` reads. Calling `uppercase_name(&Product { name: 7 })` gave rustc's `E0271`, which Rule 1 replaces with `L0466`. The instrumented `main` build of Rule 5 changed only two `eprintln!` lines in `src/types/infer.rs`.
