#!/usr/bin/env bash
# Run the checks of the GitHub workflows locally (.github/workflows/), as far
# as this machine has the tools. Stops at the first failure.
set -euo pipefail

cd "$(dirname "$0")/.."

stage() { printf '\n==> %s\n' "$*"; }

# Change-aware: a stage runs only when a file it checks changed since the last
# green run (target/ci-green-tree, the work tree's git tree hash), or since the
# merge base with main when there is no green run yet. --all runs everything.
all=false
[ "${1:-}" = "--all" ] && all=true
mark=target/ci-green-tree
index=$(mktemp)
trap 'rm -f "$index"' EXIT
GIT_INDEX_FILE=$index git read-tree HEAD
GIT_INDEX_FILE=$index git add -A
tree=$(GIT_INDEX_FILE=$index git write-tree)
base=""
if [ -s "$mark" ] && git cat-file -e "$(cat "$mark")" 2>/dev/null; then
    base=$(cat "$mark")
elif merge_base=$(git merge-base HEAD main 2>/dev/null); then
    base=$merge_base
else
    all=true
fi
changed=""
if ! $all; then
    if [ "$(git rev-parse "$base^{tree}")" = "$tree" ]; then
        echo "Nothing changed since the last green run (scripts/ci.sh --all to run everything)."
        exit 0
    fi
    changed=$(git diff --name-only "$base" "$tree")
    printf 'Checking what changed since %s (%s files); --all for everything.\n' \
        "$(git rev-parse --short "$base")" "$(printf '%s\n' "$changed" | grep -c .)"
fi
# touched REGEX: true when a changed path matches (always true with --all).
touched() { $all || printf '%s\n' "$changed" | grep -qE "$1"; }
skip() { echo "unchanged, skipping"; }

# Every workspace member: the core crates, the Linux app and the re kit.
rust='^(crates/|apps/linux/|Cargo\.(toml|lock)$|rust-toolchain|\.cargo/|(clippy|rustfmt|deny)\.toml$)'

# The crates a change reaches: those with a changed file and every crate that depends
# on one, as `-p name` arguments. A change to the workspace itself (Cargo.toml, the
# lock, the toolchain, lints) reaches them all: `--workspace`.
if $all || printf '%s\n' "$changed" | grep -qE '^(Cargo\.(toml|lock)$|rust-toolchain|\.cargo/|(clippy|rustfmt)\.toml$)'; then
    crates="--workspace"
