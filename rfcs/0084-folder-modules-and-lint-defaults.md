# RFC 0084: the `folder-modules` lint, folder locations and lint defaults

**Status:** Accepted
**Date:** 2026-11-24

## Problem

A project can grow to hundreds of modules in one folder, most often when a tool or an agent generates it, and nothing tells its author. A reader then browses a flat list that a few subfolders would order. A split also costs more the later it comes: a module's path is its file's path under the source root (`docs/language.md`, Modules), so moving a module changes every `use` of it.

The project owner decided the rule:

- `folder-modules` warns when a folder directly holds more than N `.knv` files. Files in subfolders do not count.
- N is 15 by default, and a project sets it as `(folder-modules warn <n>)` in the `lints` section, in the entry shape of [RFC 0073](0073-lint-exit-and-lints-section.md).
- It is off by default, at `allow`, as `needless-type` is.
- Folders under `tests/` and `tasks/` count. Once on, a library project warns too.
- The warning points at the folder itself. Its help is one fixed line, ``split `<folder>` into subfolders, one for each feature``, and its explanation says that moving a module changes every `use` of it.
- A threshold on a rule left at `allow` is `L0738`, with a suggestion that adds `warn`.
- No lint states its default level in Rust. Default levels move into `prelude/lint` (roadmap item 66), before or with this rule.

The language cannot express this today, for four reasons:

1. Every rule in `prelude/lint` sees one definition or one module, so no rule sees a folder.
2. Every diagnostic is located by a span in a source's text, and a folder has no text.
3. The `lints` section rejects every number with `L0733`, "no rule takes a threshold yet". RFC 0073 decided that this lasts until the first size rule, which this is.
4. `needless-type`'s default level is stated in Rust, in `LintDefaults::ALLOWED_UNLESS_WRITTEN`.

## Options

### Where the rule runs

#### A. A project entry point in `prelude/lint`

`prelude/lint.knv` exports `project-rule-names` and `check-project`. `knave lint` calls `(check-project rule settings folders)` once per run, for each listed rule whose level is not `allow`. `folders` holds one pair per folder that holds a module: the folder as a located string, and its module count. `settings` holds the limit.

```lisp
(check-project (quote folder-modules) (list 15)
  (list (list "./" 3) (list "src/" 23) (list "tests/" 9)))
```

`src/cli/` builds the counts from the file lists that the three walks of `knave lint` already return, so it walks nothing new:

- `SourceFiles::paths` from the project root;
- the test tree's walk;
- the task tree's walk.

#### B. The whole rule in `src/cli/`

Rust counts and words the warning. But the limit and the wording would sit in the compiler, though the prelude can express them.

### How a diagnostic is located at a folder

#### A. A folder is a kind of source

- `SourceFile` gains `SourceKind::File | SourceKind::Folder`.
- `SourceMap::add_folder(name)` adds a folder with empty text.
- `Label::at_folder(source)` labels it with an empty span and no message.
- The renderer draws a folder label as annotate-snippets' `Origin::path`, a location line with no excerpt. The JSON writer gives a folder label a `file` and no line or column.
- Every other consumer already reads a label's source by its name: source order, `PathScope`, the lint levels and the printed cap. They need no change, and `knave lint <folder>` filters folders as it filters files.

#### B. A location type beside the label

`Diagnostic` holds `Location::Label(Label) | Location::Folder(path)`. Every consumer of the primary label, 24 call sites of `primary().span()` among them, must handle both cases, `PathScope` included. Each case added there is a rule that each consumer must state again.

#### C. Label a file of the folder (round 1)

The owner ruled it out: a file is not the problem.

Under A, the warning renders as follows.

Plain:

```text
warning[L0812]: folder `src/` holds 23 modules
 --> src/
  |
  = note: `folder-modules` warns when a folder holds more than 15 modules
  = help: split `src/` into subfolders, one for each feature
  = note: run `knave doc L0812` for an explanation
```

Styled, with annotate-snippets' default styles, which `DiagnosticRenderer::styled` already uses. `‹…›` marks a style, and `‹/›` resets it:

```text
‹bold yellow›warning[L0812]‹/›‹bold›: folder `src/` holds 23 modules‹/›
 ‹bold bright-blue›-->‹/› src/
  ‹bold bright-blue›|‹/›
  ‹bold bright-blue›=‹/› ‹bold›note‹/›: `folder-modules` warns when a folder holds more than 15 modules
  ‹bold bright-blue›=‹/› ‹bold bright-cyan›help‹/›: split `src/` into subfolders, one for each feature
  ‹bold bright-blue›=‹/› ‹bold›note‹/›: run `knave doc L0812` for an explanation
```

