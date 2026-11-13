# RFC 0073: the lint exit code and the `lints` section

**Status:** Accepted
**Date:** 2026-11-13

## Problem

[RFC 0069](0069-lint.md) made `knave lint` exit 0 on warnings alone (its Q-c answer), so a CI step that runs `knave lint` passes whatever it prints. A project that wants lints to fail CI had only `deny`, which also stops `knave check`, `knave build` and `knave run` during development.

RFC 0069 also decided that a project silences or tunes a lint in a `lints` section of `project.knv`, as `(lints (function-lines 20) (unused-binding allow))`, but left the section's exact forms open:
- how a level and a threshold are told apart;
- what an unknown name gets;
- where a level applies;
- where the section's words come from.

This RFC records both.

## Options

### The exit code of `knave lint`

- **A: exit 0 on warnings, as RFC 0069 decided.** CI needs `deny` in the project file, which also fails `check` and `build`.
- **B: exit 1 when it prints a lint, with a per-lint opt-out in the `lints` section.** A plain `knave lint` fails CI on any lint. A project that wants a lint printed without failing writes `warn`, one that wants it gone writes `allow`, and `deny` also fails `check` and `build`.
- **C: exit 1 under a command-line flag such as `--deny-warnings`.** CI scripts carry the policy instead of the project.

Only lints, the `L08xx` codes, count under B. L0217, L0249, L0251 and L0256 are warnings that are no lint, so they print and never change the exit code.

### An entry of the `lints` section

- **A: one entry per rule, holding either a level or a threshold.**
- **B: an entry holds a rule, at most one level and at most one threshold, in either order.** An integer literal is a threshold, a symbol is a level, so the kind tells them apart.
- **C: a rule may appear twice, once per kind.**

```lisp
(defproject browser
  (lints (function-lines 20 deny) (unused-binding allow)))
```

The levels are `warn`, `allow` and `deny`. A rule with no level keeps the default, which prints the lint and fails `knave lint`.

### Unknown names and thresholds

- **An unknown lint name** is an error, as an unknown project key is, with a suggestion of the closest lint name. The names come from the code table in `expand/src/diagnostic/code.rs`, which already gives every lint its code and its name, and through which `warn` resolves `:code`. No command loads `prelude/lint` to read the section.
- **An unknown level word** is an error naming the levels, with a suggestion.
- **A threshold on a rule that reads none:** only `prelude/lint` knows which rules read a threshold, and only `knave lint` loads it. The options:
  - every command accepts any integer, and `knave lint` reports a threshold its rule never reads;
  - `build.rs` extracts the threshold rules into the prelude state, which gives `build.rs` work beyond the prelude state;
  - Rust lists them, which restates the prelude.

### Where a level applies

- **A: each stage applies the levels before it returns its warnings.** A stage does not know the project.
- **B: `src/cli/` applies them once, where it prints.** One value holds each lint's level. The report that prints the warnings drops an allowed lint, gives a denied lint the error severity, and sets the exit code, so every command applies the levels once.
- **C: the `warn` built-in drops allowed lints.** The lints that Rust reports would escape it.

### Where the section's words come from

`prelude/project-file.knv` states the project file's forms. The section's name and its level words belong there, beside `libraries`, `dependencies` and `dev-dependencies`. The module leaves a note in the saved prelude state, as `prelude/internal/docstrings.knv` does for the docstring syntax, and the project reader reads the words from that note. Rust keeps only what each level means. This note is the one that the reading of the project file's head and section names needs too.

### Cost

A project file without a `lints` section reads as today. Applying an empty set of levels returns the warnings unchanged, with no copy and no walk, so a project without the section and every single file pay one emptiness check per report.

## Recommendation

- B for the exit code.
- B for an entry.
- An unknown name or level is an error with a suggestion.
- A threshold is rejected until the first size rule, then checked by `knave lint` alone.
- Levels apply once in `src/cli/`.
- The words come from `prelude/project-file.knv`.

Only the root project's section applies, because a command has one exit code. A library's section is still read and checked, so the library checks alone.

## Decision

**The exit rule, decided on 2026-11-13.** The owner asked, "maybe we exit 1 on warning and have an option to opt out?", then chose "option 1":
- `knave lint` exits 1 when it prints a lint at its default level, and only lints, the `L08xx` codes, affect the exit code;
- the opt-out is per lint in the `lints` section, with no command-line flag;
- `warn` prints without failing, `allow` silences, and `deny` also fails `check` and `build`.

This replaces RFC 0069's Q-c answer.

**The `lints` section, decided on 2026-11-13.** The owner's words: "yes, those lints section details look right".
- An entry holds a rule and at most one level and one threshold, in either order. A rule listed twice, or an entry with neither, is an error.
- An unknown lint name or level is an error with a suggestion. The lint names come from the code table, so no command loads `prelude/lint` to read the section.
- A threshold is rejected until the first size rule lands. After that, `knave lint` reports a threshold its rule never reads.
- Only the root project's section applies. A library's section is still read and checked.
- The section's name and level words come from `prelude/project-file.knv`, through its note in the prelude state. Rust keeps only what each level means.
- The levels apply once, in `src/cli/`, wherever a lint prints.

The exit rule is built. The `lints` section is not built yet: its rules enter `docs/language.md` with that build.

## Rejected options and why

- **Exit 0 on warnings (A):** a CI step that runs `knave lint` passes whatever it prints.
- **A command-line flag (C):** the policy would live in each CI script instead of the project, and the same program would exit differently by flag.
- **One entry per rule, holding a level or a threshold:** a rule that takes both could not say both.
- **A rule listed once per kind:** a reader must collect a rule's settings from several places in the file.
- **Applying levels in each stage, or in `warn`:** a stage does not know the project, and `warn` misses the lints that Rust reports.
- **Restating the section's words or the threshold rules in Rust:** the prelude already states them.

## Rule

[docs/language.md: Lints](../docs/language.md#lints)
