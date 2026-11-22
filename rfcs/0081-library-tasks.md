# RFC 0081: Tasks from libraries

**Status:** Proposed
**Date:** 2026-11-22

## Problem

`knave task` runs only the project's own tasks ([RFC 0066](0066-project-tasks.md)). A library that is a tool cannot offer a command. [ledelint](https://github.com/KarlHarnois/ledelint) is a linter that runs as a task, so its README asks each user to list it under `libraries` and then copy a wrapper into `tasks/newspaper.knv`:

```lisp
(use ledelint ->settings print-problems run)

(deftask ledelint (:path string ".")
  (let (problems (run (->settings :root (str path))))
    (print-problems problems)
    (when (not (empty? problems))
      (exit 1))))
```

The wrapper repeats ledelint's own `tasks/newspaper.knv`, drifts from it when ledelint changes, and must avoid the file name `tasks/ledelint.knv`, which `L0258` reserves for the library. The owner chose the Mix model (roadmap item 584): a library declares the tasks it exports, and listing the library makes them available. With this RFC, the `libraries` entry alone makes `knave task ledelint` work.

## Options

### Declaration: every task under a library's `tasks/`

Nothing to write: a library exports each task it defines. A library that exports nothing has no `tasks/`.

### Declaration: a marker in the task's code

`(deftask ledelint :export (:path ".") ...)`, or `(export ledelint)` in the task file.

### Declaration: an `exported-tasks` section in the library's `defproject`

```lisp
(defproject ledelint
  :knave "0.21.0"
  (exported-tasks ledelint)
  (lints (needless-type deny)))
```

A library that exports nothing writes no section. A dependent writes only its `libraries` entry:

```lisp
(defproject shop
  :knave "0.21.0"
  (libraries
    (ledelint
      :git "https://github.com/KarlHarnois/ledelint"
      :rev "<commit>")))
```

```text
$ knave task
deploy                Deploys the shop.

ledelint
  ledelint [--path .] Checks that each Knave file reads top-down, like a newspaper.
$ knave task ledelint --path src
```

## Recommendation

Take the `exported-tasks` section. The decisions the owner asked for:

1. **Declaration.** A library's `defproject` MAY hold `(exported-tasks name ...)`, bare task names. Every other task of the library stays private to the library's own project, as a name stays private to its module until `export` lists it. No section means the library exports nothing. Reasons:
   - A library's `tasks/` also holds its author's own commands, such as a release or a fixture refresh. Exporting all of them, the first option, puts those commands in every user's listing, and runs them at the user's root, where they do harm.
   - A task's code has no binding for `export` to list: `deftask` defines `task-<name>`, not `<name>`. A marker on `deftask` breaks the shape of `defn` that RFC 0066 gave it. Either is an exception to a rule of the syntax.
   - The project file already states what a library offers its dependents, through `:lib`. RFC 0066 kept the list of tasks out of the project file because it repeats the code. This list does not repeat the code: it states which tasks are public, which no code states.
   - The section is data, so a command learns which libraries export anything without expanding a task, and loads the task files of those libraries alone.
   - `prelude/project-file` states the section's shape and its role with `project-role`, so the compiler takes its name from the prelude.
2. **Resolution.** `knave task <name>` takes, in order:
   - The project's own task of that name. A project's code is the user's choice, and it can wrap or replace a library's task.
   - Otherwise, the one direct library that exports the name. Two or more is an error that names each, with the qualified form of each.
   - `knave task <library>/<task>` always runs that library's export, shadowed or clashing. Task and library names are kebab-case, so the `/` cannot be part of either, and the form reads as a module path does.

   `knave task` with no name lists the project's tasks as today, then one block per library that exports a task, in library name order, headed by the library's name. Each row shows the name that runs its task: the bare name when it resolves to that task, the qualified name otherwise. An unknown name suggests the closest name among every row. The error only arises when a name is run, as a clash of two glob imports does in Rust: two libraries may export one name, and the user who never runs it never sees an error.
