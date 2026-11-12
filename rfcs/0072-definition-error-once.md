# RFC 0072: Report a definition error once

**Status:** Accepted
**Date:** 2026-11-11

## Problem

A `defrule` rule whose template uses a pattern variable without the `...` that the pattern gives it reports `L0301` at every call that selects the rule. The fault lies in the rule, so every report is the same error at the same place, and only the `in this macro call` label differs:

```lisp
(defrule spread (((a ...)) (vec a)))

(defn main ()
  (spread (1))
  (spread (2))
  (spread (3)))
```

```text
error[L0301]: this pattern variable needs `...` after it
 --> spread.knv:1:33
  |
1 | (defrule spread (((a ...)) (vec a)))
  |                                 ^ here
...
5 |   (spread (2))
  |   ------------ in this macro call
```

The same error then repeats for lines 6 and 4. "nothing before this `...` repeats" behaves the same way.

The manager's ruling on item 45 asks for one report per definition, at call time. A check at definition time was rejected because it cost +356K instructions per program and rejected unused rules that compile today.

The language cannot do this today, for two reasons:

1. The prelude cannot keep a fact across calls. A failed call rolls back the module notes it added, so `defrule` cannot remember that it already reported.
2. The expander cannot tell an error of the definition from an error of the call. The review of PR 375 ruled that a general merge of identical errors is wrong. Take `(defmacro needs-symbol (x) (if (symbol? x) x (fail "expected a symbol" (quote here))))`. The calls `(needs-symbol 1)` and `(needs-symbol "text")` report two errors, both at `(quote here)` in the definition. Like the `defrule` errors, they differ only in the call label, yet each is a distinct mistake that `docs/diagnostics.md` wants listed.

## Options

### A. A `:once-per definition` description part

`fail` and `warn` take one more part in their description: `:once-per definition`. It says that the error's cause lies in the macro's definition, so every call that reaches it reports the same thing. Of the marked errors that match in all but their `in this macro call` labels, the expander reports the one at the first call in source order. The prelude marks its two template errors:

```lisp
(fail "this pattern variable needs `...` after it"
  template "write `...` after it"
  (quote (:once-per definition)))
```

The example above then reports once:

```text
error[L0301]: this pattern variable needs `...` after it
 --> spread.knv:1:33
  |
1 | (defrule spread (((a ...)) (vec a)))
  |                                 ^ here
...
4 |   (spread (1))
  |   ------------ in this macro call
  |
  = help: write `...` after it
```

`needs-symbol` is unmarked and keeps both of its errors. A rule that no call selects still compiles. Two wrong rules of one `defrule` report once each. A user macro whose own `fail` does not depend on its arguments may mark it too.

The emitted Rust does not change, because only diagnostics change.

### B. Merge errors whose label lies in the macro's definition

The expander would merge every failure whose primary label lies outside the call, in the macro's definition. This needs no new surface, but `needs-symbol` labels its own body too, so its two errors would merge into one, against the PR 375 ruling.

### C. Track whether the error depends on the arguments

The interpreter would taint every value that derives from the call's input, including the branches an `if` on such a value takes, and treat a `fail` with no taint as a definition error. That needs no new surface and gets `needs-symbol` right. But every value and branch of every macro call would pay for the taint, far above the item's cap of 20K instructions per program, since macro expansion is most of the front end.

## Prior art

- Rust `macro_rules!` reports "variable `x` is still repeating at this depth" while it transcribes each call (`compiler/rustc_expand/src/mbe/transcribe.rs`). A macro that is never called reports nothing. The allow-by-default lint `meta_variable_misuse` (`compiler/rustc_expand/src/mbe/macro_check.rs`) reports the same fault once, at the definition.
- Racket's `syntax-rules` and `syntax-case` raise "missing ellipsis with pattern variable in template" while they compile the template (`racket/collects/racket/private/template.rkt`), once, when the `define-syntax` form expands.
- Both report a mismatch of repetition counts at each use, because it depends on the arguments. `L0330` does the same.

Option A reports once, as Racket does, and stays silent on unused rules, as Rust does.

## Recommendation

Option A. It is the only option that keeps `needs-symbol`'s two errors and adds no work to a valid program. The compiler learns nothing about `defrule`: the prelude states which errors belong to the definition, and the expander obeys the mark. The rule is uniform, so a user macro gets it on the same terms as the prelude.

## Decision

The project owner accepted option A (D108):

1. Add the description part.
2. The single report labels the first call in source order, not every call. A rule used fifty times would otherwise print fifty labels, and the fix lies at the definition.
3. The part is spelled `(quote (:once-per definition))`. `definition` is its only value, and any other value is `L0321`.
4. `fail` keeps its shape, so a description still follows a help. The two prelude errors gain the helps "write `...` after it" and "remove this `...`, or repeat a variable that the pattern repeats".
5. Every macro may use the part, user macros included.
6. Marked errors merge when they match in all but their `in this macro call` labels, helps included. A marked `fail` in a helper function that several macros call therefore reports once, at the first call in source order of any of them. The definition that holds the fault is the helper, and fixing it fixes every caller.
7. The REPL checks each input on its own, so it reports a marked error once per input.

## Rejected options and why

- **B, merge by label position:** it merges the distinct mistakes of `needs-symbol`, which the PR 375 review ruled out.
- **C, track argument dependence:** its cost on every valid program exceeds the cap of 20K instructions, and it changes the interpreter throughout.
- **Every call as an extra label:** it lengthens the report with each use, while the fix lies in one place.
- **A form `(fail message form description)` without a help:** it changes `fail`'s shape, and the two errors have a fix to state anyway.

## Rule

[docs/language.md: The `code` module](../docs/language.md#the-code-module)