else
    crates=$(cargo metadata --no-deps --format-version 1 | CHANGED="$changed" python3 -c '
import json, os, sys
m = json.load(sys.stdin)
root = m["workspace_root"] + "/"
changed = os.environ["CHANGED"].split("\n")
dirs = {p["name"]: os.path.dirname(p["manifest_path"]).replace(root, "") + "/" for p in m["packages"]}
users = {}
for p in m["packages"]:
    for d in p["dependencies"]:
        if d.get("path"):
            users.setdefault(d["name"], set()).add(p["name"])
reached = {n for n, d in dirs.items() if any(f.startswith(d) for f in changed)}
todo = list(reached)
while todo:
    for u in users.get(todo.pop(), ()):
        if u not in reached:
            reached.add(u)
            todo.append(u)
print(" ".join("-p " + n for n in sorted(reached)))')
fi
# reaches NAME: true when the change reaches that crate.
reaches() { [ "$crates" = "--workspace" ] || printf '%s\n' "$crates" | grep -qE "(^| )-p $1( |\$)"; }
# The mobile app follows its own sources, the design tokens and the Rust core it links.
mobile_changed() { touched '^(apps/mobile/|design/)' || reaches oscyl-ffi; }

stage "workflows"
if ! touched '^\.github/'; then skip; else
# .github lint: actionlint (with shellcheck) and zizmor's pedantic persona.
if command -v actionlint >/dev/null 2>&1; then
    actionlint
else
    echo "actionlint not installed, skipping (go install github.com/rhysd/actionlint/cmd/actionlint@latest)"
fi
if command -v zizmor >/dev/null 2>&1; then
    zizmor --quiet --persona pedantic --offline .github
else
    echo "zizmor not installed, skipping (uv tool install zizmor)"
fi
fi

stage "issue forms"
if ! touched '^\.github/ISSUE_TEMPLATE/'; then skip; else
# Valid YAML; every field a known type, labelled, with a unique id, and every
# choice with options (Ruby's own YAML).
if command -v ruby >/dev/null 2>&1; then
    ruby -ryaml -e '
      types = %w[markdown input textarea dropdown checkboxes]
      ARGV.each do |f|
        d = YAML.load_file(f)
        next unless d["body"]
        ids = d["body"].map { |b| b["id"] }.compact
        abort "#{f}: duplicate ids" if ids.uniq != ids
        d["body"].each do |b|
          abort "#{f}: unknown field type #{b["type"].inspect}" unless types.include?(b["type"])
          next if b["type"] == "markdown"
          abort "#{f}: a field without a label" unless b.dig("attributes", "label")
          if %w[dropdown checkboxes].include?(b["type"])
            opts = b.dig("attributes", "options")
            abort "#{f}: #{b["id"]} has no options" unless opts.is_a?(Array) && !opts.empty?
          end
        end
      end
      puts "issue forms: ok"' .github/ISSUE_TEMPLATE/*.yml
else
    echo "ruby not installed, skipping the issue-form check"
fi
fi

stage "spelling"
if command -v typos >/dev/null 2>&1; then
    if $all; then
        typos
    else
        # A deleted file is skipped (and must not fail the loop when it comes last);
        # so is one that left the repository but stays on disk, ignored.
        printf '%s\n' "$changed" | while read -r f; do if git cat-file -e "$tree:$f" 2>/dev/null; then printf '%s\0' "$f"; fi; done \
            | xargs -0 -r typos --force-exclude
    fi
else
    echo "typos not installed, skipping (cargo install typos-cli)"
fi

stage "release scripts"
if ! touched '^(CHANGELOG\.md$|scripts/(release|version))'; then skip; else
scripts/release-notes-test.sh
# Every released version keeps its section, and every compare link has one.
for version in $(sed -n 's/^## \[\([0-9][^]]*\)\] - .*/\1/p' CHANGELOG.md); do
    scripts/release-notes.sh "$version" >/dev/null
done
for link in $(sed -n 's/^\[\([^]]*\)\]: https.*/\1/p' CHANGELOG.md); do
    grep -q "^## \[$link\]" CHANGELOG.md || { echo "CHANGELOG.md: the link [$link] has no section" >&2; exit 1; }
done
scripts/version-test.sh
fi

if ! touched "$rust" || [ -z "$crates" ]; then
    stage "rust"
    skip
else
stage "fmt"
cargo fmt --all --check

# Only the crates the change reaches (see above), never the whole workspace for one file.
echo "crates: $crates"

stage "clippy"
# shellcheck disable=SC2086 # $crates is a list of arguments.
cargo clippy $crates --all-targets -- -D warnings

stage "tests"
# shellcheck disable=SC2086
cargo test $crates

if reaches oscyl-midi || reaches oscyl-app; then
stage "without midir (mobile build of the transport)"
cargo clippy -p oscyl-midi -p oscyl-app --no-default-features --all-targets -- -D warnings
cargo test -p oscyl-midi -p oscyl-app --no-default-features
fi

for manifest in crates/drivers/*/Cargo.toml; do
    dir=$(dirname "$manifest")
    # Shared family crates are not drivers; list them here by name.
    case "$(basename "$dir")" in logue) continue ;; esac
    if [ ! -f "$dir/tests/conformance.rs" ]; then
        echo "$dir: a driver crate needs tests/conformance.rs" >&2
        exit 1
    fi
    driver=$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest" | head -n 1)
    # A driver's conformance depends on its own crate and what it is built on.
    reaches "$driver" || continue
    stage "conformance: $driver"
    cargo test -p "$driver" --test conformance
done

stage "docs"
# shellcheck disable=SC2086
RUSTDOCFLAGS="-D warnings" cargo doc $crates --no-deps

if reaches oscyl-ffi; then
stage "ffi bindings"
# Kotlin bindings from the host library; the Android libraries only when
# cargo-ndk and an NDK are installed (the script says when it skips).
scripts/mobile-rust.sh

stage "ios (what Linux can prove)"
# The Rust core for the iPhone and the simulator (no SDK needed to check), and the
# iOS platform code's klibs (CoreMIDI, UIKit). Linking, cinterop and the app itself
# are proven on macOS only (.github/workflows/ios.yml).
if rustup target list --installed 2>/dev/null | grep -qx aarch64-apple-ios-sim \
    && rustup target list --installed | grep -qx aarch64-apple-ios; then
    for target in aarch64-apple-ios aarch64-apple-ios-sim; do
        cargo clippy -p oscyl-ffi --target "$target" -- -D warnings
    done
else
    echo "iOS Rust targets not installed, skipping (rustup target add aarch64-apple-ios aarch64-apple-ios-sim)"
fi
fi
fi

# The Xcode project's plists, asset catalogs, icon, launch colour and input list.
touched '^(apps/mobile/|design/|apps/linux/data/(icons|fonts)/|LICENSE-|scripts/mobile-assets\.sh$)' \
    && scripts/mobile-assets.sh --check

stage "licences"
# The About pages' third-party licences match Cargo.lock.
if touched '^(Cargo\.lock$|scripts/licences\.sh$|about\.(toml|hbs)$)'; then scripts/licences.sh --check; else skip; fi

stage "mobile app"
if ! mobile_changed; then
    skip
elif [ -z "${ANDROID_HOME:-}" ]; then
    echo "ANDROID_HOME not set, skipping the mobile app (see docs/mobile/README.md)"
else
    # The preview screenshots (nine tenths of the suite's time) only when something that
    # draws changed; a change to the Rust core alone runs the behaviour tests.
    if touched '^(apps/mobile/|design/|po/)'; then
        tests=":androidApp:verifyRoborazziDebug :androidApp:testDebugUnitTest"
    else
        echo "no UI source changed, skipping the preview screenshots"
        tests=":androidApp:testDebugUnitTest -Poscyl.goldens=false"
    fi
    # --warning-mode=all, not fail: Kotlin 2.4.21 warns about its own iOS setup.
    # The shared code's common half compiled on its own: an Android-only API there (which
    # the Android build accepts) fails here, as it would on the iOS build only macOS runs.
    # shellcheck disable=SC2086 # $tests is a list of tasks.
    (cd apps/mobile && ./gradlew --warning-mode=all \
        :shared:compileCommonMainKotlinMetadata :shared:check $tests :androidApp:assembleDebug \
        :iosPlatform:compileKotlinIosArm64 :iosPlatform:compileTestKotlinIosSimulatorArm64)
    # Accessibility (ATF) checks: on an emulator already running (no boot), installed as
    # io.github.oscyl.atf beside anything else on it; else on a headless emulator Gradle
    # manages. A phone is not used: the checks read English words, and a phone keeps its
    # owner's language (and API 37 drops an input API the runner's Espresso still uses).
    attached=$(adb devices 2>/dev/null | awk 'NR > 1 && $2 == "device" && $1 ~ /^emulator-/ { print $1; exit }')
    if ! touched '^apps/mobile/(shared|androidApp)/src/(commonMain|androidMain|main|androidTest)/'; then
        echo "no UI source changed, skipping the accessibility checks"
    elif [ -n "$attached" ]; then
        (cd apps/mobile && ANDROID_SERIAL=$attached ./gradlew --warning-mode=all -Poscyl.idSuffix=.atf :androidApp:connectedDebugAndroidTest)
    elif [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        (cd apps/mobile && ./gradlew --warning-mode=all :androidApp:pixel6Api34DebugAndroidTest)
    else
        echo "no usable /dev/kvm, skipping the accessibility checks on the managed device"
    fi
fi

stage "cargo deny"
if ! touched '^(Cargo\.(toml|lock)$|crates/.*/Cargo\.toml$|deny\.toml$)'; then
    skip
elif command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check
else
    echo "cargo-deny not installed, skipping (cargo install cargo-deny --locked)"
fi

mkdir -p target
echo "$tree" > "$mark"
printf '\nAll checks passed.\n'
