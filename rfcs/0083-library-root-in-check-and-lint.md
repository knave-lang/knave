# RFC 0083: The project's files decide what check and lint cover

**Status:** Accepted. Its rule 7 is superseded by 0085 for `knave build` and `knave run`. Where its rules 2 and 3 treat a written `:lib` whose file is missing as no library root, they are superseded by 0085's rule 12, which makes it an error.
**Date:** 2026-11-23

## Problem

`knave check` and `knave lint` with no file start from the project's entry, `src/main.knv` unless `:entry` names another file. A library project has a library root and no entry, so both commands fail on the missing `src/main.knv`, and a library cannot lint itself. [RFC 0081](0081-library-tasks.md) and the rule that a project of tasks needs no entry already let a project without an entry check its task tree, but a project whose code sits in its library root has no such path.

When a project holds both files and the entry never uses the library root, `knave check` checks the entry alone, so an error in the library root passes unnoticed.

## Options

### The files decide

`knave check` and `knave lint` with no file check whichever of the entry and the library root exist, as Cargo checks whichever of `lib.rs` and `main.rs` a package holds. The project file states nothing new.

```lisp
(defproject shelf)
```

```text
shelf/
  project.knv
  src/lib.knv
```

`knave check` in `shelf/` checks `src/lib.knv` and every module it reaches.

### A setting names the project's kind

`project.knv` writes `:kind :library`, and the commands start from the library root when the kind says so, as Zig's `build.zig` and Cabal's `.cabal` file list each target the package builds.

```lisp
(defproject shelf :kind :library)
```

Go, Gleam and Mix take a third path: they check every package or module of the project, whatever reaches it. `knave lint` already covers every file of a project, but in a library project it fails on the missing `src/main.knv` as `knave check` does, so the starting point of both commands is at stake.

## Recommendation

Let the files decide. The project file already names the library root through `:lib` and its default, so a `:kind` setting would repeat what the files show, and a project that holds both files would need a kind that means both. A setting is one more rule to learn and one more way for the project file and the files to disagree.

## Decision

The project owner chose on 2026-11-23: "`knave lint` and `knave check` in a project start from `src/lib.knv` when `src/main.knv` does not exist, and cover both when both exist. The files decide, as Cargo does with `lib.rs` and `main.rs`", with "no `:kind :library` setting in `project.knv`."

The rules of the decision:

1. The library root is the file `:lib` names, `src/lib.knv` when the project file does not write `:lib`. A project that writes `:lib "src/shelf.knv"` starts from `src/shelf.knv`.
2. `knave check` and `knave lint` with no file skip a missing entry that the project file does not write when the project has a library root, a `tasks/` directory or an `exported-tasks` section.
3. When the entry exists, they check the library root after the entry as a program of its own.
4. When the entry's program already loads the library root through a `use`, the library root is not checked again, so its errors print once.
5. A written `:entry` whose file is missing stays an error, even when the library root exists, so a typo never turns into a silent check of the library alone.
6. With no entry file, no library root, no `tasks/` directory and no `exported-tasks` section, the missing entry stays an error.
7. `knave check <file>` and every other command, `build`, `run`, `test`, `types`, `expand`, `emit` and `doc`, keep their behavior. `knave build` and `knave run` in a library project still fail on the missing entry.

## Rejected options and why

- **A setting names the project's kind:** the files already show what the project holds, so the setting repeats them and can contradict them, and a project with both files needs a kind for each.
- **Check every module whatever reaches it:** `knave lint` already does, and `knave check` stays a check of programs, so a module that no program reaches is not checked as part of one.

## Rule

[docs/language.md: Projects](../docs/language.md#projects)

[docs/language.md: Lints](../docs/language.md#lints)
