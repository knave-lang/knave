# RFC 0078: One text type

**Status:** Proposed
**Date:** 2026-11-18

This RFC answers the owner's issue #589: its five rules and its four open questions. It designs issue #590, on auto-borrowing read-only references, with it (section 2, decision 14). PR 598 (branch `infer-string-ownership`) is the owner's rough exploration of the same end state. This RFC weighs it as one option, and "Differences from PR 598" lists each point where this design departs from it.

Code references are against `main` at `a6545d9a2` unless they name another tree. PR 598 references are against its head `081cb1e48`. The Problem section's "today" outputs were run again with `knave check` built from `a6545d9a2` and are unchanged. Every other "today" output below comes from the probe binary of `5234c8817`, an earlier `main`, downloaded from the project's Cachix as `docs/pull-requests.md` says, and every PR 598 output from its merge binary (`513d3f62c`), downloaded the same way. Where an output went through rustc, the rustc is the dev shell's.

## Problem

Text has two user types today: `str`, Rust's unsized view, always behind a borrow, and `string`, Rust's `String`. A string literal is `(ref str)` (`src/types/infer.rs:995`), `(str x)` is a `string` (the `str` macro types its `rust` form `string`, `prelude/text.knv:503,507`), and a field read of a `string` field is `(ref string)`. Which of these meet where is decided by several rules that a user cannot see from the source.

**Today's outputs.** Each program below is rejected by `knave check` on `5234c8817`:

```lisp
(defn shout (s) (upper-case s))
(defn main ()
  (let (owned (str "bo"))
    (println (shout (ref owned)))))
```

```text
error[L0401]: mismatched types
4 |     (println (shout (ref owned)))))
  |                     ^^^^^^^^^^^ expected `str`, found `string`
```

The user wrote `ref`, the callee only reads, and the program still needs `(as-str owned)`. `(trim (str " a" 1))` reports the same `` expected `str`, found `string` `` at the argument.

```lisp
(defn pick (c) (if c (str "x") "y"))
```

```text
error[L0401]: mismatched types
1 | (defn pick (c) (if c (str "x") "y"))
  |                      --------- ^^^ expected `string`, found `(ref str)`
```

```lisp
(defn main ()
  (let (names (vec))
    (push! names "a")
    (push! names (str "b"))
    (println (len names))))
```

```text
error[L0401]: mismatched types
4 |     (push! names (str "b"))
  |                  ^^^^^^^^^ expected `str`, found `string`
```

```lisp
(defstruct user :name string)
(let (u (->user :name (clone "Ana"))) (println (:name u)))
```

```text
error[L0401]: mismatched types: expected `string`, found `str`
error[L0404]: `str` does not implement `clone`
```

`L0503`'s help suggests `(clone x)` for a value that needs ownership, but cloning a literal is an error. The working spelling is `(str "Ana")`.

Meanwhile some meetings work without a word: `(ref string)` passes as `(ref str)` at an annotated binder, which is how `(the (ref str) (ref word))` works (`prelude/syntax.knv:833`); `len` and `pr-str` take either through their traits; and a closure whose `(ref string)` result meets `(ref str)` gets a hidden `as-str` call. The issue's first motivation is exactly this: users and agents cannot predict which conversions are implicit.

**The decision is made in many places.** RFC 0065 asks that one stage decide each fact and every later stage read it. Owned or borrowed text is decided today in:

| Place | What it decides |
|---|---|
| `src/types/unification.rs:48-60` with `src/types/view_coercion.rs:15-23` | `(ref string)` meets `(ref str)`. The result is a bool, and the typed IR keeps `(ref string)`. |
| `src/types/string_view.rs:9-47`, called from `infer.rs:1201` and `infer/shared_variables.rs:330` | inserts `as-str` calls after a closure body and beside a shared type variable |
| `expand/src/expand/string_view.rs:8-41` and its threading through `expand/src/expand/mod.rs`, `prelude_state.rs:186`, `saved_definitions.rs:182-185`, `src/types/checker.rs:145,1750`, `deferred_definitions.rs:28` | finds `as-str` by its signature so the checker can insert it |
| `src/types/infer.rs:499-508` (`pattern_type`), `:510-516` (`is_string_value`), `:727-746` | a `match` on a `string` retypes the scrutinee to `(ref str)` |
| `src/types/infer.rs:2869-2949` | `push!` onto a `string` |
| `src/types/infer.rs` `infer_field` | a non-copy field read is `Type::shared`, so a `string` field reads as `(ref string)` |
| `src/ownership/slices.rs:199-210` | an inferred `string` parameter whose mode ends at `Ref` becomes `&str` |
| `src/ownership/functions.rs:78-85`, `modes.rs:67,80`, `rewrite.rs:36-43` | an unsized `str` parameter is always `Ref` |
| `src/ownership/values.rs:534-542` | `coerced_view` asks `views_string` again for a function value |
| `src/emit/expressions.rs:566-572,680-704` | writes `.as_str()` for a `match`, with the Rust text in the emitter |
| `src/emit/expressions.rs:2012-2020` (`depth_allowance`) | asks `ViewCoercion::views_string()` again in the emitter |
| `src/emit/lent_temporaries.rs` | lifts a temporary such as `(as-str (str ...))` into a local |
| `src/emit/items.rs:1072-1085`, `src/emit/result_borrows.rs`, `held_borrows.rs`, `opaque_borrows.rs` | every lifetime of a borrowed text, in a struct or a signature |

The language cannot say "one text type" today because `str` and `string` are two `PrimitiveType` variants (`expand/src/core/primitive.rs:7-8`, rows `:44-48` and `:49-53`), and every one of the places above keys on the difference.

## Options

Each option is shown on the same program unless it says otherwise:

```lisp
(defstruct user :name str :age i64)
(defn shout (s) (upper-case s))
(defn main ()
  (let (ana (->user :name (str "Ana") :age 3)
        owned (str "bo"))
    (println (shout owned) (shout "lit") (shout (:name ana)))))
```

### A. The representation is part of the type (PR 598)

`str` lowers to a text kind variable inside Hindley-Milner inference. Unifying it with `String` or `&str` binds it, a free one is generalized into a hidden type parameter of the function, struct or enum, and an open one defaults to `&str`.

On the program above, PR 598's merge binary reports:

```text
error[L0401]: mismatched types
6 |     (println (shout owned) (shout "lit") (shout (:name ana)))))
  |                     ^^^^^ expected `str`, found `str`
```

On a program with a `user` struct whose `:name` is `str`, built with `(str "Ana")`, and `(defn shout (u) (upper-case (:name u)))`, the call `(shout (ref ana))` reports `` expected `user`, found `user` ``. With `(defn keep (name) (vec name))` called once with `"a"` and once with `(str "b")`, it emits `fn keep<A>(name: A) -> Vec<A>`.

It accepts two programs that rustc rejects. The `push!` program of the Problem section emits `names.push(&"b".to_string())`, and rustc reports `E0716`, temporary value dropped while borrowed; with the two pushes swapped, the program gets `L0503` instead, so argument order decides the outcome. And:

```lisp
(defstruct user :name str :age i64)
(defn main ()
  (let (us (vec (->user :name "a" :age 1) (->user :name (str "b") :age 2)))
    (println (len us))))
```

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct User<A> {
    name: A,
    age: i64,
}
// in main:
let us = vec![User { name: "a", age: 1 }, User { name: "b".to_string(), age: 2 }];
```

rustc reports `E0308`, because `User<&str>` and `User<String>` are two types. Both breaks violate RFC 0065.

The design reasons, apart from the prototype's state:

- **Unification is symmetric, and text flow is not.** Owned text may meet a borrowed position for free. Borrowed text may not meet an owned position without an allocation. HM equates the two sides, so PR 598 added a one-way arm, `(&str, String) => Ok(())` without binding (PR 598 `unifier.rs:327`), which also accepts `Vec<&str>` against `Vec<String>`, and the first binding of a variable decides it.
- **The decision is split again**, across PR 598's unifier, `unification.rs` and its argument passing in `infer.rs:2192-2198` and `:2402-2408`.
- **Hidden type parameters are not hand-written Rust**, and two values of one struct can have two Rust types that print alike.

### B. `str` is owned text, `(ref str)` is the view

`str` is a new spelling of `String`, a literal stays `(ref str)`, and today's adjustment rules make owned text meet a borrowed position.

```lisp
(defn pick (c) (if c (str "x") "y"))
```

```rust
// rejected before emission:
// error[L0401]: expected `str`, found `(ref str)`
```

This is today's language with `string` renamed. Every printed type that holds a literal or a field read says `(ref str)`, so a user sees two types and writes the one the printer showed. Rule 1 holds in name only. Option C with the owner's decision 8 accepts `pick`, with a mixed result.

### C. The representation is an ownership fact (recommended)

The type stage sees one type, `str`. Whether each text value is owned, a view, mixed, or a reference to owned or mixed text is decided once, in the ownership stage, by the same fixed point that already decides parameter modes. The ownership stage then writes the decision into the typed IR as Rust-level types, as `ModeRewrite` already does for parameter modes (`src/ownership/rewrite.rs:24-62`), and every later stage reads those types. Section 1 states the rules.

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
struct User {
    name: String,
    age: i64,
}
fn shout(s: &str) -> String {
    s.to_uppercase()
}
fn main() {
    let ana = User { name: "Ana".to_string(), age: 3 };
    let owned = "bo".to_string();
    println!("{}{}{}", shout(&owned), shout("lit"), shout(&ana.name))
}
```

### D. `Cow<'a, str>` everywhere

```lisp
(defn shout (s) (upper-case s))
```

```rust
fn shout<'a>(s: Cow<'a, str>) -> Cow<'static, str> {
    Cow::Owned(s.to_uppercase())
}
// (shout "lit") emits shout(Cow::Borrowed("lit"))
```

Every text value carries a tag and a lifetime, and every read branches on the tag. That is a run-time cost the source did not write, and no Rust programmer writes it by default.

### E. Always owned

```lisp
(shout "lit")
```

```rust
fn shout(s: String) -> String { s.to_uppercase() }
// emits shout("lit".to_string())
```

Simple, and an allocation at every literal passed anywhere, which rule 4 of the issue forbids.

## Recommendation

Option C. It is the only option where the type a user sees is one type, the Rust is the Rust a person writes, and one stage decides. It also reuses the machinery that decides the same fact for parameters today: an inferred `string` parameter whose mode ends at `Ref` already becomes `&str` (`src/ownership/slices.rs:199-210`), and a `string` field that stores a parameter already makes the parameter `Own`. Option C extends those rules from `string` parameters to every place a text value lives.

## 1. The model

### Five representations

Rust holds text in five shapes, and the model names each; the solver decides the first three, and the two lent shapes follow from the slot they refer to:

