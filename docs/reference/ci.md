# Checks and workflows

What `scripts/ci.sh` checks on your machine, and what each GitHub workflow
checks. [Run the checks](../how-to/run-the-checks.md) says how to run them.

## The stages of scripts/ci.sh

The script runs these stages in order and stops at the first failure. A
stage runs only when a file that it checks changed since the last green
run. The last green run is the tree in `target/ci-green-tree`; without it,
the script compares with the branch's merge base with `main`. `--all` runs
every stage.

| Stage | What it checks | When it runs |
|---|---|---|
| workflows | actionlint (with shellcheck) and zizmor, pedantic, over `.github/` | `.github/` changed |
| issue forms | Every field of every issue form is valid (Ruby's YAML) | An issue form changed |
| spelling | typos, over the changed files | Always |
| release scripts | The release notes test, every changelog section and its compare link, and the version parser's test | `CHANGELOG.md` or a release script changed |
| book | `mdbook build docs` | A Markdown file, the style rules or the tutorial's driver changed |
| prose | Vale, over every Markdown file, with warnings shown | As "book" |
| links | lychee, over every Markdown file and over the built book | As "book" |
| fmt | `cargo fmt --all --check` | A Rust crate changed |
| clippy | Clippy, with warnings as errors, over the crates that the change reaches | As "fmt" |
| tests | The tests of those crates | As "fmt" |
| without midir | Clippy and the tests of `oscyl-midi` and `oscyl-app` without their feature `midir`, as the mobile apps build them | The change reaches `oscyl-midi` or `oscyl-app` |
| conformance | Each driver's `tests/conformance.rs`. A driver crate without one fails | The change reaches that driver |
| docs | `cargo doc`, with warnings as errors | As "fmt" |
| ffi bindings | `scripts/mobile-rust.sh`: the Kotlin bindings, and the Android libraries when cargo-ndk and an NDK exist | The change reaches `oscyl-ffi` |
| ios | Clippy of `oscyl-ffi` for both iOS targets, when they are installed | As "ffi bindings" |
| (Xcode files) | `scripts/mobile-assets.sh --check` | The mobile app, the design tokens, the icons, the fonts or a licence text changed |
| licences | `scripts/licences.sh --check`: the About pages list the crates of `Cargo.lock` | `Cargo.lock` or the licence script changed |
| mobile app | The common Kotlin, `:shared:check`, the behaviour tests, the screenshot goldens, the debug build and the iOS platform klibs | The mobile app or the design tokens changed, or the change reaches `oscyl-ffi`. Needs `ANDROID_HOME` |
| accessibility | The accessibility checks, on a running emulator or on one that Gradle manages | A UI source of the mobile app changed. Needs `/dev/kvm` |
| cargo deny | Licences, advisories, bans and sources of the dependencies | A manifest, `Cargo.lock` or `deny.toml` changed |

The screenshot goldens run only when a file under `apps/mobile`, `design`
or `po` changed. For a change to the Rust core alone, the mobile stage runs
the behaviour tests without them.

## The GitHub workflows

| Workflow | Jobs and what they check | When it runs |
|---|---|---|
| CI (`ci.yml`) | `rust`: fmt, clippy and the tests of the workspace, and the build without `midir`. `desktop`: the tests and the command line on macOS and Windows. `conformance`: each driver's conformance test. `msrv`: the build with Rust 1.92. `deny`: cargo-deny. `docs`: rustdoc with warnings as errors. `book`: the book, its prose and its links. `licences`, `typos`. `flatpak-sources` and `gtk`: the desktop app, built and tested in the GNOME runtime. | Every push to `main` and every pull request |
| Android (`android.yml`) | `android`: `:shared:check`, the behaviour tests, the screenshot goldens and the debug build. `accessibility`: the accessibility checks on an emulator | A change to the mobile app, the crates, the manifests or the design |
| iOS (`ios.yml`) | The iOS tests, then the iOS app: built on macOS and started in a simulator, with a screenshot | A change to the mobile app, the crates, the manifests or the design tokens |
| Desktop apps (`desktop-apps.yml`) | The desktop app built and bundled for Windows and macOS, as a release does, then opened once with a simulated synth for a screenshot | A change to the desktop app, the crates, the design, the translations or the bundle scripts |
| Fuzz (`fuzz.yml`) | A short run of every fuzz target | A change to the crates or to `fuzz/`, and once a week |
| CodeQL (`codeql.yml`) | Code scanning | Every push to `main`, every pull request, and once a week |
| Dependency review (`dependency-review.yml`) | The dependencies that a pull request adds | Every pull request |
| Changelog (`changelog.yml`) | A pull request that changes what users get also changes `CHANGELOG.md`, or has the label `no-changelog`. Docs, workflows, tests and Markdown are exempt. | Every pull request |
| Lint workflows (`lint-workflows.yml`) | actionlint and zizmor | A change to `.github/` |
| Scorecard (`scorecard.yml`) | The OpenSSF Scorecard | Every push to `main`, and once a week |
| Site (`pages.yml`) | Publishes the website, the book and the API reference to GitHub Pages | A change to the site, the book or the crates of the API reference |
| Release (`release.yml`) | Builds, checks and attests every artifact; opens a draft release. See [Release artifacts](release-artifacts.md). | A pushed tag `vX.Y.Z`, or by hand |
