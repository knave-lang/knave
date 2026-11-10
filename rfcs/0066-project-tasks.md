# RFC 0066: Project tasks

**Status:** Proposed
**Date:** 2026-11-09

## Problem

A project has no place for its own commands. The website `knave-lang/knave-lang.org` documents `knave build && target/knave/knave-site/knave-site serve`: the user must know the build directory, the binary's name and the argument the program reads. babashka answers this with `:tasks` in `bb.edn`, as shell strings or Clojure forms inside the data file. A shell string is not portable and is never checked, and code inside the project file breaks RFC 0061's decision 2: the file is data and nothing in it runs.

The language cannot solve this from a program: no command finds a definition by name, builds it with the project's modules and libraries, parses the command line into its parameters and runs it at the project root.

## Options

The owner fixed three points first: a task lives under `tasks/`, `knave task` with no name lists the tasks, and the project file stays data.

### A file per task with a `main`

`tasks/serve.knv` defines `main`, and `knave task serve` builds it as `knave run` builds a file. The file name is the task's name, so one file holds one task, and `main` reads `command-line-args` by hand.

### `deftask`, a prelude macro shaped like `defn`

```lisp
(use preview serve-preview)

(deftask serve ()
  "Serves the site at http://127.0.0.1:8000 and rebuilds it on change."
  (serve-preview))

(deftask greet (who :times u16 1 :loud false)
  "Greets someone a few times."
  (repeat-greeting who times loud))
```

```text
$ knave task
greet <who> [--times 1] [--loud]  Greets someone a few times.
serve                             Serves the site at http://127.0.0.1:8000 and rebuilds it on change.
$ knave task greet ana --times 3
```

Any `.knv` file under `tasks/` holds any number of tasks, as `tests/` holds tests. The name comes from the form. The command builds a program whose entry calls the one task it names, so a task that nothing runs emits no Rust.

## Recommendation

Take `deftask`. The owner chose it, as the Decision records. The points the owner asked this RFC to settle:

