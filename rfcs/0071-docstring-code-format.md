# RFC 0071: Formatting the code in docstrings

**Status:** Accepted
**Date:** 2026-11-11

## Problem

`knave fmt` owns every line break of a file's code, but it treats a docstring as one string atom and keeps its text. The examples inside docstrings are therefore laid out by hand. In the prelude, a long example often runs on one line:

```lisp
(document (into-hash-map pairs)
  "Builds a [hash-map] from a vector of pairs.

  ...

  (local prices (into-hash-map (vec (pair \"pear\" 3) (pair \"pear\" 5))))
  (get prices \"pear\")
  => (some (ref 5))")
```

knave-lang.org publishes these docstrings through `knave doc --write --prelude`, and its docs column wraps such a line in the middle of a form. `knave fmt -` already lays the same code out well:

```lisp
(local prices
  (into-hash-map (vec (pair "pear" 3) (pair "pear" 5))))
```

But nothing runs it over docstrings, so the layout of documentation code depends on its author. Passing a whole example paragraph to `knave fmt -` also does the wrong thing: it reads `=> (some (ref 5))` as two forms, puts `=>` on a line of its own, and puts a blank line between top-level forms, which splits the paragraph so that it no longer runs as a test.

The doc writer has the same problem. Each term's page opens with a fenced block that `prelude/documentation.knv` builds from `doc-header` and the type line. The type line of a generic function is often longer than the docs column:

```knave
(none? option)
(type none? (where (a) (fn ((ref (option a))) bool)))
```

knave-lang.org lays these blocks out in its own build today, and will stop once the doc writer does.

## What counts as docstring code

The rules below reuse the two forms of documentation code that the language already recognizes, so a doc writer learns no new syntax:

- **An example paragraph**, as `knave test` reads it and `prelude/documentation.knv` fences it: a paragraph outside a fence that holds a line starting with `=> `, after its indentation, and holds no line starting with `;`. The docstring's first paragraph, which holds its summary line, is always prose.
- **A code fence** whose info string is empty or `knave`. A fence follows the rule that `knave test` already uses: three or more backticks or tildes, closed only by a line of the same character, at least as long, with nothing else on it. Since PR 490, `knave doc --write` tags an untagged fence as `knave` on the page, so an untagged fence already means Knave code.

Everything else stays exactly as written: prose, a fence that names another language such as `rust` or `sh`, an unclosed fence, and a paragraph that holds no arrow.

A docstring is a string in the docstring position of a form whose header shape carries `:docstring`, when another form follows it, as today. This misses the most common documenting form: `(document header text)` ends with its docstring, so 203 of the prelude's 229 documented terms with examples are out of reach. This RFC adds a header shape marker, `:docstring-only`, which states that the form's last form, right after the header, is its docstring. `prelude/macros` states `(header-shape (document header) :docstring-only)`. The formatter cannot tell `(document twice "Doubles.")` from `(def greeting "hi")` without the marker, because it does not expand, and a `def` whose last form is a string binds that string.

## Options

### On by default, the same width as code (recommended)

`knave fmt` lays out docstring code in every file, with no setting, as it lays out every other form. The code lays out as if the docstring's left edge sat at column 2, where a top-level definition's docstring sits, so a laid-out line fits the 80-column width in the file and is at most 78 columns on the page.

```lisp
(document (into-hash-map pairs)
  "Builds a [hash-map] from a vector of pairs.

  ...

  (local prices
    (into-hash-map (vec (pair \"pear\" 3) (pair \"pear\" 5))))
  (get prices \"pear\")
  => (some (ref 5))")
```

### On by default, with a narrower width for examples

The same, but docstring code gets its own width, such as the 60-column fill width, to suit a narrow docs column.

### Opt in

A project setting turns docstring code formatting on, as in Ruff and rustfmt.

### Leave docstrings alone

Authors keep laying out examples by hand, and knave-lang.org keeps reformatting the header blocks itself.

## How the rules apply