JSON. Only `eval` and `repl` replies write JSON diagnostics, and `knave lint` has no JSON output, but the rule covers every diagnostic:

```json
{"severity":"warning","code":"L0812","message":"folder `src/` holds 23 modules","file":"src/","rendered":"warning[L0812]: folder `src/` holds 23 modules\n --> src/\n  |\n  = note: `folder-modules` warns when a folder holds more than 15 modules\n  = help: split `src/` into subfolders, one for each feature\n  = note: run `knave doc L0812` for an explanation"}
```

The layout above is the decided one. The exact gutter spacing and escape bytes are those that annotate-snippets 0.12.18 prints, which the build's snapshot records.

**No existing diagnostic's output changes.** Only `ProjectFolders` adds a folder source, and only when a project rule is on. Every other source is a `File`, whose branch in the renderer and in the JSON writer is today's code. The build shows it: every stage fixture, `--test diagnostic_snapshots` and `tests/cli.rs` pass with no snapshot change, and a unit test renders a file diagnostic alike with and without a folder source in the same `SourceMap`.

Two more rules for a folder:

- **The path filter:** `knave lint src` or `src/` prints `src/`'s warning and those of folders under it. `knave lint src/order` leaves out `src/`'s. A lint of a file prints no folder warning.
- **`warn` on a folder:** `warn` refuses `:label`, `:earlier` and `:suggest` on a culprit in a folder, with an error of the rule program, because a folder has no text to label or edit.

### Where default levels and thresholds are stated

#### A. A `lint-default` entry in `prelude/lint`, saved into the prelude state

`prelude/lint.knv` writes one entry per rule that has a default, in the shape of a `lints` entry:

```lisp
(lint-default needless-type allow)
(lint-default folder-modules allow 15)
```

The macro writes a module note, `lint-defaults`, and expands to `(do)`, as `lint-level` does in `prelude/project-file.knv`. Then:

- **When `knave` is built,** `SavedPrelude::build` expands `prelude/lint` alone and reads the notes into a `LintDefaults` table. The table becomes part of the saved `ProjectVocabulary`, as RFC 0073 saves the level words.
- **Every command,** the project reader and `check`, `build`, `run` and `lint`, reads each default without loading `prelude/lint`.
- **A size rule's limit** is the written threshold or else the default, and reaches the rule in `settings`.
- **A default that names an unknown lint or level** stops `knave`'s build.

#### B. Levels in a prelude table that only `knave lint` reads

The project reader and `check` cannot know a default, though `deny` and `allow` must apply in every command.

#### C. Levels in Rust's `LintDefaults` (today)

The owner ruled it out.

A default threshold may also sit in a second table, `rule-thresholds`, that only `knave lint` reads (round 1). Then each size rule states its defaults in two forms, and only `knave lint` knows which rules take a number.

### A bad threshold

A threshold is a whole number from 0 to 4294967295.

| Written | Diagnostic |
|---|---|
| `-3`, `1.5`, `4294967296` | `L0733`, in every command |
| two numbers in one entry | `L0709`: a lint entry gives two thresholds |
| a threshold on a rule with no default threshold, as `(unused-binding warn 3)` | `L0738`, in every command: `` `unused-binding` takes no threshold `` |
| a threshold on a rule left at `allow`, as `(folder-modules 20)` | `L0738`, in every command, with the suggestion `(folder-modules warn 20)` |

### What counts

- A `.knv` file other than `project.knv` counts in the folder where a walk lists it.
- `tests/`, `tasks/`, `examples/`, the project's top folder (`./`) and every other folder count.
- Hidden folders, the build folder, nested projects and dependencies do not count.
- A folder link that a walk already walked counts once.
- The entry and a library's root count as any module.
- The rule runs beside a failing program, because the counts need only the walks. It skips when a walk fails.

### Cost

These are estimates. With the rule off, nothing is built: under 0.01 M instructions at 100 and at 1,000 modules. With it on, about 0.1 M at 100 modules and 0.3 M to 0.6 M at 1,000, under 0.01 % of `knave lint`. The saved defaults add a few bytes to the vocabulary that every command already decodes.

## Recommendation