1. **Modules.** The command loads every `.knv` file under `tasks/` as `knave test` loads `tests/`. `use` searches `tasks/` first, then the source root, and reaches the project's libraries. A file inside `tasks/` resolves `use` this way under every command, so `knave check tasks/serve.knv` checks what `knave task` builds. The source root never reaches into `tests/` or `tasks/` for a task, so a task cannot reach a test module even when `:entry` sits at the project root. A `deftask` in a module outside `tasks/` is an error at its name.
2. **Parameters.** The list is flat, as `defn`'s keyword parameters are, and the command-line arguments after the task's name bind to it. The positional parameters come first, each a name or a name and a type, as `(who string)`, and take positional arguments in order. Then each keyword is a `--name` flag, followed by an optional type and an optional literal default, a number, a string or a bool: `(who :times u16 1 :loud false)`, and the owner's `(:port 8000)`. A flag with no default is required. A bool flag whose default is `false` is a switch, and `--dry-run` turns it on. A name after a flag's type and default is an error, because the positional parameters come first. Every word after the task's name belongs to the task, so a task's `--color` or `--release` never reaches `knave`. A flag takes its value as the next word or after `=`; a switch takes none, so `--dry-run=true` is an error. A lone `--` ends the flags, a flag given twice is an error, and `--help` prints the usage line and exits 0. **This differs from `defn`'s defaults on purpose, by the owner's choice:** a task's defaults come from a command line, so a default is a literal, never an expression, and so it cannot be confused with a list type such as `(ref str)`. A string default is a `string`, because a `(ref str)` has no `from-str`.
3. **Types.** A parameter's type comes from the usual inference or a written type, and each argument parses into it with `parse`, so the rule is the prelude's `from-str` and no second one. A default takes its type from the usual literal pinning, so `8000` for a `u16` parameter is a `u16`. A parameter whose type cannot come from text has no `from-str`, and the type checker reports the missing trait at that parameter. Inference wins: in `(deftask pinned (port) (println (double port)))`, `port` is the `u16` that `double` takes. **A parameter that nothing fixes, as `who` in `(deftask greet (who) (println who))`, falls back to `string`,** by the owner's decision, and the fallback never overrides inference. The owner chose to keep it out of the type checker: `(task-signature name)` in `prelude/task` expands to `(infer task-function)`, `knave task` checks a program that holds it and reads the text from the typed program, and passes it to `(run-task name signature)`, which gives `string` to each parameter whose signature type is a type variable of its `where`, unless the parameter has a written type, a literal default or a switch. The rule lives only in the prelude, and Rust names neither `string` nor the fallback. `run-task` places each argument at its parameter in the `deftask` with `pinned-at`, a builtin that only it uses, and the call at the task's name, so a body that needs more than a `string`, as `(square side)` with a `number` bound, gets the checker's `L0404` labeled at `side`, and a trait the program implements for `string` works. Every error of a task labels the user's `deftask`, never the generated runner. A task's value must be `unit` or a `result` whose error implements `display`: the runner passes it through `task-outcome`, a trait of `prelude/task`, so any other value is `L0404` at the task's name.
4. **Command-line errors.** Each is exit 2 and names the task or the parameter: a missing positional argument, an extra one, an unknown flag, a flag with no value, a value given to a switch, a flag given twice, a required flag that is absent, and a value that does not parse. Each but the last also prints the task's usage line.
5. **Exit code.** The command exits with the task's exit code. A task's value is the program's result, as `main`'s is, so a task that returns `(err ...)` fails as `main` does, and a task killed by a signal ends the command by the same signal. An entry whose result is a `result` with an open success type that no bound constrains, such as a server loop that only returns on an error, takes `unit` there, for `main` and for a task alike, so neither needs a written type. Any other open type in the entry's result, a bounded success type as in `(let-try (x (parse "1")) (ok x))` or the error type of `(ok (do))`, is `L0407` at the returned form.
6. **Working directory.** A task runs at the project root, wherever the command was typed.
7. **Listing.** `knave task` lists each task in name order: its name, its positional parameters as `<name>`, its flags as `[--port 8000]`, `--port <port>` or `[--dry-run]`, then the first line of its docstring, written where `defn` takes it. The project file lists nothing.
8. **Other errors.** Usage errors with a help, exit 2: outside a project, a project with no `tasks/` directory, and an unknown name, which suggests the closest task or tells how to list them. Each task error has its own code: two tasks of one name `L0356`, a name that is not kebab-case `L0357`, a positional parameter after a flag `L0358`, and a `deftask` outside `tasks/` `L0359`.
9. **Other commands.** `knave check` with no file checks `:entry`, then the task tree in two checks whatever the number of tasks: one reads every task's signature, and one checks a program whose entry runs every runner with its signature. `run-task` reads the result type from the signature: an open, unbounded success is pinned to `unit` by `unit-success` in the prelude, and an open error, or a bounded open success, is `L0362` at the task's name under `task` and `check` alike, so the type stage has no task rule. An error that two checks report prints once. `knave fmt` with no path lays out the project root, `tasks/` included. `knave build` and `knave test` do not see `tasks/`.
10. **Building.** With no interpreter, a task compiles through the path `knave run` uses: rustc, or Cargo when the project uses crates, into `<root>/target/knave/<project>/tasks/<name>/`, with a Cargo bin named `<project>-task-<name>-<hash>` so it never replaces the program's binary in the shared target. **The command's interface stays when an interpreter arrives:** the name, the arguments, the working directory, the exit code and the listing do not depend on how a task runs.

### Known tech debt

**Every task command type checks the task tree twice:** once to read the signatures, once to build or check the runners, whatever the number of tasks. The interpreter RFC's type checking of macro code is meant to remove the first check, because a macro could then ask for the signature itself.

### What the compiler adds, and why

The prelude module `prelude/task` holds `deftask`, the entry `run-task`, the command-line parser, and the lookups `task-names` and `task-listing`. The compiler adds only what the prelude cannot express:

