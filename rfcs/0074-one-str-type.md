# RFC 0074: One `str` type with inferred ownership

**Status:** Proposed
**Date:** 2026-11-13

## Problem

Text has two user-facing types today:

- `str`, the unsized borrowed view. A string literal has type `(ref str)`, a
  `str` field or parameter is always borrowed, and Rust never owns a `str`.
- `string`, the owned Rust `String`.

The split leaks into the standard library. Text functions disagree about
which side they take: `chars`, `lines`, `split`, `trim`, `subs` and `parse`
take `(ref str)`; `clone`, `pr-str` and `len` borrow automatically; `as-str`
converts an owned `string` to a `(ref str)`; and `the` lets a `(ref string)`
pass as a `(ref str)`. A reader cannot predict which conversions are
implicit. `(str x)` builds a `string`, not a `str`, so the constructor does
not match the type it reads as. The compiler already infers parameter modes
and lifetimes, so the split asks the user to write an ownership decision the
compiler can make.

The language needs one text type. The compiler decides, per use, whether the
value is Rust `String` or `&str`.

## Options

Each option below shows the source and the Rust it would emit for:

```lisp
(defn greet (name) (str "hello " name))
(defn keep (name) (->user :name name))
```

### A: Keep the pair and infer only parameters

`str` stays the borrowed view and `string` stays the owned type. Written
`string` parameters become shrinkable to `&str` when read-only.

```lisp
(defn greet (name) (str "hello " name))
```

```rust
fn greet(name: &str) -> String {
    format!("hello {name}")
}
```

Fields, locals and containers still need the user to choose `str` or
`string`, and a value cannot move between a `(vec str)` and a `(vec string)`.

### B: Surface alias only

`str` names the owned type, and `(ref str)` stays the borrowed view. The
compiler inserts views where an owned string meets a borrowed position.

```lisp
(defn greet (name) (str "hello " name))
```

```rust
fn greet(name: &str) -> String {
    format!("hello {name}")
}
```

`(vec "a" (str "b"))` still fails: the literal is a borrow and the created
string is owned, and one vector cannot hold both without an allocation the
source did not write. The one type is only a spelling.

### C: Text kind with inferred representation parameters

`str` is one surface type with two representations. The compiler infers which
one each value and type position uses, generalizes a hidden representation
parameter when a definition is used at both, and pins it to the concrete
Rust form when every use agrees.

```lisp
(defn greet (name) (str "hello " name))
(defn keep (name) (->user :name name))
```

```rust
fn greet(name: &str) -> String {
    format!("hello {name}")
}

fn keep<A: AsRef<str>>(name: A) -> User<A> {
    User { name }
}
```

A `str` field is a representation parameter. `User` pins to
`User<'a> { name: &'a str }` or `User { name: String }` when all uses agree,
and stays `User<A>` when they do not. A value never becomes owned implicitly:
a borrowed value at an owning position is `L0503`, with `(str value)` or
`(clone value)` as the explicit fix.

### D: `Cow<'a, str>` at run time

One Rust type represents both states:

```rust
fn greet(name: Cow<'_, str>) -> Cow<'static, str> { ... }
```

The emitted Rust gains an enum tag and a lifetime on every text value, and
the program pays for a branch the source did not write. It is not the Rust a
person would write.

### E: Always borrowed, with owned temporaries

Every text value is `&str`; creation sites produce temporaries that the
compiler binds to owners.

```rust
fn greet(name: &str) -> &str { ... }
```

A function cannot return created text, a struct cannot own text, and every
returned value borrows a local. The model cannot express the standard
library's owned results.

### F: A `text` trait as the type domain

`str` lowers to an ordinary type variable bounded by a prelude `text` trait,
implemented for `String` and `&str`.

This reuses the type variable machinery, but the domain is a user-visible
trait. A user can implement it for another type, `str` stops meaning exactly
two representations, and display/defaulting depend on recognizing the trait
rather than on the type. A compiler-owned kind states the domain once.

## Recommendation

Option C.

- **One type.** `str` names text. The two representations are an
  implementation choice the compiler makes.
- **No hidden allocation.** Owned text is created only where the source
  creates it: `str`, file and standard-input reads, `upper-case`,
  `lower-case`, `pr-str`, documentation pages, command-line arguments,
  parsing into text, and Rust templates that return `String`. Borrowing an
  owned value is free. A borrowed value never becomes owned without
  `(str value)` or `(clone value)`.
- **Predictable defaults.** A parameter borrows unless its body needs
  ownership. A literal borrows for `'static`. A created value owns. A field,
  element or local that receives only borrows borrows; one that receives a
  created value owns; one used at both representations generalizes.
- **User experience.** `(str x)` builds a `str`. `as-str` and the `the` view
  special case disappear. A `string` type is `L0201` with `str` as its
  suggestion.
- **Rust's model.** A hidden representation parameter is a Rust type
  parameter with the prelude's text bound, and a pinned one is `String` or
  `&str`. No wrapper type, clone or allocation is introduced.

### Decisions

