# RFC 0087: Assignment is a kernel form

**Status:** Accepted. Decision 4 rests on the principle that a `rust` template is never an assignment, whatever its type, so only the assignment node counts as one. Amended 2026-11-25: the change to `WholeValueChanges` lands with the `set!` switch, not in a later pull request, because once `set!` writes the node, no call has the shape the old check matched.
**Date:** 2026-11-24

## Problem

The checker cannot tell an assignment from any other call that changes a place.

`set!` is a prelude macro. For a target that is not an `at` form, it expands to `(set-place! target value)`, and `set-place!` is a prelude `rust` template:

```lisp
(def set-place!
  (rust "*{0} = {1}"
    (where (a) (fn ((mut a) a) unit))))
```

To the checker, `(set! o v)` is a call that passes `o` to a `mut` parameter, exactly like `(replace! o v)` for a function that calls `std::mem::replace`. Rust does not treat the two alike: only an assignment statement ends the borrows that the old value held.

```lisp
(defn main ()
  (let (s (str "kept"))
    (let (o (some (as-str (str "zz"))))
      (set! o (some (as-str s)))
      (println (unwrap o)))))
```

Here is a sketch of the Rust the program needs. It is not checked output:

```rust
let mut o = Some(owner.as_str());
o = Some(s.as_str());
println!("{}", o.unwrap());
```

Rust accepts this program. The assignment kills the loan on the temporary, so only `s` is still borrowed at the read. Two other ways of writing the same value keep that loan alive: passing `&mut o` to a function that writes it, and calling `std::mem::replace(&mut o, ...)`. A `&mut` holds the lifetime inside its referent fixed (roadmap item 614, probes g07 and g08).

The body of a template is text, and no signature a template can write says "this call is an assignment". So the checker treats every changed local the same way. Roadmap item 603 must then either report false errors or leave these programs to rustc, and the owner's rule that no failure reaches rustc forbids the second. Item 603's draft recognized the assignment by matching the template text `*{0} = {1}`, and the owner rejected that.

On 2026-11-24 the project owner decided that assignment becomes a kernel form that `set!` expands to (item 614, option B). This RFC records the form's shape, its typed representation, and how each stage carries it.

## Options

### The form's shape

- **A: `set-at!` with an empty key list.** `(set-at! total () 2)`, with keys grouped as today: `(set-at! items (0) 9)`.
- **B: a new kernel form `assign`.** `(assign total 2)`.
- **C: a new kernel form named `set-place!`.**
- **D: `set-at!` with inline keys.** Keys are written inline in `set-at!` and its twin `assoc-at`. The first argument is the place or value, the last is the item, and the arguments between them are the path:

```lisp
(set-at! total 2)        ;; total = 2
(set-at! items 0 9)      ;; items[0] = 9
(set-at! grid 2 3 9)     ;; grid[2][3] = 9
(assoc-at m "a" 2)       ;; { let mut m = m; m.insert("a", 2); m }
```

  Clojure's `aset` and `assoc`, Fennel's `tset` and Scheme's `vector-set!` work the same way. `set!` becomes one expression with no branch:

```lisp
(concat (list (quote set-at!) (at-base target))
        (at-indices target)
        (list value))
```

### The typed representation

- **A synthetic call** whose callee is a compiler-built template, `*{0} = {1}`. A typed view recognizes it by the compiler's own constant.
- **A dedicated node**, `TypedExprKind::Assign`. It holds:
  - the place, as the typed argument that Rust writes into;
  - the path: `Whole`, `Elements(keys)` or `MapEntry(key)`;
  - the value;
  - the instantiated parameter types.

  The node derives, and does not store, the place it writes: the root local, then field, index and key steps, and whether the write goes through a reference. It also answers whether a whole local is replaced.

### What the checker may conclude

Only an owned local assigned whole, emitted as `x = value`, ends the old value's borrows. That covers:

- a `let` or `local`;
- an owned parameter;
- a closure's own parameter;
- a `loop` binder;
- a written `(mut x)` of any of these.

A field, an element and a place reached through a `mut` reference, emitted as `*x = value`, are written in part. They keep the borrows that their earlier values held.

## Recommendation

Shape D, with a dedicated node.

- **D adds no kernel form**, following the principle "Few rules, no exceptions". The kernel's assignment form already exists, and the empty path is the uniform reading of its keys. Inline keys need no empty `()`, and they remove a trap of the grouped shape: there, a key that is a call needs a doubled list, `((f x))`. Each argument is one form, and position alone gives its role. So a key that is a list expression is unambiguous, and no old-shape program compiles with a new meaning.
- **The dedicated node** lets the borrow checker, item 603 and the interpreter of RFC 0063 read which place is written without a recognizer. The emitter rebuilds today's template text from the node and hands it to its existing template path. So the emitted Rust stays byte for byte the same, including compound assignment, argument roles and call order.
- **The help that names the callee** names `set-place!` today. That is a function no user can write, so the help changes to the fix as the user writes it, `(set! x (clone f))`.
- **`WholeValueChanges` reads only the node.** A `rust` template is not an assignment, so a template of type `(fn ((mut a) a) unit)` no longer gets the labels for "the `set!` of the whole parameter". `changed_locals` stays as it is, because any `mut` argument can store a borrow in the local.

The build lands in six pull requests:

1. Inline keys for `set-at!` and `assoc-at`. Only the source spelling changes.
2. The node replaces today's keyed `set-at!` lowering. No snapshot changes.
3. `(set-at! place value)` assigns the whole place. Paired fixtures write each case with `set!` and with `set-at!`, and show the same Rust and the same diagnostics.
4. `set!` expands to `set-at!`, and `set-place!` is deleted. The only snapshot that changes is the help that named `set-place!`.
5. `WholeValueChanges` reads only the node.
6. Roadmap item 603's PR 2 reads the node in place of matching template text.

## Decision

The project owner decided on 2026-11-24:

- Shape D: `set-at!` with inline keys, which applies to `assoc-at` too.
- The dedicated typed node.
- The help that names `set-place!` changes. Text that names a deleted function is never kept.
- `WholeValueChanges` reads only the assignment node, in its own pull request after the `set!` switch. `changed_locals` stays as it is.

## Rejected options and why

- **Matching the template's Rust text (item 614, option A):** the checker would depend on how the prelude spells a template, which `AGENTS.md` forbids, and a user's template with the same text would count as an assignment.
- **A `replace` mode word in the template's signature (item 614, option C):** the checker would still trust a template's body without reading it. A `mut` argument or `std::mem::replace` behind the word would not end the borrows that the word promises.
- **Leaving the cases to rustc (item 614, option E):** the owner's rule is that no failure reaches rustc.
- **Shape A:** the empty `()` is unreadable, and grouped keys need a doubled list for a key that is a call.
- **Shapes B and C:** each adds a kernel form for a case that the existing assignment form already covers with its empty path.
- **The synthetic call:** readers would still learn about an assignment by recognizing a call. Today a call reaches 53 reader sites, and each would keep treating the assignment as a call.
- **A stored place in the node:** the ownership rewrite edits typed expressions in place, so a stored copy would disagree with what the emitter writes.

## Rule

[docs/language.md: Kernel forms](../docs/language.md#kernel-forms)
