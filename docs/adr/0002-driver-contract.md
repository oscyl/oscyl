# ADR 0002: Driver contract, conformance and native library format

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Supporting several synths needs a clear line between what is generic (the
library, sync, audition, the editor's rendering) and what is specific to one
synth (protocol, program bytes, quirks). It also needs a way to trust a
driver without owning the hardware, and a library file that is not tied to
one vendor's format.

## Decision

1. **Drivers are compiled-in crates behind an object-safe contract.** Each
   driver is a crate under `crates/drivers/` that implements the traits in
   `oscyl-driver` and exposes a static `DRIVER`. Apps and the core use
   drivers only as `&dyn Driver`, so they can pick one at run time (from a
   connected synth's identity, or from a library's driver id) with no
   synth-specific code. There are no dynamic plugins.
2. **Capabilities are declared, not assumed.** A driver describes its
   identity, capabilities, field schema, curation, protocol, commit planning
   and file formats as data; the apps read capabilities and assume nothing
   else.
3. **Drivers are proven against simulators.** Every driver ships a sans-IO
   simulator, with fault injection for faults known on the hardware. The
   `oscyl-conformance` suite runs the same checks against every driver's
   simulator in CI. Hardware tests stay `#[ignore]`d and back the Tier 1
   release validation (see [GOVERNANCE.md](../../GOVERNANCE.md)).
4. **Libraries use a native format, `.oscyllib`.** It records which driver a
   library belongs to and stores programs as the driver's own bytes. Vendor
   library and program files are import and export formats handled
   losslessly by each driver. The format is described in
   [native-library-format.md](../protocol/native-library-format.md).

## Consequences

- Any change to the contract needs an RFC and updates every driver in the
  same change.
- A driver without a simulator, or one that fails conformance, is not
  accepted.
- Libraries written by older versions are migrated to `.oscyllib`
  automatically, keeping the originals.
- The contract crate is versioned with SemVer and will be published to
  crates.io once the project is public.
