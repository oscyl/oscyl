# Governance

This document describes who looks after Oscyl, how decisions are made, and
what a synth driver needs before it is accepted.

## Roles

**Core maintainers** look after the project as a whole. That is the driver
contract (`crates/oscyl-driver`), the core (`crates/oscyl-core`), the
conformance suite (`crates/oscyl-conformance`), the transports, the CLI, the
apps, CI and releases. They review and merge changes anywhere in the repository and decide
on RFCs.

**Driver maintainers** look after one synth driver under `crates/drivers/`.
They review changes to their driver and keep its simulator honest to the
hardware. For a Tier 1 driver, they also validate it on real hardware before
each release. Each driver maintainer is
named in [`.github/CODEOWNERS`](.github/CODEOWNERS) for that driver's paths.

**Contributors** are everyone else who sends issues, reviews or changes.
Anyone can become a driver maintainer by getting a driver accepted (see
below). The core maintainers can invite a driver maintainer with a record of
good reviews to become a core maintainer.

The current maintainers are listed in [`.github/CODEOWNERS`](.github/CODEOWNERS).

## Decision making

Most decisions happen in pull requests. A change merges when a maintainer who
owns the changed paths (per `CODEOWNERS`) approves it and CI passes. A change
to a driver needs approval from that driver's maintainer; a change to shared
code needs a core maintainer.

Maintainers aim for consensus. When it cannot be reached, the core
maintainers decide by simple majority. They write the reasoning down in the
pull request, the RFC or an architecture decision record in
[`docs/adr/`](docs/adr/README.md).

## When an RFC is required

Write an RFC (see [`docs/rfcs/`](docs/rfcs/README.md)) before the code for:

- any change to the driver contract in `crates/oscyl-driver`, breaking or not;
- any change that touches more than one driver, or that changes what every
  driver must provide (for example a new conformance check);
- a change to the native library format (`.oscyllib`).

Bug fixes, new features inside one driver, and changes that keep the contract
and the formats as they are do not need an RFC.

## Support tiers

Every driver has a tier, shown in
[Supported synths](docs/reference/synths.md).

- **Tier 3 (community):** the driver compiles but is not verified. Apps may
  hide it behind an "experimental" setting.
- **Tier 2 (conformance-proven):** the driver passes the conformance suite in
  CI against its simulator on every commit.
- **Tier 1 (hardware-validated):** Tier 2, plus a named driver maintainer
  validates the driver on real hardware for each release. A Tier 1 failure
  blocks the release.

A driver drops a tier when it stops meeting the requirements, for example
when its maintainer steps down and nobody takes over the hardware validation.

## Driver acceptance

A new driver is accepted when its pull request provides all of the following:

1. A driver crate under `crates/drivers/<vendor>-<model>` implementing the
   contract in `crates/oscyl-driver`.
2. A simulator for the synth, with fault injection for the faults known on
   the real hardware.
3. A `tests/conformance.rs` that calls `oscyl_conformance::run(&DRIVER)` and
   passes, and the driver package added to the `conformance` matrix in
   `.github/workflows/ci.yml`.
4. The sources, named in the pull request and in the crate's module docs:
   the maker's public MIDI documentation, manuals and open references. Also
   the known limits, and the firmware versions covered.
5. Synthetic test fixtures only. Never vendor firmware, factory presets,
   binaries, or content scraped from third-party sites.
6. A named maintainer listed in `.github/CODEOWNERS` for the driver's paths.

A driver that meets these is accepted at Tier 2. It becomes Tier 1 when its
maintainer commits to hardware validation for each release.
