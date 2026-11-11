# RFC 0070: Names from other languages

**Status:** Accepted
**Date:** 2026-11-11

## Problem

Language models write knave from what they know of Rust, Python, JavaScript, Clojure and the other Lisps. They know those languages far better than knave, so they call functions that knave names differently: `length` for `len`, `into-iter` for `iter`, `is-some` for `some?`, `push-str!` for `push!`. A person who comes to knave from one of those languages makes the same mistakes.

Today each such call reports `L0201` with a suggestion ranked by edit distance alone ([docs/diagnostics.md: Tone](../docs/diagnostics.md#tone)). A synonym is rarely a near spelling, so the error usually says only that the name is unknown:

```text
error[L0201]: cannot find `length` in this scope
 --> t.knv:1:25
  |
1 | (defn main () (println (length (vec 1 2 3))))
  |                         ^^^^^^ not found in this scope
```

Worse, a short foreign word often lies one or two edits from an unrelated knave name, and the suggestion then points the wrong way: `for` suggests `or`, `return` suggests `recur`, `#t` suggests `at`, `to-string` suggests `docstring`. The reader, or the model, has to search the prelude, and a model often guesses again with a second foreign name. Appendix B lists 35 foreign names that models wrote in recorded sessions. Of their `L0201` reports, only `push`, `String` and `'origin` got a help that named the knave form: `push!`, `string` and `(quote origin)`.

A second case is a foreign name that is also a knave name with another meaning. Clojure's `some` tests whether any item passes a test. knave's `some` wraps a value in an `option`. `(some items (fn (x) (> x 1)))` fails with `L0301` "no rule of `some` matches this form", and the help shows `(some value)`, with no word about `any?`.

The language cannot solve this today for two reasons:

1. `AGENTS.md` forbids the compiler to state a name or a fact that the prelude states. A table of synonyms in the compiler would restate the prelude's names. `ExpandProblems::CODE_REPLACEMENTS` in `expand/src/expand/problems.rs` is such a table today, for macro code.
2. No prelude form says "this definition is what language X calls Y", so there is nothing for a diagnostic to read.

This RFC builds on the owner's decision that `count` leaves the prelude and `len` serves collections and iterators. Every example below assumes it: `count` is a foreign name, from Clojure and Rust, with no knave binding. The order matters: the change that removes `count` lands before this RFC's build. If it did not, rule 5 would judge `:rust (count)` and `:clojure (count)` as bound entries, and `(count v test)` would get its hint at `L0402` instead of `L0201`. If this build comes first, the `count` rows of Appendix C1 wait for that change.

## Options

Every option shares the diagnostics of the Diagnostics section. The options differ in where the synonyms are stated.

### A. A `foreign-names` form beside the definition

The prelude states each definition's names in other languages with a form that sits next to the definition, as `header-shape` and `document` do. Every example form in this RFC uses entries of Appendix C1, the list that ships first. The form names the definition, a short purpose phrase, and the names by language:

```lisp
(foreign-names len "get the length"
  :help "to count the items that pass a test, write `(len (filter items test))`"
  :rust (count)
  :javascript (length size)
  :clojure (count)
  :common-lisp (length)
  :scheme (length vector-length string-length)
  :racket (length vector-length string-length))

(foreign-names vec "build a vector"
  :clojure (vector)
  :common-lisp (vector)
  :scheme (vector)
  :racket (vector))
```

The form emits nothing:

```rust
// no Rust: the form adds notes to its module, as `header-shape` does
```

The form adds one note per foreign name to its module's notes under `foreign-names`, with `add-module-note`. A note holds the foreign name, the language keyword, the purpose phrase, the optional help, and the symbol of the definition, so the note refers to a binding, not to a spelling. `docs/language.md` gets a section that states the form, as it states `header-shape`.

Rules of the form, each checked when the form expands:

1. **The definition must be bound where the form is written,** as a `header-shape` name must. A misspelled one is `L0201`. `prelude/prelude.knv`, which exports the kernel, may name kernel forms, as it may in `document`. Any other module states foreign names only for terms it defines.
2. **`(type name)` names the type binding.** A bare name names the value binding, or the type binding when no value binding exists, as a `doc` lookup does.
3. **Each language keyword is one that `prelude/foreign` declares** with `(foreign-language :common-lisp "Common Lisp")`. The display name in a diagnostic comes from that declaration, so the compiler states no language name. There is no `:knave` language.
4. **A foreign name is written as the language's standard library writes it,** with two changes: `_` becomes `-` and `::` becomes `/`, the separators knave writes. `to_string` is written `to-string`, `Box::new` is `Box/new`, `console.log` is `console.log`, `std::process::exit` is `std/process/exit`. The reader keeps a program's `::` as written, so the matcher reads it the same way (see "How a name matches"). Nothing else is allowed: no added `!` or `?`, no other transliteration. Matching against what a program writes is in "How a name matches" below.
5. **Bound or unbound is decided against the names programs see by default,** the exports of `prelude`, and in the value namespace only. `:clojure (count)` beside `len` lives in `prelude/sequence`, which imports nothing from `prelude/iterator`, but the entry is judged against `prelude`'s exports, so its kind does not depend on the module's imports. A trait such as `eq` or `hash` is in the type namespace, so Common Lisp's `eq` and Racket's `hash` are unbound entries. A user module's entries are judged against that module's own scope.
6. **A foreign name that equals a definition of the same name in `prelude` and means the same,** such as Python's `len`, is not written. The form states only the names that differ.
7. **`:help "text"` states a fix that is not a name swap.** It serves two cases:
   - An entry with no purpose phrase, for a statement or a loop that knave writes as another form. The help shows the text and no replacement:

     ```lisp
     (foreign-names fn :help "a body's last form is its value, so write the value last"
       :rust (return) :python (return) :javascript (return))

     (foreign-names for-each :help "`for-each` calls a function on each item, as in `(for-each (range 0 3) (fn (i) ...))`"
       :rust (for) :python (for) :javascript (for) :clojure (doseq dotimes))

     (foreign-names loop :help "`loop` with `recur` repeats while a test holds"
       :rust (while) :python (while) :javascript (while))
     ```

   - An entry with both a purpose phrase and a `:help`. The argument count chooses between the two: a call whose count fits the definition's header gets the replacement, and any other call gets the `:help` text. The `len` entry above shows the replacement `len` for `(count items)` and the `:help` text for `(count items test)`, the habit of Rust's `filter(..).count()` and Python's `list.count(x)`. The header is the one `doc-header` returns, with a trailing `...` read as any number. A header that lists no parameters, as the bare name of an undocumented `defrule`, fits every call.

   An entry with a purpose phrase and no `:help` always shows the replacement, whatever the count. So `fold`'s entry on `reduce` replaces `(fold v 0 +)` with `(reduce v 0 +)`, although `reduce`'s header, `(reduce items step)`, documents only its two-argument form. The count never turns a replacement into no help.

8. **A definition may carry several `foreign-names` forms.** A foreign name whose arguments come in another order needs its own form with a `:help`, so the replacement of the other languages stays. One foreign name of one language goes in one form only. Entries of several languages that share a spelling and a target merge into one help that names every language, as in ``which JavaScript and Common Lisp call `length` ``, so they must all show the replacement or all carry the same `:help`, and must name the same `:given` type or none. The expander reports a second entry that breaks this, at its form:

   ```lisp
   (foreign-names any? "test whether any item passes"
     :rust (any))

   (foreign-names any? :help "write `(any? items test)`, with the items first"
     :javascript (some) :clojure (some))
   ```

9. **`:given type` narrows an entry for a bound name** to an `L0401` mismatch whose found type is an application of that type binding. A `:given` entry stays quiet at `L0301`, `L0402` and `L0404`, which have no found type. Diagnostics, "A foreign name that is a knave name", states it.

### B. A clause in every definer and in `document`

`defn`, `defmacro`, `defstruct`, `defenum`, `deftrait`, `deftype`, `defrule`, `def` and `document` take an optional `:also` clause after the header:

```lisp
(document (len collection) :also (:clojure (count) :javascript (length size))
  "Returns the number of items of a collection or an iterator as a [usize].")
```

```rust
// no Rust
```

### C. A line in the docstring

Each docstring ends with a fixed Markdown line, which the compiler parses:

```lisp
(document (len collection)
  "Returns the number of items of a collection or an iterator as a [usize].

  Other names: `length` (JavaScript, Common Lisp), `count` (Clojure, Rust).")
```

```rust
// no Rust
```

### D. A table in the compiler

A constant in `ExpandProblems` maps each foreign name to the knave name, as `CODE_REPLACEMENTS` does for macro code.

```rust
const FOREIGN_NAMES: [(&str, &str); 120] = [("length", "len"), ("into-iter", "iter"), /* ... */];
```

### Rust spellings, a rule beside every option

A model that knows Rust writes Rust's names back into knave, in snake case or kebab case, sometimes with knave's `!` or `?` added: `is_some`, `is-some`, `push-str!`, `HashMap`, `PartialEq`. Two facts let the compiler match these without a list, one for each namespace.

**Values: the compiler's own mangling.** [docs/language.md: Names in Rust](../docs/language.md#names-in-rust) states how the compiler turns a knave name into a Rust identifier: `admin?` becomes `is_admin`, `add-item!` becomes `add_item`, `make-point` becomes `make_point`. The compiler owns this rule and the emitter applies it. For an unbound value name, the compiler mangles it and every visible value binding by that rule. If exactly one binding mangles to the same identifier, that binding is the suggestion: `is-some` and `is_some` both mangle to `is_some`, as `some?` does. When two bindings share the identifier, the rule suggests nothing. Rust names with no knave twin, such as `push_str`, come from rule 4 of option A instead: the entry `:rust (push-str)` beside `push!` matches `push-str!` and `push_str`, because the match compares mangled forms (see "How a name matches").

**Types: the prelude's Rust templates.** A prelude type does not take its Rust name from the mangling rule. `vec`, `hash-map` and `box` are `rust-type` forms whose templates the prelude writes, such as `"Vec<{0}>"` (`prelude/types.knv`), and `partial-eq` is `(rust-trait partial-eq "PartialEq")` (`prelude/number.knv`). The mangling rule would give `Vec2` and `PartialEq2`, because a program type that hides a Rust prelude name takes the suffix `2`. So for an unbound type name, the compiler reads the Rust template of each visible `rust-type` and `rust-trait` binding, and the Rust path of each `enum` that names one, as `option` and `result` do, takes the template's last path segment before its first `<` or `{`, after a leading `impl ` or `dyn `, and compares that with the name: `HashMap` finds `hash-map`, `PartialEq` finds `partial-eq`, `Option` finds `option`. A program type made by `defstruct` or `defenum` has no template and takes its Rust name from the mangling rule, so `Point` finds `point` by mangling.

The type half reads Rust text that the prelude states. That is allowed, because it reads the prelude instead of restating it: a renamed `rust-type` changes the suggestion with no compiler change. It does parse the head of a Rust path, which the emitter already treats as text to inline. The RFC limits it to the head: a path segment of letters, digits and `_`. `iterator`'s template, `"impl Iterator<Item = {0}>"`, gives `Iterator` once the leading `impl ` is dropped.

Today `String` and `Vec` already get `string` and `vec`, by edit distance. The rule earns its place for `HashMap`, `PartialEq`, `is-some`, `is_empty` and `push-str!`, which get no help today.

## Diagnostics

The wording follows [docs/diagnostics.md](../docs/diagnostics.md): rustc's layout, a plain title, a `help:` with the replacement text, and the user's names. The examples use the diff layout the compiler prints today.

### An unbound foreign name

`L0201` keeps its title and label. The help names the purpose, the knave name and the language, and shows the replacement:

```text
error[L0201]: cannot find `length` in this scope
 --> src/main.knv:3:12
  |
3 |   (println (length items))
  |             ^^^^^^ not found in this scope
  |
help: to get the length, knave uses `len`, which JavaScript calls `length`
  |
3 -   (println (length items))
3 +   (println (len items))
  |
```

Without a purpose phrase the help is ``knave calls JavaScript's `length` `len` ``. A Rust spelling reads ``help: `is_some` is the Rust name of `some?` ``, with the replacement. An entry whose help applies, by rule 7 of option A, shows its text with the language and no replacement:

```text
error[L0201]: cannot find `return` in this scope
 --> src/main.knv:2:3
  |
2 |   (return 1))
  |    ^^^^^^ not found in this scope
  |
help: knave has no `return`, which Rust uses: a body's last form is its value, so write the value last
```

The name of the language matters to the reader. It tells a person why the name looked right, and it tells a model which of its habits to correct, so the next guess comes from knave instead of from the same language.

### How a name matches

A program's unbound name matches an entry when the two are equal after both are read the same way: `_` as `-`, `::` as `/`, and for a `:rust` entry that holds a letter, both mangled by the Names in Rust rule, so that a `!` or `?` the model added does not matter. An operator entry, such as `%`, `!`, `==`, `!=`, `&&` or `||`, is compared as written, because the mangler spells punctuation out, and a program's unbound `percent` or `bang` must not match `%` or `!`. Qualified names match whole: `console.log` and `std::process::exit` are one symbol each, as the reader reads them, so their entries are written whole, and `String::from` matches the entry `String/from`.

A foreign name counts only when its definition is visible where the error is, through the same `visible_name_bindings` and `is_public` filter that the spelling suggestion uses today (`expand/src/expand/names.rs`). Every name in the first build is re-exported by `prelude`, so every program sees it. A hint never names a definition the reader cannot call.

### How the hints combine with the spelling suggestion

`L0201` shows at most one suggestion help. The order is:

1. A declared foreign name, because it is an exact match that the prelude states.
2. A Rust spelling, because it is an exact match under a stated rule.
3. The closest name by edit distance, as today.

The first rule that finds a candidate decides, and the others add nothing, so no error shows two competing fixes. This order removes the wrong helps that the probes found: the entries for `for`, `return`, `#t` and `to-string` decide before edit distance offers `or`, `recur`, `at` and `docstring`.

When rule 1 finds several knave definitions for one foreign name, because two entries of different languages share the spelling, the help lists them in name order with no replacement text, each with its language. A replacement shows only when there is one candidate. No row of the first build has two candidates.

**Should a spelling suggestion be dropped when it names a form of a different kind?** In one case, yes, and otherwise no:

- **Drop it where the position rules the candidate out.** A name in a value position that is not the head of a call cannot be a kernel form or a macro without a value form. So `(println retrun)` must never suggest `recur`. The compiler happens not to today, so this rule is preventive: it states the guarantee instead of leaving it to the distance limit. The expander knows both the position and the candidate's kind, so the rule states no name. It keeps the existing namespace filter, which already keeps type candidates out of value positions.
- **Keep it at the head of a call.** An unbound name has no kind of its own, so at the head of a call the compiler cannot tell that `for` meant iteration rather than a typo of `or`. Any kind rule there would also drop good suggestions, such as `print` for `prnt`. The declared entries handle the common cases, and the first build lists the loop and statement words of each language so that none reaches edit distance.

### A struct name at the head of a call

[docs/language.md: Bodies](../docs/language.md#bodies) adds a note to an unbound value name that names a type, except for a struct name at the head of a call, "because there it reads as a call of a constructor". This RFC fills that gap and only that gap. `(point 1.0 2.0)` after `(struct point (x f64) (y f64))` suggests `print` today.

When an unbound name at the head of a call resolves as a struct type, the help builds the struct with the kernel `new` form, before the spelling rule runs: `(new point (x 1.0) (y 2.0))`, with the struct's fields in order and the call's arguments as their values. When the call has another number of arguments than the struct has fields, the help shows the fields with the placeholder `value`, as `L0334` does for a `let`. The rule reads the struct's kernel definition and the kernel form `new`, so it states no prelude name and works the same for every struct, whether `defstruct` or the kernel `struct` made it.

The help does not name the constructor that `defstruct` writes, such as `->point`. The expander reports `L0201` before any type is known, so it could find that constructor only by knowing how `defstruct` builds it, which `AGENTS.md` forbids. The `new` form is one uniform rule, and the model guide states the `->name` constructors for a reader who wants them.

### A foreign name that is a knave name

The owner decided this case. When a foreign name is bound in knave, the call uses the knave definition. If the call works, no hint shows, because the program means what knave says. If the call fails, the error adds the hint.

The hint is stated in the prelude, beside the definition it points to, by the same `foreign-names` form. The second `any?` form under rule 8 names `:clojure (some)`. `some` is bound in `prelude`'s exports, so by rule 5 that entry is bound. Its note holds the binding of `some`.

A misused prelude name often fails before the type stage, and a prelude name is often a macro whose expansion calls another binding. So the hint attaches at two stages, keyed on bindings both times:

1. **When the expander lowers the program,** it reads every `foreign-names` note of the loaded modules once, as `find-doc` reads docstring notes. For each entry whose foreign name is bound, it records `binding -> (target binding, language, purpose, help)` in a `ForeignNameHints` table, keyed by binding identity.
2. **In the expander:** any `L0301` at a call of a macro whose binding is in the table adds the hint. Every prelude `fail` reports `L0301`, so the stage reads the code and the binding, never the message. This is the path of `some`, a `defrule-with-value`, which fails in expansion on two arguments.
3. **In the type stage:** the expander hands the table to the type stage with the program, as it hands macro origins today. A `MacroOrigin` holds the macro's name, its definition span, the call site and whether it came through the prelude, but no binding (`expand/src/expand/macro_call.rs`). The build records the macro's binding beside the name. When the type stage reports `L0401` mismatched types, `L0402` a wrong argument count or `L0404` a missing trait, and the primary label is on a call or on an argument of a call, it looks up two bindings: the callee's, and the binding of the macro whose expansion wrote the call, if any. Either one in the table adds the hint, subject to the entry's `:given` clause, below. `map` is a plain `def`, so its callee binding finds the entry. A macro such as `->` that writes the call is found through its origin.
4. **Neither stage compares a name.** Each reads binding identities and the strings of the entry.

A bound name is usually right, and a failing call of it is usually some other mistake. So an entry for a bound name may narrow when it fires with `:given type`: the type stage adds the hint only to a mismatch whose found type is an application of that type binding. `(foreign-names map-some "apply a function to an option's value" :given option :rust (map))` fires on `(map (first scores) double)`, which finds `(option (ref _))`, and stays quiet on `(map double scores)`, the Clojure order, which finds a `fn`. The checker compares type bindings, not names. `L0402` and `L0404` report no found type, so a `:given` entry stays quiet at them and at `L0301`. An entry without `:given` fires on every failing call, as the owner's rule says.

```text
error[L0301]: no rule of `some` matches this form
 --> src/main.knv:4:3
  |
4 |   (some items (fn (x) (> x 1)))
  |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ here
  |
  = help: write it as `(some value)`
note: `some` wraps a value in an `option`
help: Clojure's `some` is `any?`: write `(any? items test)`, with the items first
```

```text
error[L0401]: mismatched types
 --> src/main.knv:4:8
  |
4 |   (map (first scores) (fn (s) (* s 2)))
  |        ^^^^^^^^^^^^^^ expected `(iterator _)`, found `(option (ref _))`
  |
note: `map` returns an iterator over a function's result for each item
help: to apply a function to an option's value, knave uses `map-some`, which Rust calls `map`
  |
4 -   (map (first scores) (fn (s) (* s 2)))
4 +   (map-some (first scores) (fn (s) (* s 2)))
  |
```

The note is the first sentence of the bound definition's docstring, with its first letter lowercased after the name and each bracket link rendered as code: `some`'s summary "Wraps a value in an [option]." becomes ``wraps a value in an `option` ``. A definition without a docstring gives no note, and the help stands alone. The help follows the same rule as an unbound entry: the replacement when the entry has no `:help`, and when it has one, the replacement only for a call whose argument count fits the target's header.

Two notes on this case:

- **A working call gets no hint.** `(some 3)` and `(map (iter v) f)` are correct knave and show nothing. Under the old prelude, `(count (vec 1 2 3))` was the motivating example. With `count` removed, `count` is an unbound foreign name, and both `(count v)` and `(count v test)` get the `len` entry's hint at `L0201`.
- **A hint does not replace the error.** The title, the label and any existing help stay, so the reader still learns what failed. The hint is one note and one help at the end.

### Old knave spellings stay out

The owner's rule against migration aids covers old knave spellings: a name that an earlier knave release used, such as `append!` or `box-new`. This RFC keeps the two apart by these rules:

1. **An entry names a language other than knave.** There is no `:knave` keyword, and the help always names the language. No hint says "renamed", "formerly" or "deprecated".
2. **An entry needs a source.** The pull request that adds a foreign name links the standard library page of that language where the name appears, such as Python's `len` or Clojure's `count`. A name with no such page is not a foreign name.
3. **An entry is spelled as rule 4 of option A says, and nothing else.** `Box::new` is `Box/new`, never `box-new`. So an old knave spelling can only match an entry when another language's library writes the same name.
4. **A name that only old knave used gets nothing.** `append-char!`, `box-new`, `sort-descending!` and `lookup`, which sessions probed after RFC 0046 removed them, have no source in another language, so they keep the plain `L0201`.
5. **A name that is both gets the foreign hint, for the foreign reason.** `to_string`, `fold`, `values`, `any` and `all` are Rust names and old knave names. Each gets a hint because Rust uses it, and the help says so. When knave drops a name in the future, that change adds no entry unless another language's library gives a source. `count` leaves the prelude by the owner's decision, and gets entries because Clojure and Rust use it, not because knave did.

## Other mistakes a diagnostic could catch

Each row recognizes a known form from another language and says the knave form. They are ranked by evidence: the first build ships the rows marked **1**. The others are recognizers in the compiler that the evidence does not yet pay for, and each adds code to the front end.

| First build | Mistake | Today | Proposed addition |
|---|---|---|---|
| **1** | `return`, `for`, `while` | `L0201`; `for` suggests `or`, `return` suggests `recur` | `:help` entries (rule 7 of option A). Prelude only. |
| **1** | Rust predicates and types, `is-some`, `is_empty`, `HashMap`, `PartialEq` | `L0201`, no help. `String` and `Vec` already get `string` and `vec` by edit distance | The Rust spelling rule. |
| **1** | Rust parameter types, `(defn double (x: i64) ...)`, and commas, `(defn reducer (action, state) ...)` | `L0201` on each use of `x` in the body, no help, and nothing at the definition, because `x:` and `i64` are two parameters. For the comma, `L0201` with the help `action,`, the broken binder itself | One rule: when an unbound name plus a trailing `:` or `,` is a binder in scope, the help points at that binder and shows the list fixed, `((x i64))` or `(action state)`. It reads binders and the type namespace. The comma help is wrong today. |
| **1** | `format`, `format!` | `L0201`, no help | `:help` entry on `str`: "`str` joins its parts: write `(str \"hello \" name)`". Not a replacement, because the arguments differ. Prelude only. |
| **1** | A placeholder in `println`, `(println "x = {}" x)` | Nothing: it runs and prints `x = {}3` | A warning from the `println`, `print`, `eprintln`, `eprint` and `str` macros, with the lint name `format-placeholder` under [RFC 0069](../rfcs/0069-lint.md). It runs during expansion, so `knave check` shows it. The Decision section states it. |
| **1** | A struct name at the head of a call | `L0201`, suggests `print` for `point` | The rule in Diagnostics. |
| **1** | `#t`, `#f` | `L0201`; `#t` suggests `at` | Entries on `true` and `false`, `:scheme (#t #f) :racket (#t #f)`. The reader accepts `#` in a symbol. Prelude only. |
| **1** | A literal where a `string` is expected, `(->user "ana")` | `L0401` "expected `string`, found `str`", no help | Accepted: a string literal takes `string` from context. A `(ref str)` variable there keeps `L0401` and gets the help `(str name)`. The Decision section states both. |
| | A binding named `mut`: `(let (mut count) 0)` and `(let ((mut count) 0) ...)` | Before `count` leaves the prelude, the first binds `mut` to the prelude function `count` and reports `L0407` with the help `(the (fn ((iterator _)) usize) count)`, which is wrong. After it lands, `count` is unbound and the first reports `L0201` on `count`. The second reads as the name `mut` with the type `count` and reports `L0201` "cannot find type `count`", no help. | When a `let` or `local` binds the name `mut`, the error adds the note "knave infers `mut`" and the help `(local count 0)`. |
| | Clojure's argument order, `(map f coll)` | `L0401` mismatched types | When a function whose first parameter is an iterator receives a `fn` first and an iterable second, the help shows the swapped call. The order is a fact of the signatures, which the checker has. |
| | `&x` and `&mut x` | `L0201` on `&scores` with the help `scores` | Keep the replacement, and add the note "knave infers borrows; write `(ref scores)` only to force one" when the name starts with `&` and the rest resolves. |
| | `p.x`, field access | `L0201`, no help | When an unbound name splits at one `.` into a local and a field of its struct, the help shows `(:x p)`. |
| | A Clojure `cond`, `(cond test value ... :else value)` | `L0301`, with the shapes of `cond` | A rule in `cond`'s `defrule` for the flat shape, failing with the clauses paired. Prelude only. |
| | `=>` in a `match` arm | `L0210` and `L0201` | When a `match` arm list holds `=>`, the help shows the arm as `((some v) v)`. The kernel `match` owns this error. |
| | Rust generic types, `Vec<i64>`, `&str` | `L0201` "cannot find type `Vec<i64>`", no help | Follow-up. It needs a Rust type parser in the compiler, and a type with a space, `HashMap<String, i64>`, reads as two symbols that the rule must rejoin. One probe and no session evidence. |
| | Rust and Lisp character literals, `'a'`, `#\a` | `'a'`: `L0201` with "there is no quote shorthand" and the wrong fix `(quote a')`. `#\a`: `L0201`, no help | When the name is `'x'` with one character between quotes, or starts with `#\`, the help shows `(char "a")`. |
| | `shape::circle` | `L0201` with the help `shape/circle` | Already right by edit distance. |
| | `[ ]` parameters, struct fields as pairs, top-level expressions, `defn` in a body, `main` that returns a value | `L0106`, `L0301`, `L0207`, `L0208`, `L0455`, each with help | Already caught. |

Names with no knave way at all today, such as `panic`, `contains?`, `join`, `starts-with?`, `hash-set`, `product`, `inc`, `second` and `pop-front`, are demand for prelude functions, not for hints. A hint that says "knave has no `contains?`" invites the next guess. `return`, `for` and `while` differ: knave has a way to do each, in another form, so they get `:help` entries. The Decision section records which ones knave adds.

## Documentation page for the model guide

A page `docs/writing-knave-with-a-language-model.md`, for people who let a model write knave and for the model to read. It should hold:

1. **What a model gets wrong and why.** One paragraph: it reaches for Rust, Python, JavaScript and Clojure names and forms.
2. **The loop that works.** Run `knave check` after each change, read the `help:` line of each error before guessing again, and use `knave doc <term>`, `(apropos text)` and `(find-doc text)` to find a name. With this RFC, `find-doc` also searches the `foreign-names` notes, so `(find-doc length)` finds `len`.
3. **A table of foreign names by language.** Generated from the `foreign-names` notes, as `knave doc --write --prelude` writes the prelude pages, so the page never disagrees with the prelude.
4. **The syntax differences a model must know**, each with a wrong line and the right line: no `[]`, `{}`, `'` or commas; `(char "a")`; keyword struct fields; `local` for a step and `let` for scoped bindings; no top-level expressions; `main` returns `unit` or a `result`; no format strings.
5. **Argument order.** knave follows Rust's method order: the collection comes first, as in `(map (iter v) f)` and `(reduce v 0 +)`, the opposite of Clojure.
6. **A snippet to paste into a project's `AGENTS.md` or `CLAUDE.md`** that points the model at `knave check`, `knave doc` and this page.

## Prior art

- **Python** suggests a close name since 3.10: `NameError: name 'schwarschild_black_hole' is not defined. Did you mean: schwarzschild_black_hole?` ([What's New in Python 3.10: Better error messages](https://docs.python.org/3/whatsnew/3.10.html)). Python 3.12 adds suggestions that are not spellings: `NameError: name 'sys' is not defined. Did you forget to import 'sys'?` and, inside a method, `Did you mean: 'self.blech'?` ([What's New in Python 3.12: Improved Error Messages](https://docs.python.org/3/whatsnew/3.12.html)). The second step is the one this RFC takes: a suggestion from knowledge of the language, not from edit distance.
- **Rust** ranks similar names by edit distance in rustc's name resolution, as knave's `SimilarName` does. Its standard library also declares other languages' names on its own definitions with the internal attribute `rustc_confusables`: `Vec::push` carries `#[rustc_confusables("push_back", "put", "append")]` in `library/alloc/src/vec/mod.rs`, and rustc reads the attribute to suggest `push` for `push_back`. This is the design of option A: the library states the synonyms beside each definition, and the compiler reads them.
- **Elm** names the operators of other languages in its error for an unknown operator. For `!=` the report is titled `UNKNOWN OPERATOR` and says "Elm uses a different name for the “not equal” operator: ... Switch to (/=) instead."; `===` suggests `==`, and `%` gets "Elm does not use (%) as the remainder operator" with links to `remainderBy` and `modBy` ([`compiler/src/Reporting/Error/Canonicalize.hs`, case `NotFoundBinop`](https://github.com/elm/compiler/blob/master/compiler/src/Reporting/Error/Canonicalize.hs)). Elm states the table in the compiler, which `AGENTS.md` rules out for knave, but its tone, naming the habit and the fix, is the model for the help lines above. Its `%` case is the model for `:help` entries: when the fix is not a name swap, say what to do instead.

## Recommendation

Option A, the `foreign-names` form, with the Rust spelling rule beside it.

- **It obeys "small compiler, big prelude".** The prelude states every synonym, beside its definition. The compiler states only its own mangling rule, reads the Rust templates the prelude writes, and words the diagnostics.
- **It covers definitions that no definer writes,** such as `rust` definitions and kernel forms, because it names a binding, as `header-shape` and `document` do. Option B would need a clause in nine forms and still miss the kernel forms.
- **It keys notes by binding,** so one statement serves both cases: an unbound foreign name at `L0201`, and a bound one at `L0301` in expansion or at a type error. No stage compares a name, and a macro-backed name such as `some` is found through its binding or its macro origin.
- **It serves `find-doc`, the documentation pages and the model guide** from the same notes.
- **It keeps old knave spellings out by construction,** because every entry names another language, needs a source, and is spelled as that language writes it.
- **It removes the wrong helps the probes found,** `or` for `for`, `recur` for `return`, `at` for `#t`, `docstring` for `to-string` and `print` for a struct name, because a declared entry or the struct rule decides before edit distance runs.

The first build ships the form, the Rust spelling rule, the struct rule, the binder rule for `x:` and `,`, the `foreign-names` entries of Appendix C1, and the hint path at `L0201`, `L0301`, `L0401`, `L0402` and `L0404`, the `:given` clause, the `format-placeholder` warning, the string literal rule, and the `mismatch-help` form with its `(str name)` entry. The other rows of "Other mistakes" are follow-ups, each with its own evidence. Moving `CODE_REPLACEMENTS` into a prelude form beside the `code` module is a follow-up too: it states a fact of the prelude in the compiler today, but it is a phase hint, not a foreign name.

## Decision

The owner accepted option A with the Rust spelling rule, and decided each question the design left open.

1. **A foreign name that is a knave name hints only when its call fails.** A working call shows nothing. A failing call adds the note and the help, at `L0301` in the expander and at `L0401`, `L0402` and `L0404` in the type stage, as Diagnostics states.
2. **`:given` narrows that hint, and ships in the first build.** An entry with `:given type` fires only on an `L0401` whose found type is an application of that type, and stays quiet at the other codes. `map-some`'s entry for Rust's `map` takes `:given option`, so Clojure's `(map f coll)` gets no `map-some` hint. An entry without `:given` keeps rule 1.
3. **`(println "x = {}" x)` warns, in the first build.** The `println`, `print`, `eprintln`, `eprint` and `str` macros call `warn` with `(:code format-placeholder)` when a literal argument holds `{}` or `{:?}` and that many arguments follow it. The warning differs from the rejected hint on working code: its output, `x = {}3`, is almost never what the writer meant, and under [RFC 0069](0069-lint.md) a project's `lints` list silences it. It lives in the macros rather than in `prelude/lint`, because RFC 0069 runs `prelude/lint` only under `knave lint`, and a `warn` during expansion reaches `knave check`, the command the model guide's loop runs. `warn` resolves the lint name through `CodeStage::LINTS`, so the build adds a `FORMAT_PLACEHOLDER` constant in `expand/src/diagnostic/code.rs`, with the next free `L08xx` code when the build lands, in the order RFC 0069 assigns codes.
4. **A string literal takes its type from context, as a number literal does.** A literal where a `string` is expected emits `String::from`, so this program is accepted:

   ```lisp
   (defstruct user :name string)

   (defn main ()
     (local u (->user "ana"))
     (println (:name u)))
   ```

   ```rust
   let u = make_user(String::from("ana"));
   ```

   The literal follows the rules of [docs/language.md: Literals](../docs/language.md#literals) for number literals: a call or a use pins it to `string`, the pin passes through unannotated functions, so `(defn default-name () "ana")` used only where a `string` is expected emits `fn default_name() -> String { String::from("ana") }`, and a literal still open after pinning is a `(ref str)`.

   The rule covers literals only. A `(ref str)` variable where a `string` is expected keeps `L0401`, and the error adds the help `(str name)` with the variable's name. The help text is stated in the prelude beside `str`, with a `mismatch-help` form, where `{value}` stands for the labeled value as the user wrote it:

   ```lisp
   (mismatch-help string (ref str) "build a `string` with `(str {value})`")
   ```

   The checker adds the help to an `L0401` whose expected type is the first type and whose found type is the second, comparing types, never names. Every other conversion, such as an `i64` where an `f64` is expected, waits for a later RFC, which may replace `mismatch-help` with a general form.

   **How the rule squares with `AGENTS.md`.** `AGENTS.md` forbids the compiler to emit "a clone or an allocation that the source does not write". A string literal in an owned position writes the allocation: the program wrote a new text where an owned text is wanted, and `String::from` is the only Rust that means that, as `(str "ana")` would. The literal is the source writing it. A `(ref str)` variable differs: it names text the program already holds, and turning it into a `string` would copy that text without the source asking, which the rule forbids, so the variable gets a help and no conversion.
5. **The missing functions are settled by a separate decision on prelude helpers, which its own RFC records.** knave adds `contains?`, `second` on a pair, `join` and `starts-with?` there. Their `foreign-names` entries, such as the membership names of Appendix C2, ship with or after that change. `panic` needs its own decision on what a knave program does on an unrecoverable error, which is larger than a name.
6. **The first build ships Appendix C1.** Each row of Appendix C2 ships later in its own pull request that links its source, as rule 2 requires, and checks its replacement against the prelude signature.
7. **`count` leaves the prelude, and `len` serves collections and iterators.** This RFC assumes that change and lands its `count` rows after it.

## Rejected options and why

- **B, a clause in every definer:** nine forms change, kernel forms and `rust` definitions get no place for the clause, and a header grows a second optional part before the docstring, which the formatter and `header-shape` must learn.
- **C, a line in the docstring:** the compiler would parse prose, and a docstring is Markdown for people. A typo in the line would silently drop a hint, and a docstring rule that no check enforces breaks "if an automated check can enforce a rule, enforce the rule there".
- **D, a table in the compiler:** it restates prelude names in Rust, which `AGENTS.md` forbids, and it drifts when the prelude renames a function.
- **The mangling rule for types:** a prelude type's Rust name comes from its template, not from mangling, and mangling would give `Vec2` for `vec`.
- **A hint for every bound synonym, even when the call works:** a hint on correct code teaches the reader to ignore hints, and no lint list could silence it. The owner rejected it.
- **Absence hints for names knave lacks, such as `contains?`:** a hint that names no knave way sends the model to its next guess. Adding the function is the fix.
- **Dropping every spelling suggestion whose kind differs:** an unbound name has no kind, so at the head of a call no rule can tell a typo of `or` from a loop word.
- **A `conversion` form before this build:** a form that states a function per pair of types needs a design of its own, as the checker must choose among several conversions. The literal rule and one `mismatch-help` cover the evidenced case.
- **The placeholder check in `prelude/lint`:** RFC 0069 runs those rules only under `knave lint`, so a model that runs `knave check` would never see it.
- **An unnarrowed hint on every failing call of a bound name:** Rust's `map` entry would answer Clojure's `(map f coll)` with `map-some`, the wrong fix.
- **An import help for a definition that is not visible:** the spelling suggestion's visibility filter cannot find it, and every name of the first build is re-exported by `prelude`.

## Rule

- [docs/language.md: Foreign names](../docs/language.md#foreign-names), a new section beside Header shapes, which states the form and its rules, and the `mismatch-help` form with the help it adds to `L0401`.
- [docs/language.md: Standard types](../docs/language.md#standard-types), whose paragraph on `print`, `println`, `eprint`, `eprintln` and `str` gains the `format-placeholder` warning.
- [docs/language.md: Names in Rust](../docs/language.md#names-in-rust), which gains the reading of the mangling rule backwards for a suggestion.
- [docs/language.md: Bodies](../docs/language.md#bodies), whose note on a type in a value position gains the struct-at-the-head-of-a-call case.
- [docs/language.md: Literals](../docs/language.md#literals), whose rule "A string literal has type `(ref str)`" becomes the context rule of Decision 4.
- [docs/diagnostics.md: Tone](../docs/diagnostics.md#tone), whose rule "If a name does not resolve, suggest the closest name" gains the order of the three suggestions, the position rule, the `:help` form of an `L0201` help, the help for a binder written with a trailing `:` or `,`, and the note and help that a foreign name adds to `L0301` and to a type error.

---

## Appendix A: Method

1. The corpus is about 400 recorded sessions in which models wrote knave while working on the compiler, each of which reported `L0201` at least once, 1.2 GB of transcripts in all. A script parsed each transcript's tool calls and results. It printed no whole file and copied nothing but the error text and the source line.
2. It extracted 1,243 `L0201` reports, each with the source line under the caret, the help, and the command that produced it.
3. Most reports are deliberate: the compiler's developers test the error with names such as `distnce` (193 reports), `missing-helper`, `nope` and `foo`, or read fixture snapshots. The script dropped reports that came from a test run, a file read or a snapshot, and a hand-made list of test names. 938 distinct reports remained, of 361 names.
4. For each report, the script searched the next 40 tool calls of the same session for the edit that removed the name, and took the tokens that replaced it. The pairing is noisy, so each row of Appendix B was checked by hand against the source line and the current prelude.
5. A name counts as foreign when a standard library of the named language has it and no knave release ever defined it (checked with `git log -G` over `prelude/`), or when it has both histories, which the tables mark. Five names that models wrote are knave-shaped guesses with no library source, `str-len`, `vec-first`, `parse-i64`, `to-i128` and `formatter`, and are left out, as is the generic `cast`. 35 foreign names remain.
6. A set of 21 probe programs, written for the model guide, adds one program per habit. Appendix B lists them apart from the sessions. A review of this RFC probed further cases, `#t`, `to-string`, `console.log` and `std::process::exit`, against `origin/main`.
7. A second scan collected every compiler error that followed a model's own run of `knave`, 877 errors in 159 sessions, to find mistakes that are not names.
8. The prelude names come from `(dir prelude)` on a knave 0.14.13 build, which still had `count`.

The sessions are compiler development sessions, not users writing applications, so the counts are low and every model there had read the prelude. Users who have not will make these mistakes more often. The counts rank the names. They do not measure the rate.

## Appendix B: Evidence by source language

"Reports" counts distinct `L0201` reports, "sessions" counts sessions. A name can match more than one language: `length` is JavaScript, Common Lisp, Scheme and Racket, and the tables list it under each.

### Rust

| Name written | Reports | Sessions | knave | Notes |
|---|---|---|---|---|
| `into-iter` | 4 | 4 | `iter` | `into_iter` |
| `panic` | 4 | 4 | none | `panic!` |
| `push-str!` | 4 | 3 | `push!` | `push_str`, matched by mangling |
| `default` | 3 | 2 | none | `Default::default` |
| `is-some` | 3 | 2 | `some?` | Rust spelling rule |
| `String` | 6 | 1 | `string` | edit distance already finds it |
| `to-string` | 3 | 1 | `str` | `to_string`; also an old knave name; edit distance suggests `docstring` |
| `is-none` | 3 | 1 | `none?` | Rust spelling rule |
| `push` | 3 | 1 | `push!` | edit distance already finds it |
| `'a'`, `'b'` | 4 | 1 | `(char "a")` | character literal |
| `return` | 2 | 1 | `:help` on `fn` | |
| `==` | 1 | 1 | `=` | |
| `is-ok` | 1 | 1 | `ok?` | Rust spelling rule |
| `insert!` | 1 | 1 | `assoc` | `HashMap::insert` |
| `pop-front` | 1 | 1 | none | `VecDeque::pop_front` |
| `into-vec` | 1 | 1 | `collect` | `into_vec` |

Probed after RFC 0046, Rust names that are also old knave names: `fold`, `values`, `any`, `all`, `!=`, `to-string`.

### Python

| Name written | Reports | Sessions | knave | Notes |
|---|---|---|---|---|
| `for` | 7 | 4 | `:help` on `for-each` | also Rust, JavaScript, Clojure; edit distance suggests `or` |
| `%` | 2 | 2 | `rem`, with `:help` | Python's `%` is a modulo; `rem` follows Rust's sign |
| `join` | 2 | 2 | none | `str.join` |
| `return` | 2 | 1 | `:help` on `fn` | |
| `starts-with?` | 1 | 1 | none | `str.startswith`; also Clojure |

### JavaScript

| Name written | Reports | Sessions | knave | Notes |
|---|---|---|---|---|
| `length` | 4 | 3 | `len` | also the Lisps |
| `size` | 2 | 2 | `len` | `Set.size`, `Map.size` |
| `concat` | 2 | 2 | `str` | `String.prototype.concat`; also Clojure |

### Clojure

| Name written | Reports | Sessions | knave | Notes |
|---|---|---|---|---|
| `second` | 6 | 3 | none | |
| `dotimes` | 3 | 2 | `:help` on `for-each` | |
| `vector` | 4 | 1 | `vec` | also Common Lisp, Scheme |
| `contains?` | 3 | 1 | none | |
| `inc` | 2 | 1 | none | `(+ x 1)` |
| `when-not` | 1 | 1 | `unless` | |
| `case` | 1 | 1 | `:help` on `match` | |
| `hash-set` | 1 | 1 | none | |
| `[ ]` parameters | 3 | 2 | `( )` | `L0106` |
| `(map f coll)` | 2 | 2 | `(map coll f)` | `L0401` |

### Common Lisp, Scheme and Racket

| Name written | Reports | Sessions | knave | Notes |
|---|---|---|---|---|
| `length` | 4 | 3 | `len` | |
| `#\l` | 2 | 1 | `(char "l")` | |
| `'origin` | 2 | 1 | `(quote origin)` | the help already says so |
| `else` in `cond` | 1 | 1 | `else` as the last clause | knave's `cond` takes `else` only last |
| `product` | 1 | 1 | none | |
| `defn` inside a body | 18 | | `local` | `L0208` |

### Old knave spellings, which get no entry

Probed after RFC 0046: `append-char!`, `append!`, `box-new`, `sort-descending!`, `sort-by-key!`, `lookup`. None has a source in another language's library, so none gets an entry. `fold`, `values`, `any`, `all` and `to_string` appear there too, and get entries for their Rust or Python source.

### Mistakes that are not names, after a model's own `knave` run

| Code | Count | Shape |
|---|---|---|
| `L0401` mismatched types | 126 | mostly ownership and inference; 2 are Clojure argument order |
| `L0407` type annotations needed | 74 | |
| `L0301` | 47 | 6 are struct fields written as pairs, as Common Lisp and Racket write them |
| `L0207` expected a definition at module level | 38 | script-style top-level expressions |
| `L0208` `def` inside a body | 18 | Scheme internal define, Python nested `def` |
| `L0455` `main` returns a value | 17 | Clojure's `-main` habit |
| `L0106` `[` or `]` | 6 | Clojure vectors |

A further session wrote `(defn reducer (action, state) ...)`, where the spelling help suggested `action,`.

### Probes for the model guide

One small program per habit, each checked with `knave check`. "Help today" is the suggestion the probe got.

| Habit | Source | Error today | Help today | This RFC |
|---|---|---|---|---|
| `(for i (range 0 3) ...)` | Rust, Python | `L0201` on `for`, then on `i` twice | `or`, wrong | `:help` entry on `for-each` |
| `(while ...)` | Rust, Python, JavaScript | `L0201` | none | `:help` entry on `loop` |
| `(return 1)` | Rust, Python, JavaScript | `L0201` | `recur`, wrong | `:help` entry on `fn` |
| `(defn double (x: i64) ...)` | Rust | `L0201` on the `x` in the body | none | binder rule |
| `(fn (Vec<i64>) usize)` | Rust | `L0201` on type `Vec<i64>` | none | follow-up |
| `(format "hello {}" name)` | Rust, Python | `L0201` | none | `:help` entry on `str` |
| `(println "x = {}" x)` | Rust, Python | none; prints `x = {}3` | none | `format-placeholder` lint |
| `(->user "ana")` for a `string` field | Rust | `L0401` expected `string`, found `str` | none | accepted by the string literal rule |
| `(struct point (x f64) (y f64))` then `(point 1.0 2.0)` | Rust | `L0201` on `point` | `print`, wrong | struct rule |
| `(+= count 1)` | Rust, Python, JavaScript | `L0201` | `(+ count 1)` | already helpful |
| `(total &scores)` | Rust | `L0201` on `&scores` | `scores` | borrow note, follow-up |
| `p.x` | Rust, Python, JavaScript | `L0201` | none | follow-up |
| `(.len items)` | Clojure interop | `L0201` | `len` | already right |
| `shape::circle` | Rust | `L0201` | `shape/circle` | already right |
| `println!` | Rust | `L0201` | `println` | already right |
| `Ok`, `Err` | Rust | `L0201` | `ok`, `err` | already right |
| `(impl Display point ...)` | Rust | `L0201` on type `Display` | `display` | already right |
| `(match n (some v) => v ...)` | Rust | `L0201` and `L0210` | the shape of `match` | follow-up |
| `(cond test value ... :else value)` | Clojure | `L0301` | the shapes of `cond` | follow-up, prelude only |
| `(let (mut count) 0)` | Rust | `L0407` | a `the` on the prelude `count`, wrong | `mut` binder rule, follow-up |
| `(defn main [] ...)` | Clojure | `L0106` twice | "every list uses `(` and `)`" | already caught |

Of these 21 probes, four get a wrong help today and seven get none. The review's probes add two more wrong helps, `at` for `#t` and `docstring` for `to-string`. The first build fixes the wrong helps except the `mut` binder, and five of the seven missing helps.

## Appendix C: Proposed entries by source language

Each row maps a foreign name to a knave definition. **R** means the help shows the replacement: the knave definition takes the same arguments in the same order and means the same. **H** means the entry carries a `:help` text, because the order, the meaning or the form differs, and the text says what to write. **B** marks a foreign name bound in `prelude`, which hints only when its call fails. **E** marks a name from Appendix B.

Every row was checked against the prelude signature of its target. Names that agree with knave, such as Python's `len` or Clojure's `filter`, are left out by rule 6.

### C1. The first build

| Language | Foreign names | knave | Kind |
|---|---|---|---|
| Rust | `into-iter` **E**, `iter-mut` | `iter` | R |
| Rust | `push-str` **E**, `push` **E** | `push!` | R |
| Rust | `to-string` **E**, `to-owned`, `String/from` | `str` | R |
| Rust | `format`, `format!` | `str` | H |
| Rust | `count` | `len` | R, H with a test |
| Rust | `insert` **E** | `assoc` | H: `(assoc map key value)` returns the new map, so bind it |
| Rust | `into-vec` **E** | `collect` | R |
| Rust | `fold` | `reduce` | R, the three-argument form |
| Rust | `all` | `every?` | R |
| Rust | `any` | `any?` | R |
| Rust | `values` | `vals` | R |
| Rust | `map` **B** | `map-some` | R, with `:given option` |
| Rust | `==`, `!=`, `&&`, `\|\|`, `!` | `=`, `not=`, `and`, `or`, `not` | R |
| Rust, Python, JavaScript | `return` **E** | `fn` | H |
| Rust, Python, JavaScript | `for` **E**, `while` | `for-each`, `loop` | H |
| Rust, Python | `%` **E** | `rem` | H: `rem` keeps the sign of the first number, as Rust's `%` does and Python's does not |
| Python | `None`, `True`, `False` | `none`, `true`, `false` | R |
| JavaScript | `length` **E**, `size` **E** | `len` | R |
| JavaScript | `concat` **E** | `str` | H: "to join strings, write `(str a b)`"; an array `concat` has no knave form |
| JavaScript | `some` **B** | `any?` | H, in the same form as Clojure's |
| JavaScript | `every` | `every?` | R |
| JavaScript | `null`, `undefined` | `none` | R |
| JavaScript | `===`, `!==` | `=`, `not=` | R |
| Clojure | `count` | `len` | R, H with a test |
| Clojure | `vector` **E** | `vec` | R |
| Clojure | `when-not` **E**, `if-not` | `unless`; `if` with the branches swapped | R; H |
| Clojure | `case` **E** | `match` | H |
| Clojure | `doseq`, `dotimes` **E** | `for-each` | H |
| Clojure | `some` **B** | `any?` | H: the test comes second in knave |
| Clojure | `nil` | `none` | R |
| Common Lisp | `length` **E** | `len` | R |
| Common Lisp | `defun` | `defn` | R |
| Common Lisp | `lambda` | `fn` | R |
| Common Lisp | `progn` | `do` | R |
| Common Lisp | `let*` | `let` | H: `let` takes a flat list, `(let (x 1 y 2) ...)` |
| Common Lisp | `defparameter`, `defvar`, `defconstant` | `def` | R |
| Common Lisp | `setf`, `setq` | `set!` | R |
| Common Lisp | `vector` **E** | `vec` | R |
| Common Lisp | `nil`, `t` | `none`, `true` | R |
| Scheme, Racket | `length` **E**, `vector-length`, `string-length` | `len` | R |
| Scheme, Racket | `define` | `def` | H: `defn` for a function |
| Scheme, Racket | `lambda`, `λ` | `fn` | R |
| Scheme, Racket | `begin` | `do` | R |
| Scheme, Racket | `let*`, `letrec` | `let` | H: `let` takes a flat list, `(let (x 1 y 2) ...)` |
| Scheme, Racket | `vector` **E** | `vec` | R |
| Scheme, Racket | `list->vector` | `collect` | R |
| Scheme, Racket | `#t`, `#f` | `true`, `false` | R |
| Scheme, Racket | `string-append` | `str` | R |

The Rust spelling rule covers `is_some`, `is_none`, `is_ok`, `is_err`, `is_empty`, `HashMap`, `PartialEq`, `Option` and the other names that the prelude's Rust templates or the mangling rule spell, with no entry.

### C2. Candidates for later pull requests, each with its source link

**Python.** R: `append` to `push!`, `pop` to `pop!`, `reversed` to `reverse`, `str.strip` written `strip` to `trim`, `upper` and `lower` to `upper-case` and `lower-case`, `input` to `read-line`, `dict` to `hash-map`, `list` to `vec`, `any` to `any?`, `all` to `every?`. H: `sorted` (knave's `sort!` changes in place and returns `unit`), `extend` (no knave form appends a sequence), `items` (`iter` on a map), `int` and `float` (`(the i64 (unwrap (parse text)))`), `del` (`remove-at!` for a list index), `count` (`(len (filter items test))`), `print(..., end="")` needs nothing (`print` agrees).

**Rust.** R: `pop` to `pop!`, `remove` to `remove-at!`, `sort` and `sort-unstable` to `sort!`, `sort-by-key` to `sort-by!`, `dedup` to `dedup!`, `reverse` to `reverse!`, `to-uppercase` and `to-lowercase` to `upper-case` and `lower-case`, `std/fs/read-to-string` to `slurp`, `std/process/exit` to `exit`, `Box/new` to `box`, `Vec/new` to `vec`, `HashMap/new` to `hash-map`. H: `position` (`find` over `(enumerate items)`), `sort-by` (knave sorts by a key, not a comparison), `insert` on a vector (no knave form inserts at an index), `dbg!` (`prn` prints but returns `unit`).

**JavaScript.** R: `pop` to `pop!`, `reverse` to `reverse!` (both change the array), `sort` to `sort!`, `forEach` to `for-each`, `toUpperCase` and `toLowerCase` to `upper-case` and `lower-case`, `toString` to `str`, `console.log` to `println`, `console.error` to `eprintln`, `process.exit` to `exit`, `Math.max`, `Math.min`, `Math.sqrt` to `max`, `min`, `sqrt`. H: `slice` and `substring` (`subs` takes byte offsets), `splice` (`remove-at!` for one item), `findIndex` and `indexOf` (`find` over `enumerate`), `parseInt` and `parseFloat` (`parse` with `the`), `Object.keys`, `Object.values`, `Object.entries` (`keys`, `vals`, `iter` on a map).

**Clojure.** R: `nth` to `at`, `peek` to `last` on a vector, `doall` needs nothing. H: `conj` (`push!` changes the vector; `conj` returns a new one), `into` (`collect`, `into-hash-map` or `into-sorted-map` by target), `sort` (`sort!` changes in place), `distinct` and `dedupe` (`dedup!` removes neighbors only), `mod` (`rem` keeps the sign of the first number), `quot` (`/`), `if-let` and `when-let` (`let-match`), `condp` (`cond`), `defrecord` (`defstruct`), `defprotocol` (`deftrait`), `extend-protocol` (`impl`), `defn-` (`defn` without `export`).

**Common Lisp.** R: `elt`, `aref` to `at`, `vector-push-extend` to `push!`, `nreverse` to `reverse!`, `remove-duplicates` needs H, `make-hash-table` to `hash-map`, `princ` to `print`, `write-line` to `println`, `string-upcase`, `string-downcase` to `upper-case`, `lower-case`, `string-trim` H (it takes the characters first), `parse-integer` H, `subseq` to `subs` H (byte offsets), `/=` to `not=`, `eq` and `equal` to `=`, `null` to `none?`. H: `nth` (`(at items index)`, the index comes second), `gethash` (`(get map key)`, the key comes second), `mapcar` (`(map items f)`, the function comes second), `remove-if-not` (`filter`, test second), `find-if` (`find`, test second), `every` and `some` (`every?` and `any?`, test second), `reduce` with `:initial-value` (`(reduce items init f)`), `dolist`, `dotimes` and `loop for` (`for-each`), `format` (`str`), `error` (none).

**Scheme and Racket.** R: `vector-ref`, `list-ref` to `at`, `hash-ref` to `get`, `displayln` to `println`, `display` to `print`, `number->string` to `str`, `equal?`, `eqv?`, `eq?` to `=`, `null?` to `empty?`, `string-upcase`, `string-downcase` to `upper-case`, `lower-case`. H: `foldl`, `fold-left`, `fold` (`(reduce items init f)`, the function comes last), `andmap` and `ormap` (`every?` and `any?`, the function second), `findf` (`find`, the function second), `sort` (`sort!` changes in place and sorts by `<` only), `remove-duplicates` (`dedup!` removes neighbors only), `hash-set!` (`assoc`), `make-hash` and `hash` (`hash-map`), `newline` (`(println)`), `string->number` (`parse` with `the`), `substring` (`subs`, byte offsets), `modulo` (`rem` keeps the sign of the first number), `struct` in Racket (`defstruct`; knave's kernel `struct` takes another shape), `error` and `raise` (none).

**Membership, which the prelude lacks today.** The names split by what they test:

- `contains` (Rust, on a slice or a string), `in` (Python), `includes` (JavaScript), `member` and `memq` (the Lisps) test whether a string or a sequence holds an item. They wait for `contains?` (Decision 5), and each is R or H by its signature.
- `contains-key` (Rust) and `has` (JavaScript's `Map` and `Set`) test a key. No helper gives them a target, so they are H entries with `(some? (get map key))`, which can ship as C2 rows now.
- Clojure's `contains?` tests a key, and on a vector an index. Once knave's `contains?` exists with another meaning, it is a B entry with a `:help`. The Lisps' `find` is bound in knave today, test second, so it is a B entry with a `:help` too.

Until `contains?` lands, `(any? (iter items) (fn (x) (= x needle)))` is the knave form for an item, and the model guide states it.

### Names with no knave operation today, ranked by session count

| Name | Reports | Sessions | Languages |
|---|---|---|---|
| `panic` | 4 | 4 | Rust |
| `second` | 6 | 3 | Clojure |
| `join` | 2 | 2 | Python, JavaScript, Clojure |
| `default` | 3 | 2 | Rust |
| `contains?` | 3 | 1 | Clojure, Rust, JavaScript, Python |
| `inc` | 2 | 1 | Clojure |
| `starts-with?` | 1 | 1 | Clojure, Rust, Python, JavaScript |
| `hash-set` | 1 | 1 | Clojure, Rust, Python |
| `pop-front` | 1 | 1 | Rust |
| `product` | 1 | 1 | Python, Rust |
