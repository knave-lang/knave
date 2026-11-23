# RFC 0082: Knave owns its files

**Status:** Accepted
**Date:** 2026-11-22

## Problem

[RFC 0079](0079-project-knave-version.md) let `knave upgrade` rewrite a file that Knave wrote only when the file held a text that some release wrote. It kept every other file with a warning and two diffs, and it never created a missing file. That needed a saved copy of every released text and a prepush check that kept the copies complete. A user who added one step to the workflow then kept an outdated workflow for good, and a user who lost one copy of the skill never got it back.

`knave new` now also writes a line into `.gitattributes`, a file that the user shares with other tools, so the command needs a rule for a single owned line in a file it does not own.

## Options

### Keep and merge

Keep the history of released texts. Rewrite a file that holds a released text, keep a changed file with its diffs, and leave the merge to the user.

```sh
knave upgrade
# warning: kept `.github/workflows/knave.yml`, because it differs from every text Knave wrote
```

### Knave owns its files

Rewrite every file that Knave owns with the current text, and refuse while one of them has uncommitted changes, as `cargo fix` does. Git holds the user's committed edits, so a rewrite loses nothing that git cannot show. The user's own steps go in a file of their own.

```sh
knave upgrade
# error: `knave upgrade` would overwrite uncommitted changes to these files that Knave owns:
#   .github/workflows/knave.yml
```

## Recommendation

Knave owns its files. The rule is short, needs no history of texts, keeps every project on the current workflow and skill, and git already shows each change that a rewrite makes.

## Decision

- **D134.** `knave upgrade` writes a missing component file only when the project shows that component, and it never deletes a file or a line.
- **D135.** Knave owns its files: the workflow, both copies of the agent skill, and its one line in `.gitattributes`. `knave upgrade` always rewrites them with the current text, with no keep, no merge and no prompt. A user's additions go in their own files, such as their own workflow. It refuses when an owned file has uncommitted changes, as `cargo fix` does. A committed edit is overwritten, and git shows it. This replaces D127's rule that keeps a changed file with a warning and its diffs.
- D126 stands: `knave upgrade` records the running compiler in `:knave`, and a compiler older than `:knave` refuses with `L0736`.

The project owner ruled on the details:

- A component shows when one of its owned files exists. `.gitattributes` never counts, because a project can hold it without Knave. The GitHub component shows when `.github/workflows/knave.yml` exists. The agent skill shows when either copy of its `SKILL.md` exists.
- The GitHub component owns each line of `.gitattributes` whose pattern is `*.knv` and that sets an attribute starting with `linguist-`. Every other line stays byte for byte.
- Under a shown GitHub component, a missing `.gitattributes` is written, holding the line. A file with no owned line gets the line appended, after a newline when the file does not end with one. Each owned line that differs from the current line is replaced in place.
- The refusal covers the workflow and the two skill copies, each that exists. `project.knv` and `.gitattributes` never trigger it, because `knave upgrade` changes only its own field or line in them.
- An untracked owned file counts as uncommitted. A file that git ignores gets no check. Outside a git work tree there is no check.
- On a refusal, `knave upgrade` changes no file, `project.knv` included, names each file with uncommitted changes, and exits 1.
- The saved copies of old texts go, with the prepush check that kept them complete.
- Git failures: `knave upgrade` runs `git status` with `LC_ALL=C`, once per repository that holds an owned file. When `git` cannot start, it warns that it could not check for uncommitted changes, and goes on. When git exits nonzero and its error says `not a git repository`, there is no check, as outside a work tree. On every other nonzero exit, it refuses: it prints git's error, changes no file, `project.knv` included, and exits 1.
- Unreadable files: an owned whole file is compared as bytes after each CRLF becomes LF, so a file that is not UTF-8 differs and is rewritten. A file that cannot be read is reported with its name, as a failed write is, and the command exits nonzero. A `.gitattributes` that is not UTF-8 is reported the same way and stays byte for byte.
- Line endings: an appended linguist line takes the line ending of the file's first line, so a CRLF file stays CRLF.
- B1: `git status` lists every untracked file whatever the user's git config says. `knave upgrade` passes `--untracked-files=all` and `--ignore-submodules=none`. A file that git ignores still gets no check.
- M1: a file that differs from the current text only in its line endings is current and is not rewritten. A file that differs in content is rewritten with the current text as Knave holds it.
- M2: an owned file belongs to the nearest folder at or below the project root that holds a `.git` entry, a file or a folder, searched from the owned file's folder up to the project root. When no folder below the root holds one, the file belongs to the repository that holds the project root. `knave upgrade` runs one `git status` per repository that holds an owned file, from that repository's folder, with paths relative to it. A failure of any of them applies the git failure rulings.
- M3: `knave upgrade` reads every owned file and the `.gitattributes` it would change before it writes any file, `project.knv` included. If one cannot be read, it reports each such file by name, changes no file, and exits 2.
- M5: a UTF-8 byte order mark at the start of `.gitattributes` is not part of the first line's pattern. An owned first line after the mark is replaced in place, and the mark stays.

## Rejected options and why

- **Keep and merge:** it needs a saved copy of every released text and a check that keeps the copies complete. A project with one added step never gets the current workflow, and a user must merge Knave's changes by hand after every release.

## Rule

[docs/language.md: Projects](../docs/language.md#projects)
