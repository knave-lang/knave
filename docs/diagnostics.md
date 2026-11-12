# Writing diagnostics

This guide covers every error the compiler prints. A diagnostic uses rustc's layout and the tone of Elm's error messages.

## Layout

**Build every diagnostic from these parts, in this order:**

| Part | Content | Required |
|---|---|---|
| Header | `error[L0001]: title`, or `warning[L0001]: title` for a warning | yes |
| Location | `--> path:line:column` | yes |
| Primary label | A caret under the exact form at fault, with a short message | yes |
| Secondary labels | Other forms that explain the error, such as the earlier definition of a duplicate name | no |
| Notes | `note:` lines that state facts the reader needs | no |
| Help | `help:` lines that state a fix | yes, if a fix exists |

A warning does not stop the build: the compiler prints it and continues.

**The CLI colors diagnostics only when standard error's color decision is to color.** [docs/language.md: Colors](language.md#colors) states the decision.

**The CLI prints at most `CappedReport::MAXIMUM_PRINTED_DIAGNOSTICS` diagnostics, in source order, then one `note:` line that counts the rest.** Every stage still returns every diagnostic.

```text
error[L0201]: cannot find `distnce` in this scope
 --> src/main.knv:4:10
  |
4 |   (print (distnce origin target))
  |           ^^^^^^^ not found in this scope
  |
help: a function with a similar name exists
  |
4 |   (print (distance origin target))
  |           ~~~~~~~~
```

## Tone

**Write the title as a plain statement of what is wrong.** Write "cannot find `distnce` in this scope", not "unresolved symbol".

**Point the primary label at the smallest form that is wrong.** A label on a whole function hides the fault.

**If a fix exists, give it in `help:`.** If the fix is an edit, show the replacement text.

**Use the names the user wrote.** Never show a Rust name such as `make_point` in place of `make-point`.

**If a name does not resolve, suggest the closest name.** Rank candidates by edit distance. If no candidate is within a third of the name's length, leave out the suggestion.

**If the error is inside macro output, name the expansion.** Add `note: in this expansion of` followed by the macro name, with a label on the user's macro call. When a macro expands to an `if`, a note about the `if`'s test or branches names the user's macro instead of `if`.

**Never blame the reader.** Describe the code, not the person.

## Codes

**Give each diagnostic its own code with the `L` prefix.** The first digit groups codes by stage:

| Range | Stage |
|---|---|
| `L01xx` | Reader |
| `L02xx` | Names, modules and hygiene |
| `L03xx` | Macros |
| `L04xx` | Types |
| `L05xx` | Ownership |
| `L06xx` | Emitting and building |
| `L07xx` | The project file |
| `L08xx` | Lints, which macros report with `warn` |

**Keep rustc's own code on an error that rustc reports.** Add `note: reported by rustc` and rewrite every Rust name in the message to the name the user wrote.

**Name the definer the user wrote.** A definition inside a body is `L0208`, and its title names the head of the outermost macro call that the user wrote around the `def` or `struct`: `(defn inner () 1)` in a body reports `` `defn` can only appear at module level ``, a `deftest` reports `deftest`, and a user macro over `defn` reports the user macro. A `def` or `struct` that the user wrote keeps its own name.

**Name the prelude form that wrote a kernel definition.** An error of an `enum`, `trait`, `rust-type` or `rust-trait` that a macro expanded to names the macro, read from the macro origin, so `(deftype (pair a) (rust "{item}"))` reports ``a `deftype` template cannot name `item` ``, and the help of a malformed form gives the kernel form's shape. When a prelude definer is in the expansion, that shape starts with the macro's name and spells a Rust template as a `(rust "...")` clause, as in ``write it as `(deftrait name (rust "Path") ...)` ``. An error that has no such form to read, as a `with` outside a Rust trait, names no definer.

**Name a repeated test at the second `deftest`.** A second test of one name in one module is `L0333`, ``test `pads` is defined more than once``, with its primary label on the second test's name and a secondary label on the first, and the help `rename one of the tests`.

**Name a repeated task at the second `deftask`.** A second task of one name is `L0356`, ``task `serve` is defined more than once``, with its primary label on the second task's name, a secondary label on the first, and the help `rename one of the tasks`. A label never points at the source that `knave task` generates.

**Label a task's own definition errors on the form at fault.** A task name that is not kebab-case is `L0357` on the name, a positional parameter after a flag is `L0358` on that name, a `deftask` outside `tasks/` is `L0359` on the task's name, a flag named `:help` is `L0360` on the keyword, a task whose only form is a string is `L0361` on the string, and a task whose result type stays open is `L0362` on the task's name, each with a help that states the fix.

**Point an unreadable example at its code.** A docstring example whose result does not read as one form is `L0354`, with its primary label on the example's code and a help that names the fenced code block.

**Point unreadable docstring code at its first line.** Docstring code that does not read is the warning `L0109`, which `knave fmt` and `knave doc --write` report with the primary label on the first line of the code and a help that names the fence's language. When the code that does not read follows a result and closes more lists than it opens, the help says to indent the result's later lines deeper than the code instead. The formatter and the doc writer leave that code as written, and the warning does not change the exit status.

**Point a partly labeled call at the argument outside a pair.** `L0338` puts its primary label on the first argument that no `:field value` pair holds, with "this argument has no label" on a value and "this label has no value" on a keyword.

**Report each error once.** A macro can copy one form of its input into several places of its output, so the expander drops a recovered error, or an error of a form that a top-level `do` splices, that repeats an earlier error of the same top-level form with the same code, title, labels and notes. Of the copies, it keeps the one with the most helps. An error or a warning whose description holds `:once-per definition` reports once per program, at its first call in source order, because its cause lies in the macro's definition. The type stage does the same for the diagnostics of a whole program: a macro's output can give two forms one location, so two errors can match exactly.

**Report a missing annotation only when no other error explains it.** A `type annotations needed` error is a symptom of another error, so the type stage drops it when the program has any other error.

**Report an unresolved type variable once, at the innermost value whose type holds it.** The type stage visits a value's parts before the value. On a call it visits the arguments, then the call, then the callee, so a callee reports only a variable that neither an argument nor the result holds. Variables that inference merges later count as one.

**Wrap the labeled value in `the`.** The help of `L0407` is a fix that writes the value's open type around the value, with `_` for each unresolved part. If the label's text is not a value the user wrote, such as a binder or a form inside a macro's expansion, the help names `the` and a `type` signature in words. A value that an annotated `local` or a `the` already holds labels that form, with the same help.

**Say so when no annotation can help.** A variable that only a bound holds reports `L0407` at the function that brought the bound, and its help says that the function's signature must change.

## Tests

**Add a fixture and a snapshot for every new diagnostic.** Place the fixture under `tests/fixtures/`, in the directory of the stage that reports it. Use fake data only.
