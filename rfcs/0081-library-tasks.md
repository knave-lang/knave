# RFC 0081: Tasks from libraries

**Status:** Accepted
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

The wrapper repeats ledelint's own `tasks/ledelint.knv`, drifts from it when ledelint changes, and must avoid the file name `tasks/ledelint.knv`, which `L0258` reserves for the library. The owner chose the Mix model (roadmap item 584): a library declares the tasks it exports, and listing the library makes them available. With this RFC, the `libraries` entry alone makes `knave task ledelint` work.

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
deploy  Deploys the shop.

ledelint
  ledelint [--path .]  Checks that each Knave file reads top-down, like a newspaper.
$ knave task ledelint --path src
```

### Loading: library task files join the project's task program

The command loads each exporting library's `tasks/` into the one program it builds from the project's `tasks/`, as library modules under the library's prefix.

### Loading: a library task runs in the library's own task program

The command builds a library's tasks as the library's own `knave task` builds them, in a program of their own, and changes only where the program runs and where it is built.

## Recommendation

Take the `exported-tasks` section, and run a library's tasks in the library's own task program. The decisions the owner asked for:

1. **Declaration.** A project's `defproject` MAY hold `(exported-tasks name ...)`, bare task names. A dependent of the project can run those tasks; every other task stays private to the project, as a name stays private to its module until `export` lists it. No section means the project exports nothing. Reasons:
   - A library's `tasks/` also holds its author's own commands, such as a release or a fixture refresh. Exporting all of them, the first option, puts those commands in every user's listing, and runs them at the user's root, where they do harm.
   - A task's code has no binding for `export` to list: `deftask` defines `task-<name>`, not `<name>`. A marker on `deftask` breaks the shape of `defn` that RFC 0066 gave it. Either is an exception to a rule of the syntax.
   - The project file already states what a library offers its dependents, through `:lib`. RFC 0066 kept the list of tasks out of the project file because it repeats the code. This list does not repeat the code: it states which tasks are public, which no code states.
   - The section is data, so a command learns which libraries export a name without expanding a task, and loads only the library it needs.
   - `prelude/project-file` states the section's shape and its role with `project-role`, so the compiler takes its name from the prelude.

   A section entry that is not a symbol is `L0708`, whose explanation gains that case, and a name listed twice is `L0709`. The `lints` section has codes of its own for both, `L0735` and `L0731`, because its entries carry a level; these entries are bare names, as a dependency's name is.
2. **The library's own task program.** A library task builds and checks as `knave task <task>` in the library's checkout would: the program loads the library's `tasks/` tree as the root project's, with the library's source root, the directory of its `:entry`, its module paths, its own libraries, its crates and its `Cargo.lock`, copied in and never back. Reasons:
   - `use` in a library task resolves the same in the author's repository and in a dependent, whatever the library's `:entry` and `:lib`, so the author's `knave check` covers what a dependent runs, given the same compiler.
   - A library task module's path is the path it has in the library's own project, and the module namespace is that project's. The tree of RFC 0066 holds unchanged: `tasks/` first, then the source root, `tests/` unreachable, the source root never reaching back into `tasks/`, `L0256` when a task module takes a source module's path. A library's task modules never meet the project's own task program, so no `use` of the user's, `(use ledelint/util)` included, can bind one, and two libraries' trees never meet each other.
   - Every task rule of `prelude/task` holds as written. `project-folders`, `check-task-directory`, `check-task-module`, `L0356`, `L0257`, `L0258`, the hidden-folder skip and the lookups by bare name all work inside one project's tree, and each program holds one project's tree.
   - A dependent's crates never join the library task's build, and its libraries never reach the task's code. The task depends on the library at its commit and on the compiler.

   The program differs from the library's own in four ways, each because the user cannot edit the library's files:
   - A git library's task program keeps the checkout bounds it has as a library: a path that leaves the checkout is `L0728`, and a module file that a symbolic link leads out of it is `L0254`.
   - It compares no `:knave`, as RFC 0079 decides for every library, so it prints neither `L0736` nor `L0737`.
   - It applies no project's `lints` levels and prints no warning: its errors print, and its lints and warnings, such as `L0803` or `L0256`, belong to the library's own repository, as `knave lint` never lints a library's task.
   - Its lookups, which `src/cli/` writes, take the name of the user's `libraries` entry as an argument, for the listing's block and `L0367`'s note.

   A tool-only library needs no root module and no entry, by two rules that hold for every project:
   - A library whose `:lib` names no file is no longer an error at its `libraries` entry. A `use` of its root module, the form that needs the module, is `L0262`, which module resolution reports and `ExpandProblems` words, so `L0724` leaves the code table. A library with no root module then lists and runs its tasks.
   - `knave check` with no file skips a missing `:entry` that the project file does not write, when the project has a `tasks/` directory or an `exported-tasks` section, and checks the task tree alone. A written `:entry` that names no file stays an error. So the author's `knave check` covers what a dependent runs.
3. **Resolution.** `knave task <name>` takes, in order:
   - The project's own task of that name. A project's code is the user's choice, and it can wrap or replace a library's task.
   - Otherwise, the one direct library whose section lists the name. Two or more is an error that names each, with the qualified form of each.
   - `knave task <library>/<task>` always runs that library's export, shadowed or clashing. Task and library names are kebab-case, so the `/` cannot be part of either, and the form reads as a module path does.

   **`src/cli/` resolves the name and prints its errors.** The rule chooses which program to build, and the prelude runs only inside a program, so it cannot choose between programs. The command takes the project's own task names from the project's task program, as `TaskCommand` does today, and each direct library's exported names from the libraries' project files, which it reads already. It loads no library's tasks to resolve a name. A project with no `tasks/` directory and no `exported-tasks` section has no task names of its own and builds no task program of its own, so ledelint's README case, a project with only a `libraries` entry, resolves `ledelint` from the sections alone and runs it. A project with a section and no `tasks/` builds a task program that holds no task file, as "The new diagnostic" states. Every resolution error is an uncoded usage error that exits 2, as RFC 0066's unknown name is, with its wording in `src/cli/`.

   A run checks the project's own task program, when `tasks/` or an `exported-tasks` section exists, and only when the name resolves to a library, that library's program: `knave task deploy` never loads a library's tasks, so a broken library never blocks the user's own task. A qualified run, `knave task ledelint/ledelint`, needs no task name of the project's, so it builds only the library's program, and a broken task of the user's never blocks it. The clash arises only when a name is run, as a clash of two glob imports does in Rust: two libraries may export one name, and the user who never runs it never sees an error.

   `knave task` with no name lists the project's tasks as today, then one block per library that exports a task, in library name order, headed by the library's name and indented by two spaces. With no task of its own, the listing starts with the first library block. Each row shows the name that runs its task: the bare name when it resolves to that task, the qualified name otherwise.
   - The listing loads the task program of every direct library that exports a task, as `knave check` does, because a row's parameters and summary come from the expanded task. A project that lists no exporting library pays nothing.
   - `src/cli/` knows from the project's task names and the libraries' sections which exported names the project's own tasks shadow and which two libraries share, and passes the library's name and those names to the library program's `task-listing`. `prelude/task` lays out the block, heading, indentation, qualified names and columns, so each block aligns its own columns.
   - When the project's own task program fails, its diagnostics print, no library block prints, because no row can tell a bare name from a qualified one without the project's task names, and the command exits 1, as it does today.
   - When a library's program fails, its diagnostics print on standard error and the library gets no block. The project's own tasks and the other blocks print as usual, and the command exits 1, so a CI that lists tasks learns of the failure. `L0367` and every other error located in the library's files name the library, so the missing block needs no layout of its own.
4. **Where it runs.** A library task runs at the root of the project where `knave task` was typed, so `ledelint` lints the user's project. Its arguments pass as a project task's do: every word after the name belongs to the task, `--help` prints its usage and docstring, and the command exits with the task's exit code. It builds into `<root>/target/knave/<project>/library_tasks/<library>/<task>/`, with a Cargo bin named `<project>-library-task-<library>-<task>` and the build directory's hash. The underscore keeps the directory apart from every kebab-case name, so neither a project task's directory nor the entry binary of a project named `library-tasks` can meet it, as `test_runner` does for `knave test`.
5. **Transitive libraries.** Only direct libraries export tasks to a project. A `use` of a library that only another library lists is already `L0252`, so a project reaches commands by the rule it reaches code. A library's library is its implementation, which it can change without a breaking release, and the user who wants its task lists it.
6. **Other commands.**
   - `knave check` with no file checks what `knave task` can check, so RFC 0066's decision 8 holds: the project's own task tree, and the task program of each direct library whose section lists a task, with RFC 0066's two checks each. A user's CI then learns that a pinned library's task fails on its compiler before someone runs it. The cost falls only on projects that list an exporting library.
   - `knave lint` checks the programs `knave check` checks, and lints only the project's own modules, so it never lints a library's task.
   - `knave test` and `knave build` do not read `tasks/`, a library's included.
   - `knave upgrade` reads only the root project file and loads no library. It keeps the section, as it keeps any section, and the files it owns under the owner's decision D135 hold no task.
   - `knave fmt` lays out the section with the shape `prelude/project-file` gives it.
7. **An older compiler.** No command compares a library's `:knave`, as RFC 0079 decides, in the user's project or in a library's task program. Once ledelint adds its section, a user whose compiler predates this RFC and who moves the `:rev` gets `L0707`, an unknown section, at ledelint's project file. Every section that a later Knave adds to a library meets the same case, `lints` included, so a fix belongs to RFC 0079's rule for every library and not to this section.

### The new diagnostic

`prelude/task` reports a section name that no `deftask` of the project's tree defines, so it takes the macro range, `L0367`, `missing-exported-task`, and exits 1 as every coded error does. The task program's lookup compares the section's names with the tree's tasks: `src/cli/` runs one lookup per name, `(exported-task-check :exported-task name)`, reading the project file into that lookup's own session so the name keeps its place in `project.knv`, and so reports every missing name, each beside the tree's other errors. It fires in every program that checks the project's tree: in the library's own project under `knave check` and `knave task`, and in a dependent under each command that loads the library's task program. A project with a section and no `tasks/` still gets a task program, holding no task file, under `knave check` and `knave task`, except a qualified run, so each name is `L0367` there. A name that is not kebab-case cannot name a task, so it is `L0367` too.

```text
error[L0367]: no task named `ledelnt` to export
 --> project.knv:3:19
  |
