## What and why

<!-- What this change does and why. Link the issue or RFC. -->

## Checklist

- [ ] Commit messages follow Conventional Commits with a scope.
- [ ] `scripts/ci.sh` passes locally (see CONTRIBUTING.md for its stages).
- [ ] UI changes: goldens recorded and looked at (mobile), or screenshots attached (Linux).
- [ ] New strings have their Italian translation (`po/it.po` or `values-it/strings.xml`).
- [ ] User-visible changes are in `CHANGELOG.md` under `## [Unreleased]` (or the PR is labelled `no-changelog`; CI checks).
- [ ] No vendor firmware, factory presets, binaries or scraped content; fixtures are synthetic.
- [ ] A driver-contract or cross-driver change has an accepted RFC in `docs/rfcs/`.

### New driver only

- [ ] Driver crate under `crates/drivers/<vendor>-<model>` implements the contract.
- [ ] Simulator included, with fault injection for known hardware faults.
- [ ] `tests/conformance.rs` passes and the package is added to the `conformance` matrix in `.github/workflows/ci.yml`.
- [ ] This pull request names the sources (public documents only), the known limits and the firmware versions covered.
- [ ] A named maintainer is listed in `.github/CODEOWNERS` for the driver.
