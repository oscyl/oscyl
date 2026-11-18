# Update screenshots and goldens

Oscyl's pictures come from the apps themselves. This guide updates the
mobile app's screenshot goldens, a screenshot of the desktop app, and the
pictures that the README and the website show.

## Update the mobile goldens

Every `@Preview` in the mobile app is a screenshot golden, under
`apps/mobile/androidApp/src/test/snapshots/`. The previews are the only
source of the app's screenshots.

1. Change the UI and its previews.
2. Record the goldens:

   ```sh
   cd apps/mobile && ./gradlew :androidApp:recordRoborazziDebug
   ```

3. Look at every golden that changed (`git status`). Look at each theme,
   size and font scale, and at the mirrored and pseudo-locale versions. A
   golden that nobody looked at hides a bug.
4. Check them as CI does:

   ```sh
   ./gradlew :androidApp:verifyRoborazziDebug
   ```

   When the check fails, the comparison images are in
   `apps/mobile/androidApp/build/outputs/roborazzi/`.

## Take a screenshot of the desktop app

The desktop app draws a scripted state and saves its window as a PNG:

```sh
cargo run -p oscyl-gtk -- --demo demo:filter:edited --screenshot shot.png
```

The function `run_demo` in
[`apps/linux/src/onboarding.rs`](https://github.com/oscyl/oscyl/blob/main/apps/linux/src/onboarding.rs)
lists the other states. `--theme light`, `--theme dark` and
`--size 1600x1000` set the theme and the window's size.

## Update the README's and the website's pictures

`scripts/readme-images.sh` makes every picture in `docs/images/` again: the
desktop app's windows from its demo states, and the phone's screens from
the goldens.

1. Record the mobile goldens first, if the phone's screens changed.
2. Build the desktop app: `cargo build -p oscyl-gtk`.
3. Run the script. It needs a display and ImageMagick 7:

   ```sh
   scripts/readme-images.sh
   ```

4. Look at each picture that changed in `docs/images/`.