| Representation | Rust | Arises from |
|---|---|---|
| **owned** | `String` | created text, a local or field that owns it |
| **view** | `&str` | a literal (`&'static str`), a parameter that only reads, a view a prelude form returns (`trim`, `split`, `subs`) |
| **mixed** | `std::borrow::Cow<'static, str>` | a slot that stores both `'static` text and owned text (owner's decision 8): a literal goes in as `Cow::Borrowed`, created text as `Cow::Owned`, and a read derefs to `&str` |
| **lent mixed** | `&Cow<'static, str>` | a reference to mixed text, built the same ways as lent: a read of a mixed field, `first` of mixed elements, the item of `(iter names)` over them. It meets a view as lent does, by a deref at a coercion site or `.as_ref()` elsewhere, and like lent it cannot enter an owned or mixed slot without `(str ...)`. |
| **lent** | `&String` | a reference to owned text that a prelude template or the language builds: a field read of an owned field, `(ref owned)`, an element read from an owned collection (`first`, `get`, `at`), the item of `(iter names)` |

The solver chooses among owned, view and mixed, for the slots it decides (below). A view is `'static` when only `'static` text reaches it, which the mixed representation needs to know. Lent is never chosen: it is what a reference to an owned slot is, so it follows from the owned slot it refers to. Today's binary already emits it for `string`: `(first names)` gives `Option<&String>`, and `(map (iter names) ...)` gives `|w: &String|`.

**References two deep.** A closure parameter that a higher-order form fixes can hold a reference to one of these: `filter` over owned names gives `|w: &&String|`, and `find` over `split` gives `|w: &&str|`. These are not another representation: they are a shared reference to a lent value or to a view, as `(ref (ref i64))` is a reference to a reference. The type stage erases one layer (below), so such a parameter's HM type is `str`, and `TypeDisplay` prints text behind any number of shared references as `str`, under decision 2(a). `=` and every comparison keep today's shared-parameter rule (`docs/language.md: Ownership`, "Shared parameters"): "Arguments with the same number of references pass as they are. Otherwise, each argument gets one reference, or none where Rust borrows the argument by itself." Today `(filter (iter names) (fn (w) (= w "a")))` over owned names is `L0401`, `` expected `string`, found `str` ``; with `(as-str w)` it emits `w.as_str() == "a"`, and over view elements it emits `***w == *"a"`. After this RFC, over owned names, the expected output is `**w == *"a"`, which compiles through `PartialEq<str> for String`; a comparison whose trait has no such cross row takes a view node (section 1, "A free view"). PR 2 pins both in fixtures.

### The type stage: one type

- **One user type.** The `string` row leaves the kernel's exports (decision 5 covers the name). The `PrimitiveType::String` variant stays inside the compiler as the owned representation. `str` is a sized type to the checker: it is no longer `is_unsized()` (`src/types/ty.rs:82-84`), so `borrow_views` (`src/types/type_declaration_collector.rs:276-284`, called from `lower_stored_type` at `:267-274`) and `borrow_unsized_result` (`src/types/checker.rs:937-947`) stop wrapping it, and `FunctionTable::fixed_mode` (`src/ownership/functions.rs:78-85`) stops forcing it to `Ref`.
- **Shared reference layers on text are erased for HM.** In expression types, any number of shared references directly around text is text: a literal, `(:name u)`, `(ref s)` and `(first names)`'s payload, `(ref str)` or `(ref (ref str))` in today's types, all have HM type `str`, so `(if c (:name u) "y")` and `(unwrap-or (first names) "none")` over view elements unify; today the second is `L0401`, expected `(ref (ref str))`, found `(ref str)`. The layer is not lost: section 1's solver recomputes it from the slot it refers to, and the rewrite writes it back. `infer_field` keeps making a non-copy field read `Type::shared`, and the erasure is the one place that reads it for text. `(mut str)` is a mutable reference to text in the slot's representation: `&mut String` for an in-place change, which only owned text allows, and `&mut &'static str` or `&mut Cow<'static, str>` for a whole replacement (the `mut` row).
- **Schemes keep their references.** A `rust` form's registered scheme keeps `Type::Ref` around `str`, so `template_ties.rs` (`lifetimes_in`, `src/types/template_ties.rs:165-174`, sees only `Type::Ref` and `Type::Mut`) still finds the ties of `trim`, `subs`, `lines` and `split`. The erasure applies to the instantiated types of expressions, not to the scheme the ties read.
- **Literals.** A string literal has HM type `str` (`infer.rs:995`).
- **Written forms (decision 3 and 4).** A bare `str` states no representation: in a `defn`, a field, a binder or a `type` signature, the compiler chooses it. `(ref str)` states a view. `(own str)` states owned text. A `rust` form's signature must state each text position as `(ref str)`, `(own str)` or `(cow str)` (decision 17), because its Rust is fixed: a bare `str` there is an error that names them. An implementor row of a `rust-trait` may name `(own str)`, `(ref str)`, `(cow str)`, or bare `str` for a trait that every representation holds.
- **Traits decide where a representation qualifies.** A trait whose implementor rows exclude a representation makes a value of that representation `L0503` where the value needs the trait, and a trait whose rows cover only one representation forces it on a slot that the solver may still choose. The rows say so, not a list in the compiler: today only `string` implements `from-str` (`text.knv:189`) and `clone` (`ownership.knv:11`). After the respelling `from-str`'s row names `(own str)`, because Rust has no `FromStr` for `Cow`, so `parse` to text makes owned text. `clone`'s rows name `(own str)` and `(cow str)`, because Rust's `Cow` is `Clone`: a `clone` of owned text is owned, a `clone` of mixed text stays mixed and emits `x.clone()`, a clone the source writes, and a `clone` of a view is `L0503` with the help `(str x)` (decision 19(c)): `clone`'s definition, `(def clone (rust "{0}.clone()" (where ((a clone)) (fn ((ref a)) a))))` (`ownership.knv:30-32`), types a view's clone as a view, and nothing in the prelude can state that a clone of a view makes owned text. So a `clone` on one line never forces a mixed field owned and puts `L0503` on a literal elsewhere. `eq`, `hash`, `ord`, `partial-eq` and `partial-ord` hold for `Cow<'static, str>` in Rust, so their bare `str` rows cover it. `copy` is decision 9.
- **Facts the type stage records.** The checker does not choose a representation. It records, per text expression, whether it creates owned text (any `rust` form result written `(own str)`: `display-string`, which a one-part `str` expands to, the `str` macro's `rust` form, `upper-case`, `lower-case`, `pr-str`, `slurp`, `read-line`, `line-seq`, `command-line-args`; and `parse` to text, whose `from-str` row is `(own str)` only), is a literal, is a view a form returns, or refers to a slot (a field read, a `ref`, a template's `(ref a)` over a text slot).

### The ownership stage: one decision

A **text slot** is each place a text value lives and the solver decides: a parameter, a function or closure result, a local, a struct or enum field, and each text position inside a type argument of one of those, such as the element of a `(vec str)` local. A struct field has one slot for the whole program. A generic function's type variable has one slot per call, because Rust instantiates the variable once per call, so a call of `(defn wrap (x) (some x))` makes the argument and the result's payload one slot. A closure parameter that a higher-order form fixes, as `filter` fixes its test's parameter to `(ref a)` over the iterator's item, is not a slot: it takes the representation the template gives it, lent over owned elements and a view over view elements.

Each slot is a view or owned. The rules:

| Slot | Representation |
|---|---|
| Parameter | A view, unless the body moves it into an owned slot or returns it as an owned result. This is the existing mode inference: `Ref` is a view, `Own` is owned. Under decision 19(b), a parameter that the body passes to a `(ref a)` position whose `a` is owned at that call is lent, `&String`. A caller passing owned text never makes a parameter that the body only reads owned; a parameter that the body stores follows its own row below. |
| Result | Owned when every returned value is owned; mixed when returned values are owned and `'static` text, as `(if c (str "x") "y")` is; a view when no returned value is owned, a lent value meeting the result as a view by Rust's coercion at the return. When any returned value is owned and another is a non-`'static` view or a lent value, the result is owned and that value gets `L0503` with `(str ...)`, as for a field: `(defn pick2 (c s) (if c (str "x" 1) s))` reports `L0503` at `s`, not today's `L0510` at the owned branch. |
| Field | Decision 1(d), with decision 8: `&'static str` when every stored value is `'static` text; owned when every stored value is owned; mixed when both kinds are stored; owned when nothing stores into it. A written `(ref str)` field is a view with a lifetime. Every text position inside a field's type, such as the element of a `:names (vec str)` field or the payload of `(option str)`, follows this row, not the Element row, so no struct gets a lifetime from it: `(->team :names (collect (split line " ")))` stores non-`'static` views into an owned element, which is `L0503` with a help that makes owned text, `(collect (map (split line " ") str))`. |
| Element or payload | Owned when every stored value is owned, mixed when owned and `'static` values are stored, as `(push! names "a") (push! names (str "b"))` does. A view otherwise, lent values included, and every lent value stored into a view element or payload takes the `.as_str()` view node where it is stored, wherever it stands: `vec![a.name.as_str(), b.name.as_str()]`, `(a.name.as_str(), 1)` in a pair, `Some(a.name.as_str())`, `map.insert(1, a.name.as_str())`. The rewrite then needs no model of where rustc carries an expected type into a nested value, and the emitter needs no rule that writes a binding's type: rustc accepts `let mut v = vec![a.name.as_str()]; v.push("x");` untyped. "Payload" here means the payload of `option`, `result` and other generic types; a user enum's payload follows the Field row, so enums get no `'a` by default either. |
| Element of a collection parameter | Decision 15, decided (d): Rust cannot make `&[String]` meet `&[&str]`, so no free view exists between the two. Owned when the body needs owned elements; otherwise the one representation every caller passes; when callers disagree, generic over `AsRef<str>` if the body only reads elements, and otherwise each owned caller gets the new ownership code. |
| Text position of a function-typed parameter | The callee decides each parameter position by what it passes, a view unless it passes owned text, as it decides what flows into any slot; a caller's closure that needs to own its argument gets `L0503` in its own body, where it stores the value. Each result position is decided by the callers' functions, by decision 15's rule, because the callee consumes it. No `AsRef` analogue exists between `impl Fn(&str)` and `impl Fn(String)`, so no generic is offered. |
| Element of a collection result | Follows the body, as a result does: `(map (iter users) :name)` over an owned field yields lent items, `impl Iterator<Item = &String>`, which a caller meets through decision 13. |
| Parameter whose body stores it | A stored parameter takes the representation of what its callers pass, as the owner's decision 8 names parameter slots: `&'static str` when every caller passes `'static` text, owned when every caller passes owned text, mixed when callers pass both. A caller that passes a view that is not `'static`, or a lent value, gets `L0503` with `(str ...)`, because neither fits `String` or `Cow<'static, str>` without an allocation. This is decision 1(f), reopened by decision 8. |
| Local | Its stores decide as for a field: owned when every stored value, by its initializer or a `set!`, is owned; mixed when owned and `'static` text are stored; lent or lent mixed when its initializer is such a reference and nothing else is stored; a view otherwise. A non-`'static` view stored beside owned text is `L0503`, as for a field. |
| Written `(ref str)`, `(own str)` or `(cow str)` | As written (decision 17). |

Starting every decided slot as a view, owned spreads forward from creation sites through flows, and backward only from a slot where the fact O holds (below) to the parameters that flow into it, including a call's type-variable slot: O reaches the `a` of a call from another argument that shares `a` by unification and from a slot with O that the call's result flows into. A slot's representation is never a source: a field with no facts reads as owned, but it passes nothing backward, so the solve depends only on facts. A slot with no facts passes on nothing, neither S nor N; a slot that receives owned and `'static` text becomes mixed, and a mixed value makes mixed every owned slot it flows into, except a slot that is owned by a need of its own (the `mut` row and decision 18). **Three monotone facts.** The solver keeps three facts per slot, each of which only ever turns from false to true: *owned reaches it* (O: an owned or mixed value flows in), *`'static` reaches it* (S: a `'static` view flows in), and *non-`'static` reaches it* (N: a view that is not `'static`, a lent value or a lent mixed value flows in). A read of a field or element is non-`'static` once O holds for its slot, because the read is then a reference to owned or mixed text. The representation is read from the three facts by one table; a value whose kind the representation cannot hold gets `L0503`, and its help is `(clone x)` for a lent mixed value and `(str x)` for any other:

| O | S | N | Field, stored parameter | Local, element, result, read-only parameter |
|---|---|---|---|---|
| no | no | no | owned (nothing is stored) | a view |
| no | yes | no | `&'static str` | a `'static` view |
| no | any | yes | owned, `L0503` at each N value | a view with a region |
| yes | no | no | owned | owned |
| yes | yes | no | mixed | mixed |
| yes | no | yes | owned, `L0503` at each N value | owned, `L0503` at each N value |
| yes | yes | yes | mixed, `L0503` at each N value | mixed, `L0503` at each N value |

**A tied `(ref a)` parameter.** This table covers only a parameter that the body does not store. A stored parameter takes its own row ("Parameter whose body stores it"), flows into the call's `a` as any slot does, and is passed as `&name`: with `(defn keep (name) (println (tagged name (str "x"))) (->user :name name))`, owned callers give `fn keep(name: String)` with `tagged(&name, "x".to_string())`, callers of both kinds give `name: Cow<'static, str>` with `tagged(&name, Cow::Owned("x".to_string()))`, and literal callers give `name: &'static str`, whose S meets the O of `(str "x")` at `a`, so `a` is mixed and the call is `tagged(&Cow::Borrowed(name), Cow::Owned("x".to_string()))`; rustc accepts all three. When a function passes a parameter it does not store at a `(ref a)` position whose `a` another argument ties by unification, as `label` passes `name` to `tagged`, the parameter's callers add facts to that call's `a` through the parameter: a caller passing mixed, lent mixed or `'static` text adds S, a caller passing a non-`'static` view adds N, and a caller passing owned or lent text adds a fourth monotone fact, *a place is lent here* (L), because `&o` and `&u.name` lend a place rather than hand over a value. The parameter's representation is read from the facts of `a`:

| O at `a` | L | S | N | Parameter | Errors |
|---|---|---|---|---|---|
| no | any | any | any | a view, `name: &str`, passed as `&name` so that `a` is `&str` | `L0503` at the argument when the call needs a trait whose rows exclude a view (decision 19(c)) |
| yes | yes | any | any | lent, `&String` (19(b)) | `L0503` at each S and each N caller, help `(str x)`; a mixed caller adds S, so it is among them |
| yes | no | yes | any | lent mixed, `&Cow<'static, str>` (19(d)) | `L0503` at each N caller, help `(str x)` |
| yes | no | no | any | lent, `&String` | `L0503` at each N caller, help `(str x)` |

L only grows, like the other facts, and the errors are read off the final facts. The S and N that callers add to `a` flow on wherever `a` flows, such as into the slot that receives `clone`'s result, like any fact; L does not, because it records a loan at the call, not a value. An N already reported at a caller is not reported again where it flows: with `(defn mk (s) (->user :name (clone s)))` beside `(->user :name (str "z"))`, a literal caller makes `a` mixed and `:name` mixed, and `fn mk(s: &Cow<'static, str>) -> User { User { name: s.clone() } }` compiles, while a caller `(mk (trim t))` gets one `L0503`, at the caller. A read-only parameter that is not passed at a tied `(ref a)` never has O or S reach it from its callers, because callers do not decide it; a field written `(ref str)` keeps its view and its region; a slot owned by a `mut` need follows decision 18. With all three kinds, as in `(make "ana")`, `(make (str "u" 1))`, `(make (:name a))`, the field and `n` are mixed, `"ana"` enters as `Cow::Borrowed`, `(str "u" 1)` as `Cow::Owned`, and `(:name a)`, a lent mixed read, gets `L0503` with `(clone (:name a))`, whose result is mixed and fits. Because the three facts only grow, the solve reaches one least fixed point whatever order it visits slots in, and errors are read off the final facts, not reported during the solve. Owned and `'static` text both fit `Cow<'static, str>`, so the solution is the least fixed point: rule 2 of the issue, borrow by default, where borrowing is free. It runs inside `ModeInference`'s worklist (`src/ownership/modes.rs:23-36`), which already re-queues callers when a callee's modes change.

Each flow then means one thing:

| Value | Slot | Meaning |
|---|---|---|
| owned | owned | a move, tracked by `MoveChecker` as today |
| owned | view | a borrow of the value's place, `&owned`, which the loan rule tracks |
| lent | view | a free view of the referent (below) |
| view | view | a copy of the reference |
| literal or other `'static` text | owned | the slot becomes mixed, and the literal goes in as `Cow::Borrowed("a")`, with no allocation (decision 8) |
| owned | mixed | `Cow::Owned(x)`, a move with no copy |
| `'static` view | mixed | `Cow::Borrowed(x)` |
| mixed | view | a deref: `&c` at a concrete coercion site, `c.as_ref()` where Rust does not coerce |
| mixed | owned | the owned slot becomes mixed, by the order above; a mixed value never becomes `String` without a written `(str ...)` |
| mixed, or a literal | owned by a need (`mut`) | decision 18: `L0503` at the value, help `(str x)` |
| view that is not `'static`, or lent | mixed | `L0503`, naming the value: `Cow<'static, str>` cannot hold it; help `(str x)` |
| lent mixed | mixed | `L0503` with the help `(clone x)`, which stays mixed and allocates only when the value is owned |
| view | lent (a `(ref a)` position whose `a` is owned at that call) | decision 19 |
| owned | lent | `&o`, a borrow of its place |
| mixed, or `'static` text | lent mixed (a `(ref a)` whose `a` is mixed at that call, 19(d)) | `&m`, or `&Cow::Borrowed("y")` for a literal |
| view that is not `'static`, or lent | owned | `L0503`, naming the value. The compiler never writes `.to_string()` for it. |
| any | `mut` | An in-place change, a `push!` onto text or a `set!` of part of it, needs `&mut String`, so it makes the slot owned. A `set!` that replaces the whole value needs only a mutable place of the slot's own representation, so it owns nothing: `(defn reset! (s) (set! s "b"))` emits `fn reset(s: &mut &'static str) { *s = "b" }` when its callers hold `'static` views, and `*s = Cow::Borrowed("b")` through `&mut Cow<'static, str>` when they hold mixed text, both free; a written `(mut (ref str))` stays `&mut &str`, as today. A view, a lent value, a literal or a mixed value reaching a slot made owned this way is `L0503` (decision 18), whose label names the change ("`push!` changes `s` in place, which needs owned text") and whose help writes the owned value, `(local s (str "..."))`. A slot made owned by a `mut` use is never mixed: changing a `Cow::Borrowed` in place copies it, through `Cow::to_mut`, an allocation the source does not write. A `mut` argument that is not a place keeps today's `L0419` or `L0514`. |

**A free view.** `&String` to `&str` is a pointer projection, not a clone or an allocation. Rust does it by itself at a coercion site whose type is concrete: an argument of a concrete parameter, a `let` with a type, a struct field, a return. Where Rust does not, the rewrite records a view node, and the emitter writes the view's template, which the prelude states (decision 16):

- `.as_str()` on a closure result or a `match` scrutinee, as today;
- `.as_str()` at an argument of a generic parameter whose bound text meets only through a view row, such as a user `(impl label str ...)`, which is `impl Label for str` (below). Without it rustc infers the parameter as `String` and reports `E0277`, `` `String: Label` is not satisfied ``, as review 2's probe shows for `show(&owned)`;
- `.as_str()` or `.as_ref()` on every side of a comparison group that is not a view, under "Shared parameters", when the group's trait has no implementor row that pairs the two representations. So `(< o (:name a))` with `o` owned and `:name` mixed emits `o.as_str() < a.name.as_ref()`: viewing one side only is `E0308`, and viewing neither is `E0277`. `partial-eq` has such rows today, `(with string str)`, `(with str string)`, `(with string (ref str))` and `(with (ref str) string)` (`prelude/number.knv:38-41`), so `(= "a" (:name u))` needs no node; `partial-ord` and `ord` have none, so `(< "b" (:name u))` emits `"b" < u.name.as_str()`, and `(max "b" (:name u))` passes `u.name.as_str()`. Without it rustc reports `E0308`, because Rust has `PartialEq<str> for String` and no `PartialOrd<str> for String`. The rows decide, so no trait is named in the compiler. Today these programs are `L0401`.
- `.as_str()` or `.as_ref()` on every text argument at a view position that is not a receiver, unless the argument is already a view (decision 20). A view position is a `(ref str)` position of a `rust` form, the text query of a template the checker builds (the map query of `get` and `set-at!`, `infer_map_lookup` and `infer_set_at_map`, `src/types/infer.rs:3004-3038,3196-3228`), and the argument of a clause chosen for text, such as `slurp`'s. It is not a `(ref a)` position whose variable another argument ties by unification (a trait relation, as between the map query and its key through `borrow`, is not a tie), as `tagged`'s first parameter is tied to its second: a node there would break the tie. A receiver is the template's receiver as `TemplateBorrows::receives` (`src/types/templates.rs:92`) already records it, so the exemption reads a fact the compiler holds, not template text. `subs`'s template `&{0}[{1}..{2}]` has no receiver, so a mixed or owned argument emits `&o.as_str()[1..2]`, which compiles but reads less plainly than today's `&o[1..2]`. Some templates put such an argument where Rust's parameter is generic: `get`'s key, `Q` in `K: Borrow<Q>`, and `split`'s separator. A `HashMap<&str, _>` cannot be read by `&String`, and a `HashMap<String, _>` cannot be read by `&Cow<str>`: each is `E0277`. `split`'s separator is a generic `p` bounded by the rust-trait `pattern` (`crate::rt::Separator`, `prelude/text.knv:213-229`), whose rows are `(ref str)`, `(ref (ref str))`, `(ref string)`, `char` and `(fn (char) bool)`, with one clause per row and no row or impl for `Cow`. PR 4 folds `split`'s `(ref str)`, `(ref (ref str))` and `(ref string)` clauses into one clause written bare `str`, and `pattern`'s three text rows into one bare `str` row, as it does for `slurp` ("Clauses keyed on text"): after the erasure every text separator has the type `str`, so the three clauses would match the same calls, and one clause keeps the clause chooser and the erasure in agreement. A text separator is therefore the argument of a clause chosen for text: one that is not a view takes the view node, so the clause receives a `&str`, and `Separator` needs no `Cow` impl. Decision 20 is unchanged: `split` moved from a `(ref str)` position to a clause chosen for text, both of which decision 20 lists. With the node, `line.split(a.name.as_ref())`, `m.get(o.as_str())` and `m.get(a.name.as_ref())` compile against owned, view and mixed keys. A receiver, `{0}` in `{0}.trim()`, needs no node, because Rust's method call derefs it.
- `.map(String::as_str)` through an `option`, a `result`'s success side, or an `iterator` (decision 13). So `(unwrap-or (first names) "none")` on owned names emits `names.first().map(String::as_str).unwrap_or("none")`, where today's program writes `(map-some (first names) (fn (n) (as-str n)))` and gets `.map(|n: &String| n.as_str())`. Inside any other type, such as a lent element meeting a view element of a `vec` that is already built, no free view exists: that is an error with its own ownership code (decision 6), whose help binds the value to a local of its own type.
- `.as_str()` on every lent value stored into a view element or payload, and the `iterator` view before a `collect` into view elements (decision 13). Rust would coerce some of these by itself, at a typed `let` that reaches through `vec!`, a pair or `some`, but not through a borrowed vector literal, a generic `rust` argument or a map literal's `insert`, and today's emitter writes no binding type. Writing the view on every stored lent value removes that dependence: `loudest(&vec![a.name.as_str(), b.name.as_str()])`, `count_pairs(&vec![(a.name.as_str(), 1)])`, `count_opts(&vec![Some(a.name.as_str())])`, `vec![(a.name.as_str(), 1_i64)].into_iter().collect::<HashMap<_, _>>()`, `map.insert(1, a.name.as_str())`, and `.map(|u: &User| &u.name).map(String::as_str).collect::<Vec<_>>()`, each of which rustc accepts, where the same Rust without the views is `E0308`. It is also the Rust a person writes.

**Text inside a compound map query.** A query such as `(get m (pair "a" 1))` against `(pair str i64)` keys has no view node, because Rust's `Borrow` for a tuple is the identity. A lookup reads the key's representation and adds nothing to the key's facts. Each text slot inside a compound query takes the representation of the key's matching slot: against owned keys the literal is `L0503` with `(str "a")`, emitting `m.get(&("a".to_string(), 1_i64))`; against mixed keys it goes in free, `m.get(&(Cow::Borrowed("a"), 1_i64))`; against view keys it stays `m.get(&("a", 1_i64))`. Today this shape passes `check` and reaches rustc as `E0308`. PR 2 adds a fixture for each.

**Clauses keyed on text.** A `rust` form's clause, such as `slurp`'s `(str "std::fs::read_to_string({0})")` and `(string "std::fs::read_to_string({0})")` (`prelude/io.knv:22-23`), selects a template by its argument's type. A `rust` form with clauses chooses by the first type parameter of its `where`, which may bind several, and a clause may drop the template's borrow for a shared-reference argument (`L0247`). PR 4 respells the two into one `(str ...)` clause, `contains?`'s `str` and `string` clauses (`prelude/sequence.knv:30-31`) the same way, and `split`'s three text clauses (`prelude/text.knv:225-227`) into one `(str ...)` clause. A clause written bare `str` matches text in every representation, as a bare `str` implementor row does, and its argument is a view position under decision 20, so the template always receives a `&str`: `std::fs::read_to_string(c.path.as_ref())` for a mixed path and `std::fs::read_to_string(l.as_str())` for a lent one. Without the node, `std::fs::read_to_string(&c.path)` is `E0277`, because `Cow<'_, str>` is not `AsRef<Path>`. The `Source` impl for `Cow` is reached only when no clause matches, through a generic `source` caller.

**Who reads the cross rows.** PR 4 respells the four `partial-eq` cross rows as `(with (own str) (ref str))` and `(with (ref str) (own str))`, and adds the pairs Rust gives `Cow`: `(with (cow str) (ref str))`, `(with (cow str) (own str))` and their mirrors, since rustc accepts `a.name == o`, `o == a.name` and `"a" == a.name`. `partial-ord` and `ord` have no cross row, so every non-view side of their comparisons is viewed. The rows answer one question, whether owned text and a view compare without a view node, and only `TextRewrite` asks it. How many dereferences each side of a comparison gets is decided by the shared-parameter rule over the rewritten types, without the rows. So `**w == *"a"` uses Rust's `PartialEq<str> for String`, which Rust provides beside `PartialEq<&str> for String`; the respelled rows state the fact the rewrite reads, not each Rust impl the emitted comparison uses. The type stage checks `partial-eq` on text through the plain `str` row, since both sides are `str` to it.

**Implementor lists that name `(ref str)` beside text.** `partial-eq` (`number.knv:34-35`) and `partial-ord` (`:75-76`) list `str string` and then `(ref str)`; `ord` (`:260-261`) lists `string` and `(ref str)` with no bare `str`. After this RFC a view is text, so each list keeps one bare `str` row, which covers text in every representation; the `(ref str)` row folds into it, because Rust implements each of these traits for `String`, for `str`, so for `&str`, and for `Cow<'static, str>`. The prelude's own runtime traits are written out per type, so a bare `str` row covers mixed text only once the runtime implements them for `Cow`: `crate::rt::Sequence` (`prelude/runtime/sequence.rs:5,11`, row `types.knv:37-38`), `crate::rt::Show` (`show.rs:140,146`, used by `prn`, `print-form` and `pr-str`), `crate::rt::Source` (`source.rs:5,11`, row `io.knv:16`), `crate::rt::Contains` (`contains.rs:5,11`, rows `sequence.knv:15-22`) and `crate::rt::StartsWith` (`starts_with.rs:5,11`, rows `text.knv:243-245`) each gain an `impl` for `std::borrow::Cow<'_, str>` in PR 2, which forwards to the `str` impl. `pattern` needs none, because its text argument takes the view node (above), and `joinable` (`crate::rt::Join`) is generic over `Borrow<str>` in Rust, so it already takes `Cow`. Without them, `size(&a.name)` for `(defn size (x) (len x))` and `crate::rt::Shown(&name)` on a mixed value are `E0277`; with them rustc accepts both, and `Shown` over a `Vec<Cow<'static, str>>`. The type stage accepts `(sort! names)` on `str` elements through `ord`'s new bare `str` row, and the rewritten Rust sorts a `Vec<String>` or a `Vec<&str>`, both of which rustc accepts. `eq` (`number.knv:54-55`) and `hash` (`number.knv:64-65`) have the same `str string` and `(ref str)` shape and fold the same way.

**`borrow`.** `borrow` (`prelude/map.knv:19-25`) lists `(ref str)` and `string` with no bare `str`, plus `(with string str)` and `(with (ref str) str)`, where the second `str` is Rust's unsized `str`, the target of `Borrow<str>`. It becomes one bare `str` row and one `(with str str)` row. In the second position of a `with` row, as in every position of an implementor row, a bare `str` means text; for `borrow` it states that a text key is looked up by a view of text, which Rust gives as `Borrow<str>` for `String`, `&str` and `Cow<'_, str>`, and only for a `&str` argument. Decision 20's node makes every key argument that is not already a view into one, so `(get m k)` always passes a `&str`: `(get m "a")` emits `m.get("a")`, and `(get m (:name u))` on view keys emits `m.get(u.name.as_str())`, where today's `m.get(&u.name)` would be `E0277` against a `HashMap<&str, _>`.

A view or a lent value is a reference, so the move checker copies it, as it copies `(ref str)` today. An owned one moves, as `string` does today.

### The mixed representation

Decision 8 adds it. A slot is mixed exactly when it stores `'static` text and owned text, so under decision 1(d) a field that stores only literals stays `&'static str` and a field that stores only created text stays `String`, and neither pays for a tag.

**Where it appears.** In any decided slot that receives both: a field, a collection element or payload, a parameter whose body stores it (callers pass both), a local, and a result. A read-only parameter stays a view, because a mixed argument meets it by a deref.

```lisp
(defstruct user :name str :age i64)
(defn make (n) (->user :name n :age 1))
(defn pick (c) (if c (str "x") "y"))
(defn main ()
  (let (a (make "ana")
        b (make (str "u" 1))
        names (vec))
    (push! names "a")
    (push! names (str "b"))
    (println (shout (:name a)) (< "b" (:name a)) (pick true))))
