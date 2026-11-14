# ADR 0001: Monorepo

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Oscyl is growing from a minilogue xd tool into a companion for several
synths. Each synth gets a driver; the core, the conformance suite, the
transports, the CLI and the apps all depend on the driver contract. The
contract will change often while the second and third drivers are written.

## Decision

Keep everything in one repository and one Cargo workspace: the contract
(`crates/oscyl-driver`), the core, the conformance suite, the transports,
the CLI, every driver under `crates/drivers/`, and the apps under `apps/`.


## Consequences

- A contract change and the fixes in every driver land in one commit, so
  `main` always builds and passes conformance.
- CI runs the conformance suite for every driver on every commit.
- Driver maintainers own their paths through `CODEOWNERS` instead of owning
  separate repositories.
- Third-party drivers outside the repository are not supported for now;
  they can come later, once the contract crate is published and stable.
