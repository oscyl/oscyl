# Run the checks

This guide runs the checks that CI runs, on your own machine, before you
send a change. [Checks and workflows](../reference/ci.md) says what each
check proves.

## Check a change

Run the script from the repository's root:

```sh
scripts/ci.sh
```

It runs only the stages that your change reaches, since its last green run
or since your branch left `main`. It stops at the first failure. A stage
whose tool is not installed says so and is skipped.

- To run every stage, for a release: `scripts/ci.sh --all`.
- To skip the mobile app on purpose: `env -u ANDROID_HOME scripts/ci.sh`.

[Build and run](build.md#the-optional-tools-of-the-checks) lists the
optional tools.

## Check one part while you work

Run the smallest check that can fail for your change, for example:

- one crate: `cargo test -p oscyl-app` and `cargo clippy -p oscyl-app`;
- one driver's conformance: `cargo test -p oscyl-sequential-prophet-rev2 --test conformance`;
- one mobile test class: see [Fast loops](mobile.md#fast-loops);
- the book: `mdbook build docs`, then open `target/book/index.html`.

## Run the hardware tests

Tests that need a synth are marked `#[ignore]`. Connect the synth, then run
them:

```sh
cargo test -- --ignored
```

## Fuzz a parser

The parsers that read synth replies and files have fuzz targets, in
`fuzz/`. They use [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
with nightly Rust:

```sh
cd fuzz && cargo +nightly fuzz run sysex    # or: program, files
```

A new parser gets a fuzz target, or a line in one.

## After you change a dependency

The About pages of both apps list the licences of the Rust crates. Write
them again:

```sh
scripts/licences.sh
```

It uses cargo-about. `scripts/ci.sh` runs `scripts/licences.sh --check`.
The Kotlin and Android libraries' notices come from the AboutLibraries
Gradle plugin, on every build.

## Check the book's prose and links

`scripts/ci.sh` runs three stages for the documentation: "book", "prose"
and "links". To run them alone:

```sh
mdbook build docs
git ls-files -z '*.md' | xargs -0 vale --minAlertLevel=warning
git ls-files -z '*.md' | xargs -0 lychee --no-progress --config lychee.toml
lychee --no-progress --config lychee.toml target/book
```

Vale applies the house style of `.vale.ini`. A page must have no errors and
no warnings. lychee checks every link and anchor offline. `scripts/ci.sh`
also checks the links to this repository on GitHub, against your work tree.

When you write a link:

- Link a page of the book with a relative `.md` link.
- Link a folder's `README.md` by its folder, for example `../adr/`. The
  book renames that file to `index.html`.
- Link a file outside `docs/` with its full GitHub address, because the
  book does not copy it.
