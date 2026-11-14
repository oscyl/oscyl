# Security policy

## Reporting a vulnerability

Please report security problems privately, through GitHub's private
vulnerability reporting: open the repository's **Security** tab and choose
**Report a vulnerability**. Do not open a public issue for a security problem.

Include what you found, how to reproduce it (a synthetic input file or MIDI
message sequence is ideal), and what you think the impact is. A maintainer
will acknowledge the report, work on a fix with you, and credit you in the
release notes unless you prefer otherwise.

Oscyl is pre-release; fixes land on `main` and in the next release.

## Scope

Areas where we especially want reports:

- **MIDI and SysEx input parsing.** Oscyl decodes replies from connected
  devices. A malformed or hostile message must never crash the app, corrupt
  memory, or cause unbounded allocation.
- **Library file parsing.** Oscyl opens native `.oscyllib` libraries and
  vendor library and program files that users may download from anywhere. A
  crafted file must fail cleanly, never crash or write outside the files the
  user chose.
- **Firmware-write safety.** Writes to a synth's memory (programs, user units,
  and anything that touches firmware) must only happen when the user asked
  for them, and must leave the device in a committed, safe state. A way to
  trigger an unintended or unsafe write is a security issue.

Out of scope: problems in the synth's own firmware, and issues that require
an attacker who already controls the user's account or machine.