3 |   (exported-tasks ledelnt)
  |                   ^^^^^^^ no `deftask` under `tasks/` defines this name
  |
help: a task with a similar name exists
  |
3 |   (exported-tasks ledelint)
  |                   ~~~~~~~~
  = note: run `knave doc L0367` for an explanation
```

With no similar task, the help is ``define the task with `deftask` in a file under `tasks/`, or remove the name``. In a dependent, the file sits in the git store, which the user cannot fix, so the error adds `note: library `ledelint` exports this name`. For a `:git` library, the help ``pin a `:rev` of `ledelint` that defines the task`` replaces the help that edits the name, since that help would edit a file in the git store. Both name the library by the user's `libraries` entry, which the lookup takes as an argument.

The command's own usage errors exit 2, as RFC 0066's unknown name does:

```text
error: libraries `alpha` and `beta` export a task named `fmt-check`
help: run `knave task alpha/fmt-check` or `knave task beta/fmt-check`

error: libraries `alpha`, `beta` and `gamma` export a task named `fmt-check`
help: run `knave task alpha/fmt-check`, `knave task beta/fmt-check` or `knave task gamma/fmt-check`

error: no library named `ledelnt`
help: did you mean `ledelint`?

error: library `hints` exports no tasks
help: run `knave task` to list the tasks