```

```rust
struct User {
    name: std::borrow::Cow<'static, str>,
    age: i64,
}
fn make(n: std::borrow::Cow<'static, str>) -> User {
    User { name: n, age: 1 }
}
fn pick(c: bool) -> std::borrow::Cow<'static, str> {
    if c { std::borrow::Cow::Owned("x".to_string()) } else { std::borrow::Cow::Borrowed("y") }
}
// in main:
let a = make(std::borrow::Cow::Borrowed("ana"));
let b = make(std::borrow::Cow::Owned(format!("u{}", 1_i64)));
let mut names = Vec::new();
names.push(std::borrow::Cow::Borrowed("a"));
names.push(std::borrow::Cow::Owned("b".to_string()));
println!("{}{}{}", shout(&a.name), "b" < a.name.as_ref(), pick(true))
```

rustc accepts each of these shapes, together with `a.name == "ana"`, `names.first().map(|n: &Cow<'static, str>| n.as_ref()).unwrap_or("none")`, `get("k")` on a `HashMap<Cow<'static, str>, i64>`, `sort()` on a `Vec<Cow<'static, str>>`, and `show(a.name.as_ref())` over `impl Label for str`.

**How it reads.** The rewrite records a wrap node on each value that enters a mixed slot, `Borrowed` for `'static` text and `Owned` for owned text, and the emitter writes the wrap's template from the prelude rows of decision 16: `std::borrow::Cow::Borrowed({0})` and `std::borrow::Cow::Owned({0})`. `.into()` reads shorter, and rustc accepts it where the target type is concrete, such as a struct field, but in an untyped `vec!` or `let` its target comes from a later use, so it would bring back the dependence on rustc's inference that review 6's n1 removed for views; the explicit constructors need no such model. A read derefs: `&a.name` at a concrete coercion site, method calls by autoderef, and `.as_ref()` where Rust does not coerce, which the view rows state for a mixed value as they state `.as_str()` for a lent one.

**Cost at run time.** `Cow<'static, str>` is 24 bytes, the size of `String` (measured with `std::mem::size_of` on this machine's rustc), so a mixed field costs no space over an owned one. Each read checks the tag, a branch that the source does not write: this is the cost that rejected option D for every text value, accepted here only in slots that mix, where the alternatives were an allocation per literal (RFC 0070's `String::from`) or an error asking for `(str ...)`. A literal into a mixed slot allocates nothing, and created text moves in without a copy.

**What it does not cover.** A view that is not `'static`, such as a parameter that only reads or a `trim` of a local, and a lent value, cannot enter a mixed slot: `Cow<'static, str>` holds only `'static` borrows. Each is `L0503` with `(str ...)`, as before. A `Cow<'a, str>` with a region would take them, and is left out: it gives the slot a lifetime, which decision 1(d) keeps off structs.

### How the decision reaches the emitter

A `TextRewrite`, beside `ModeRewrite`, writes each decision into the typed IR: `Primitive(String)` for owned, `Ref(_, Primitive(Str))` for a view, `Ref(_, Primitive(String))` for lent, `Ref(_, Primitive(CowStr))` for lent mixed, and, for mixed, a third internal primitive, `PrimitiveType::CowStr`, beside the internal `String` variant, whose Rust name is `std::borrow::Cow<'static, str>` and which no program names except through `(cow str)` (decision 17). The first three are types today's later stages already know; `CowStr` is new to them, and each stage handles it as an owned value that holds no borrow. It rewrites struct and enum field types, parameter and result types, local and expression types, puts back the reference layer the type stage erased, copies a view out of a reference to a view field, as a Copy value of 16 bytes or less already copies out of a borrow, sets the argument `Borrow::Shared` at each owned-to-view meet, and places the view nodes above.

After the rewrite:

- `MoveChecker`, `LoanFacts`, `ReturnedBorrows` and the emitter see the types they handle today, plus `CowStr`, which moves as `String` does and holds no loan. None of them decides a representation.
- `TypeDisplay` prints `Primitive(String)`, `Primitive(CowStr)`, `Primitive(Str)` and a reference to any of them as `str` (`src/types/display.rs:52,55-57`), so an ownership or emit diagnostic never shows a representation the user did not write.
- A debug assertion in the emitter checks that no text type it writes is still an undecided `str`.

Every row of the Problem section's table goes away, with three exceptions: `lent_temporaries.rs` (section 3); the emitter's lifetime deciders, which RFC 0065's step 3 moves; and, in row 1, the slice arm of `Unification::coerces` (`unification.rs:56`), which lets `(ref (vec a))` meet `(ref (slice a))` through `ViewCoercion::viewed_items` and has nothing to do with text. Only the disjunct `ViewCoercion::new(&shared, &owned).views_string() ||` at `unification.rs:49` goes, with `views_string` (`view_coercion.rs:15-23`); the arm `(Type::Ref(_, shared), Type::Ref(_, owned))` at `unification.rs:48-51` stays, because `(ref (vec i64))` against `(ref (slice i64))` enters it and recurses at `unification.rs:50` into the slice arm at `unification.rs:56`, the path `(window (ref xs))` takes.

**Every reader of `is_unsized` for text.** Making `str` sized to the checker changes each place that reads `Type::is_unsized` (`ty.rs:82-84`) or `DeclaredTypes::is_unsized` (`declared_types.rs:41-46`) for text. Each keeps its slice half, and for text:

| Site | Today, for `str` | After |
|---|---|---|
| `type_declaration_collector.rs:276-284` `borrow_views`, `checker.rs:937-947` `borrow_unsized_result` | a bare `str` field or result becomes `(ref str)` | no wrap: the field or result is a slot |
| `ownership/functions.rs:78-85` `fixed_mode`, `rewrite.rs:36-43` (`:38`), `modes.rs:67,80` | a `str` parameter is always `Ref` | mode inference decides, as for `string` today |
| `infer.rs:2082-2084` `layers_above_held_view` | holds a view where a signature stores the type parameter | the stored slot's representation, from the solver |
| `infer.rs:2388-2391` `borrowed_view`, `:2451-2457` `pass_value` | passes a `str` argument as a view | the flow table's owned-to-view borrow, set by `TextRewrite` |
| `infer/shared_variables.rs:149,549-553` `UNSIZED_VALUE_DEPTH` | depth allowance for a `str` beside a reference | the shared-parameter rule over rewritten types, plus the comparison view node |
| `ownership/slices.rs:146` `Shrink::Str`, `:199-210`, `:344-351` `accepts_str` | an inferred `string` parameter shrinks to `&str` | the solver's view parameter; `Shrink::Str` goes |
| `emit/result_borrows.rs:96` | an unsized result carries a borrow | after the rewrite a view is `Ref(Str)`, a reference, so no bare `Str` reaches it; a debug assertion says so |
| `emit/unsized_parameters.rs:85` | `?Sized` for a generic over `str` | unchanged: a generic over Rust's `str` still needs it. `emit/types.rs:349` and `emit/emitted_types.rs:41` read the set `UnsizedParameters` computes, so this row covers them |
| `infer.rs:1489` | `infer`'s text is `Type::str_ref()` | `str`, a `'static` view |
| `emit/expressions.rs:978-1002` `literal_shape`, and the guard at `:988-996` | a literal pattern on a `String` scrutinee becomes a guard `== "lit"` | the scrutinee's `.as_str()` view node lets the literal match directly; the guard path goes |
| `implementation_check.rs:66` (`L0444`) | gated on `is_unsized` | keys on an implementor written bare `str` |

### Why the ownership stage

Three facts that the representation depends on, or that depend on it, are decided there already: a parameter's mode, which is its representation; whether a value moves or copies; and, after RFC 0065's step 3, every region. Deciding the representation in the type stage would need the modes before they exist, which is why PR 598 had to make parameters generic or default them to `&str`. Deciding it in the emitter would break RFC 0065. The type stage keeps what it can know alone: that a value is text, and which values are created, literal, views or references.

### Closures and impls on text

- A closure's own parameters and result are slots like a function's, unless a higher-order form fixes the parameter (above). A captured text keeps its slot: a `Fn` closure that reads it borrows it, and an `FnOnce` closure that moves it moves an owned value, by today's closure-kind rules.
- **Function and method values.** `(map (iter names) shout)` over owned names, with `shout`'s parameter a view, meets `&String` items with a `&str` parameter, and `.map(shout)` is `E0631` in rustc. Today's "Function values" rule wraps a named function in a closure when a parameter needs an adjustment, and today `coerced_view` counts the text view as no adjustment (`values.rs:534-542`). `TextRewrite` counts a lent-to-view meet as an adjustment, so the value emits `.map(|argument: &String| shout(argument))`, where the call site coerces. A method value, which emits a bare path such as `Shape::area`, is wrapped by the same rule.
- **Trait methods.** Each text position of a trait method's declared type is one slot, shared by the trait item, its default body and every impl, because Rust gives the trait item one signature: owned when any of those bodies needs it owned. This is RFC 0065's "a trait's record is the meet of its default body and every impl", applied to text.
- `(impl trait str ...)` implements the trait for Rust's `str`, with `self` as a view. A method call reaches it from owned and lent text through Rust's autoderef, and a generic argument through the `.as_str()` view node above. `(impl trait (own str) ...)` implements it for `String`. A program that writes both for one trait gets `L0459`, `` `str` already implements `label` ``, the error for two impls of one trait on one type: both are `str` to the type stage, and which body ran would depend on a representation the source does not show.
- Impls on text exist today: `tests/fixtures/run/prelude_impls.knv:7` and `emit/prelude_impls.knv:7` write `(impl label string ...)`; `tests/fixtures/check/unsized_implementor.knv:3` writes `(impl consumed str ...)`; `tests/fixtures/check/second_impl_called.knv:12,14` writes `(impl consumed str ...)` twice and `unsized_implementor_borrowing_self.knv:4` writes `(impl viewed str ...)`; `tests/fixtures/format/layout_methods_impl.knv:12` writes `(impl mailbox (vec string) ...)` and `:20` `(impl sized string ...)`, which its snapshot repeats; `tests/cli.rs:10177` writes `(impl loud string ...)` and `:10175` declares `(deftrait loud (loud-text (fn ((ref self)) string)))`; `L0444.md:22,36` implement for `str`, and `:48` for `string`, with its prose at `:42`; the unit tests `src/types/implementor_listing.rs:130` and `src/types/missing_trait_explanation.rs:158` write `(impl shown (vec string) ...)` and `(impl shown string ...)`; and `docs/language.md:1434` says an `impl` names "a primitive such as `int` or `bool`, `string`". The respelling turns each `string` implementor into `(own str)`, not bare `str`, so the impl's Rust type stays `String`.
- **`L0444`** reports a by-value `self` method in an impl on `str` or a slice, through `is_unsized()` (`src/types/implementation_check.rs:66`). Once `str` is sized to the checker, it keys on an implementor written bare `str`, which is Rust's unsized `str`, and its help and page name `(impl trait (own str) ...)` in place of `string`. **`L0445`**, a `rust-trait` impl on a type that is not a struct or enum, does not change: `str` is still a primitive.

### Examples under the recommendation

```lisp
(defstruct user :name str :age i64)
(defn make (n) (->user :name n :age 1))
(defn load () (make (str "u" 1)))
```

```rust
struct User { name: String, age: i64 }
fn make(n: String) -> User { User { name: n, age: 1 } }
fn load() -> User { make(format!("u{}", 1_i64)) }
```

The parameter is stored, and its only caller passes created text, so `n` and `:name` are owned, which is today's output for a `string` field. A second caller `(make "Ana")` makes both mixed under decision 8: `make(std::borrow::Cow::Borrowed("Ana"))` and `make(std::borrow::Cow::Owned(format!("u{}", 1_i64)))`, with no allocation for the literal.

```lisp
(defn tag (u names)
  (let (n (:name u))
    (push! names n)))
```

with `names` a vector of owned text elsewhere:

```text
error[L0503]: `n` is a reference to `u`'s text, but the elements of `names` own their text
3 |     (push! names n)))
  |                  ^ this needs owned text
  = help: make owned text with `(str n)`
```

```lisp
(defn first-word (line) (unwrap-or (find (split line " ") (fn (w) (> (len w) 0))) ""))
```

```rust
fn first_word(line: &str) -> &str {
    line.split(" ").find(|w: &&str| w.len() > 0).unwrap_or("")
}
```

This is today's output, with no `(ref str)` in the source.

## 2. The issue's open questions

Each is a decision for the owner, gathered in section 7. This section gives the evidence.

### Struct fields (decision 1)

A field's slot is decided from every value stored in it, across the program. Today a bare `str` field is always a view and gives its struct a lifetime: built with `"Ana"`, `(defstruct user :name str :age i64)` emits `struct User<'a> { name: &'a str, age: i64 }` and derives `Copy`. A struct that holds such a struct, and every function that returns either, then carries `'a`: the lifetime spreads, as the issue's `Rogue<'a>` shows.

- (a) **A view unless an owned value is stored.** A struct built only from literals or views gets `'a`, as today. A constructor helper that stores its parameter makes the field a view and the parameter `&str`: `(make (str "u" 1))` from section 1 then emits `make(format!("u{}", 1_i64).as_str())` inside `load`, which returns the struct, and rustc reports `E0515`. Today's binary emits exactly that Rust for `(make (as-str (str "u" 1)))` and `knave check` exits 0; the checks that catch it are RFC 0065's steps 3 and 4. Under (a) the common way to build a struct from created text falls into that class.
- (b) **Always owned.** No struct gets a lifetime from text. A literal stored in a field follows decision 8.
- (c) **`&'static str` for literals, owned for created text, `&'a str` for any other view.** It has (a)'s `E0515` class for a stored parameter, and its `'a` is not what a Rust programmer writes for a constructor, who writes `name: String`.
- (d) **Owned unless every stored value is `'static` text.** `'static` text is any text the solver proves `'static`: a literal, a `def` of one, a read of a `def` of created text, which today emits `static MADE: std::sync::LazyLock<String>` and reads as `MADE.as_str()`, a `&'static str`, a read of a field that is itself `&'static str`, an `if`, `match` or block whose every branch is `'static` text, a call whose result is `'static` (below), and a stored parameter whose callers pass only `'static` text (1(f)). A call's result is `'static` in two ways. When every value the function returns is `'static` whatever its arguments, the result is `'static` at every call, and the rewrite writes `&'static str` on it: `fn pick_name(u: &User) -> &'static str`. When the function returns a view that borrows only from its parameters, as `(defn or-none (s) (if (empty? s) "none" s))` does, the result is `'static` at a call whose every reference argument is `'static`, and the signature stays elided. That rule is sound whichever parameters the emitter ties, so the ownership stage needs no tie fact from the later stage that decides ties until RFC 0065's step 3; step 3 can refine it to the borrowed arguments alone. The signature stays `fn or_none(s: &str) -> &str`: `Cow::Borrowed(or_none("a"))` compiles, because rustc gives the elided lifetime `'static` at that call. This is the per-call rule that a generic type variable already follows (section 1). 1(f) applies only to a parameter that the body stores; a parameter that the body only reads stays `&str` whatever its callers pass, and callers never change it. Owned whenever any stored value is anything else, a parameter included, which then gets mode `Own` by the rule other types follow when stored. `&'a str` only where the user writes `(ref str)`. **Recommended.** No struct gets a lifetime the user did not write; the decision is local to the struct's stores; it closes the `E0515` class above without steps 3 and 4; and it reproduces today's Rust for every field that a program writes `string` today. A literal-only struct, the common case in tests and tables of names, emits `struct User { name: &'static str, age: i64 }`.
- (e) **Generalize** (PR 598): the field becomes a type parameter. Rejected under option A.
- (f) **As (d), with `'static` flowing through parameters**, as a literal's pin passes through unannotated functions (`docs/language.md: Literals` states it for number literals, and RFC 0070 `:373` extends it to string literals): a parameter that every call feeds `'static` text is itself `'static`, so `(make "ana")` and `(make "bo")` keep `&'static str` and allocate nothing. It lets callers change a callee's signature, which no other text rule does, though number literal pins do, and it makes `knave test`, whose tests may pass created text, emit a different Rust type from `knave run` more often.

**The cost of (d), and why it is the owner's call.** This program compiles today with no allocation:

```lisp
(defstruct user :name str :age i64)
(defn make (n) (->user :name n :age 1))
(defn main ()
  (let (us (vec (make "ana") (make "bo")))
    (println (len us))))
