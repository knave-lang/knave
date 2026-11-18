---
name: writing-knave
description: This skill is the rule set for writing, designing, fixing and reviewing Knave code. Knave is a Lisp that compiles to Rust. Use this skill for every task that edits, runs, tests or explains a `.knv` file or a `project.knv`. Use this skill to read an error that the `knave` command reports. Load this skill before writing any Knave code.
metadata:
  knave-version: "{knave-version}"
---

# Writing Knave

## Design principles

### Data-driven application design

**Give each domain concept a struct or an enum.** Make illegal states unrepresentable.

- Give each state its own enum case, such as `(defenum button-state on off)`.
- Choose behavior with an enum value. NEVER choose behavior with a `bool` parameter or field.
- Wrap a unit or an identifier in a one-field struct, such as `(defstruct money :cents int)`.
- Use a `defenum` in place of a `bool` flag plus an `option` field, such as `(defenum shipment pending (shipped tracking-number))`.

### Pipelines of transformations

**Handle each kind of input, such as a command or a request, with its own pipeline.** A pipeline parses the input into structs and enums, transforms those values with pure functions, then writes the result.

- Parse input into structs and enums where the input enters the application. NEVER pass raw input text beyond that point.
- Keep the application state in one value. Compute the next state with a pure function of the old state and the input.
- Read input and write output in `main` and in the functions it calls for that. NEVER read or write inside a function that transforms data.

### Data over functions, functions over macros

**PREFER data OVER functions, and functions OVER macros.** A function wins only where no value can express the logic. A macro wins only for a form that binds a name or leaves an argument unevaluated.

- Represent a request, an event or a rule as a struct or an enum value.
- Keep a table of fixed facts in a `def`, such as a `vec` of structs.
- If the caller defines the computation, as with `map`, take a `fn` parameter. Otherwise, take a value.
- PREFER an enum with a `match` OVER a `deftrait`. A `deftrait` wins only when code outside the application adds a type, or when the trait names a Rust trait.

### Configuration and dependencies as data

**Pass each setting and each dependency, such as an input source, as a parameter in a struct or an enum.**

- Gather the settings in one struct. Build that struct in `main`.
- NEVER read a setting or a dependency from a `def`.

## Code rules

- NEVER write a `match` or an `if` inside another one. Write the inner form as a named function.
- Thread a value through two or more calls with `->`, which passes the value as the first argument. Every sequence function, such as `map` and `filter`, takes the sequence first.
- If each call takes the value as its last argument, thread with `->>` instead.
- End the name of a predicate with `?`.
- End the name of a function that mutates its argument with `!`, as `push!` does.
- Name a conversion `a->b`, such as `text->number` or `order->json`. A conversion returns the same information in another type.
- Return a `result` for a failure the caller can expect, such as bad input. Give each such failure its own case of one error enum.
- `try` converts a failure of another type to the enum case that holds that type. If `try` converts a failure in a function, give that function a `type` signature that names the error enum.
- A `loop` body is a function of its own, so `try` converts no failure inside it. Inside a `loop`, call a function whose `type` signature names the error enum.
- Put the work that can fail in a function that returns a `result`. In `main`, call that function and `match` on the result. Print a failure with `eprintln`. Then call `(exit 1)`.
- Return an `option` for a value that can be absent.
- NEVER call `unwrap`, `expect`, `unwrap-ok`, `unwrap-err` or `expect-ok` outside a `deftest`.
- Unless `knave check` or a rule in this file asks for a type, leave that type unwritten. A written type without `ref` takes ownership of the argument.
- If `knave check` reports a use of a moved value, remove the written type that took ownership. If no written type did, read the value before the move. If two places each own the value, write `(clone value)`. NEVER write a `clone` only to silence a moved-value error.

## Idioms

**The two code blocks form one file.** A `=>` line shows the value of the form above it, as `knave eval` prints that value.

| To | Write |
| --- | --- |
| define a struct | `(defstruct name :field type ...)` |
| define an enum | `(defenum name case (case type) ...)` |
| build a struct | `(->name :field value ...)` |
| read a field | `(:field value)` |
| write an enum case | `enum/case` or `(enum/case value)` |
| choose by enum case | `(match value (case body ...) ((case name) body ...))` |
| bind names | `(let (a 1 b 2) body ...)` |
| branch on tests | `(cond (test body ...) (else body ...))` |
| return a failure to the caller | `(try value)` |
| bind values that can fail | `(let-try (a value b value) body ...)` |
| state the type of a function | `(type name (fn (type ...) type))` |
| read the arguments | `(command-line-args)` |
| read a line of input | `(read-line)` |
| print text or a value | `(println text)` or `(prn value)` |