error: library `ledelint` exports no task named `newspaper`
help: run `knave task` to list the tasks

error: `a/b/c` is not a task name
help: name a task as `<task>` or `<library>/<task>`
```

A word with a `/` that does not split into two nonempty names, as `ledelint/`, `/fmt` or `a/b/c`, is the last error. A qualified name suggests among the libraries the project lists. An unknown bare name suggests the closest name among every row of the listing. A project with no `tasks/`, no `exported-tasks` section and no direct library that exports a task is the existing usage error, whose help also names a library's `exported-tasks`.

### What the compiler adds, and why

`prelude/task` keeps the missing export and each block's layout. The compiler adds only what the prelude cannot express:

- **Reading the section,** because the project file is data that the compiler reads.
- **Resolving a name and wording its errors,** in `src/cli/`, because the rule picks a program, which no program's prelude can do.
- **Building a library's task program** with the library as its root project, its working directory at the user's root, its build directory under `library_tasks/`, its checkout bounds, and no lint levels or warnings, in `src/cli/` and `ModuleSources`, because only they touch the file system and run processes.
- **No builtin.** The section's names reach `L0367` as `:exported-task` arguments of the lookup that `src/cli/` writes, each a symbol read from `project.knv` that keeps its place there, and the library's entry name is an argument of the same lookups.
- **`L0262` on a `use`** of a library's missing root module, for every library, in module resolution.
- **`knave check` skipping a missing default `:entry`** when the project has tasks to check.

### Supersession

When the owner accepts, these status lines change:

- RFC 0066 becomes `Superseded by 0081` in part: its recommendation item 7, the listing, which gains library blocks; item 8, which makes a project with no `tasks/` a usage error, now only when no direct library exports a task; item 9, `knave check` with no file, which now skips a missing default `:entry` and checks the task program of each exporting library; and item 10, the build layout, which gains `library_tasks/`.
- RFC 0079 keeps its rule that no library's `:knave` is compared; this RFC changes nothing of it.
- No RFC states `L0724`. Its replacement by `L0262` changes `docs/language.md` and the explanations: `L0724.md` goes, whose "whether or not the program uses the library" and "stop at the first such problem" no longer hold, and `L0262.md` comes.
- `docs/language.md`'s statement that `L0256` always prints gains the library task program, which prints no warning.

## Decision

The owner accepted this RFC as recommended on 2026-11-22.

## Rejected options and why

- **Every task under a library's `tasks/`:** leaks the author's own commands into every dependent and runs them at the user's root. Mix accepts this: a task without `@shortdoc` is hidden from `mix help` but still runs.
- **A marker in the task's code:** `export` would need a binding that `deftask` does not make, and a keyword breaks `deftask`'s shape; both are exceptions to a rule. A command would also expand every library's tasks to learn which it exports.
- **Library task files in the project's task program:** a library task module would take the library's prefix and share the namespace of the library's source modules, a user's task could reach it, and two libraries' trees would meet. `use` would resolve from `:lib`'s directory in a dependent and from `:entry`'s in the library's own repository, so the author's check would not cover the dependent's run. `project-folders`, `L0356`, `L0359` and the lookups by bare name would each need a library case.
- **Requiring `:entry` and `:lib` in one directory:** fixes only the search, and is a rule that only exporting projects obey.
- **Resolving in the prelude:** the prelude runs inside one program, and a prelude `fail` is a coded error that exits 1, where a wrong command line exits 2. With no `tasks/`, no program would exist to run it.
- **Comparing a library's `:knave` in its task program or on a project-file error:** an unused library task would fail the user's `knave check` on a version that the library's code, never compared, does not need, and it adds an exception to RFC 0079's rule.
- **Leaving library tasks out of `knave check`:** breaks RFC 0066's decision 8, so `knave check` would pass where `knave task` fails.
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

  Mix fits because Knave already pins a library by commit in `libraries`: the command a user runs is the one their project pinned, with no second list, and it runs at their root. The `exported-tasks` section borrows npm's declaration in the library's data file to keep the author's own tasks private, and the library's own task program borrows `cargo install`'s build of a crate as its own project.

## Rule

[docs/language.md: Projects](../docs/language.md#projects), once the owner decides.