1. **Each form of the code lays out as `knave fmt` lays out a top-level form**, with the shapes of the file, at the column that rule 2 gives. In an example paragraph, the forms stay tight: the formatter adds no blank line, because a blank line would split the paragraph, and `knave test` and the doc writer would then read it as prose. In a fence, top-level forms get a blank line between them, as in a file, and consecutive one-line `use` or `export` forms stay together. A result line follows its code directly in both, and in a fence a blank line separates it from the next form.
2. **The code keeps its written indentation inside the docstring**, its least indented line staying where it was, and lays out as if the docstring's left edge sat at column 2. A docstring nested deeper than column 2 may run past 80 columns by its extra depth, as a long atom may today.
3. **A result line stays as written, directly under the code it belongs to.** A line that starts with `=> ` after its indentation, and each later line indented deeper than the code, is the result. The formatter keeps its text byte for byte and removes any blank line between the code and its arrow. It never reads the result as code, so `=> (some (ref 5))` stays on one line and a result such as `=> into-sorted-map ...` keeps its `...`.
4. **Code that does not read stays exactly as written, and `fmt` warns.** An unbalanced list or string, or another language in an untagged fence, keeps its region of the docstring byte for byte. `knave fmt` and `knave fmt --check` print `warning[L0109]: this docstring code does not read, so `fmt` leaves it as written`, labeled at the first line of the code that does not read, with a help to close each list and string or to name the fence's language. The warning never changes the exit code. `fmt` never fails because of a docstring's prose or examples.
5. **A docstring that writes an escape other than `\"` and `\\` stays as written.** The formatter writes the laid-out code back with those two escapes, so it changes no other escape a docstring writes, such as `\t`.
6. **`knave fmt --check` reports a file whose docstring code is not laid out**, by listing its path and exiting 1, as for any other layout change. The `source-format` flake check then holds the rule in CI for the prelude, the examples and the fixtures.
7. **Before it writes a file, the formatter checks that each docstring keeps its meaning**: the same prose lines after removing the shared indentation, the same code regions, and the same tokens in each region. A failed check is the internal error it is today.
8. **`knave doc --write` lays out each `knave` fence of a page by rules 2 to 5**, including the header block, so the type line above becomes:

   ```knave
   (none? option)
   (type none?
     (where (a) (fn ((ref (option a))) bool)))
   ```

   A page's fence keeps its forms' written separation and adds no blank line: the header block keeps the usage line and the type line together, an example paragraph that the doc writer fenced stays tight, and a fence from a docstring keeps the blank lines that rule 1 gave it in the source.

## Precedents

