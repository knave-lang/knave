# Knave

[![CI](https://github.com/knave-lang/knave/actions/workflows/ci.yml/badge.svg)](https://github.com/knave-lang/knave/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/knave-lang/knave)](https://github.com/knave-lang/knave/releases/latest)
![line coverage](https://img.shields.io/badge/line%20coverage-99%25-brightgreen)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Nix flake](https://img.shields.io/badge/nix-flake-5277C3?logo=nixos&logoColor=white)](#develop)
[![Rust 2024](https://img.shields.io/badge/rust-2024-B7410E?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/rust-2024/)

> [!WARNING]
> **Knave is experimental.** Expect breaking changes to the syntax, the prelude and the command line in any commit.

```clojure
(def knave
  "A small Lisp for humans and agents
  that infers types and ownership
  and compiles to readable Rust.")

(def values
  (vec "Few rules, no exceptions."
       "Small compiler, big prelude."
       "No clones or allocations you did not write."
       "Interpret while you iterate, compile when you ship."
       "Errors worth reading."))
```

[knave-lang.org](https://knave-lang.org) is the Knave website.

[docs/language.md](docs/language.md) describes the language.

## Install

Install the compiler with [Cargo](https://www.rust-lang.org/tools/install):

```sh
cargo install --locked --profile dist knave
```

The compiler runs the `rustc` and `rustfmt` on your `PATH`, and needs Rust 1.88 or later.

If you use [Nix](https://nixos.org/download/), install the compiler with the `rustc` and `rustfmt` that CI tests against:

```sh
nix profile install github:knave-lang/knave
```

To download a prebuilt binary instead of building it, run [`cachix use rslisp`](https://docs.cachix.org/installation) first.

## Use

A program starts at `main`, as `examples/hello.knv` shows:

```clojure
(defn main ()
  (println (str "hello" " " "world")))
```

Run `knave` alone to start the prompt, as `knave repl` does. Run `knave --help` to list the commands. Each command except `doc` and `repl` takes a source file, and `fmt` and `test` also take a directory:

```sh
knave run examples/hello.knv
knave emit examples/geometry/main.knv
knave fmt src/
knave test tests/
```

A project is a directory with a `project.knv` file, and `knave new <name>` creates one. Inside one, `build`, `run`, `check`, `types`, `expand` and `emit` take the project's entry file when given no file. Given no path, `fmt` lays out the whole project and `test` runs the project's `tests/` directory.

## Develop

The repository uses a [Nix](https://nixos.org/download/) flake.

1. Enter the development shell:

   ```sh
   nix develop
   ```

   If you use [direnv](https://direnv.net/), run `direnv allow` once instead. The shell then loads when you enter the directory.

2. Run the tests with the coverage, CRAP and emitted-Rust lint checks:

   ```sh
   cargo xtask coverage
   ```

   `cargo test --workspace` runs the tests alone.

3. Build the compiler:

   ```sh
   nix build
   ```

   The binary is `result/bin/knave`. To install a checkout without Nix, run `cargo install --locked --profile dist --path .`.

Run `nix fmt` to format the Nix files, and `nix flake check` to build the package and run the checks that `cargo xtask coverage` leaves out.

## Contribute

Read [AGENTS.md](AGENTS.md) before you change the code. It holds the rules for this repository and lists the commands a change must pass. Follow [docs/diagnostics.md](docs/diagnostics.md) when you add or change a compiler error.
