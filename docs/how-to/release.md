# Cut a release

This guide makes a release. A release is a signed tag `vX.Y.Z` on `main`.
When you push the tag, `.github/workflows/release.yml` builds every
artifact, writes `SHA256SUMS`, attests the build provenance, and opens a
**draft** GitHub Release. Its notes are the version's section of
`CHANGELOG.md`. Nothing is public until a maintainer publishes the draft.

[Release artifacts](../reference/release-artifacts.md) lists what the
workflow builds and what it refuses.

## Make the release

1. **Check the hardware.** Each Tier 1 driver's maintainer checks the
   driver on the real synth for this release
   ([Check a driver on hardware](hardware-check.md)). Name any driver that
   missed the check in the changelog.
2. **Change the version** in a release pull request:
   - In `Cargo.toml`, set `[workspace.package] version`. Then run
     `cargo check` to update `Cargo.lock`. The version is `X.Y.Z` or
     `X.Y.Z-rc.N`, with Y and Z under 100 and N from 1 to 98.
   - In `apps/mobile/iosApp/Configuration/Config.xcconfig`, set
     `MARKETING_VERSION` to the `marketing` line of `scripts/version.sh`,
     and `CURRENT_PROJECT_VERSION` to its `code` line.
   - In `apps/linux/data/io.github.oscyl.Oscyl.metainfo.xml`, add a new
     `<release>` as the first one in `<releases>`.
3. **Edit the changelog.**
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`.
   - Add a new, empty `## [Unreleased]` above it.
   - At the bottom, make `[Unreleased]` compare `vX.Y.Z...HEAD`, and add
     `[X.Y.Z]: …/compare/vPREV...vX.Y.Z`.
   - Write the section for users. It is the release note.
4. **Check the pull request.** `scripts/release-check.sh X.Y.Z` checks that
   the versions agree and that the section exists.
   `scripts/release-notes.sh X.Y.Z` prints what the release will say.
   Merge the pull request when CI passes.
5. **Do a dry run (optional).** In GitHub, choose Actions, Release, Run
   workflow, on `main`. The run builds and attests everything and
   publishes nothing. The artifacts are on the run.
6. **Tag** the merge commit, sign the tag, check the signature, and push it:

   ```sh
   git switch main && git pull --ff-only
   git tag -s vX.Y.Z -m "Oscyl X.Y.Z"
   git tag -v vX.Y.Z
   git push origin vX.Y.Z
   ```

   A version with a pre-release suffix, such as `v1.0.0-rc.1`, becomes a
   pre-release.
7. **Approve** the `release` environment when the workflow asks. The
   Android signing and the publish job wait for it.
8. **Review the draft**: its notes, its assets and `SHA256SUMS`. Then
   publish it.

## How the version numbers work

`scripts/version.sh` is the one parser of the version. It prints the
version, iOS's numeric `marketing` version, and the build `code`:

- for a release candidate, `code` is ((X×100 + Y)×100 + Z)×100 + N;
- for a final release, it is ((X×100 + Y)×100 + Z)×100 + 99.

So 0.4.0-rc.1 is 40001, and 0.4.0 is 40099. Every release candidate gets
its own store build, below its final release. The Android app takes its
`versionName` and `versionCode` from the script when it builds.
`scripts/release-check.sh` checks the iOS numbers.

## Set up the release, once

- **The environment `release`** (Settings, Environments): the maintainers
  as required reviewers. Limit its deployment refs to the tags `v*.*.*` and
  to the branch `main`, for dry runs.
- **The Android signing secrets**, as secrets of the `release` environment:
  - `ANDROID_KEYSTORE_BASE64`: the output of `base64 -w0 release.jks`;
  - `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and
    `ANDROID_KEY_PASSWORD`.

Keep the keystore and its passwords offline as well. A lost upload key
needs a reset from Google Play, and installed APKs update only from the
same key.