- **The rule:** a project entry point in `prelude/lint` (A).
- **The location:** a folder as a kind of source (A). It is one rule for every diagnostic. Only the two writers that read a source's text learn the kind, and every other consumer keeps one path.
- **The defaults:** `lint-default` entries saved into the prelude state (A), holding both the default level and the default threshold.
- **`L0738`:** reported by the project reader in every command, now that every command knows which rules take a threshold.
- **Item 66 lands in its own pull request, merged before `folder-modules`.**
  - Its one commit adds `lint-default`, saves the table and deletes `LintDefaults::ALLOWED_UNLESS_WRITTEN`, so no default is stated in Rust and the prelude at once.
  - It changes no output, which the existing `needless-type` tests show.
  - Its review stays one logical change, apart from a new rule.

## Decision

The project owner decided the rule's behavior before this design pilot, and every other ruling below on 2026-11-24.

- **Before the pilot: the rule's behavior,** as the Problem lists: `folder-modules` warns above N `.knv` files directly in one folder, N is 15 by default and set as `(folder-modules warn <n>)`, it is off by default, and folders under `tests/` and `tasks/` count. Once on, a library project warns too, and the explanation says that moving a module changes every `use` of it.
- **2026-11-24: a threshold on a rule left at `allow`** is `L0738`, with a suggestion that adds `warn`.
- **2026-11-24: the warning points at the folder itself,** not at a file. A folder is a kind of source, and every diagnostic about a folder follows the same rule.
- **2026-11-24: the help is one fixed line,** ``split `<folder>` into subfolders, one for each feature``. The rule suggests no groups, and Rust hands it no use graph.
- **2026-11-24: no lint states its default level in Rust.** Default levels move into `prelude/lint` (roadmap item 66), before or with this rule.
- **2026-11-24: a rule's default threshold sits in its `lint-default` entry,** beside its default level, as `(lint-default folder-modules allow 15)`.
- **2026-11-24: the project reader reports `L0738` in every command,** for a threshold on a rule that takes none and for a threshold on a rule left at `allow`.

`rslisp-lead` confirmed the codes `L0812` and `L0738` and the number 0084. It also bounded item 66 to land before `folder-modules` or in its first pull request, never after, so no rule's default level is stated in Rust and in the prelude at once.

**Item 66 lands in its own pull request, merged before `folder-modules`.** This RFC covers item 66 too, so it merges as that pull request's first commit.

**This RFC revises RFC 0073.** RFC 0073's Decision says: "A threshold is rejected until the first size rule lands. After that, `knave lint` reports a threshold its rule never reads." The rule is now: every command reads a threshold, the project reader reports `L0733` for a malformed one, and it reports `L0738` in every command for a threshold that its rule never reads. RFC 0073's status line gains: "Superseded by [RFC 0084](0084-folder-modules-and-lint-defaults.md) for its rule that `knave lint` alone reports a threshold its rule never reads." Its reasoning stays as written.

## Rejected options and why

- **The whole rule in `src/cli/`:** the prelude can express the count's comparison and the wording.
- **A location type beside the label:** every consumer of a diagnostic's location would branch on it, and `PathScope` would need a second rule.
- **Labeling a file of the folder:** the owner ruled that the warning points at the folder.
- **Suggested groups (prefix groups, use clusters):** the owner ruled for one fixed help line.
- **Default levels that only `knave lint` reads:** `allow` and `deny` apply in every command.
- **Default levels in Rust:** the owner ruled them into the prelude.
- **Item 66 after `folder-modules`:** `folder-modules`' default would sit in Rust for a time.
- **Item 66 as the first commit of the `folder-modules` pull request:** it meets the bound, but mixes a refactor that changes no output with a new rule in one review and one mutant run.
- **A label message on a folder:** it would never print.
- **Default thresholds in a separate `rule-thresholds` table:** each size rule would state its defaults in two forms, and only `knave lint` could tell which rules take a number.
- **`L0738` from `knave lint` only:** once every command knows which rules take a threshold, `check`, `build` and `run` would otherwise accept a number that does nothing.

## Rule

- [docs/language.md: Lints](../docs/language.md#lints) and the `lints` section under [Projects](../docs/language.md#projects): the `L0812` row, `lint-default`, the threshold shape, and `L0733` and `L0738`.
- [RFC 0073](0073-lint-exit-and-lints-section.md): its status line, as the Decision states.
- [docs/diagnostics.md: Layout](../docs/diagnostics.md#layout) gains: "A diagnostic about a folder labels the folder, not a file. Its location is the folder's path with a trailing `/`, as `--> src/`, with no line, column, excerpt or label text, and `knave lint <path>` prints it when the folder is the path or lies under it."