- **Ruff** formats Python code examples in docstrings with `docstring-code-format`. Its formatter documentation calls it "an opt-in feature" and lists the formats it recognizes: doctest, Markdown fences with no info string or a Python one, and reStructuredText literal blocks and code directives. It skips an example "if the code does not parse as valid Python or if the reformatted code would produce an invalid Python program". Its `docstring-code-line-length` defaults to `dynamic`, which "instructs the formatter to respect the line length limit setting for the surrounding Python code". ([Ruff: Docstring formatting](https://docs.astral.sh/ruff/formatter/#docstring-formatting), [source](https://raw.githubusercontent.com/astral-sh/ruff/main/docs/formatter.md))
- **rustfmt** has `format_code_in_doc_comments`, "Format code snippet included in doc comments", default `false`, unstable, tracked in rust-lang/rustfmt#3348. A second unstable option, `doc_comment_code_block_width`, default `100`, sets the "Max width for code snippets included in doc comments", tracked in rust-lang/rustfmt#5359. ([rustfmt Configurations.md](https://raw.githubusercontent.com/rust-lang/rustfmt/master/Configurations.md))
- **rust-lang/rust#105198**, "Discussion: Enforce formatting of doc comments", weighs turning that option on for the Rust repository. Its main drawback is that doc examples cannot get rules apart from code: the author notes that matching the repository's settings "would enable `use_small_heuristics = "Max"`, which is very undesirable" for examples, and asks for `doc_comment_*` options. ([rust-lang/rust#105198](https://github.com/rust-lang/rust/issues/105198))

Both tools ship the feature off, because they added it to formatters that many projects already ran, and turning it on would have rewritten their docstrings in a minor release. Knave's formatter has no settings and few users, and the owner decided that docstring code is formatted for everyone. Ruff's default width, `dynamic`, is the one this RFC recommends. The drawback in #105198 is real but smaller here: Knave's formatter keeps short lists on one line by their own length, at most 40 or 60 columns, so only a long call reaches the 80-column width at all.

## Recommendation

**On by default, the same width as code.** It is the only option with no setting and no second number, which is what Knave's formatter has so far: one layout for every file, so a reader never asks which settings a file was formatted with. The doc writer and `knave fmt` then agree on every code block, and knave-lang.org can drop its own layout pass.

A width of its own for examples is the one choice to revisit. If the docs column of knave-lang.org shows fewer than 78 columns of code without wrapping, the narrower width is worth its second number, and 60, the fill width that the formatter already names, is the natural value.

### Prototype

Local branch `fmt-docstring-code-prototype` holds two commits. `f4f05a220` implements rules 1 to 7 and the `:docstring-only` marker in `expand/src/format/`, one concept per file (`DocstringPass`, `DocstringCode`, `DocstringRegions`, `MarkdownFence`, `CodeRegion`, `CodeChunk`, `DocstringLines`, `FormatProblems`), with unit tests, the format fixture `docstring_code.knv` and two `tests/cli.rs` tests: `fmt --check` lists a file whose docstring code is not laid out, and warns of code that does not read with exit 0. `cargo xtask design-rules` and `cargo xtask file-sizes` pass with no baseline change, and clippy passes on both crates. `eff90373b` is the prelude reformat, made only by running `knave fmt prelude/`, so the build can rebase it last over other docstring changes. `knave test prelude/tests` passes on the reformatted prelude, so every example still runs. Rule 8 is not in the prototype. The reformat gives:

| Measure | Result |
| --- | ---: |
| Files changed | 16 of the prelude's `.knv` files |
| Docstrings changed | 54: 46 `document` texts and 8 `defmacro` docstrings |
| Lines | +171, −78 |
| `L0109` warnings | 0 |
| Docstrings with code left as written because of another escape (rule 5) | 0 |

The change is a layout change only: each changed docstring keeps its prose lines and its code tokens, as rule 7 checks. Two kinds of change make most of the diff. A long example breaks, as `into-hash-map` above does. And an example that fits on one line but is longer than 40 columns and holds more than two lists breaks too, because rule 1 applies the formatter's short-list rule to it:

```lisp
(assoc-in (vec (vec 1 2) (vec 3))
          (1 0)
          7)
=> (vec (vec 1 2) (vec 7))
```

The second kind makes examples taller, not narrower. It is what `knave fmt` does to the same call in code, so the RFC keeps it rather than adding a rule for examples.

### Cost

- **A new header shape marker.** `:docstring-only` is a fifth keyword that the formatter names. `docs/language.md` states that the formatter names only its markers, so the list grows by one, and `L0331` lists it.
- **A new warning code.** `L0109` sits with the reader's codes, because the fault is code that does not read.
- **The prelude reformat lands as its own commit**, made only by running `knave fmt prelude/`, so it can be rebased last over other sessions' docstring changes. It moves prelude lines that about six snapshots name, five through a `prelude/<file>.knv:<line>` location and one doc page, so that commit also updates those snapshots and nothing else.
- **Fixtures with docstring examples move.** The `source-format` check covers `tests/fixtures/`, so their docstrings are laid out too, and a snapshot that names one of their lines moves.

## Questions for the owner

- **Q-a. On by default?** Recommended: yes, with no setting.
- **Q-b. A width of its own for examples?** Recommended: no, the same 80 columns as code, measured from column 2. The alternative is 60.
- **Q-c. The marker for `document`.** Recommended: `:docstring-only`. Without it, `document` docstrings, most of the prelude's examples, stay out of reach.
- **Q-d. The header block of a doc page.** Recommended: lay it out with no blank line between the usage line and the type line, as rule 1 gives. `knave fmt -` puts a blank line there, because a file separates top-level forms.
- **Q-e. Module docstrings.** `(module text)` has no header shape, so a module docstring stays as written in the prototype. Stating `(header-shape (module) :docstring-only)` covers it by the same marker. Recommended: state it in the build, after checking that it moves no module form of the prelude.

## Decision

The project owner decided on 2026-11-11 (D107):

- **Q-a.** Docstring code formatting is on by default, with no setting.
- **Blank lines.** An example paragraph stays tight, with no blank line inside it, because a blank line would split it. Inside a fence, top-level forms get a blank line between them, as the formatter gives in a file. Rule 1 states this.
- **Q-b.** Docstring code takes the same 80-column width as code, counted from column 2. The docs column of knave-lang.org is narrower than 78 columns, so a narrower example width may come later if the site needs it.
- **Q-c.** The header shape marker `:docstring-only`, stated for `document`.
- **Q-d.** The header block of a doc page keeps the usage line and the type line together, with no blank line.
- **Q-e.** `(header-shape (module) :docstring-only)` covers module docstrings.

## Rejected options and why

- **On by default, with a narrower width for examples:** a second width is a second number for every reader to learn, and only a long call reaches the 80-column width. It stays open if knave-lang.org's narrower docs column needs it.
- **Opt in:** Knave's formatter has no settings, and the owner decided that docstring code is formatted for everyone.
- **Leave docstrings alone:** the layout of documentation code would keep depending on its author, and knave-lang.org would keep its own layout pass.
- **One spacing rule for paragraphs and fences:** adding no blank line in a fence would lay out a fence unlike the file it shows, and adding blank lines in a paragraph would split it.

## Rule

[docs/language.md: Source formatting](../docs/language.md#source-formatting) and [Documentation pages](../docs/language.md#documentation-pages)