1. **Representations.** `str` has two representations: owned (`String`) and
   borrowed (`&'a str`). A type position that writes `str` is a text kind
   variable. It unifies with the two representations and other text kind
   variables, and with nothing else.
2. **Explicit forms.** `(ref str)` is `&str`, the borrowed representation.
   `(mut str)` is `&mut String`, owned mutable text. `(box str)` is
   `Box<str>` under the existing unsized-parameter rule. A string literal is
   `&'static str`. `(str ...)` is owned.
3. **Generalization and pinning.** A text kind variable that is free in a
   function, struct, enum, trait or impl scheme becomes a hidden
   representation parameter. Every use at one representation pins it to the
   concrete form, including a borrowed lifetime form. Every use at both
   keeps the Rust type parameter. Surface output prints every representation
   parameter as `str`.
4. **Owned positions.** A created value moves into a field, element or local
   and makes that position owned. A borrowed value leaves it borrowed. A
   position that receives both is owned, and each borrowed construction is
   `L0503` with the fix `(str ...)` or `(clone ...)`. Function parameters are
   borrowed unless the body moves, stores or mutates them; a caller passing
   an owned value does not make a parameter owned.
5. **Locals and results.** A local follows its initializer. A later owning
   use of a borrowed local is `L0503`, with the fix to build the initial
   value with `(str ...)`. A result is owned when the body creates or owns
   it, borrowed when the body returns a borrow or a literal, and a hidden
   representation parameter when the body returns a parameter of a
   representation-polymorphic function.
6. **Function types.** A Rust `fn` pointer cannot be generic over a type, so
   a function type pins its text representations. A named
   representation-polymorphic function passed as a function value is wrapped
   in a closure that instantiates the representation. A function type that
   genuinely needs both representations is an error with a wrapper, trait or
   struct fix.
7. **Traits.** A `str` row or implementor means both representations; the
   compiler emits the per-representation or generic impls the trait needs.
   Cross-representation text equality, order and hashing work. Owned-only
   operations (`from-str`, `push!`, mutation) constrain the representation
   to owned and report the missing representation. The text domain is
   compiler-owned and no program can implement it.
8. **Text bound.** Generic text parameters carry the prelude's text trait,
   consulted like the copy, order, equality, hash and borrow traits. It
   provides `AsRef<str>` for views; the prelude's text operations add their
   own display, sequence and show bounds as usual.
9. **Rust templates.** A prelude-internal owned-text binding with the Rust
   template `String` lets a `rust` template return owned text. The compiler
   recognizes it like `Vec` and `Box`, prints it as `str`, and no program
   can import it.
10. **`the` and `infer`.** `(the str x)` does not pin a representation;
    `(the (ref str) x)` forces borrowed and `(the (mut str) x)` owned
    mutable. The old view special case is deleted. `infer`, `types` and
    diagnostics print `str` for text, and `(ref str)` or `(mut str)` only
    where the source wrote one.
11. **Diagnostics.** `L0503` remains the needs-ownership code. Its page
    covers text, fields, containers and returns, and its help names
    `(str value)` for text and `(clone value)` where cloning preserves the
    meaning. `L0510`'s help names the owned-text fix instead of `string`.
12. **Migration.** `string` is removed with no alias. The prelude, examples,
    documentation and fixtures migrate in the same change. This is a
    breaking change and takes the `release:minor` label.

### Prototype

The prototype is the change that carries this RFC. It implements the text
kind, representation inference, pinning, emission, the prelude migration and
the fixture migration, and records the emitted Rust and diagnostics for:

- a literal and a created value at a parameter, a local and a result;
- a `str` field pinned borrowed, pinned owned and generalized;
- a literal and a created value in one vector;
- a returned value that needs ownership;
- `infer`, `the`, `(ref str)` and `(mut str)`;
- `as-str` and a `string` type as errors.

## Decision

Pending the project owner's review.

## Rejected options and why

- **A: keep the pair and infer parameters only.** It still asks the user to
  choose a representation in fields, locals and containers, and it cannot
  hold a literal and a created string in one container. The two-type
  unpredictability remains.
- **B: surface alias only.** It renames the owned type but does not make the
  literal and the created value one type. `(vec "a" (str "b"))` still fails,
  and a borrowed value still needs a `(ref ...)` spelling the compiler could
  infer.
- **D: `Cow`.** The tag, the branch and the lifetime on every text value are
  a run-time cost the source did not write.
- **E: always borrowed.** It cannot express owned fields, owned collection
  elements or a function that returns created text.
- **F: a trait as the domain.** The domain is a trait a program can
  implement, not exactly `String` and `&str`, and display/defaulting would
  have to recognize a trait rather than a type.

## Rule

[docs/language.md: Types](../docs/language.md#types),
[Literals](../docs/language.md#literals),
[Type texts](../docs/language.md#type-texts),
[Standard types](../docs/language.md#standard-types),
[Ownership](../docs/language.md#ownership),
[Structs](../docs/language.md#structs),
[Rust interop](../docs/language.md#rust-interop),
[Names](../docs/language.md#names)