3. **Where it runs.** A library task runs at the root of the project where `knave task` was typed, so `ledelint` lints the user's project. Its arguments pass as a project task's do: every word after the name belongs to the task, `--help` prints its usage and docstring, and the command exits with the task's exit code. Its code is library code: a `use` in a library task file searches the library's `tasks/`, then the directory of its `:lib`, with the library's prefix, and reaches only the libraries that the library's own project file lists. It builds into `<root>/target/knave/<project>/tasks/<library>/<task>/`, with a Cargo bin named `<project>-task-<library>-<task>` and the build directory's hash.
4. **Transitive libraries.** Only direct libraries export tasks to a project. A `use` of a library that only another library lists is already `L0252`, so a project reaches commands by the rule it reaches code. A library's library is its implementation, which it can change without a breaking release, and the user who wants its task lists it.
5. **Other commands.**
   - `knave check` with no file checks the project's own `exported-tasks` against its task tree, so a library author learns of a missing task in their own repository. It does not check a library's tasks: they hold no code of the user's, and their result depends only on the library at its commit, which the library's own `knave check` covers.
   - `knave lint` lints only the program's own modules, so a library's tasks never get linted in a dependent.
   - `knave test` and `knave build` do not read `tasks/`, a library's included.
   - `knave upgrade` reads only the root project file and loads no library. It keeps the section, as it keeps any section.
   - `knave fmt` lays out the section with the shape `prelude/project-file` gives it.

   One diagnostic is new. A section entry that is not a symbol is `L0708`, and a name listed twice is `L0709`, as for every section. A name that no `deftask` of the project's task tree defines is `L0738`, under `knave check` and `knave task` in the library's project, and under `knave task` in a dependent, which loads the library's tree:

   ```text
   error[L0738]: no task named `ledelnt` to export
    --> project.knv:3:19
     |
   3 |   (exported-tasks ledelnt)
     |                   ^^^^^^^ no `deftask` under `tasks/` defines this name
     |
   help: a task with a similar name exists
     |
   3 |   (exported-tasks ledelint)
     |                   ~~~~~~~~
     = note: run `knave doc L0738` for an explanation
   ```

   With no similar task, the help is ``define the task with `deftask` in a file under `tasks/`, or remove the name``. A name that is not kebab-case cannot name a task, so it is `L0738` too.

   The command's own usage errors, exit 2, as RFC 0066's unknown name is:

   ```text
   error: two libraries export a task named `fmt-check`
   help: run `knave task alpha/fmt-check` or `knave task beta/fmt-check`

   error: no library named `ledelnt`
   help: did you mean `ledelint`?

   error: library `ledelint` exports no task named `newspaper`
   help: run `knave task` to list the tasks
   ```

   A project with no `tasks/` and no library that exports a task is the existing usage error, whose help now also names a library's `exported-tasks`.

### What the compiler adds, and why

`prelude/task` keeps the listing's layout, the resolution order and the clash, and states them once. The compiler adds only what the prelude cannot express:

- **Reading the section.** The project file is data that the compiler reads.
- **Loading each exporting library's task tree** into the task program, as library modules, because only `src/cli/` and `ModuleSources` touch the file system.
- **Two builtins:** `(module-library module)`, the name of the library that holds a module, or `none`, and `(exported-tasks library)`, the names its section lists, because only the compiler knows a module's origin and the project files.
- **The working directory and the build directory** of a library task, in `src/cli/`.

## Decision

Pending the project owner.

## Rejected options and why

- **Every task under a library's `tasks/`:** leaks the author's own commands into every dependent and runs them at the user's root. Mix accepts this: a task without `@shortdoc` is hidden from `mix help` but still runs.
- **A marker in the task's code:** `export` would need a binding that `deftask` does not make, and a keyword breaks `deftask`'s shape; both are exceptions to a rule. A command would also expand every library's tasks to learn which it exports.
- **A transitive scope:** exposes a library's implementation and breaks the rule that a project reaches only the libraries it lists.
- **Prior art:**

  | Tool | How a library offers a command | Reach | Fit for Knave |
  | --- | --- | --- | --- |
  | Mix | every `Mix.Tasks.*` module in a dependency's code | every dependency on the code path | the model: listing the library is enough, and a task runs in the user's project |
  | Leiningen plugins | a plugin is a separate dependency in `:plugins`, apart from the code's | what `:plugins` lists | a second list repeats `libraries` |
  | Go 1.24 `tool` directives | the consumer writes each `tool` path in `go.mod` | what `go.mod` lists | the user lists commands one by one; ledelint would still need a line |
  | npm `bin` | the package's `package.json` lists its binaries; `npx` or scripts run them | direct and transitive, through `node_modules/.bin` | a declaration in the package's data file, as here, but transitive |
  | Clojure CLI aliases | the consumer writes `:aliases` with `:main-opts` or `:exec-fn` in `deps.edn` | what the consumer writes | code in the data file, which RFC 0061 forbids |
  | Cargo | `cargo install` puts a crate's binaries on `PATH`, outside any project | global | a tool runs apart from the project and its pinned version |

  Mix fits because Knave already pins a library by commit in `libraries`: the command a user runs is the one their project pinned, with no second list, and it runs at their root. The `exported-tasks` section borrows npm's declaration in the library's data file to keep the author's own tasks private.

## Rule

[docs/language.md: Projects](../docs/language.md#projects), once the owner decides.
