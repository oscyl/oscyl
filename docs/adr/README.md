# Architecture decision records

An ADR records one architecture decision that has been made: the context,
the decision, and its consequences. Unlike an RFC, which proposes a change,
an ADR is the short record left after the decision.

Files are numbered in order (`0001-short-title.md`) and are not edited
afterwards except to mark them **Superseded** by a later ADR.

| ADR | Decision |
|-----|----------|
| [0001](0001-monorepo.md) | One repository for the core, every driver and every app |
| [0002](0002-driver-contract.md) | Drivers are compiled-in crates behind an object-safe contract, proven by conformance against simulators, with a native library format |