```

```rust
struct User<'a> { name: &'a str, age: i64 }
fn make(n: &str) -> User<'_> { User { name: n, age: 1 } }
let us = vec![make("ana"), make("bo")];
```

Under (d) as the owner decided it, read with decision 8's parameter slots, `n` is a stored parameter whose every caller passes a literal, so `n` is `&'static str` and so is `:name`: `fn make(n: &'static str) -> User` and `struct User { name: &'static str, age: i64 }`, with no allocation and no lifetime parameter. This is option (f): decision 8 names parameter slots among those that can be mixed, and a parameter can only store both kinds when its callers pass both, so the callers decide a stored parameter. Decision 1 is reopened on that point alone (section 7). A caller that passes created text makes `n` owned, and callers of both kinds make it mixed. 77 snapshots hold a `&'a str` struct field, counted on `a6545d9a2` (`git grep -lP '^\s+(pub )?\w+: &.a str,' -- '*.snap'`); PR 1's `--print-text` reports how many become `&'static str`, owned or mixed.

**The homepage sample.** `content/sample.knv` in the knave-lang.org repository stores only literals into its `rogue` struct, so under 1(d) its text fields are `&'static str` and `Rogue` loses its `'a`: the lifetime spread that the issue's first open question names does not arise there.

**Two edges.** A field that nothing stores into, because the struct is only declared or built in another program, is owned: no value of it exists, so the choice costs nothing, and it gives the struct no lifetime. A field is decided over the program one build compiles, so `knave test`, whose tests may store created text, can emit `String` for a field that `knave run` emits as `&'static str`. Both compile and print the same text; `knave emit` shows the run build.

`knave doc` and `types` print the field as `str` under every choice.

### Function signatures, `infer`, diagnostics and `knave doc` (decisions 2 and 3)

A parameter is a view unless its body needs it owned, the rule modes already follow, and a result is owned when it returns owned text.

- **What prints (decision 2).** (a) `str` everywhere: `infer`, `knave types`, `knave doc`, the REPL and every diagnostic. **Recommended.** It is rule 1, it matches how modes print today ("a parameter's passing mode does not show", `docs/language.md: Type texts`), and an ownership diagnostic says "owned", "a view" or "a reference to `u`'s text" in words where it matters. (b) `str` for owned and `(ref str)` for a view. It shows more, and puts the second spelling back in front of the user. (c) As (a), with a later `knave types` column for every parameter's passing, for every type.
- **What a bare `str` means in a `type` signature (decision 3).** (a) It states no representation, as where nothing is written, and `(ref str)`, `(own str)` or `(cow str)` pin one. **Recommended.** An `infer` text then pastes into a `type` signature and keeps its meaning, which RFC 0054 promises. (b) Owned, by the written-types rule: pasting `(fn (str) str)` from `infer` turns a view parameter into an owned one, and every caller that passed a view then fails.

**The exceptions, counted.** Today `str` carries one exception: written bare, it is always borrowed (`docs/language.md: Ownership`, "Unsized values"). Under 3(a) and 4(a) it carries two: a bare `str` in a `type` signature or a binder annotation is not owned, an exception to "A type written without `ref` or `mut`, in a `type` signature or a binder annotation, means owned, as in Rust" (`docs/language.md:321`), and `own` and `cow` are type forms that apply to `str` alone. The meaning of a bare `str` is one: no representation stated. A `rust` form, which must state one, rejects it, and an implementor row, which states a trait for text, covers both.

### The written owned form (decision 4)

Rule 5 of the issue asks for an explicit way to request ownership. A value-level hatch exists (decision 7), but a `type` signature, a field and a `rust` form also need to state owned text: the prelude's `upper-case` returns `String`, and a user may want a parameter owned for an API reason.

- (a) **`(own str)`, a kernel type form.** **Recommended.** One form serves the prelude's `rust` forms, the implementor rows of `from-str` and `clone`, and a user who pins a parameter or a field. The prelude cannot express it: no prelude form binds a type to a representation of `str` that unifies with `str`, for the reason RFC 0074 gives for aliases (`(rust-type ...)` makes a new nominal type). `docs/language.md: Kernel forms` records that. `(own t)` on any other type is an error, because every other written type is already owned.
- (b) **A prelude-internal owned-text type** that only the prelude can write, as PR 598's `owned-text` (`prelude/internal/text.knv`, matched by its template text `String` at its `lowering.rs:383`). Users get no type-level way to request ownership, and the compiler recognizes a binding by its template text.
- (c) **None:** a bare `str` in a `rust` form means owned. That is a second meaning of bare `str`, which "Few rules, no exceptions" weighs against, and users still get no type-level form.

### `as-str` and the `the` special case (decision 5)

Both lose their reason to exist: owned text meets a view slot by the rewrite's `Borrow::Shared`, lent text by a free view, and `(ref str)` is text. The pull requests remove:

- the `as-str` definition, export and re-export (`prelude/text.knv:28,256-258`, `prelude/prelude.knv:43,69`);
- `StringViewSearch` and its threading (`expand/src/expand/string_view.rs`, `mod.rs:95,158,171,296,306-312,405-418`, `prelude_state.rs:186,565`, `saved_definitions.rs:182-185,566`, `checker.rs:145,212,1750`, `deferred_definitions.rs:28,159`, `graph.rs:618`);
- the insertions in `src/types/string_view.rs` and its module line (`src/types/mod.rs:132`); `ViewCoercion::views_string` (`view_coercion.rs:15-23`) and its tests, its disjunct at `unification.rs:49` with the unification tests at `:209,217`, the text half of `coerced_view`, and the emitter's `depth_allowance` (`src/emit/expressions.rs:2012-2020`). `ViewCoercion` itself, its module and export (`src/types/mod.rs:181,253`), `viewed_items` and the slice arm at `unification.rs:56,71-72`, `declared_types.rs:48-49,95` and the slice half of `coerced_view` stay: on `5234c8817`, `(window (ref xs))` against a `(ref (slice i64))` parameter emits `window(&xs)` through them;
- the `(as-str place)` help (`infer.rs:2197,2236-2249`, whose text is at `src/types/infer.rs:2248`);
- `self.string_view` passed to `DefinitionSaver::new` (`expand/src/expand/mod.rs:316`).

`the` has no code of its own (`prelude/syntax.knv:678-680`): its special case is the view coercion, so it goes with it. `(the str x)` checks that `x` is text and pins no representation. `(the (ref str) x)` pins a view and `(the (own str) x)` pins owned text, by the binder annotation `the` expands to, so a value that cannot take the pinned representation gets the flow table's error. Its docstring example at `syntax.knv:832-834` goes, and the page shows those three.

After removal, `string` and `as-str` are unbound names, `L0201`.

They get the closest-name suggestion that every unbound name gets, and nothing else: no `foreign-names` entry, help, note, alias or hint for `string`, `String` or `as-str`. The owner's standing rule forbids any migration aid, because the owner is the only user. Whether Rust's `String` should ever be a foreign name for `str` is a question for RFC 0070's own process, which adds a row only with evidence from writers of that language, not for code that this RFC changes.

### The diagnostic for a value that needs ownership (decision 6)

The case is a view or a lent value flowing into an owned slot. `L0503` (the constant at `expand/src/diagnostic/code.rs:216`) already means a use that needs to own a value it can only borrow, as its page `L0503.md:3-4` states, and `OwnershipNeeds::check_with_parameters` (`src/ownership/checks.rs:79-85`) reports it for an argument passed through a borrow into an owning parameter. The text case is the same fact.

- (a) **`L0503`, with a text wording.** **Recommended.** The message names the value as the source writes it and says what it is ("a view", "a reference to `u`'s text"). A secondary label points at the value that made the slot owned, and the help writes `(str value)` with the value spelled out, which is RFC 0070 decision 4's help for a variable. The `L0503` page gains a section on text. `L0510`'s help, which suggests the owned field type `string` through `owned_view` (`src/ownership/returned.rs:145-160`), suggests `(own str)` instead.
- (b) A new code for text.

A lent value inside a type that has no free view (section 1) gets its own ownership code, the next free one when the pull request lands (`L0517` is on main, `RETURNED_CLOSURE_USES_CHANGED_PARAMETER` at `code.rs:227`, and RFC 0065 reserves `L0518` and `L0519`, so today that is `L0520`), because no `str` call fixes it: the help binds the value to a local, `(local first (first names))`, and uses that.

### The value-level escape hatch (decision 7)

- (a) **`(str x)`.** **Recommended.** It exists, it is the form the issue names as a creation site, and with one part that is not a literal it already emits `x.to_string()` (`docs/language.md: Standard types`). No new name.
- (b) A new `own` value form, `(own x)`, emitting `x.to_owned()`. On text it does what `(str x)` does, and on other types what `clone` does: a second name for an existing concept, which Names rule 5 forbids.
- (c) `clone` of text gives owned text. After the respelling, `clone`'s rows name `(own str)` and `(cow str)`, so `(clone (:name u))` gives owned text when the field is owned and mixed text when it is mixed. For a view, `.clone()` in Rust gives a view, so `clone` would need a template that changes with the representation.

### A literal in an owned slot, and RFC 0070 (decision 8)

**Decided by the owner: a new option (d), the mixed representation.** A field, element, parameter, local or result slot that stores both `'static` text and created text is `Cow<'static, str>`: a literal goes in as `Cow::Borrowed` with no allocation and no written `(str ...)`, created text goes in as `Cow::Owned` with no copy, and a read derefs to `&str`. Under decision 1(d), a slot that stores only `'static` text stays `&'static str` and a slot that stores only created text stays `String`. The owner wants literals to pass freely wherever the type is `str`, with no hidden allocation. Section 1, "The mixed representation", gives the rules, the Rust and the cost.

The options weighed before it, kept for the record:

- (a) `L0503` with `(str "a")`. It kept rules 3 and 4 of issue #589, at an error for every `if`, `match`, `cond`, `set!`, `push!` and collection that mixes a literal with created text.
- (b) The slot stays a view and each created value is lifted into a hidden local. It compiles only while the slot's value lives inside that local's scope.
- (c) The literal emits `String::from("a")`, by RFC 0070 decision 4: an allocation per literal that the source does not write.
- (d) **`Cow<'static, str>` in a slot that mixes.** **Decided.** Literals pass freely, nothing allocates that the source does not write, and the cost is a tag check on each read of a mixed slot, in those slots only.

**The Problem section's programs under each option.**

| Program | Today | C with 8(a) | C with 8(c) | C with 8(d), decided |
|---|---|---|---|---|
| `(shout (ref owned))`, `(trim (str " a" 1))` | `L0401` | accepted | accepted | accepted |
| `(if c (str "x") "y")` | `L0401` | `L0503` at `"y"`, help `(str "y")` | accepted, `String::from("y")` | accepted, a mixed result: `Cow::Owned("x".to_string())` and `Cow::Borrowed("y")` |
| `(push! names "a") (push! names (str "b"))` | `L0401` | `L0503` at `"a"`, help `(str "a")` | accepted, `String::from("a")` | accepted, `Vec<Cow<'static, str>>` |
| `(->user :name (clone "Ana"))`, `:name` written `str` | `L0401`, `L0404` | `L0503` at `"Ana"`: `clone`'s row is `(own str)`, so it needs owned text and a literal is a view; help `(str "Ana")` | accepted, `String::from("Ana")`, then cloned: `(clone "Ana")` allocates twice, so the help on 8(c) still names `(str "Ana")` | `L0503` at `"Ana"` as under 8(a): `clone` needs owned or mixed text; `(->user :name "Ana")` with no `clone` is accepted |

Under 8(d), option C accepts every program of the Problem section except `(clone "Ana")`, which asks `clone` for owned or mixed text and gets a view, and allocates only where the source writes `str`, `clone` or an allocating form.

**What changes in RFC 0070.** Its `mismatch-help` entry for `string` and `(ref str)` and its `L0401` for a `(ref str)` variable where a `string` is expected go under every option: no `L0401` compares those two types any more, and the case is `L0503`, whose wording carries the `(str name)` help. The `mismatch-help` form itself still ships, because RFC 0075's integer row needs it (RFC 0075, section "The help on a mismatch, stated in the prelude", whose row is `(mismatch-help int usize ...)`, `rfcs/0075-collections-speak-int.md:531`). RFC 0070's status line records the change as earlier RFCs record a partial supersession. Under the decided (d): "Accepted. Decision 4's string literal rule, its `mismatch-help` entry for `string` and `(ref str)`, and its `L0401` for a `(ref str)` variable where a `string` is expected, are superseded by 0078." A literal where owned text is wanted no longer emits `String::from`: its slot becomes mixed and the literal goes in as `Cow::Borrowed`.

### A text collection passed to a function (decision 15)

```lisp
(defn loudest (names) (upper-case (unwrap-or (first names) "")))
```

`loudest` can be called with `(vec (str "a"))`, whose elements are owned, and with `(collect (split line " "))`, whose elements are views. Rust cannot make `&Vec<String>` meet `&[&str]`, and building one from the other allocates a vector, so the element slot of `names` has no representation that both callers meet for free. Today both calls fail at the type stage, `` expected `(vec str)`, found `(vec (ref str))` `` and `` found `(vec string)` ``; with one `str` type the program looks valid, so users will write it.

- (a) **The body decides, a view by default.** A caller with owned elements gets an error whose help collects views, `(collect (map (iter names) (fn (n) n)))`, which allocates a vector the user then writes.
- (b) **The callers decide**, as number literal pins already let a call decide a callee's parameter type (`docs/language.md: Literals`, "a helper called once with a `usize` keeps `usize`"). **Recommended.** Owned when the body needs owned elements, a `mut` collection whose body pushes created text included; otherwise the representation every caller passes; a view when no caller exists. Callers that disagree leave the slot at its default, a view, as the literal pins' rule leaves a disagreeing literal open and lets its default decide (`docs/language.md:187`: "Two uses that want different types leave the literal open, and the default then fails at the first use that disagrees"). Each caller that passes owned elements then gets a new ownership code at its call, with its page, whose help collects views, `(collect (map (iter names) (fn (n) n)))`, a vector the user writes. The result does not depend on the order of calls. **How it runs in the fixed point.** A collection parameter's element slot flows back into its callers' element slots only when the body needs owned elements: then the callee's slot is owned by the body, each caller's vector element becomes owned by the backward flow that a stored parameter already follows, and a caller that stored a literal or a view gets `L0503` at that value, not the new code. Otherwise nothing flows back: when every caller agrees the slot takes their representation, and when they disagree it is a view and each owned caller gets the new code once, at the call, with no `L0503` for it. The solve runs in the ownership stage's worklist, and no visiting order enters the result. The Rust is concrete in every case: `fn loudest(names: &[String])` or `fn loudest(names: &[&str])`, and a map, `sort!` and `push!` parameter compile as they do for any concrete type: rustc accepts `m.get("a")` on a `HashMap<String, i64>` and on a `HashMap<&str, i64>`, `names.sort()` and `names.push("none")` on a `Vec<&str>`.
- (c) **Generic over `AsRef<str>`** whenever the body does not need owned elements: `fn loudest<S: AsRef<str>>(names: &[S]) -> String`, with `.as_ref()` at each text use of an element. It compiles for reading elements (rustc accepts `loudest` over `&Vec<String>` and `&Vec<&str>`), and fails for the rest: review 4's probes show `m.get("a")` on `HashMap<S, i64>` (`E0599`, it needs `S: Borrow<str> + Hash + Eq`), `names.sort()` (`E0277`, `S: Ord`) and `names.push("none")` (`E0308`, no `S` builds from a literal). Making it sound needs a bound rule (bounds read from the prelude rows of every trait the body uses on an element, with `Borrow<str>` from the `borrow` rows for a key), a rule that a `mut` collection parameter is never generic, a new IR fact for the generic parameter and its bounds, written by the ownership stage, and the `AsRef` name stated in the prelude. None of that is designed here.
- (d) **(b), and (c) when callers disagree and the body only reads elements.** It avoids (b)'s error for the read-only case at the cost of (c)'s new IR fact and bound rule, limited to `AsRef<str>`. **Decided by the owner.** The generic needs only `AsRef<str>` when the body only reads elements, which is the case (c) can serve: `fn loudest<S: AsRef<str>>(names: &[S]) -> String`, with `.as_ref()` at each text use of an element, compiles over `Vec<String>`, `Vec<&str>` and `Vec<Cow<'static, str>>`. The generic parameter and its bound are an IR fact the ownership stage writes beside the representation, and the bound's name, `AsRef<str>`, is stated in the prelude as a view row, not in the compiler. When callers disagree and the body writes elements, sorts them or keys a map with them, each owned caller gets the new ownership code, as under (b).

**A note on `HasField`.** The owner plans a `HasField` accessor soon: with `(defstruct animal :name str)` and `(defn hello (named) (str "Hello " (:name named)))`, `hello` reads `:name` from any struct that has it. A `HasField` accessor for a text field returns a `&str` view, whatever each struct stores: `fn name(&self) -> &str` reads `self.name` for a `&'static str` field and `&self.name` for a `String` or `Cow<'static, str>` field, and `fn hello<T: HasName>(named: &T) -> String` compiles over all three. It is the same idea as 15(d), a read through a trait that views text, so the two agree when `HasField` lands: a field's representation stays decided by its own stores (decision 1), and a generic reader never needs it.

A result that holds text in a collection follows its body (section 1), so no caller changes it.

**With the mixed representation.** A caller's vector that mixes literals and created text has mixed elements, `Vec<Cow<'static, str>>`. For decision 15(b) a mixed caller counts as an owned one: when every caller passes mixed elements the parameter is `&[Cow<'static, str>]`; when mixed callers meet view callers, the slot is a view and each mixed caller gets the new code, as an owned caller does. A caller with only owned elements beside a mixed caller does not disagree: both fit `Cow<'static, str>`, so the slot is mixed and the owned caller's element slot becomes mixed by the backward flow, each element wrapped in `Cow::Owned` with no copy. Only a view caller disagrees with a mixed one.

### Where the view's Rust is stated (decision 16)

Today the one view a program can ask for is in the prelude: `as-str` is `(rust "{0}.as_str()" ...)` (`prelude/text.knv:256-258`). The emitter writes a second one itself, `.as_str()` for a `match` (`src/emit/expressions.rs:566-572,680-704`), which the Problem table lists as one of the scattered deciders. This design asks for views at a closure result, a `match` scrutinee, a generic argument, one side of a comparison, and through `option`, `result` and `iterator`. `AGENTS.md`: "Before you add a feature to the compiler, write the feature as a prelude macro or a prelude function", and "NEVER restate in Rust a name or a fact that the prelude states". Neither option below costs the user anything, so that principle decides.

- (a) **The prelude states each view's template, in a new kernel declaration form.** **Recommended.** A form in `prelude/text.knv`, in the shape RFC 0070 gives `mismatch-help` (which is not built: `git grep mismatch-help -- prelude src expand docs` finds nothing, and this form does not depend on it), holds one row per lent type: the lent type, its view, and the template, so `(ref (own str))` views as `(ref str)` by `"{0}.as_str()"`, and `(option (ref (own str)))`, `(result (ref (own str)) e)` and `(iterator (ref (own str)))` view by `"{0}.map(String::as_str)"`. A row `(option (ref (ref str)))` to `(option (ref str))` by `"{0}.copied()"`, with the same for `result` and `iterator`, states the view of a reference to a view, so `(unwrap-or (first names) "none")` over view elements emits `names.first().copied().unwrap_or("none")`. Decision 8 adds three rows: `(ref (cow str))` views as `(ref str)` by `"{0}.as_ref()"`, and the two wraps, `'static` text into `(cow str)` by `"std::borrow::Cow::Borrowed({0})"` and `(own str)` into `(cow str)` by `"std::borrow::Cow::Owned({0})"`, with the `option`, `result` and `iterator` views of a mixed value by `"{0}.map(|text| text.as_ref())"`. Each row carries a scheme as well as its template, such as `(fn ((ref (own str))) (ref str))` for `.as_str()`, so `template_ties.rs` reads the node's elided tie from it and the loan rule knows that the view borrows its argument; PR 2 adds a `loans` fixture in which a stored view of a field is live across a move of the field's owner. The prelude state reads the rows when `knave` is built, so a program pays nothing to find them. `TextRewrite` places a view node by matching the lent type against the rows, and the emitter writes the row's template, so the Rust of every view, the `match` one included, lives in the prelude, and the emitter's own `.as_str()` path goes. It is not a signature search: the form names its rows, so `StringViewSearch` still goes. A lent type with no row has no free view, which is decision 6's error. The form is a kernel form, as `rust-trait` is (`kernel.rs:56,178`), so `docs/language.md: Kernel forms` records why the prelude cannot express it: the prelude can write each template as a `rust` definition, but the compiler must learn which template views which type, and with ordinary definitions it could learn that only by knowing their names, which "NEVER restate in Rust a name or a fact that the prelude states" forbids, or by searching signatures, which is what `StringViewSearch` does and this RFC removes. A kernel form's head is the compiler's own name, and its rows carry the facts.
- (a2) **Prelude-internal `rust` definitions**, `(def text-view (rust "{0}.as_str()" (fn ((ref (own str))) (ref str))))` and one each for `option`, `result` and `iterator`, which `TextRewrite` inserts by a binding the kernel names, as PR 598 kept `text-view`. No new form, but the compiler names four prelude bindings, a prelude fact restated in Rust, and the next view needs a compiler change, where under (a) it needs a row.
- (b) **The emitter writes them**, and `docs/language.md: Kernel forms` records why the prelude cannot. Nothing here shows that it cannot.

**Decided by the owner: (a).** The view rows live in the prelude, through the new kernel declaration form in `prelude/text.knv`.

**One table, three readers.** The same rows serve three uses, so the facts about text's representations are stated once:

- **The compiler.** `TextRewrite` places a view or wrap node by matching a type against the rows, and the emitter writes the row's Rust template, as section 1 states.
- **The interpreter, under RFC 0063's decision 1A.** The interpreter holds one runtime form for text, so owned, view, mixed and lent text are the same value to it. Each row therefore carries a native clause whose meaning is the identity: where the compiled program writes `.as_str()`, `Cow::Borrowed(...)` or `.map(String::as_str)`, the interpreter passes the value on unchanged. A row without that clause would leave the interpreter no meaning for the node.
- **The dynamic-library boundary of RFC 0064.** When an interpreted program calls a compiled library, the boundary must hand each text argument over as the Rust type the library expects, `&str`, `String` or `Cow<'static, str>`, and take each result back. It reads the same rows to choose the conversion, so a library's text crosses by the rule the compiler uses, not by a second table.

Adding a representation or a view then means adding a row, and all three readers learn it together.

### A written name for the mixed representation (decision 17)

Decision 16's rows must name the mixed representation to state its templates, and the rewrite must name it in `TextRewrite`'s output. `(ref str)` names a view and `(own str)` owned text.

- (a) **`(cow str)`, a kernel type form valid around `str` only, as `(own str)` is.** **Recommended.** The prelude's rows name it, a user may pin a slot mixed with it, as `(own str)` pins one owned, and decision 3's rule is unchanged: a bare `str` states no representation. `docs/language.md: Kernel forms` records it beside `(own str)`.
- (b) A prelude-internal name that the rows use and no program can write. Users then cannot pin a mixed slot, and the kernel knows one name that the prelude states, which `AGENTS.md` forbids.
- (c) No name: the rows state only the lent and owned templates, and the emitter writes the `Cow` Rust itself, which decision 16 moved out of the emitter.

### A parameter that needs a reference to owned text (decision 19)

Today mode inference gives an inferred `string` parameter `Ref` mode with an owned type, `&String`, in two cases: when the body clones it, `(defn dup (s) (clone s))` emitting `fn dup(s: &String) -> String { s.clone() }`, and when the body passes it to a `(ref a)` position whose `a` is owned at that call, as `label` does in `tests/fixtures/emit/view_refused_for_held_generic.knv`, passing `name` to `(fn ((ref a) a) string)` beside `(str "x")`, which emits `fn label(name: &String)`. The emit snapshots on `a6545d9a2` hold 19 `&String` parameters in 8 snapshots; most come from comparisons, which a view serves.

- (a) **A clone of a view makes owned text, by `{0}.to_string()`.** No mechanism exists for it today. `clone` is a `rust-trait` whose rows are types only (`ownership.knv:10-12`) and a `def` whose one template is `{0}.clone()` and whose signature `(fn ((ref a)) a)` gives a view argument a view result (`:30-32`). A clause keyed on a view could switch the Rust text, as `slurp`'s clauses do, but no clause can state that its result is `(own str)`, so the solver would not learn that `(clone v)` creates owned text unless the compiler knew `clone`'s name, which "NEVER restate in Rust a name or a fact that the prelude states" forbids. A result type on a clause would be new kernel work, recorded in `docs/language.md: Kernel forms`. It would also make `(clone v)` and `(str v)` two names for one Rust, `v.to_string()`, against Names rule 5 (`docs/language.md:1152`), answered only by `clone`'s docstring, "Returns a copy of a value that owns its own data" (`ownership.knv:35`).
- (b) **The solver chooses a lent parameter** when the body passes it to a `(ref a)` position whose `a` is owned at that call. `label` keeps `fn label(name: &String)`, and a caller that passes a view gets `L0503` with `(str x)`.
- (c) **`L0503` in the body**, with the help `(str s)`. `(defn dup (s) (clone s))` reports `L0503` at `s` in `clone`'s argument: `s` is a view, and `clone` holds for owned and mixed text only. The fix the help writes, `(str s)`, emits `fn dup(s: &str) -> String { s.to_string() }`.
- (d) **As (b), with a lent mixed parameter when S reaches `a` and L does not.** An owned or lent caller cannot pass a `&Cow` without moving or copying its value (`label(&o)` against `&Cow<'_, str>` is `E0308`), so any such caller keeps the parameter lent under (b). The `a` at the call is then mixed, so `label(name: &Cow<'static, str>)` calls `tagged(name, Cow::Owned(...))`, and a literal caller passes `&Cow::Borrowed("y")` with no allocation, which decision 8 favors. A caller passing a non-`'static` view still gets `L0503`.

**Recommended: one rule, the tied `(ref a)` table of section 1, whose rows are (b), (d) and (c).** A parameter passed at a `(ref a)` takes its representation from the facts of the call's `a`: O from another argument tied to `a` by unification or from a slot with O that the call's result flows into, and S, N and L from the parameter's own callers. The solver reads only facts and the trait's rows, never a function's name. The outcomes the table gives:

- `(defn label (name) (tagged name (str "x")))`: `(str "x")` gives `a` O. With the fixture's caller `(label (str "y"))`, an owned temporary that is lent, `a` has L: `fn label(name: &String)`, as today, and owned and lent callers `label(&o)` and `label(&u.name)` compile. With only literal, mixed and lent mixed callers, `a` has S and no L: `fn label(name: &Cow<'static, str>)`, called as `label(&Cow::Borrowed("y"))` and `label(&a.name)`. With `(label o)` and `(label "y")`, `a` has L and S: lent, and `L0503` at `"y"`, help `(str "y")`.
- `(defn label (name) (tagged name "x"))`: nothing gives `a` O, so the parameter is a view, `fn label(name: &str)`, and the call passes `tagged(&name, "x")` with `a` as `&str`, which compiles for `label("y")` and for a non-`'static` caller, `label(t.trim())`. Today this program passes `check` and emits `tagged(name, "x")`, which rustc rejects with `E0277` and `E0308`.
- `(defstruct user :name str) (defn mk (s) (->user :name (clone s)))`: `clone`'s result flows into `:name`, and `:name`'s only store is that result, so `:name` has no facts and passes nothing back. `clone`'s `a` has no O, so it is a view, and `clone`'s rows exclude a view: `L0503` at `s`, help `(str s)`, whose fix `(->user :name (str s))` emits `fn mk(s: &str) -> User { User { name: s.to_string() } }`, the same one allocation as today's `fn mk(s: &String) -> User { User { name: s.clone() } }`.
- `(defn dup (s) (clone s))`: `clone`'s result is `dup`'s result, which has no facts, so `a` is a view: `L0503` at `s`, help `(str s)`, which emits `fn dup(s: &str) -> String { s.to_string() }`.

PR 1's `--print-text` reports every `&String` parameter, so the owner sees the count before PR 2.

### A text argument at a position that Rust makes generic (decision 20)

A `rust` form's template can pass a text argument where Rust's parameter is generic: `get`'s key is `Q` in `K: Borrow<Q>`, and `split`'s separator, the argument of a clause chosen for text, is generic over `crate::rt::Separator`. Rust's coercion stops at a generic parameter, so `&String` and `&Cow<str>` reach it unconverted, and only some of them satisfy the bound.

- (a) **A view node on every text argument at a view position, except a receiver, unless the argument is already a view.** A view position is a `(ref str)` position of a `rust` form, the text query of a template the checker builds (the map query of `get` and `set-at!`), and the argument of a clause chosen for text (`slurp`), but not a `(ref a)` whose variable another argument ties, where "ties" means that two positions share one type variable by unification, not by a trait relation such as the map query's `borrow` bound. **Recommended.** The compiler needs no model of which templates are generic in Rust, and every such call passes a `&str`, which satisfies `Separator` and `Borrow<str>` for every key type. The cost is a few `.as_str()` calls where a coercion would have served, such as `read(o.as_str())` beside `read(&o)` for a concrete parameter; a receiver keeps `o.trim()`.
- (b) **A `rust` form's signature marks a position as generic in Rust**, and the node is placed only there. The Rust is shorter, but each template author must know which positions Rust makes generic, and a missed mark reaches rustc.

### Every position against every representation

Each cell is the Rust the model emits for a text value of that representation at that position, and every one compiles: the first rows in one rustc program (`cells.rs`), the rows for a global, `mut`, `match`, payloads, closure parameters, function values, clause argument, map query, `subs`, tied `(ref a)` in a second (`cells2.rs`), those for `mk`, the tied `(ref a)` over owned, lent and literal callers, compound keys, whole-value replacement through `mut`, `.copied()` on a view payload in a third (`r10.rs`), those for `mk` and `dup` after their `(str s)` fix, `label` as `&String` over owned, lent and owned-temporary callers, as `&Cow` over literal and mixed callers, and the view `label` passing `tagged(&name, "x")` in a fourth (`r11.rs`), those for `keep`'s three outcomes and `mk` over a mixed field in a fifth (`r12.rs`), and decision 15(d)'s generic `loudest` over owned, view and mixed vectors with the `HasField` sketch in a sixth (`d15.rs`). A cell marked "no" is a value that the rules reject before emission, with the Knave diagnostic named.

| Position | Owned `String` | View `&str` | Mixed `Cow<'static, str>` | Lent `&String` | Lent mixed `&Cow<'static, str>` |
|---|---|---|---|---|---|
| Local | `let o = format!("o{}", 1);` | `let v = "v";`, `let t = o.trim();` | `let mut m = Cow::Borrowed("m"); m = Cow::Owned(...);` | `let l = &u.name;` | `let lm = &a.name;` |
| Field | `struct F { name: String }` | `&'static str` when only `'static` text is stored; `&'a str` only where written `(ref str)` | `name: Cow<'static, str>` | no: a lent value stored is non-`'static`, so the field is owned and the value gets `L0503` with `(str x)` | no: as lent, help `(clone x)` |
| Element | `vec![o.clone(), "b".to_string()]` (each written) | `vec!["a", t]` | `vec![Cow::Borrowed("a"), Cow::Owned(o)]` | `vec![u.name.as_str(), "x"]`: the element is a view, with the node | `vec![a.name.as_ref(), "x"]` |
| Parameter by value (stored) | `fn store(s: String)`, all callers owned | `fn store(s: &'static str)`, all callers `'static` (1(f)) | `fn store(s: Cow<'static, str>)`, callers of both kinds | no: `L0503` at the caller's value, `(str x)` | no: `L0503`, `(clone x)` |
| Parameter by reference (read only, `s: &str`) | `read(&o)` | `read(v)` | `read(&m)` | `read(l)`, `read(&u.name)` | `read(lm)`, `read(&a.name)` |
| Clause argument chosen for text (`split`) or Rust-generic argument (`get`) | `line.split(o.as_str())`, `keys.get(o.as_str())` | `line.split(v)`, `keys.get(v)` | `line.split(m.as_ref())`, `keys.get(m.as_ref())` | `line.split(l.as_str())` | `line.split(lm.as_ref())` |
| Receiver of a method template | `o.trim()` | `v.trim()` | `m.trim()` | `l.trim()` | `lm.trim()` |
| Comparison | `o == "a"`; `o.as_str() < m.as_ref()` | `"b" < v` | `m == "a"`, `m == o`; `"b" < m.as_ref()` | `*l == "a"`; `l.as_str() < "b"` | `**lm == *"a"`; `lm.as_ref() < o.as_str()` |
| Result | `fn r() -> String` | `fn r(s: &str) -> &str`; `fn r() -> &'static str`; `or_none("a")` is `'static` at the call | `fn r(c: bool) -> Cow<'static, str>` | `fn r(u: &Owned) -> &str { &u.name }`, a view by coercion | `fn r(a: &Mixed) -> &str { &a.name }` |
| Map key | `HashMap<String, _>`, `insert(o.clone(), 1)` | `HashMap<&str, _>`, `insert("a", 1)` | `HashMap<Cow<'static, str>, _>`, `insert(Cow::Borrowed("a"), 1)` | stored into view keys: `insert(l.as_str(), 2)` | `insert(lm.as_ref(), 3)` |
| Map lookup by each key kind | `get(o.as_str())` against owned, view and mixed keys | `get(v)` against all three | `get(m.as_ref())` against all three | `get(l.as_str())` against all three | `get(lm.as_ref())` against all three |
| Global `def` of created text | `static MADE: LazyLock<String>`, read as `MADE.as_str()`, a `'static` view | `static` of a literal | `Cow::Borrowed(MADE.as_str())` into a mixed field | n/a | n/a |
| `mut` position (`&mut String`) | `grow(&mut w)` | no: `L0503` (decision 18) | no: `L0503` (decision 18) | no: `L0503` | no: `L0503` |
| `match` scrutinee | `match o.as_str()` | `match v` | `match m.as_ref()` | `match l.as_str()` | `match lm.as_ref()` |
| `option`, `result`, `iterator` payload, viewed | `first().map(String::as_str)`, `Result::map(String::as_str)` | `first().copied()` | `first().map(\|text\| text.as_ref())`, `iter().map(\|text\| text.as_ref())` | as owned | as mixed |
| Closure parameter a higher-order form fixes | `\|w: &&String\| **w == *"a"` | `\|w: &&&str\| ***w == *"a"` | `\|w: &&Cow<'static, str>\| **w == *"a"` | the same as owned | the same as mixed |
| Function value | `.map(\|argument: &String\| shout(argument))` | `.map(\|argument: &&str\| shout(argument))` | `.map(\|argument: &Cow<'static, str>\| shout(argument))` | as owned | as mixed |
| Clause argument (`slurp`) | `read_to_string(o.as_str())` | `read_to_string(v)` | `read_to_string(m.as_ref())` | `read_to_string(l.as_str())` | `read_to_string(lm.as_ref())` |
| Checker-built map query (`get`, `set-at!`) | `get(o.as_str())`, `*get_mut(o.as_str()).unwrap() = 9` | `get(v)`, `get_mut(v)` | `get(m.as_ref())`, `get_mut(m.as_ref())` | `get(l.as_str())` | `get(lm.as_ref())`; each against owned, view and mixed keys |
| `subs`, a non-receiver template argument | `&o.as_str()[0..1]` | `&v[0..1]` | `&m.as_ref()[0..1]` | `&l.as_str()[0..1]` | `&lm.as_ref()[0..1]` |
| Parameter at a tied `(ref a)` (no node) | caller `label(&o)`, L, with `fn label(name: &String)` (19(b)) | with no O at `a`: `fn label(name: &str)`, `tagged(&name, "x")`; with O: a `'static` caller adds S, giving `&Cow` when no L, `L0503` with `(str x)` when L; a non-`'static` caller is `L0503` | `fn label(name: &Cow<'static, str>)`, `label(&m)`, S without L (19(d)) | `fn label(name: &String)`, `label(&u.name)`, L (19(b)) | `label(&a.name)` with `&Cow` (a mixed caller adds S); with L present the parameter is `&String`, and this caller is `L0503`, `(str x)` |
| Text inside a compound map query | `get(&("a".to_string(), 1_i64))` after `L0503` asks for `(str "a")` | `get(&("a", 1_i64))` | `get(&(Cow::Borrowed("a"), 1_i64))` | against view keys `get(&(l.as_str(), 1_i64))`; against owned keys `L0503`, `(str x)`; against mixed keys `L0503`, `(str x)` | against view keys `get(&(lm.as_ref(), 1_i64))`; against mixed keys `L0503`, `(clone x)`; against owned keys `L0503`, `(str x)` |
| Whole-value `set!` through `mut` | `fn reset(s: &mut String)`, when the caller's slot is owned by an in-place change elsewhere (decision 18): a literal there is `L0503`, `(str "b")`; otherwise the literal makes the caller's slot mixed | `fn reset(s: &mut &'static str) { *s = "b" }` | `fn reset(s: &mut Cow<'static, str>) { *s = Cow::Borrowed("b") }` | n/a | n/a |
| Closure capture | `\|\| o.len()`, `move \|\| o2.len()` | `\|\| v.len()` | `\|\| m.len()` | `\|\| l.len()` | `\|\| lm.len()` |

Without the nodes these cells name, rustc rejects these shapes: `line.split(&a.name)` and `m.get(&a.name)` against owned keys are `E0277`, `view_keys.get(&u.name)` is `E0277`, `a.name < o` is `E0277` and `o.as_str() < a.name` is `E0308`.

### A mixed value or a literal where a change needs owned text (decision 18)

A `push!` onto text or a `set!` of its contents needs `&mut String`, so the slot is owned by that need. A mixed value or a literal can reach it: `(let (s (pick c)) (push! s "!"))`, where `pick` returns mixed text, and `(let (s "a") (push! s "b"))`.

- (a) **`L0503` at the value, with the help `(str x)`.** **Recommended.** Nothing allocates that the source does not write, which is decision 8's own condition, and the label names the change that needs owned text.
- (b) **`Cow::to_mut`.** The slot stays mixed, and the change emits `s.to_mut().push_str("!")`, which copies a borrowed value once, at its first change. A Rust programmer writes this, and it is the same allocation that hand-written code makes with `String::from` before `push_str`; but the source writes no allocation, so it departs from issue #589's rule 4 as RFC 0070's `String::from` did.

### Auto-borrow at read-only parameters, issue #590 (decision 14)

Issue #590 asks that a `(ref a)` parameter given an owned value borrow it, with moves and mutable borrows unchanged, and no clone or conversion inserted. This RFC designs it together with issue #589.

**Today this rule already holds for every type except text.** `docs/language.md: Ownership`, "Call sites": "An argument is borrowed automatically when the parameter is a reference." On `5234c8817`, `(area s)` with `(type area (fn ((ref shape)) i64))` emits `area(&s)`, `(pr-str s)` emits `crate::rt::Shown(&s)`, and `(label (:name s))` on a `(ref str)` parameter with a `string` field emits `label(&s.name)`. What fails is owned text at a `(ref str)` parameter: `(label o)` with `o` a `string` reports `` expected `str`, found `string` `` and the help `(as-str o)`, and so does `(trim o)`. So the issue's motivation, "the text functions need `as-str`", is the whole of the gap.

Option C closes it by the existing rule: owned text meeting a view slot is a borrow of its place (section 1's flow table), which is "Call sites" applied to text. No text-only rule is added.

What #590 adds beyond that is its trade-off: "If a signature changes from borrowing to owning, callers silently switch from borrowing to moving, and a later use fails with use-after-move. The diagnostic should name the call that moved the value and the signature that changed." Under option C a text parameter's mode follows its body, so an edit to a callee's body can turn a view parameter owned. `L0501` points at the move today; it does not say that the move comes from the callee's parameter.

- (a) **The general rule, with text as one case of it.** **Recommended.** This RFC's PR 2 closes the text gap by the "Call sites" rule, and PR 3 adds to `L0501` a note at a move into a parameter whose mode was inferred: the note names the parameter and the use in the callee's body that made it owned, as `L0514` already labels the callee's `set!` for an inferred `mut`. That note serves every type, so #590 closes with this RFC.
- (b) Text only now; the `L0501` note follows in its own pull request under issue #590.
- (c) Leave #590 out. Text then auto-borrows by this RFC and every other type by today's rule, so text is no exception; only the diagnostic of #590's trade-off is missing.

### Copy (decision 9)

Today a struct whose text field is a bare `str` derives `Copy`, because `&str` is Copy. The type stage decides struct traits before the representation is known.

- (a) **Text is never `copy` to the type stage.** No struct with a text field derives `Copy`. Two places then speak about copying text: the type stage says text is not `copy`, and the move checker copies a view because it is a reference. So `s` can be used twice, a struct holding it cannot, and `(where ((a copy)) ...)` rejects a view that Rust accepts.
- (b) The ownership stage derives `Copy` when every text field ends a view. A trait of a struct is then decided by two stages.
- (c) **`Copy` when every text field is written `(ref str)`.** **Recommended.** The type stage knows that alone, so one stage decides, and a written `(ref str)` is the one case where a struct deliberately holds a view. A literal-only field under 1(d) is `&'static str` and could be Copy, but derives nothing, so the derive never depends on where the struct is built. A `copy` bound on bare text is rejected with help to write `(ref str)`. A program that used a literal-only struct by value twice gets `L0501`, use after move, with `(clone x)` as its help; PR 2 counts the fixtures this changes before the owner confirms.

### A lent value inside an `option`, a `result` or an `iterator`, or stored into a view element or payload (decision 13)

- (a) **Write `.map(String::as_str)`.** **Recommended.** It is a pointer projection, not a clone or an allocation, the Rust a person writes, and the same kind of view the compiler writes today at a closure result. Without it, `(unwrap-or (first names) "none")` has no spelling, since `as-str` is gone.
- (b) An error with the code of decision 6, asking for a local. Every `option` of an element of owned text then needs one.

A mixed value meets these places as a lent one does, with `.as_ref()` in place of `.as_str()`: `names.first().map(|n: &Cow<'static, str>| n.as_ref()).unwrap_or("none")`, which rustc accepts. The same choice covers every lent value stored into a view element or payload. Under (a) it takes `.as_str()` where it is stored, and a `collect` takes the iterator view first. The alternative, keeping such an element lent, `Vec<&String>`, is not what a person writes, and a lent element vector disagrees with a view element vector at a collection parameter, which sends more programs into decision 15's error.

## 3. Interaction with other work

### The loan rule (RFC 0065 step 2: PR 580 and the next three)

The approved split is: 1, places, loans and liveness with no diagnostic (PR 580, on main); 2, conflicts, `L0518` and `L0519`; 3, closure captures and stored loans; 4, escaping borrows, taking over `L0510` and `L0512`. Steps 2 to 4 are still to come: `L0518` and `L0519` are reserved, not built. User function results lend nothing until RFC 0065's step 3, regions.

**Text feeds the loan rule through the rewritten types.** The loan rule on main decides what holds a loan from types and adjustments:

- `holds_a_borrow` (`loans/loan_rules.rs:26-28`) goes through `BorrowingTypes::keeps_borrow` (`src/types/borrowing_types.rs`), which is `type_holds_borrow || pipelines.holds_an_iterator` (`TypedProgram::type_holds_borrow`, `typed.rs:509-511`), and drops a value's holdings when its type holds no borrow and no iterator. After `TextRewrite`, owned text is `String` and holds none; a view and a lent value are references and keep their loans.
- `field_read` (`loans/body_walk.rs:104-111`) lends the field's place whenever the read's type is a reference. A read of an owned field is lent, `&String`, and lends the field's place; the fixture `struct_field_holds_a_borrow`, whose field is `:label (ref string)`, shows a read of that kind ("borrow `t.label`: `v` lent at 6:30, held by `t`"). A read of a written `(ref str)` field is a reference too, so by the same filter it lends the field's place and keeps the owner's loans. Two `loans` fixtures already hold a `(ref str)` field, `clone_of_a_lent_struct_holds_its_text.knv` and `clone_of_a_temporary_struct_holds_its_text.knv`, and the first shows such a read lending the field and keeping the owner's loan. RFC 0065's "a copy out of a borrow holds no loan" applies only when the result's type holds no borrow, which a view's does.
- A read of a `&'static str` field under decision 1(d) is the exception: it holds no loan of anything. `Lifetime` (`src/types/ty.rs:31-34`) has only `Elided` and `Written(BindingId)`, so PR 2 adds `Lifetime::Static`, which the mixed representation needs for its `'static` parameters and results, and every `match` on `Lifetime` handles it. `type_holds_borrow_with` (`typed.rs:513`) returns false for a `'static` reference, and `field_read` does not lend for one. That function is shared by `definition_holds_borrow` (`typed.rs:529`), which must not give the struct a lifetime, and by the loan walk, so PR 2 changes loan facts and adds a `loans` fixture.
- `Passing::of` (`loans/argument_passing.rs:42-56`) lends an argument whose `Borrow` is `Shared`. The rewrite sets `Shared` at each owned-to-view meet, so `(shout owned)` lends `owned` for the call.
- A literal has no `LoanPlace` (`loans/loan_place.rs:14`) and walks with no holdings (`body_walk.rs:64-75`), so it is loan-free, as `'static` is.
- A mixed value is `Cow<'static, str>`: it holds no borrow of any place, so `type_holds_borrow_with` returns false for `PrimitiveType::CowStr`, it moves as `String` does, and `Cow::Borrowed("a")` lends nothing. A `Cow::Owned(x)` moves `x`, as storing it into an owned slot does. A read of a mixed field, `&a.name`, is a reference and lends the field's place, as a read of an owned field does. So the loan rule meets the mixed representation through the same three facts it reads for owned text.
- Template ties come from declared schemes, which keep `Type::Ref` (section 1), so `trim`, `subs`, `lines` and `split` settle as they do now. `split` no longer writes `'a`; the only written prelude lifetime is `line-seq`'s `'input` (`io.knv:28`), which settles the same way.

So the order inside the ownership stage is: modes and representations in one fixed point, `TextRewrite` beside `ModeRewrite`, then moves and loans. **`TextRewrite` must run before `LoanFacts` collects anything**, and PR 2 adds a test that `--print-loans` on a text program shows the loan of an owned local lent to a view parameter.

**Keeping accepted programs out of rustc before steps 3 and 4.** Under decision 1(d) no field holds an inferred borrow, which closes review's `E0515` class for constructors. The inferred borrows left are view parameters, view and lent locals, and view results that borrow a parameter, the same shapes `(ref str)` has today, with the same `L0510` checks. One class stays: created text passed to a view parameter of a user function whose view result outlives the statement. Today it needs `as-str` and reaches rustc (`(make (as-str (str "u" 1)))` under a view field, `E0515`, `knave check` exits 0). Under option C it needs no `as-str`, so it is easier to reach. Inside one function, `(let (w (trim (str " a" 1))) ...)`, the emitter's existing lift in `lent_temporaries.rs` binds the temporary first. Today `(let (w (trim (as-str (str " a" 1)))) ...)` emits `let owner = format!(" a{}", 1_i64); let w = owner.as_str().trim();`. After this RFC the same lift emits `let owner = format!(" a{}", 1_i64); let w = owner.trim();`, which allocates nothing the source did not write. A returned or stored result cannot be lifted. Decision 10 orders PR 2 against the steps that close it.

**Step 3, regions.** A view slot's lifetime is a region. Until step 3, the emitter's lifetime deciders (`items.rs:1072-1085`, `result_borrows.rs`, `held_borrows.rs`, `signature_borrows.rs`) see `&str` and `&String` after the rewrite and name lifetimes as they do for `(ref str)` and `(ref string)` today. This RFC writes one lifetime itself, `'static`, on every field, parameter and result the solver marks `'static`: `fn pick_name(u: &User) -> &'static str` for a function that returns only literals, `fn label(s: &str) -> &'static str`, and `fn make(n: &'static str)` under 1(f), whose parameter is stored. A read-only parameter is never written `'static`, and a view result that borrows from its parameters stays elided, `'static` only at a call whose arguments are. Today's emitter elides those results, `fn pick_name(u: &User) -> &str`, which ties them to `u`, so `Cow::Borrowed(pick_name(&u))` beside a later move of `u` is `E0505`, and `Cow::Borrowed(label(&o))` outliving `o` is `E0597`; with the written `'static` rustc accepts both. When step 3 lands, the regions record takes `'static` over with every other lifetime.

**Where the code meets the loan rule on main.**

- `src/types/infer.rs`: `infer_push` at 2860-2887 and `infer_string_push` at 2904-2933, with `SyntheticTemplate::PUSH_TEXT` and `Type::str_ref()` at 2914. That is one of this RFC's rows, so PR 2 rewrites the synthetic template's types for text.
- The files `template_ties.rs`, `registered_template.rs` and `synthetic_template.rs` under `src/types/`: the scheme-keeps-its-references rule protects them.
- `src/types/written_type_lowering.rs` and `scheme_lowering.rs`, which replaced `lowering.rs`, and `infer_inline_rust` (`infer.rs:3613`), which strips written lifetimes at instantiation; this RFC changes the lowering of `str` (`written_type_lowering.rs:226`). Different functions.
- `src/emit/expressions.rs`, and the `loans` fixture stage, 45 of whose 84 fixtures write `string` or `as-str` (counted on `a6545d9a2`), and its unit tests (section 4).

### RFC 0074, `int` and `float`

They meet in the kernel rows, the printers and the prelude's text:

- **Kernel rows and exports.** RFC 0074's PR 1 moved `PrimitiveType` to `expand/src/core/primitive.rs`, whose rows carry `PrimitiveNames` (`expand/src/core/primitive_names.rs`), and `builtin_exports` exports each primitive under its shown name and its kernel spelling (`expand/src/expand/builtin_export.rs:18`, `kernel.rs:307-320`), through `PrimitiveType::all()` (`kernel.rs:282`). The internal `String` and `CowStr` rows must be excluded from that set outright, each with the shown name `str`, not only given the shown name `str`, or it would export `string` and a second `str`. Its `names().rust()` is `String`, and its `names().shown()` is `str` for `TypeDisplay`. The one rule this RFC adds to `TypeDisplay` is that a reference to text prints as `str`.
- **Decision 8 of RFC 0074,** decided (b) and built as `L0259`, warns on every type-namespace binding whose name is a kernel type's name; it is a rule over kernel names, not a list. It looks names up through `KernelType::named` over the same set as the exports. After this RFC the kernel still holds the internal `String` row and the new `CowStr` row, so excluding them from that one set fixes both the exports and the warning. The exclusion belongs in `KernelType::all()` or the exports, not in `PrimitiveType::all()`, which `template_operation.rs:334`, `numeric_bound.rs:36` and `pinning.rs:636` also read; otherwise a user type named `string` would warn about a type no program can write. `docs/language.md:216` uses `string` as its `L0259` example name and must change.
- **Implementor lists (0074 decision 11).** 27 lines in 17 traits in 7 files name `string` on `a6545d9a2` (`map.knv:22,24`, `number.knv:34,38-41,54,64,75,260`, `ownership.knv:11`, `text.knv:45,51,69,73,107,111,189`, `types.knv:38`, `io.knv:16`, and the four text traits `contains` at `sequence.knv:18`, `pattern` at `text.knv:217`, `joinable` at `text.knv:233,235` and `prefixed` at `text.knv:244`). Most list `string` beside `str` or a view; `clone` (`ownership.knv:11`) and `from-str` (`text.knv:189`) list `string` only; `ord` (`number.knv:260`) lists `string` beside `(ref str)`; and `text.knv:51,73,111` are `(parse-error string)` rows. PR 4 respells a pair as one bare `str` row, a `string`-only row as `(own str)`, and `(parse-error string)` as `(parse-error (own str))`. The four text traits' `string` rows fold the same way into the view row beside them, as `(vec string)` and `(vec (ref str))` become `(vec str)`, and `pattern`'s `(ref str)`, `(ref (ref str))` and `(ref string)` rows become one bare `str` row, and `join`'s result (`text.knv:241`), a `rust` form's, becomes `(own str)`. So a list prints `str` once because the source states it once, and no printer deduplicates.
- **Relayed rustc errors.** 0074 decision 9 keeps Rust's spelling in relayed rustc errors, so `String` and `&str` show there. That is an exception to "one text type" on the same ground as 0074's: the text is Rust, and before 1.0 only user `rust` text and crate calls reach it. `docs/language.md` records it beside 0074's.
- **Prelude examples.** 0074's step 4 ("Prelude docstrings and signatures") has respelled `result.knv`'s examples, which its table counts as 16 examples of every kind; 11 of its lines now read `(the (result int string) ...)`, the ones this RFC also touches. This RFC respells them to `(result int str)`, one edit per line.
- **Shared snapshot.** `stages__emit_fixtures@infer.knv.snap` already shows 0074's `"(vec int)"`. Its text entry is `"string"`, from `(infer (str x))`, and becomes `"str"` here.

### RFC 0075, collections speak `int`

- `subs` (`prelude/text.knv:802-804`) takes `(ref str)` and two indexes, still `usize`. 0075's PR 3, not yet built, changes its index template to `&{0}[{index 1}..{index 2}]`. This RFC changes no part of it.
- `len` on text goes through `sequence`, whose runtime impls for `String` (`prelude/runtime/sequence.rs:5`) and `str` (`:11`) both stay. RFC 0075's PR 3 wraps the call, `crate::rt::length(items.len())`, and adds no cast (`len` is still `{0}.len()` on main); the `Cow` impl of `Sequence` (section 1) is called through the same wrapper.
- Both RFCs rewrite `LiftFixture::SOURCE` in `src/emit/lent_temporaries.rs` (`:144-340`): 0075 for its `usize` positions, this RFC for its 33 `as-str` lines. Whichever lands second rebases it.
- RFC 0075's integer help needs RFC 0070's `mismatch-help` form, which is not built yet, and RFC 0075 builds the form itself if RFC 0070's build has not, naming no text entry (`rfcs/0075-collections-speak-int.md:558`). Only RFC 0070's build adds the `(mismatch-help string (ref str) ...)` row; if that row is in the prelude when this RFC's PR 5 lands, PR 5 removes it, because `string` becomes unbound; decision 12 records the dependency. This RFC drops only the form's `string` entry from RFC 0070's design (decision 8), so the form ships as RFC 0070 and RFC 0075 describe it, whenever RFC 0070's build lands; no pull request of this RFC waits for it.
- Both change `docs/language.md: Standard types`.

## 4. What breaks

Counted on `a6545d9a2` with `git grep -P` and a lookbehind and lookahead for Knave symbol characters, because `-w` treats `-` and `?` as word boundaries and would match `pr-str` and `string?`. Uses of the macro-time builder `string` (`Builtin::MakeString`, `expand/src/core/builtin.rs:110`) were removed by hand. Docs counts are approximate because prose and types share lines.

| Area | `string` as a type | `as-str` |
|---|---|---|
| Prelude, without `prelude/tests/` | 27 lines in 17 traits' implementor rows (7 files). Signatures: `slurp`, `line-seq`, `read-line`, `command-line-args` (`io.knv:21,30,35,39`, and the template row `:23`), `display-string`, `pr-str`, `lower-case`, `upper-case` (`text.knv:141,173,787,791`), `join`'s result (`:241`), `as-str`'s own signature (`:258`), the `str` macro's `(quote string)` (`:503,507`), and `task-argument`, `task-command-line`, `task-value`, `task-value-or`, `task-switch` (`task.knv:508,697-737`): 18 lines with `io.knv:23` and `text.knv:258`, and two more clause rows of the `:23` kind, `split`'s `((ref string) "{0}.split({1})")` (`text.knv:227`) and `contains?`'s `(string "{0}.contains({1})")` (`sequence.knv:31`). About 79 docstring and example lines, including `(document string ...)` at `prelude.knv:559-564`. | 23 lines in 7 files that name the symbol: 4 code lines (definition, export, import, re-export) and 19 doc lines, two of them the `help` strings of `contains` (`sequence.knv:16`) and `pattern` (`text.knv:214`); 24 with the template line `text.knv:257`, `"{0}.as_str()"`, as a fifth code line |
| `prelude/tests/`, `examples/`, `README.md` | 1 line (`prelude/tests/text.knv:9`) | 0 |
| Fixtures, either name | about 443 files, 910 lines: run 169 files (355 lines), emit 110 (254), check 115 (150), expand 18 (25), agree 12 (13), format 4 (7), and the corpus below | counted in the same files |
| Corpus entries | 15 files, 106 lines: `evaluator.knv` and 4 submodules (57 lines), `warehouse.knv` and 4 submodules (37), `generic_types.knv` (6), `matrix_geometry.knv` and 2 submodules (4), `macro_heavy.knv` (2) | included |
| `loans` fixtures | 45 files, 49 lines, of 84 fixtures: `string` in 7 files and 7 lines, `as-str` in 38 files and 42 lines; and 10 lines in its unit tests (`body_walk.rs`, `loan_place.rs`) | included |
| Explanation pages | about 36 pages and 74 lines, since prose lines need judgment; `L0507` describes the view, and `L0444`'s fix implements a trait for `string` | 0 |
| `docs/language.md` | about 30 lines, including the primitive list (`:85`), the Standard types row, the adjustment rule, the `match` `.as_str()` rule and the `L0259` example (`:216`) | 5 lines |
| `tests/cli.rs` | 13 lines | 3 lines |
| Snapshots, of 2,694 | 70 with `string` anywhere inside backticks (check 45, expand 20, cli 4, run 1), and 42 with exactly `` `string` `` (32, 7, 2, 1), counted with `git grep -l` over `tests/snapshots`; 162 with emitted `String` | 132 with `as-str` or `as_str` |
| Unit tests with Knave source | 14 files, 96 lines, the 2 `loans` files and their 10 lines included: `src/emit/lent_temporaries.rs` (38), `src/emit/short_form.rs` (28), `src/ownership/loans/body_walk.rs` (9), `src/types/templates.rs` (3), `src/types/derive_blockers.rs` (3), `src/emit/assigned_in_place.rs` (3), `src/types/structural.rs` (2), `src/types/derive_rules.rs` (2), `src/emit/lending_expression.rs` (2), `src/emit/held_borrows.rs` (2), and one line each in `src/types/struct_trait_help.rs`, `src/types/missing_trait_explanation.rs`, `src/types/implementor_listing.rs` and `src/ownership/loans/loan_place.rs`, counted with `git grep -cP '(?<=[( ])(string\|as-str)(?=[ )])'` and read by hand; `lent_temporaries.rs` and `short_form.rs` are almost all `as-str`, and the three files write `(as-str (announced ...))` 45 times (18, 26 and 1) | included |
| `xtask/agree` generator | `generated_main.rs:13` writes `(defstruct car :make string :year i64)` | `reading.rs:23` writes `(as-str (:make ...))` |
| Compiler Rust naming the types | the Problem table, plus `size.rs:50`, `coverage/pattern_guard.rs:32-33,79-80`, the Rust name `String` in the primitive table (`expand/src/core/primitive.rs:51`, read through `names().rust()` at `emit/types.rs:213`), `emit/unsized_parameters.rs:85`, and help texts at `expand/src/expand/clause_chooser_problems.rs:14`, `expand/src/expand/problems.rs:791`, `module_body.rs:1345`, `src/types/problems.rs:246-262,644` and `src/types/infer.rs:2248` | |

Of the 4 `(the str ...)` and `(the (ref str) ...)` sites, only `prelude/syntax.knv:833` uses the special case; the others pass a literal or an `as-str` result.

**Respelling the repository's own sources is mechanical, with three rules.** Each `string` in a type position becomes `str`, except `(own str)` in a `rust` form, in a one-representation implementor row, and as the implementor of an `impl`, so that `(impl label string ...)` keeps its Rust type `String`. Each `(as-str x)` becomes `(ref x)` when `x` is an owned place, `x` when `x` is already a reference, and `x` otherwise, when `x` is an owned value that is not a place, such as `(as-str (str ...))` or `(as-str (announced ...))`: a bare owned local stored into a slot would move it, so `(local v (vec (as-str s) (as-str t)))` in the `loans` fixture `vector_literal_holds_each_element_s_loan.knv` becomes `(vec (ref s) (ref t))` and keeps testing the loan, as do `map_literals_hold_their_keys_loans.knv` and `(local t (as-str text))` in `borrowed_temporary_in_short_form.knv:227,240`. The third case has the same hazard: `(local t (as-str (str "a")))` respelled as `(local t (str "a"))` makes `t` owned, so a fixture that tested the temporary lift stops testing it; such a fixture keeps a view, for example by passing the temporary straight to a view parameter. `(as-str (` is written on 157 fixture lines in 37 files, `(as-str (str ...))` on 50 lines in 18 files across `tests/fixtures` and `src`, and the unit tests in `src/emit/lent_temporaries.rs`, `short_form.rs` and `lending_expression.rs` write `(as-str (announced ...))` 45 times (18, 26 and 1), of 120 in the repository: 74 more in the `borrowed_temporary_*` and agree fixtures, and 1 in `docs/language.md`. The fixtures hold 392 `(as-str` lines in all, 42 of them in `loans`, and the snapshot diff is read for any move that replaces a loan and any lift that disappears. A fixture that tests a borrowed field or lifetimes, such as `lifetimes.knv`, `function_parameter_lifetimes.knv`, `closure_borrowed_parameter_struct_result.knv` and `borrowed_string_vector_field.knv`, respells its bare `str` field as `(ref str)`, so it keeps testing `held_borrows.rs` and `items.rs:1072-1085`: under 1(d) a bare `str` field would turn owned or `'static` and drop that coverage. Between 782 and 787 `.knv` files call `(str ...)`, depending on how the call is matched, and none changes.

**What a program sees change.**

- `string` and `as-str` are `L0201`.
- A struct whose text field is not written `(ref str)` stops deriving `Copy` under decision 9(c), so a by-value second use of one becomes `L0501`.
- A field written `str` that today receives only literals emits `&'static str` in place of `&'a str` under decision 1(d), and its struct loses `'a`. A field written `str` that today stores a parameter's view takes its callers' representation: `&'static str` when they pass literals, owned when they pass created text, and mixed when they pass both. A caller that passes a view that is not `'static` gets `L0503` with `(str ...)`.
- A parameter that today is `&String` keeps it under decision 19's rule when O and L reach the call's `a`, as in `label`, and becomes `&Cow<'static, str>` when S reaches it without L. A body whose clone of a view parameter flows nowhere with O, `(defn dup (s) (clone s))` and `(defn mk (s) (->user :name (clone s)))`, reports `L0503` at `s`, and the help writes `(str s)`; `mk` loses today's `fn mk(s: &String)`, which the owner's rule against migration aids allows.
- A text argument at a non-receiver position of a prelude template gains `.as_str()` or `.as_ref()` under decision 20(a) when it is not already a view, as `m.get(u.name.as_str())`.
- A slot that mixes literals and created text emits `Cow<'static, str>`, so its reads branch on a tag that the source does not write; section 5 measures how many slots do.
- A parameter written `string` in a `type` signature is owned today. Respelled `str`, it states no representation under decision 3(a), so a parameter that only reads becomes `&str`, the Rust a person writes.
- `(str ...)` builds a `str`, so the Names exception "`(str ...)` builds a `string`, an exception to rule 4" (`docs/language.md:1156`) goes, which is issue #589's second motivation.
- `(infer "a")` prints `"str"`, not `"(ref str)"`, and every printed `(ref str)` and `string` becomes `str`.

The owner is the only user, so there is no migration aid, and the pull request that removes `string` and `as-str` carries `release:minor`.

## 5. Cost

**Front end: no stage rises more than 1% in instructions on any corpus entry, inside RFC 0065's budget.** RFC 0065's decision caps the front end's total instruction count over the corpus at 10% above its baseline across the steps of Option A, and this RFC's solve runs in the same stage as the loan checks, so its rise counts against that shared ceiling too. Each pull request body reports both: its own comparison with its base, and the total against RFC 0065's baseline.

- **Why it should hold.** The solve adds one slot per text position and one edge per flow. A slot moves at most twice, from a view to owned or mixed and from owned to mixed, so the worklist does work linear in the edges, inside the modes fixed point that already runs. A body whose types hold no text adds no slot, so `call_chain`, `deep_nesting`, `documented_defn`, `empty_program`, `literal_inference` and `many_modules`, which write no `string` and no `as-str`, should be flat. The work it removes, `StringViewSearch` at prelude build and every program load, the view coercion check at each `expect`, and the `as-str` insertions, is unmeasured. PR 1 measures the solve on its own against the base, and PR 5 measures the removals, so neither claim rests on this paragraph.
- **How it is measured.** Before a pull request is ready: `cargo xtask bench measure --workspace <base checkout> --save base.json`, the same on the branch, then `cargo xtask bench compare base.json branch.json`. Once ready, read CI's `front-end instruction counts` comment and treat any stage above 1% on any entry, or a total past RFC 0065's ceiling, as a blocker to report to the owner. The corpus sources change in PR 4 by respelling only, which removes tokens and adds none, so a comparison of the base binary on the old source with the branch on the new one measures the compiler.
- **Allocation.** The slot table allocates per text binding. CI's 1% gate on `allocator.allocated_bytes`, `allocator.allocations` and `allocator.peak_live_bytes` applies. If it trips, stop and report the programs and counters to the owner; no `Counters-Accepted:` trailer without approval.

**Emitted Rust: no clone or allocation the source does not write, and Rust that reads as hand-written.**

1. **Corpus diff.** Emit every corpus entry with the base commit's probe binary, downloaded as `docs/pull-requests.md` "Probe binaries" says, and with the branch, then `diff -r` and read each hunk. A local `cargo build --profile local` is allowed for the branch before its pull request is ready, because that builds edited code.
2. **An allocation count that cannot rise.** Count, per emitted file, the calls that allocate text or copy heap data: `.to_string()`, `.to_owned()`, `String::from`, `format!`, `.clone()`. For each corpus entry and each emit fixture, the branch's count must equal the base's, and every allocation must trace to a written `str`, `clone` or allocating prelude form. `Cow::Borrowed` and `Cow::Owned` allocate nothing and are not counted; the check also lists every mixed slot, with its count per entry, because each is a tag the source does not write. A short script in `target/scratch` does this over both outputs; it is a check of this RFC, not a repository tool.
3. **Emit snapshots.** Read every changed hunk. The expected changes are the ones under "What a program sees change", plus `.as_str()` calls that a coercion now makes unnecessary and `.map(String::as_str)` where a program wrote `(map-some x (fn (n) (as-str n)))`. Any new `.to_string()` is a bug.
4. **rustc agrees.** `cargo xtask agree --seeds 0..200` after the generator writes `str` (PR 4), with the generator grown to store literals, created text, parameters and field reads into one struct, push both kinds into one vector, read `first` of an owned vector beside a literal, and pass owned text to view helpers. Those are the shapes where PR 598's output reaches rustc's `E0716` and `E0308`, and RFC 0065's oracle catches them.

## 6. Order of work as pull requests

This work comes after the loan rule's first pull requests and after RFC 0074, and decision 10 places PR 2 later still. Each pull request runs the narrowest working commands, regenerates and reads its snapshots, and checks the counters. Each pull request that changes a rule updates `docs/language.md` in the same commit, as `AGENTS.md` asks.

1. **Prototype: the solver over today's types, behind a hidden flag.** Add the slot collection and the fixed point to the ownership stage, reading today's `string`, `str` and `(ref str)` as one text type, and a hidden `knave check --print-text` that prints each slot's decision, as `--print-loans` does for loans. No output changes and no label. Over the about 443 fixtures and the corpus, it lists every slot where the inferred representation differs from the one the user wrote today; each difference is either a case the rules get wrong or one where today's program pays for a choice it did not need. It also reports how many of the 77 snapshot fields written `&'a str` become `&'static str`, owned or mixed under decisions 1(d), 1(f) and 8, how many slots of every kind are mixed, how many values would get `L0503` because a non-`'static` view reaches an owned or mixed slot, and every `&String` parameter (decision 19), and measures the solve's cost on its own.
2. **One text type.** `str` becomes sized text to the checker, a literal is `str`, the reference layer on text is erased for HM and put back by `TextRewrite`, `(own str)` lowers, `TypeDisplay` prints `str`, `lent_temporaries` keeps the text lift, the view declaration form and its rows land in `prelude/text.knv` (decision 16), the view nodes replace the closure and `match` paths and add the generic-argument and comparison views, the emitter's own `.as_str()` path goes, a lent-to-view meet wraps function and method values, trait method text positions share one slot, collection parameters follow decision 15, with fixtures for `get` on a map parameter, `sort!` and `push!` on a `mut` vector parameter and a reader like `loudest`, each with one kind of caller and with callers that disagree, the element views of decision 13 (a borrowed vector literal, a pair and an `option` payload in one, `into-hash-map`, a map key and a map value, `collect`, and `push!` of a literal after a lent initializer), function-typed parameters follow their row, a text `push!` makes its slot owned, every `is_unsized` reader in section 1's table moves to its new rule, the mixed representation lands, with `PrimitiveType::CowStr`, `(cow str)`, the wrap rows of decision 16, `Lifetime::Static` and the `'static` signatures (folded in from PR 6), the runtime `impl`s of `Sequence`, `Show`, `Source`, `Contains` and `StartsWith` for `Cow<'_, str>` with fixtures for `prn`, `pr-str` and a generic `len` over a mixed field and a mixed vector, `clone` of mixed text, `clone` of a view (`dup`, `L0503`; `mk`, `L0503` because the field's only store is the clone and passes nothing back), `keep`, a stored parameter at a tied `(ref a)` with owned, mixed and literal-only callers, `label` with a literal-only caller set, with mixed owned and literal callers, and the view `label` that passes `tagged(&name, "x")`, a lent and a lent mixed parameter at a generic `(ref a)` (`label`, `view_refused_for_held_generic`, with a literal caller), `slurp` with an owned, mixed and lent path, `get` and `set-at!` on a map with view keys by every key kind, a `:names (vec str)` field fed views (`L0503`), a read of a `def` of created text into a mixed field, `subs` on owned and mixed text, `or-none` into a mixed slot, `split` and `get` with owned, view, mixed, lent and lent mixed arguments against owned, view and mixed maps, a comparison of mixed with owned text under `ord`, and fixtures for a mixed field, element, stored parameter, local and result, a mixed value read through a view parameter, a comparison, `option` and iterator views of mixed elements, a map with mixed keys, and a non-`'static` view reaching a mixed slot (`L0503`), `L0444` keys on an implementor written bare `str`, and fixtures cover closures, `impl` on `str` and on `(own str)` reached from owned and lent text through a generic function, and the free view through `option` and `iterator`. `string` still lowers, as owned text, and `as-str` still works. Programs still stop compiling in three ways: a `clone` of a view parameter, `L0503` under decision 19(c); and the two that follow: a struct used by value twice under decision 9, and a non-`'static` view stored into an owned or mixed slot, which today compiles only as a `(ref str)` field with a lifetime. So the pull request carries `release:minor`, and its body lists each fixture that changes for either reason.
3. **The needs-ownership diagnostics.** The `L0503` text wording with its secondary label and its help, `(str ...)` or a field written `(ref str)`, including the `mut` row's label; the two new ownership codes, each with its explanation page in the same pull request as `docs/language.md:469` requires: the code for a lent value inside a type with no view row (decision 6), and the code for an owned caller of a collection parameter whose callers disagree (decision 15(b)); under decision 14(a), the `L0501` note that names a callee's inferred parameter and the use that made it owned, for every type; the `L0510` help, check fixtures for the field, element, branch, local and parameter cases, and the `L0503` page's text section.
4. **Respell the sources.** The prelude's signatures (with `(own str)` in `rust` forms and one-representation rows), docs and implementor rows (the four text traits `contains`, `pattern`, `joinable` and `prefixed` included), the corpus, the about 443 fixtures by the three rules of section 4, the 45 `loans` fixtures, about 36 explanation pages, `tests/cli.rs`, the 14 unit test files and the `xtask/agree` generator, with `cargo xtask agree --seeds 0..200` and `xtask/agree/pull-request-findings.txt` updated if the findings move. Mechanical; its review is the snapshot diff, and CI's coverage job checks that `MINIMUM_LINE_COVERAGE` still holds for the lifetime code those fixtures reach. No label.
5. **Remove `string` and `as-str`.** The `(mismatch-help string (ref str) ...)` row if RFC 0070's build has added it, the Names exception at `docs/language.md:1156` ("`(str ...)` builds a `string`, an exception to rule 4"), which goes because `(str ...)` now builds a `str`, the kernel export, `as-str`, `StringViewSearch`, `views_string` and the text halves of `coerced_view` and `unification.rs` (`ViewCoercion` and `viewed_items` stay for slices), `string_view.rs`, the `as-str` help, the `help` texts of `pattern`, `joinable` and `contains` (`text.knv:214`, `text.knv:232`, `sequence.knv:16`), which teach `as-str` or `string` and are reworded with no migration aid (decision 5), the `match` retyping, the `the` docstring example, RFC 0070's status line under decision 8, and `docs/language.md`'s remaining lines. `release:minor`.
6. **`'static` fields, folded into PR 2.** Earlier drafts put `Lifetime::Static` and the literal-only field here. The mixed representation needs `'static` parameters and results first, so PR 2 carries all of it: `Lifetime::Static`, `&'static str` on fields, parameters and results the solver marks `'static`, `type_holds_borrow_with` false for a `'static` reference and for `CowStr`, `field_read` lending nothing for a `'static` field, with fixtures for a literal-only struct, one inside another struct, `pick-name`, `label` and `make` feeding mixed slots, and a `loans` fixture that changes the struct while a read of a `'static` field is live.

PRs 2 to 5 ship in one release. Between PR 2 and PR 5, `string` and `as-str` keep exactly today's meaning on `main`, because PR 2 changes neither and PR 5 removes both; nothing is added for them, no alias, help, note or hint. The split exists so each pull request stays small enough to review, and the one release means no user ever sees `string` beside `str`, or `str` printed for every text while the docs still teach `string`.

## 7. Decisions for the project owner

1. **Struct fields: how is a text field's representation chosen?** **Decided: (d).**
   - (a) A view unless an owned value is stored. Literal-only structs keep `'a`, and a constructor helper reaches rustc's `E0515`.
   - (b) Always owned.
   - (c) `&'static str` for literals, owned for created text, `&'a str` for other views. Same `E0515` class as (a).
   - (d) Owned unless every stored value is `'static` text; `&'static str` when every one is; owned when nothing stores into the field; `&'a str` only where written `(ref str)`. **Decided.** Reopened on one point by decision 8: whether a stored parameter takes its callers' representation. **Decided: (f) on that point.**
   - (e) Generalize the field into a hidden type parameter, as PR 598 does.
   - (f) As (d), with `'static` flowing through parameters, as a literal's pin does: callers then change a callee's signature.
2. **What prints for a text type?** **Decided: (a).**
   - (a) `str` everywhere. **Recommended.**
   - (b) `str` for owned, `(ref str)` for a view.
   - (c) As (a), with a later `knave types` column for every parameter's passing.
3. **What does a bare `str` mean in a `type` signature?** **Decided: (a).**
   - (a) No representation stated; `(ref str)`, `(own str)` and `(cow str)` pin one. **Recommended.**
   - (b) Owned, by the written-types rule.
4. **How is owned text written as a type?** **Decided: (a).** This also answers whether the issue's rule 5 needs more than a value-level hatch.
   - (a) `(own str)`, a kernel type form valid around `str` only. **Recommended.**
   - (b) A prelude-internal type that only the prelude writes.
   - (c) None; a bare `str` in a `rust` form means owned.
5. **`string` and `as-str` after removal.** Settled by the owner's standing rule on migration aids: plain `L0201` with the closest-name suggestion, and no entry, help, note, alias or hint. No options.
6. **The needs-ownership diagnostic.** **Decided: (a).**
   - (a) `L0503`, with a text wording, a label on the value that made the slot owned, and a `(str ...)` help; a new ownership code for a lent value inside a type with no free view. **Recommended.**
   - (b) A new code for the text case of `L0503` as well.
7. **The value-level escape hatch.** **Decided: (a).**
   - (a) `(str x)`, which exists and emits `x.to_string()`. **Recommended.**
   - (b) A new `own` value form.
   - (c) `clone` of text gives owned text through a template that changes with the representation.
8. **A literal in an owned slot.** **Decided: (d)**, a new option. A slot that stores both `'static` text and created text is `Cow<'static, str>`; RFC 0070 decision 4's `String::from` literal rule is superseded.
9. **`Copy` of a struct with a text field.** **Decided: (c).**
   - (a) Never.
   - (b) The ownership stage derives it when every text field ends a view.
   - (c) Derived when every text field is written `(ref str)`. **Recommended.**
10. **When PR 2 lands against RFC 0065.** **Decided: (a).** Option C makes one existing rustc class easier to reach: created text passed to a view parameter whose view result is returned or stored.
    - (a) After the loan rule's escaping-borrows pull request and RFC 0065's step 3, which close the class. PR 1 can land at any time. **Recommended**: no accepted program reaches rustc on the way, so it lands later than the first plan, which placed it after the loan rule's first pull requests and RFC 0074.
    - (b) Before them, with a conservative ownership error at such a call when the result's rewritten type holds a borrow and is returned or stored. It rejects some programs that rustc accepts, such as a helper whose view result is always a literal, until step 3 gives the precise fact.
11. **A prototype first?** **Decided: (a).**
    - (a) Yes, PR 1, before PR 2. **Recommended.** The rules of section 1 are new, and their agreement with the about 910 fixture lines that wrote a choice by hand is the cheapest test of them.
    - (b) No: start with PR 2, whose snapshots show the same differences after the fact.
12. **Order against RFC 0074 and RFC 0075.** **Decided: (a).**
    - (a) After 0074, and independent of 0075, with the shared `lent_temporaries` test rebased by whichever lands second. If RFC 0070's build lands first with its `string` row, PR 5 removes that row. **Recommended.**
    - (b) Before 0074, so 0074 respells `str` rows and examples once.
13. **A lent value inside an `option`, a `result` or an `iterator`, or stored into a view element or payload.** **Decided: (a).**
    - (a) The compiler writes `.map(String::as_str)`, or `.as_str()` on every lent value stored into a view element or payload, a free view. **Recommended.**
    - (b) An error asking for a local.
14. **Auto-borrow at read-only parameters, issue #590.** **Decided: (a).**
    - (a) The general rule: today's "Call sites" auto-borrow covers text through this RFC, and PR 3 adds an `L0501` note naming a callee's inferred parameter and the use that made it owned, for every type; #590 closes with this RFC. **Recommended.**
    - (b) Text only now; the `L0501` note follows in its own pull request.
    - (c) Leave #590 out; text still auto-borrows like every other type, and only #590's diagnostic is missing.
15. **The element of a text collection passed to a function.** **Decided: (d).**
    - (a) The body decides, a view by default; an owned caller collects views.
    - (b) The callers decide when they agree; when they disagree the slot defaults to a view, as a disagreeing literal pin falls back to its default, and each owned caller gets a new ownership code. Independent of call order. Concrete Rust for maps, `sort!` and `push!`. **Recommended.**
    - (c) Generic over `AsRef<str>` whenever the body does not need owned elements; unsound for maps, sorting and `mut` until a bound rule exists.
    - (d) (b), and (c) only when callers disagree and the body only reads elements. **Decided.** A body that writes, sorts or keys a map gives each owned caller the new code.
16. **Where the view's Rust is stated.** **Decided: (a).**
    - (a) In the prelude, through a new kernel declaration form in `prelude/text.knv`, whose rows pair a lent type with its view and give the template: `(ref (own str))` to `(ref str)` by `"{0}.as_str()"`, and `(option (ref (own str)))`, `(result (ref (own str)) e)` and `(iterator (ref (own str)))` to their views by `"{0}.map(String::as_str)"`. **Recommended.**
    - (a2) Prelude-internal `rust` definitions that the compiler finds by name.
    - (b) In the emitter, with `docs/language.md: Kernel forms` recording why the prelude cannot.
    With decision 8, the rows also state the `Cow` wraps and the mixed view.
17. **A written name for the mixed representation.** **Decided: (a).**
    - (a) `(cow str)`, a kernel type form valid around `str` only. **Recommended.**
    - (b) A prelude-internal name no program writes.
    - (c) No name; the emitter writes the `Cow` Rust.
18. **A mixed value or a literal where a change needs owned text.** **Decided: (a).**
    - (a) `L0503` at the value, with the help `(str x)`. **Recommended.**
    - (b) `Cow::to_mut`, which copies a borrowed value at its first change.
19. **A parameter that needs a reference to owned text.** **Decided: the recommended one rule.**
    - (a) `clone` of a view gives owned text by `{0}.to_string()`; needs a clause result type, new kernel work.
    - (b) The solver chooses a lent `&String` parameter at a `(ref a)` position whose `a` is owned.
    - (c) `L0503` in the body, help `(str s)`.
    - (d) As (b), with a lent mixed `&Cow<'static, str>` parameter when S reaches `a` and L does not.
    - **Recommended: one rule, the tied `(ref a)` table.** For a parameter the body does not store, at a `(ref a)`: lent when O holds with L, or without S; lent mixed when O and S hold without L; a view when O does not hold; and `L0503` when `a` is a view and the trait's rows exclude it: `label` keeps `&String`, and `mk` and `dup` are `L0503` with `(str s)`.
20. **A text argument at a position Rust makes generic.** **Decided: (a).**
    - (a) A view node on every non-receiver text argument at a view position (a `rust` form's `(ref str)`, a checker-built map query, a clause chosen for text), unless already a view; never at a tied `(ref a)`. **Recommended.**
    - (b) A mark in the `rust` form's signature, and the node only there.

## Checks against issue #589 and the reviews' probes

**Issue #589's rules, under the recommended decisions.**

| Rule | Holds? | Where it bends |
|---|---|---|
| 1. Users write only `str` | Yes for Knave code: no program needs `(ref str)`, `(own str)` or `(cow str)`. | A user `rust` form must write `(ref str)`, `(own str)` or `(cow str)` (decisions 4 and 17), and two fixes offer them: `L0503`'s help may suggest a field written `(ref str)` to keep a view, and `(impl t (own str) ...)` for a by-value `self`. Each is optional for code that has no `rust` form. |
| 2. Borrow by default | Yes for parameters, locals, results and elements: the least fixed point starts every slot as a view. | Fields (decision 1(d)) are owned unless every stored value is `'static`, to keep lifetimes off structs, which the issue's own first open question asks. A field of `'static` text is still a borrow. |
| 3. `String` only where text is created | Yes: every `String` comes from `str`, a read, an allocating prelude form or `clone`; `Cow::Owned` wraps such a `String` without a copy, and `Cow::Borrowed` wraps a literal without an allocation. | |
| 4. Zero cost, and an error naming the value | Yes for allocations: nothing allocates that the source does not write. `L0503` names a non-`'static` view or a lent value that reaches an owned or mixed slot. | A mixed slot reads through a tag check, a branch the source does not write; the owner chose it in decision 8 over an allocation per literal or an error, and it appears only in slots that mix. Decision 9(c) drops `Copy` on literal-only structs, which turns a second by-value use into `L0501`, not into a clone. |
| 5. An explicit escape hatch | Yes: `(str x)` for a value (decision 7) and `(own str)` for a type (decision 4). | |

**Every probe of the reviews, by what this RFC says.** "rustc" means this RFC's Rust was compiled, in the review or for this revision.

| Probe | Review | Today on `5234c8817` | Under the recommended decisions | Rule |
|---|---|---|---|---|
| `filter` over owned names with `(= w "a")` | 1, 3 | `L0401` | `**w == *"a"`, rustc accepts | references two deep, shared parameters |
| `(unwrap-or (first names) "none")` over owned names | 1 | needs `(map-some ... as-str)` | `names.first().map(String::as_str).unwrap_or("none")`, rustc accepts | decision 13 |
| a view field read becoming `&&str` | 1 | n/a | the rewrite copies the view out | section 1, rewrite |
| `make`/`load` constructor helper, `E0515` | 1 | `E0515` with `as-str`, `check` exits 0 | owned field, `fn make(n: String)` | decision 1(d) |
| `(make "ana") (make "bo")` with a `str` field | 2 | compiles, `User<'a>`, no allocation | `fn make(n: &'static str)`, `name: &'static str`, no allocation and no lifetime; with a created-text caller as well, `Cow<'static, str>` | 1(d), 1(f), 8(d) |
| `pick` and the `push!` pair of the Problem section | owner, 8 | `L0401` | `Cow<'static, str>` result and elements, `Cow::Borrowed` for the literal; rustc accepts | decision 8(d) |
| a mixed field read through a view parameter, compared, viewed in an `option`, used as a map key, sorted | owner, 8 | n/a | `shout(&a.name)`, `"b" < a.name.as_ref()`, `.map(\|n\| n.as_ref())`, `get("k")`, `sort()`; rustc accepts each | mixed representation |
| `Cow::Borrowed(pick_name(&u))` then a move of `u`; `Cow::Borrowed(label(&o))` outliving `o` | 7 | `E0505`, `E0597` with elided results | `fn pick_name(u: &User) -> &'static str`, `fn label(s: &str) -> &'static str`; rustc accepts | `'static` results, PR 2 |
| `(let (s (pick c)) (push! s "!"))`; `(clone (:name a))` on a mixed field | 7 | n/a | `L0503` at the mixed value, help `(str x)` (decision 18(a)); `a.name.clone()` stays mixed, rustc accepts | decision 18, `clone` rows |
| `(len (:name a))` through a generic, `(pr-str (:name a))`, `(prn names)` on mixed text | 7 | n/a | `E0277` without the runtime impls; with `impl Sequence`, `Show`, `Source for Cow<'_, str>`, rustc accepts | runtime impls, PR 2 |
| `(defn pick2 (c s) (if c (str "x" 1) s))` | 7 | `L0510` with `as-str` | `L0503` at `s`, help `(str s)` | Result row |
| `(defn or-none (s) (if (empty? s) "none" s))` into a mixed slot | 8 | n/a | elided `fn or_none(s: &str) -> &str`; `Cow::Borrowed(or_none("a"))` is `'static` at the call; rustc accepts | per-call `'static` |
| `(shout "a")` called only with literals | 8 | `fn shout(s: &str)` | unchanged: a read-only parameter is never written `'static` | 1(f) for stored parameters only |
| `(split line (:name a))` with `:name` mixed; `(get m k)` across owned, view and mixed keys with every key kind | 8 | `E0277` without a node | the separator, the argument of a clause chosen for text, takes the view node, so `split`'s one text clause, written bare `str` after PR 4, gives `line.split(a.name.as_ref())` and `Separator` needs no `Cow` impl; `get(k.as_str())` or `get(k.as_ref())`; rustc accepts every pair | decision 20(a), clause chosen for text |
| `(< o (:name a))`, owned beside mixed | 8 | n/a | `o.as_str() < a.name.as_ref()`; rustc accepts | comparison node on every non-view side |
| `(defn dup (s) (clone s))`; `label` in `view_refused_for_held_generic` | 8, 9 | `fn dup(s: &String)`, `fn label(name: &String)` | `dup`: `L0503` at `s`, help `(str s)` (19(c)); `label`: `fn label(name: &String)`, or `&Cow<'static, str>` with only literal or mixed callers (19(d)); rustc accepts both `label`s | decision 19 |
| `(slurp (:path c))` with a mixed or lent path (probe `slurp.knv`; nearest fixture `tests/fixtures/emit/slurp_sources.knv`) | 9 | `read_to_string(&c.path)` for a `string` field; `E0277` on `Cow` | one `(str ...)` clause, its argument a view position: `read_to_string(c.path.as_ref())`, `read_to_string(l.as_str())`; rustc accepts | decision 20, clauses |
| `set-at!` and `get` on a map with view keys by a lent, owned or mixed key | 9 | `E0277` without a node | `*m.get_mut(k.as_str()).unwrap() = 9`; rustc accepts | decision 20, checker-built query |
| `(->team :names (collect (split line " ")))` | 9 | n/a | the field's element follows the Field row: owned, `L0503` at the views, help `(collect (map (split line " ") str))` | Field row |
| a `def` of created text into a mixed field (probe `def.knv`) | 9 | `static MADE: LazyLock<String>` | `Cow::Borrowed(MADE.as_str())`, a `'static` read; rustc accepts | `'static` text |
| `(subs o 0 1)` on owned text | 9 | `&o[0..1]` | `&o.as_str()[0..1]`; rustc accepts | decision 20, no receiver |
| `(make "ana")`, `(make (str "u" 1))`, `(make (:name a))` | 10 | n/a | mixed field and `n`; `L0503` at `(:name a)`, help `(clone (:name a))`, which fits | three facts |
| `label` with owned, lent and literal callers | 10, 11 | `&Cow` parameter rejects `label(&o)` with `E0308` | L from the owned and lent callers: lent `&String`; `L0503` at the literal, `(str "y")`; rustc accepts `label(&o)`, `label(&u.name)` | tied `(ref a)` table |
| `(defn label (name) (tagged name "x"))` | 11 | `check` exits 0, rustc `E0277`, `E0308` | `fn label(name: &str)` with `tagged(&name, "x")`; rustc accepts `label("y")` and `label(t.trim())` | tied `(ref a)` table, no O |
| `keep`: stores `name` and passes it at a tied `(ref a)` | 12 | `fn keep(name: String)`, `tagged(&name, "x".to_string())` | owned: the same; mixed: `name: Cow<'static, str>`, `tagged(&name, Cow::Owned(...))`; literal-only: `name: &'static str`, `tagged(&Cow::Borrowed(name), Cow::Owned(...))`; rustc accepts all three | stored row wins; tied table is for parameters not stored |
| `mk` with a literal caller beside `(->user :name (str "z"))` | 12 | n/a | S flows from `a` into `:name`, mixed: `fn mk(s: &Cow<'static, str>)`; rustc accepts | caller facts flow on; L does not |
| `(defn mk (s) (->user :name (clone s)))` | 10, 11 | `fn mk(s: &String)` | `L0503` at `s`, help `(str s)`, which emits `fn mk(s: &str) -> User { User { name: s.to_string() } }`; rustc accepts | facts only; a slot with no facts passes nothing |
| `(unwrap-or (first names) "none")` over view elements | 10 | `L0401` | `names.first().copied().unwrap_or("none")`; rustc accepts | layer erasure, `.copied()` row |
| `(get m (pair "a" 1))` on `(pair str i64)` keys | 10 | `E0308` after `check` exits 0 | the literal takes the key's representation; rustc accepts each of the three | compound query |
| `(defn reset! (s) (set! s "b"))` | 10 | `fn reset(s: &mut &str)` with a written type | `&mut &'static str` or `&mut Cow<'static, str>`, no allocation; rustc accepts | `mut` row |
| `show(&owned)` over `impl Label for str` | 2 | n/a | `.as_str()` view node at the generic argument | section 1, free view |
| `if` mixing `&u.name` and `"y"`; `vec![&u.name, "a"]` | 2 | compile | unchanged: Rust coerces | free view |
| `Option<&String>` against `Option<&str>` | 2 | `E0308` | `.map(String::as_str)` | decision 13 |
| `(get m "a")` on owned keys | 2 | `m.get("a")` | unchanged | `borrow` rows |
| `(clean (as-str (str " u" 1)))` returned | 2 | `check` exits 0, rustc rejects | PR 2 waits for the steps that close it | decision 10 |
| `prelude_impls` `(impl label string ...)` | 2 | `impl Label for String` | `(impl label (own str) ...)`, same Rust | section 4, respelling |
| `loudest` with owned and view callers (`cv`, `cv2`) | 3 | `L0401` both | concrete per caller kind when callers agree; when they disagree `&[&str]`, and each owned caller gets decision 15(b)'s code | decision 15 |
| `get` on a map parameter, `sort!` and `push!` on a `mut` vector parameter, mixed callers (`gen.rs`) | 4 | n/a | concrete types, rustc accepts `m.get("a")`, `sort()`, `push("none")` on `Vec<&str>`; owned callers among disagreeing ones get decision 15(b)'s code | decision 15(b) |
| `vec![&a.name, &b.name]` into `Vec<&str>` (`lentvec`), and `push!` of a literal after a lent initializer | 4, 6 | n/a | `vec![a.name.as_str(), b.name.as_str()]`, then `v.push("x")` untyped, rustc accepts | Element row |
| `loudest(&vec![&a.name])`, `collect` of lent items, map literal with a lent key or value, a pair or `some` payload in a borrowed vector literal, `into-hash-map` (`el`, `el2`, `map2`, `pairref`, `somearg`, `intohm`, `mapval`) | 5, 6 | n/a | `.as_str()` on each stored lent value, `.map(String::as_str)` before `collect`; rustc accepts each | decision 13, element views |
| `(vec (as-str s) (as-str t))` in the `loans` fixtures | 5 | loans of `s`, `t` | respelled `(vec (ref s) (ref t))`, still loans | section 4 respelling |
| callers of a collection parameter that disagree | 5, 6 | n/a | the slot is a view; each owned caller gets the new code once, no `L0503`, independent of order | decision 15(b) |
| `(sort! names)` on `str` elements | 5 | works on `string` and `(ref str)` | through `ord`'s bare `str` row; `Vec<String>` and `Vec<&str>` sort | implementor lists |
| `(apply-name f u)` with a storing and a reading closure | 4 | n/a | the callee passes a view; the storing closure gets `L0503` in its own body | function-typed parameter row |
| `push!` onto a text local initialized with a literal | 4 | n/a | the slot is owned by the `mut` use and cannot be mixed, because `Cow::to_mut` on a borrowed literal copies it; `L0503` at the literal, with the change named | `mut` row |
| `(window (ref xs))` to a slice parameter (`sl`) | 3 | `window(&xs)` | unchanged: `viewed_items` stays | decision 5's removal list |
| `(< "b" (:name u))`, `(max "b" (:name u))` (`cmp`) | 3 | `L0401` | `"b" < u.name.as_str()`, `max("b", u.name.as_str())`, rustc accepts | comparison view node |
| `(= "a" (:name u))` | 3 | `L0401` | `"a" == u.name`, rustc accepts | `partial-eq` cross rows |
| `(map (iter names) shout)` over owned names (`fv`) | 3 | `L0401` | `.map(|argument: &String| shout(argument))`, rustc accepts | function values |
| `(push! names (str "b"))` (`strb`) | 3 | `"b".to_string()` | unchanged | `str` |
| `(trim (as-str (str " a" 1)))` lift (`s1d`) | 3 | `let owner = ...; owner.as_str().trim()` | `owner.trim()`, rustc accepts | lift |
| `(label o)`, `(trim o)`, `(area s)`, `(pr-str s)` (`d14a`-`d14d`) | 3 | `L0401` for text; `area(&s)` | text auto-borrows by "Call sites" | decision 14 |
| PR 598: `push!` pair `E0716`, swapped `L0503`, `us` `E0308`, `user`/`user`, `keep<A>` | 1, 2, 3 | reproduced on `513d3f62c` | each is an error with a fix, or concrete Rust | options |

## Differences from PR 598

Where this RFC does what PR 598 did, the reason stands on this RFC's evidence.

| Point | PR 598 | This RFC | Why |
|---|---|---|---|
| Where the representation lives | In the type, `Type::Text(TypeVar)`, bound by HM unification | An ownership fact per slot, written back as Rust-level types | Text flow is one-way and unification is symmetric; PR 598's `(&str, String) => Ok(())` arm accepts `Vec<&str>` against `Vec<String>`. A parameter's representation is its mode, which the ownership stage already decides. |
| Representations | Two, `String` and `&str` | Five: owned, view and mixed decided by the solver, and lent `&String` and lent mixed `&Cow<'static, str>` following from them | Prelude templates over owned text give `&String`, and today's output already holds it |
| Who decides | Unifier, `unification.rs` and `infer.rs` argument passing | The ownership stage only | RFC 0065 |
| Struct fields | Hidden type parameter, `User<A>` | One representation per field, decision 1 | Hand-written Rust; PR 598 accepts `(vec (->user :name "a" ...) (->user :name (str "b") ...))`, which rustc rejects with `E0308` |
| Function parameters | A view when the text variable does not reach the result; generic when it does | A view unless the body needs ownership, by modes; a collection parameter's element by decision 15 | A parameter that the body only reads is never changed by its callers. A stored parameter takes its callers' representation (1(f)), as number literal pins already let a call change a callee, and decision 15 applies the same precedent to elements |
| A literal and a created value in one place | Depends on argument order; one order emits `names.push(&"b".to_string())`, `E0716` | `Cow<'static, str>`: `Cow::Borrowed("a")` and `Cow::Owned("b".to_string())`, decision 8 | No allocation for the literal, no reference to a temporary, and no dependence on order |
| `Copy` | Derived on the generic struct | Only for written `(ref str)` fields | Decision 9 |
| Owned text in `rust` signatures | A prelude-internal `owned-text` type matched by its template text `String` | `(own str)` | A form users can write too, and no template-text match |
| `string` | A closest-name override that suggests `str` (PR 598 `expand/src/expand/names.rs:160`) | Plain `L0201`, nothing written for the removal | The owner's rule: no migration aid |
| `as-str` | Renamed to an internal `text-view` | Removed with `StringViewSearch`; the view templates are prelude rows read by type pair, decision 16 | The prelude states the Rust, as PR 598 kept it there, and no signature search finds it |

Carried over from PR 598: `string` leaves the kernel's exports while the `String` variant stays internal, `(the str x)` pins nothing, `infer` and diagnostics print `str`, `L0503` is the needs-ownership code, and the removal is `release:minor` with the prelude and fixtures migrated in the same release. PR 598's prototype cannot load its prelude (`prelude/internal/text` is not registered in `expand/src/modules/sources.rs`), so 12 CI jobs fail and none of its snapshots changes; its outputs above come from its merge binary on hand-written probes.

## Prototype

PR 1 of the order of work is the prototype, under decision 11(a). The evidence in this RFC comes from `5234c8817`'s probe binary for today's outputs, from PR 598's merge binary for option A's, and from rustc for the `E0716` and `E0308`.

## Decision

The project owner decided every question of this RFC on 2026-11-15:

- **Decision 1: (d).** A text field is owned unless every value stored in it is `'static` text; it is `&'static str` when every stored value is; it is owned when nothing stores into it; and it is `&'a str` only where the program writes `(ref str)`.
  With **(f)** on the point decision 8 reopened: A stored parameter takes its callers' representation: `&'static str` when every caller passes `'static` text, owned when every caller passes owned text, mixed when callers pass both. A read-only parameter is never changed by its callers.
- **Decision 2: (a).** `str` prints everywhere: `infer`, `knave types`, `knave doc`, the REPL and every diagnostic.
- **Decision 3: (a).** A bare `str` in a `type` signature states no representation, and `(ref str)` and `(own str)` pin one; decision 17 adds `(cow str)` as a third.
- **Decision 4: (a).** Owned text is written `(own str)`, a kernel type form valid around `str` only.
- **Decision 5: settled.** By the owner's standing rule against migration aids, `string` and `as-str` become plain `L0201` with the closest-name suggestion, and nothing else: no `foreign-names` entry, help, note, alias or hint.
- **Decision 6: (a).** A text value that needs ownership is `L0503`, with a text wording, a label on the value that made the slot owned, and a `(str ...)` help; a lent value inside a type with no free view gets a new ownership code.
- **Decision 7: (a).** `(str x)` is the value-level escape hatch: it exists and emits `x.to_string()`, so no new name is added.
- **Decision 8: (d), a new option.** A field, element, parameter, local or result slot that stores both `'static` text and created text is `Cow<'static, str>`: a literal goes in as `Cow::Borrowed` with no allocation and no written `(str ...)`, created text as `Cow::Owned` with no copy, and a read derefs to `&str`. Literal-only slots stay `&'static str` and created-only slots stay owned. RFC 0070 decision 4's `String::from` literal rule is superseded.
- **Decision 9: (c).** A struct with text fields is `Copy` only when every text field is written `(ref str)`. A mixed field, `Cow<'static, str>`, is never `Copy`, so this decision holds unchanged under decision 8.
- **Decision 10: (a).** PR 2 lands after the loan rule's escaping-borrows pull request and RFC 0065's step 3, regions, which close the class of created text passed to a view parameter whose view result is returned or stored. PR 1 can land at any time. This places PR 2 later than the first plan did.
- **Decision 11: (a).** PR 1 is a prototype and lands before PR 2: the solver over today's types behind the hidden `knave check --print-text`, with no output change, reporting each slot's representation, every `&String` parameter, every mixed slot and every `L0503` the rules would give, and the solve's cost.
- **Decision 12: (a).** This RFC's pull requests land after RFC 0074 and independent of RFC 0075; whichever of this RFC and RFC 0075 lands second rebases the shared `lent_temporaries` test, and PR 5 removes RFC 0070's `mismatch-help` row for `string` if RFC 0070's build has added it.
- **Decision 13: (a).** The compiler writes the free view: `.map(String::as_str)` through an `option`, a `result` or an `iterator`, and `.as_str()` on each lent value stored into a view element or payload; for mixed text, `.as_ref()` and `.map(|text| text.as_ref())`. The templates are prelude rows under decision 16.
- **Decision 14: (a).** Today's "Call sites" auto-borrow covers text through this RFC, PR 3 adds an `L0501` note, for every type, that names a callee's inferred parameter and the use that made it owned, and issue #590 closes with this RFC.
- **Decision 15: (d).** Callers decide a collection parameter's element when they agree. When they disagree and the body only reads elements, the function is generic over `AsRef<str>`; when they disagree and the body writes, sorts or keys a map with the elements, each owned caller gets the new ownership code. A planned `HasField` accessor for a text field returns a `&str` view whatever each struct stores, the same read through a trait, so the two agree when it lands.
- **Decision 16: (a).** The view rows live in the prelude through the new kernel declaration form in `prelude/text.knv`. One table serves three readers: the compiler places view and wrap nodes and writes their Rust from it; under RFC 0063's decision 1A each row carries a native clause whose meaning is the identity, because the interpreter holds one runtime text form; and the dynamic-library boundary of RFC 0064 reads the same rows to convert text to the Rust type a compiled library expects.
- **Decision 17: (a).** The mixed representation is written `(cow str)`, a kernel type form valid around `str` only, as `(own str)` is. The prelude's rows name it, and a user may pin a slot mixed with it; `docs/language.md: Kernel forms` records it beside `(own str)`.
- **Decision 18: (a).** A mixed value or a literal that reaches a slot owned by an in-place change, a `push!` onto text or a `set!` of part of it, is `L0503` at the value, with the help `(str x)` and a label that names the change; the compiler never writes `Cow::to_mut`. A `set!` that replaces the whole value owns nothing.
- **Decision 19: the recommended one rule, the tied `(ref a)` table.** For a parameter the body does not store, passed at a `(ref a)` whose `a` another argument ties by unification, the facts of the call's `a` decide: lent `&String` when O holds with L, or without S; lent mixed `&Cow<'static, str>` when O and S hold without L; a view, passed as `&name`, when O does not hold; and `L0503` with `(str s)` when `a` stays a view and the trait's rows exclude a view. A stored parameter takes its own row and is passed as `&name`. `label` keeps `&String`, and `mk` and `dup` are `L0503` with `(str s)`.
- **Decision 20: (a).** A view node goes on every non-receiver text argument at a view position, unless the argument is already a view: a `(ref str)` position of a `rust` form, the text query of a template the checker builds (the map query of `get` and `set-at!`, and each text slot of a compound query by the key's representation), and the argument of a clause chosen for text, such as `slurp`'s. It never goes at a `(ref a)` whose variable another argument ties by unification. A receiver, as `TemplateBorrows::receives` records it, takes no node.

The status stays Proposed. As for RFC 0074, the owner accepts this RFC after PR 1, the prototype of decision 11, reports what the solver decides over today's fixtures and corpus and what the solve costs; the RFC then becomes Accepted with these decisions, or the report reopens the ones it contradicts.

## Rejected options and why

- **A, the representation in the type:** unification cannot express a one-way flow, the prototype accepts programs rustc rejects, and hidden type parameters are not the Rust a person writes.
- **B, `str` as owned with `(ref str)` as the view:** two text types remain visible in every printed type, and a mismatch between a literal and created text is a type error between two spellings. Option C with decision 8 accepts them with a mixed slot.
- **D, `Cow` everywhere:** a tag, a branch and a lifetime on every text value at run time. The owner's decision 8 uses `Cow<'static, str>` only in slots that mix `'static` and created text, with no lifetime.
- **E, always owned:** an allocation at every literal.

## Rule

[docs/language.md: Types](../docs/language.md#types), [Kernel forms](../docs/language.md#kernel-forms), [Literals](../docs/language.md#literals), [Type texts](../docs/language.md#type-texts), [Standard types](../docs/language.md#standard-types), [Ownership](../docs/language.md#ownership), [Structs](../docs/language.md#structs)
