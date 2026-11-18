# Bindings

A binding gives a value a name. `let` binds names inside a function, `set!` changes what a binding holds, and `def` names a value for the whole module.

## Bind names with `let`

`let` takes a list of names and values, then a body. Each name is bound for the body, and the value of the `let` is the value of the body's last form.

```knave
(defn main ()
  (let (width 3
        height 4)
    (println (* width height))))
```

```text
12
```

Each value sees the names bound before it, so a later value can build on an earlier one.

```knave
(defn main ()
  (let (price 40
        tax (/ price 10)
        total (+ price tax))
    (println total)))
```

```text
44
```

A name is bound until the `let` closes. After the closing parenthesis, the name refers to what it referred to before. A name bound again in the same list or in an inner `let` shadows the earlier binding, as in Rust.

```knave
(defn main ()
  (let (label "outer")
    (let (label "inner") (println label))
    (println label)))
```

```text
inner
outer
```

The compiler infers the type of each binding. To state it, write the name and the type as a pair.

```knave
(defn main ()
  (let ((count u8) 200) (println count)))
```

```text
200
```

See [let](/docs/prelude/syntax/#let) for the full form.

## Change a binding with `set!`

`(set! target value)` stores a new value in a binding, a field or an element. You do not mark a binding as mutable: a `set!` of the binding makes it mutable.

```knave
(defn main ()
  (let (total 0)
    (set! total (+ total 5))
    (set! total (+ total 7))
    (println total)))
```

```text
12
```

A field read or an `at` form is a target too.

```knave
(defstruct point
  :x int
  :y int)

(defn main ()
  (let (corner (->point :x 1 :y 2)
        steps (vec 10 20 30))
    (set! (:x corner) 5)
    (set! (at steps 0) 15)
    (println (:x corner))
    (println (at steps 0))))
```

```text
5
15
```

See [set!](/docs/prelude/place/#set!) for every target it accepts.

## Define module-level names with `def`

`(def name value)` names a value for the whole module.

```knave
(def max-retries 3)

(defn main ()
  (println (* max-retries 2)))
```

```text
6
```

A `def` whose value is a function defines a function. `defn` is the short way to write it.

```knave
(def double (fn (n) (* 2 n)))

(defn triple (n) (* 3 n))

(defn main ()
  (println (+ (double 4) (triple 4))))
```

```text
20
```

See [def](/docs/prelude/#def) and [defn](/docs/prelude/syntax/#defn).

### What may sit below its use

A function can use a `def` or a type written below it in the module.

```knave
(defn main ()
  (println (:x (->point :x max-retries :y 0))))

(def max-retries 3)

(defstruct point
  :x int
  :y int)
```

```text
3
```

A macro is the exception: it must sit above its first top-level use, because the compiler expands the top-level forms of a module in order. See [defmacro](/docs/prelude/macros/#defmacro).

### Document a definition

A string between the name and the value documents the definition, but only when another form follows it. `knave doc` prints the documentation.

```knave
(def max-retries
  "How many times a request runs again."
  3)

(defn main () (println max-retries))
```

```text
3
```

With no form after it, the string is the value, so `(def greeting "hi")` binds `greeting` to the string `"hi"`.

```knave
(def greeting "hi")

(defn main () (println greeting))
```

```text
hi
```
