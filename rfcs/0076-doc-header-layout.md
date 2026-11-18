# RFC 0076: The layout of a doc page's header block

**Status:** Accepted
**Date:** 2026-11-17

## Problem

A doc page that shows a type opens with two forms: the name's header, such as `(for-each items function)`, and its type line, such as `(type for-each ...)`. RFC 0071 decided, as its Q-d, that the header block keeps the two together with no blank line. `knave fmt` separates two top-level forms with a blank line, so a page laid out the other way looks unlike the code it documents. `knave doc --types` prints the same two forms, the type line above the page as D15 states, also with no blank line between them.

## Options

### Keep the forms together

The header block holds the header, then the type line on the next line, as RFC 0071's Q-d states.

```lisp
(for-each items function)
(type for-each
  (where (a)
    (fn ((iterator a) (fn (a) unit)) unit)))
```

```rust
// A doc page emits no Rust.
```

### Separate the forms as `knave fmt` does

The header block holds the header, a blank line, then the type line. `knave doc --types` prints the type line, a blank line, then the page.

```lisp
(for-each items function)

(type for-each
  (where (a)
    (fn ((iterator a) (fn (a) unit)) unit)))
```

```rust
// A doc page emits no Rust.
```

## Recommendation

Separate the forms as `knave fmt` does. One layout rule then covers a file and a doc page, so a reader meets the same spacing in both.

## Decision

D122. The project owner: "I prefer having formatting that matches the `knave fmt`". Every page that `knave doc` prints or writes separates the header and the type line with one blank line, as `knave fmt` separates two top-level forms. The decision replaces RFC 0071's decision Q-d.

## Rejected options and why

- **Keep the forms together:** the page lays out its two forms unlike `knave fmt`, so a page's code differs from the same code in a file.

## Rule

[docs/language.md: Documentation pages](../docs/language.md#documentation-pages) and [docs/language.md: Docstrings](../docs/language.md#docstrings)
