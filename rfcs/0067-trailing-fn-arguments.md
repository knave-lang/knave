# RFC 0067: A trailing fn argument in the formatter

**Status:** Accepted
**Date:** 2026-11-09

## Problem

RFC 0053's layout rule 5 lines up a call's later arguments under its first argument. When the last argument is a `fn` literal, the earlier arguments each take a line of their own and the `fn` body starts far to the right, under the first argument:

```lisp
(defn join-items (items)
  (reduce (iter items)
          (ok (str ""))
          (fn (joined item)
            (let-try (text joined)
              (ok (str text (describe item)))))))
```

The body of the `fn` is what a reader reads, and it sits 12 columns in for no reason a reader can see. The deeper the call, the less of the 80-column width its body keeps. A `for-each`, `filter`, `any?` or `reduce` over a collection is the most common place a `fn` literal appears, and each one pays this indentation.

The formatter names no form, so the rule cannot list `reduce` or `for-each`.

## Options

### A trailing `fn` keeps the earlier arguments on the first line

When a call breaks and its last argument is a `fn` literal, the head and the earlier arguments stay on the first line, and the `fn` starts on the next line, 2 columns past the call's `(`:

```lisp
(defn join-items (items)
  (reduce (iter items) (ok (str ""))
    (fn (joined item)
      (let-try (text joined)
        (ok (str text (describe item)))))))
```

The rule names one form, `fn`, which is a kernel form that the compiler owns, so the formatter restates no prelude fact. It applies to every call whose head has no header shape. A shaped form, such as `when`, keeps its shape's layout.

### A header shape for each higher-order function

The prelude states `(header-shape (reduce collection initial))`, `(header-shape (for-each collection))` and so on. Each function's arguments before its `fn` become its header. The prelude then states a shape for every function that takes a closure, a user function that takes one lays out as a call until its author adds a shape, and a call that passes a named function in place of the `fn` literal keeps the shape's body indentation for a form that has no body.

### Keep rule 5

The `fn` body stays under the first argument.

## Recommendation

A trailing `fn` keeps the earlier arguments on the first line. It covers every function that takes a closure last, the prelude's and the user's, with no statement per function, and it moves only the calls whose last argument is a `fn` literal.

Two cases keep rule 5, because the rule would make them no shorter:

- A call with no argument before its `fn`, such as `(for-each (fn (item) ...))`, keeps the `fn` on the first line.
- A `fn` that is a keyword's value, such as `:on-click (fn (event) ...)`, stays beside its keyword, as rule 7 keeps a pair together.

An earlier argument fits on the first line when its own layout takes one line and the line ends within the 80-column width. A comment or a kept blank line among the earlier arguments puts a line break among them, so the call keeps rule 5.

## Decision

The project owner decided on 2026-11-09 the rule above, for every call with no header shape, with nothing named per function. If the earlier arguments do not fit on the first line, the call keeps today's layout. The owner approved these layouts:

```lisp
(reduce (iter items) (ok (str ""))
  (fn (joined item)
    ...))

(for-each (range 0 2)
  (fn (i)
    ...))

(filter (iter years)
  (fn (year)
    ...))

(any? (iter pages)
  (fn (page)
    ...))

(map (filter users
       (fn (user)
         ...))
     :age)
```

In the last example `filter` takes the rule, and its `fn` sits 2 columns past `filter`'s `(`. The last argument of `map` is `:age`, so `map` keeps its layout.

The owner decided against changing the 40-column and 60-column one-line rule of RFC 0053 at the same time, so a call that fits on one line by that rule stays on one line.

This amends layout rule 5 of RFC 0053.

## Rejected options and why

- **A header shape for each higher-order function:** the prelude would state one shape per function, a user's function would need its own, and a call with a named function in place of the `fn` would take a body indentation it does not need.
- **Keep rule 5:** the body of a `fn` argument stays under the first argument, as far right as the call's first argument starts.

## Rule

[docs/language.md: Source formatting](../docs/language.md#source-formatting)
