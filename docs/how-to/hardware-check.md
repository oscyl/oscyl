# Check a driver on hardware

This guide checks a driver on the real synth. You need the synth, a USB
cable, and the command line tool `oscyl` (from a release, or
`cargo run -p oscyl-cli --`).

When the check is needed:

- **Tier 2** needs no hardware check. The conformance suite against the
  simulator is enough for a driver to be accepted.
- **Tier 1** needs this check for each release, by the driver's named
  maintainer. A Tier 1 failure blocks the release.
- **A new driver** gains from one check before its first review, if you
  have the synth. Its simulator copies only the faults that you saw, so the
  real synth can find more.

[Supported synths](../reference/synths.md#tiers) lists each driver's tier.

## Before you start

- Find the synth's firmware version. `oscyl info` shows it for a driver that
  reads it.
- Choose a few scratch slots: slots whose programs you can lose.

## Check the driver

1. Connect the synth, and check that Oscyl finds it:

   ```sh
   oscyl ports
   oscyl info
   ```

2. Back up the whole synth before any write:

   ```sh
   oscyl backup before.oscyllib
   ```

   Keep this file until the check is done. It is your way back.

3. Read the program that the synth plays:

   ```sh
   oscyl current
   ```

4. Make a library with new programs in your scratch slots only. For
   example, open `before.oscyllib` in the desktop app, change the scratch
   slots, and save it as `scratch.oscyllib`.
5. Write it. Oscyl writes only the programs that differ, and backs up the
   synth first:

   ```sh
   oscyl restore scratch.oscyllib
   ```

   Check that the output names only your scratch slots, and that each write
   ends.
6. Compare the synth with the library. Expect `0 of N programs differ`:

   ```sh
   oscyl diff scratch.oscyllib
   ```

7. Play a program without storing it, and listen to the synth:

   ```sh
   oscyl audition scratch.oscyllib --index 3
   ```

8. Put the scratch slots back:

   ```sh
   oscyl restore before.oscyllib
   oscyl diff before.oscyllib
   ```

   The diff must say that no program differs.

9. In the desktop app, connect the synth and check that it reads every
   slot. Turn on Live, edit a value, and listen to the change.

## Record the result

Write down in the pull request, or in the release's notes:

- the synth and its firmware version;
- the platform and the app or command that you used;
- which steps passed, and the output of each step that failed.

Never commit the captures or the backups. Keep them in `private/`, which
git ignores.

A Tier 1 failure blocks the release. [Cut a release](release.md) says
where the check fits.