- **The `knave task` command**, which finds the project and `tasks/`, prints, builds and runs a process at the root. Only `src/cli/` touches the file system and runs processes.
- **An entry in `prelude/task`'s scope that keeps its value**, `ExpansionSession::task_runner_entry`. A program cannot write a `main` for code it does not import, and the test runner's entry ends in `unit`, which would drop a task's `result`.
- **`ExpansionSession::expression_in`**, which expands one form in a prelude module's scope, so the command reads `(task-names)`, `(task-listing)` and `(task-signature name)` without exporting them into every program.
- **The builtin `(pinned-at place form)`,** which `run-task` alone uses to keep a place outside its call through the expander's relocation, so a task's errors label its `deftask`. `located` is unchanged for every program, after three rounds of regressions from changing it; its own bug is item 421.
- **Directories a task cannot reach,** in `ModuleSources`, because the module search is the compiler's: `tests/` from both roots, and `tasks/` from the source root, compared as canonical paths.
- **The builtin `(in-tasks-directory? form)`,** because only the compiler knows which file a module comes from, and `deftask` must refuse a module outside `tasks/` under every command.
- **The help of a trait that declares no method shows for every type,** so `task-outcome`'s help reaches a task that returns an `i64`.
- **One report of an error shared by the entry and the tasks,** in `knave check`, which checks each task's runner as the entry of its own program.
- **Reading the signature from the typed program.** A macro runs before the checker, so only the command can carry `infer`'s answer from the first check into the runner. It reads the string literal that the checker leaves in place of `infer`, and adds nothing under `src/types`.
- **A `unit` success for an entry's open `result` that no bound constrains,** in the type checker, because a free type variable in `main`'s result reached rustc as `E0282`. Any other type that stays open in the entry's result is `L0407` at the returned form, for `main` and a task alike. It fixes `main` as much as a task, and the owner decided to keep it as its own commit.

## Decision

The owner decided on 2026-11-09:

1. **A task cannot reach `tests/` modules.** A task runs the program's code, and a test helper is not part of it.
2. **`knave check` with no file also checks every task,** changed from this RFC's first recommendation.
3. **A task is `(deftask name (parameter ...) "docstring" body ...)`,** a prelude macro shaped like `defn`, in any `.knv` file under `tasks/`, several per file. There is no `main` in a task file. Its description is the first line of its docstring.
4. **A task's name is kebab-case,** enforced now, since enforcing it later would break existing tasks.
5. **Parameters bind command-line arguments** in a flat list, `(deftask serve (:port 8000) "docstring" body ...)`: names are positional, keywords are flags with an optional type and an optional literal default, and a literal default is a deliberate difference from `defn`, as point 2 states.
6. **A parameter that nothing fixes falls back to `string`, outside the type checker:** the signature lookup of point 3, option A of the owner's two. A type checker rule scoped to `deftask` was the other option. A missing trait is the checker's own error at the parameter, with no list of `string`'s traits in the prelude.
7. **`deftask` needs no `use`:** the prelude exports it. Importing all of `prelude/task` into the prelude cost `empty_program` 39 KB of peak memory (+7.45%) and +3.15% of instructions, because its macros and their helpers load with the module. `deftask` is instead a small macro of `prelude/syntax` that expands to a `use` of `prelude/task` and a call of its `define-task`, so the module loads only when a program writes a task. That costs `empty_program` about 1.1 KB of peak memory (+0.21%), which the owner accepted.
8. **Every error of a task labels the user's code,** never `<task runner>` or `<task lookup>`, and `knave check` rejects what `knave task` rejects, with no doubled diagnostic.
9. **The entry's `unit` success stays,** settling only a variable that no bound constrains.
10. **No new fallback for an open error type:** `(deftask k () (ok (do)))` is `L0407` at the task's name under `knave task` and `knave check`, and never reaches rustc.
11. **`deftask` fails outside `tasks/`, `:help` is a definition error, and `--help` prints the docstring too.**
12. **`located` stays as it is for every program;** `run-task` pins its places with its own builtin, `pinned-at`.
13. **`knave check` is linear in the number of tasks:** two checks in all, with no task rule in the type stage.

## Rejected options and why

- **A file per task with a `main`:** one task per file, a name taken from a file name, and a `main` that parses its own arguments.
- **Tasks listed in the project file:** the list repeats what the code shows, and the owner prefers the file to list nothing.
- **Shell strings, as `bb.edn` has:** not portable, never checked, and code in the data file.

## Rule

[docs/language.md: Projects](../docs/language.md#projects)
