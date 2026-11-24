# RFC 0086: Knave owns a setup action

**Status:** Accepted
**Date:** 2026-11-24

## Problem

`.github/workflows/knave.yml` holds the step that reads `:knave` from `project.knv`, and the steps that restore the cache, run `cargo install knave` and save the cache. [RFC 0082](0082-knave-owns-its-files.md) makes Knave own that workflow and sends a project's own steps to a workflow of its own. That workflow needs Knave too, but it cannot reuse the steps of `knave.yml`: it must copy the read step and the install, and the copy falls behind each time Knave changes them.

## Options

### The read step in `knave.yml`

Keep every step in `knave.yml`. A project's own workflow copies the read step, the cache steps and the install.

```yaml
steps:
  - uses: actions/checkout@v4
  - name: Read the Knave version from project.knv
    run: |
      uncommented=$(awk '...' project.knv || true)
      ...
```

### A `.sh` file beside `knave.yml`

Knave owns a script that reads the version. Each workflow runs the script, then copies the cache steps and the install.

```yaml
steps:
  - uses: actions/checkout@v4
  - run: bash .github/workflows/read-version.sh
  - id: knave-cache
    uses: actions/cache/restore@v4
    ...
```

### A Knave-owned local composite action

Knave owns `.github/actions/setup-knave/`: `action.yml` reads the version with `read-version.sh`, restores the cache, runs `cargo install` and saves the cache. Each workflow installs Knave in one step.

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: ./.github/actions/setup-knave
  - run: knave task release
```

### A `knave version --project` command

The compiler prints the version that `project.knv` records. A workflow needs a compiler to run it before it has installed one, so it still needs a way to read the file without Knave.

```sh
knave version --project
```

### Prebuilt release binaries

Each release uploads binaries, and a workflow downloads the one for its runner. It removes the `cargo install` but not the read step, and it adds a release job and a binary per platform.

```yaml
steps:
  - uses: actions/checkout@v4
  - run: curl --fail --location "$KNAVE_RELEASE_URL" | tar xz
```

## Recommendation

The local composite action. One step installs the version that `project.knv` records, in `knave.yml` and in every workflow of the project's own. Knave owns the action, so `knave upgrade` keeps it current. The read step stays the same `awk` and `grep`, moved to a real `.sh` file.

## Decision

The project owner, 2026-11-24, D139: "I would keep the awk, maybe we extract this to a .sh file that other actions can use?" The project owner then chose the layout: `.github/actions/setup-knave/` holds `action.yml`, which reads the version, restores the cache, runs `cargo install` and saves the cache, and `read-version.sh`, which holds the `awk` as a real `.sh` file. `knave.yml` runs `uses: ./.github/actions/setup-knave`. A project's own workflow runs `actions/checkout@v4`, `uses: ./.github/actions/setup-knave` and its own steps.

Knave owns both files of the action as it owns `knave.yml`: the GitHub component writes them, and `knave upgrade` rewrites them with the current text.

## Rejected options and why

- **The read step in `knave.yml`:** a project's own workflow must copy the step, and the copy falls behind.
- **A `.sh` file beside `knave.yml`:** a workflow still copies the cache steps and the install.
- **A `knave version --project` command:** a workflow cannot run Knave before it installs Knave.
- **Prebuilt release binaries:** a workflow still reads the version, and each release must build and upload a binary for each platform.

## Rule

[docs/language.md: Projects](../docs/language.md#projects)