```knave
(defstruct money
  :cents int)

(defenum payment
  pending
  paid)

(defstruct line-item
  :price money
  :quantity int)

(defstruct order
  :payment payment
  :line-items (vec line-item))

(defn line-item-total (line-item)
  (let (price (:cents (:price line-item))
        quantity (:quantity line-item))
    (->money :cents (* price quantity))))

(defn order-total (order)
  (->money :cents (-> (:line-items order)
                      (map line-item-total)
                      (map :cents)
                      sum)))

(defn paid? (order)
  (match (:payment order)
    (pending false)
    (paid true)))

(line-item-total (->line-item :price (->money :cents 450) :quantity 2))
=> (->money :cents 900)

(paid? (->order :payment payment/pending :line-items (vec)))
=> false
```

```knave
(defenum line-item-error
  (invalid-number (parse-error int))
  (invalid-quantity int))

(type text->number
  (fn ((ref str))
    (result int line-item-error)))

(defn text->number (text)
  (ok (try (parse text))))

(defn parse-line-item (price-text quantity-text)
  (let-try (cents (text->number price-text)
            quantity (text->number quantity-text))
    (if (>= quantity 0)
      (ok (->line-item :price (->money :cents cents)
                       :quantity quantity))
      (err (line-item-error/invalid-quantity quantity)))))

(defn main ()
  (match (parse-line-item "450" "2")
    ((ok line-item) (prn (line-item-total line-item)))
    ((err failure) (eprintln (pr-str failure)) (exit 1))))

(parse-line-item "450" "-1")
=> (err (line-item-error/invalid-quantity -1))

(parse-line-item "450" "two")
=> (err (line-item-error/invalid-number (parse-int-error "invalid digit found in string")))
```

## Work loop

**NEVER write a form from memory of another language.** Take each form from this file or from `knave doc`.

**If `knave --version` prints a version other than `{knave-version}`, trust `knave doc` over this file.** If the shell cannot find `knave`, stop and tell the user.

1. If you need a form this file does not show, search the output of `knave doc "(module prelude)"` for its name. Then run `knave doc <name>`. A module lists fewer forms, such as `knave doc "(module prelude/io)"` for input and output.
2. After each definition you write, run `knave check`. Fix the first error or warning before the others.
3. For each error code, run `knave doc <code>`, such as `knave doc L0501`.
4. If an error suggests a similar name, run `knave doc` on that name before you use the name.
5. If one error remains after three fixes, stop and show the error to the user.
6. If you need the value of a form, run `knave eval <file> '<form>'`.
7. If you run the application, pass its arguments after `--`, as `knave run -- 10`.
8. Write a test beside each pure function, as `(deftest name (assert (= actual expected)))`. Run the tests with `knave test <file>`.
9. Before you finish, run `knave fmt` and `knave lint`. Fix each warning that `knave lint` prints.

## Rust habits that fail

| Rust | Knave |
| --- | --- |
| `i64`, `f64`, `String`, `&str` | `int`, `float`, `string`, `(ref str)` |
| `Vec<T>`, `Option<T>`, `Result<T, E>` | `(vec t)`, `(option t)`, `(result t e)` |
| `fn f(x: i64)` | `(defn f ((x int)) ...)` |
| `x += 1` | `(set! x (inc x))` |
| `x as u8`, `u8::try_from(x)` | `(the u8 (convert x))` |
| `x == y`, `x != y` | `(= x y)`, `(not= x y)` |
| `x?` | `(try x)` |
| `p.x`, `v.len()` | `(:x p)`, `(len v)` |
| `p.0`, `p.1` | `(first p)`, `(second p)` |
| `Some(v) => v`, `None => 0` | `((some v) v)`, `(none 0)` |
| `Ok(())` | `(ok (do))` |
| `v.iter().map(f).sum()` | `(-> v (map f) sum)` |
| `format!("x = {}", x)` | `(str "x = " x)` |
| `"ana"` for a `String` field | `(str "ana")` |
| `return x` | `x` as the last form |
| `[1, 2]`, `vec![1, 2]` | `(vec 1 2)` |
| `use m::f;`, `pub fn f` | `(use m f)`, `(export f)` |
