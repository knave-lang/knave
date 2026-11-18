# RFC 0079: A project records its Knave version

**Status:** Accepted. [docs/language.md: Projects](../docs/language.md#projects) states the rule.
**Date:** 2026-11-18

## Problem

`knave new` writes three files that carry the compiler's version: the two copies of the `writing-knave` skill, with `knave-version: "0.18.0"`, and `.github/workflows/knave.yml`, with `KNAVE_VERSION: "0.18.0"`. The project file records no version. After an upgrade of Knave, no command touches these files, so the user edits three files by hand, CI keeps installing the old compiler, and the skill tells an agent about the wrong release.

No form of the language can solve this: the files are not Knave source, and nothing compares the running compiler with the project.

## Options

### A: `:knave` in `defproject`, and `knave upgrade` with a template history

The project file records the version as an ordinary key, as `go.mod` records `go` and `package.json` records `packageManager`.

```lisp
(defproject demo :knave "0.18.0")
```

The workflow holds no version. Its first step reads `:knave` from `project.knv`, because no Knave is installed when the step runs, and exports `KNAVE_VERSION` for the install and cache steps.

```yaml
- name: Read the Knave version from project.knv
  run: |
    version=$(grep -o ':knave "[^"]*"' project.knv | cut -d '"' -f 2)
    test -n "$version" || { echo "::error file=project.knv::project.knv records no :knave version"; exit 1; }
    echo "KNAVE_VERSION=$version" >> "$GITHUB_ENV"
```

`knave upgrade` records the running compiler in `:knave`, then visits each owned file. A missing file stays missing. A file that holds a text some Knave release wrote, for any version, is rewritten. Any other text is the user's change: the command keeps it and prints a unified diff. The compiler embeds the earlier texts of its templates, so it tells the two cases apart with no state on the user's machine.

### B: a `.knave-version` file, and `knave update` that overwrites under git

A root file holds the version, as `.ruby-version` and `.nvmrc` do. The command overwrites every owned file and refuses when one has uncommitted changes, as `cargo fix` does.

### C: A, plus a published `setup-knave` action

The workflow calls an action that reads `version-file: project.knv`, as `actions/setup-go` reads `go.mod`, so the workflow rarely changes.

## Recommendation

A. The version lives in the one project file, the workflow reads it, and a user's change to an owned file is never lost. The template history is work done when `knave` is built.

## Decision

The project owner decided on 2026-11-18:

- **D125.** The workflow reads the Knave version from one project file, and a command rewrites the files Knave owns. The command is not `init`.
- **D126.** The version is `:knave "0.18.0"` in `defproject`. A compiler older than `:knave` is an error with a `cargo install knave --version <v> --locked` help. A newer compiler is a warning with a `knave upgrade` help. Every command that finds the project reports it, and only `knave upgrade` clears the warning.
- **D127.** The command is `knave upgrade`. It records the running compiler in `:knave`. It rewrites each owned file that holds a text some Knave release wrote. It keeps a changed file with a warning and a unified diff, and exits 0. It never creates a missing file, and it touches nothing else.
- A project without `:knave` gets no version check, and `knave upgrade` adds the key.
- A compiler older than `:knave` refuses to lower it in `knave upgrade`, with the same install help.
- `knave upgrade` takes no flag. Adding a skipped component later and a `setup-knave` action are out of scope.

## Rejected options and why

- **B:** it spreads the project's settings over two files, and it overwrites a committed change to the workflow, so a user's added step is lost by default.
- **C:** it needs a second repository with its own releases. It can follow A later without changing `project.knv`.
- **The name `update`:** Cargo, Gleam, npm and Bun use `update` for dependencies, and a Cargo user would expect `knave update` to refresh `Cargo.lock`.
- **The name `init`:** `cargo init` creates a package and refuses a folder that holds a manifest.

## Rule

[docs/language.md: Projects](../docs/language.md#projects)
