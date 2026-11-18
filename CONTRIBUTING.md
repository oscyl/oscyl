# Contributing

Thank you for helping. Hardware testers, drivers for new synths and
translations are the most welcome help. This page says how to send a change.
The how-to guides of the [documentation](https://oscyl.github.io/oscyl/docs/)
have the details.

Everyone follows the [Code of Conduct](CODE_OF_CONDUCT.md). Project roles
and decisions are in [GOVERNANCE.md](GOVERNANCE.md). Report a security
problem privately, as [SECURITY.md](SECURITY.md) describes.

## Find your guide

- **Build and run Oscyl:** [the desktop apps and the command line](docs/how-to/build.md),
  [the mobile apps](docs/how-to/mobile.md).
- **Add a synth:** the tutorial [Your first driver](docs/tutorials/first-driver.md),
  then [Add a driver for a new synth](docs/how-to/add-a-synth.md). To ask
  for a synth without writing its driver, open a "New synth request" issue.
- **Test a synth on hardware:** [Check a driver on hardware](docs/how-to/hardware-check.md).
- **Translate:** [Translate Oscyl](docs/how-to/translate.md).
- **Change the UI:** [Update screenshots and goldens](docs/how-to/screenshots.md).
- **Cut a release:** [Cut a release](docs/how-to/release.md).

## Run the checks

Before you send a change, run:

```sh
scripts/ci.sh
```

It runs the checks that CI runs, but only the ones that your change
reaches, and it skips a check whose tool is missing.
[Run the checks](docs/how-to/run-the-checks.md) says how to run one part
while you work, and [Checks and workflows](docs/reference/ci.md) lists every
stage.

## Send a change

- Write commit messages as
  [Conventional Commits](https://www.conventionalcommits.org/), with a scope,
  for example `feat(core): add program model`.
- Versions follow [Semantic Versioning](https://semver.org/).
- **The changelog rule:** every change that users can see adds a line to
  `CHANGELOG.md`, under `## [Unreleased]`. Write it for users: what they can
  do now, not how the code changed.
- Never add vendor firmware, factory presets, librarian binaries or content
  from third-party sites. Make test fixtures yourself, or make them
  synthetic.
- New strings need their Italian translation. If you do not read Italian,
  ask for a review, and say so in the pull request.

## Propose a bigger change

A change to the driver contract, a change that touches more than one
driver, and a change to the `.oscyllib` format start with an RFC. Copy
`docs/rfcs/0000-template.md` and follow [the RFC process](docs/rfcs/).
Decisions already made are in the [decision records](docs/adr/).

## Licence

Unless you state otherwise, any contribution you submit is dual-licensed as
MIT OR Apache-2.0, without additional terms.
