# Test on Windows hardware from Linux

This guide lets you run Oscyl on a Windows PC that has a synth plugged in,
from a Linux machine on the same network. The script
[`scripts/windows-test-host.ps1`](https://github.com/oscyl/oscyl/blob/main/scripts/windows-test-host.ps1)
opens a locked SSH door into the PC. The door stays closed until you open
it, and it closes itself after a few hours.

## What the door allows

- One standard Windows user, `oscyl-test`, with no administrator rights. It
  does not show on the sign-in screen.
- A login only with one SSH key, and only from one address: your Linux
  machine's.
- No password login, and no forwarding of ports, agents or X11.
- A firewall rule for port 22 from that address only, on private networks.
  The rule stays off while the door is closed.

## Make a key on Linux

```sh
ssh-keygen -t ed25519 -f ~/.ssh/id_oscyl_windows
```

## Install the door on the PC, once

1. Copy `scripts/windows-test-host.ps1` to the PC.
2. Open PowerShell as administrator, in the folder of the script.
3. Run the script. Replace `<linux-address>` with your Linux machine's
   address, and `<public key>` with the line in
   `~/.ssh/id_oscyl_windows.pub`:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\windows-test-host.ps1 -Install -LinuxIp <linux-address> -PublicKey '<public key>'
   ```

The script installs OpenSSH Server, set to start only by hand. It makes the
user, the key file and the firewall rule. It keeps a copy of the old
`sshd_config` as `sshd_config.before-oscyl`. Then it closes the door and
says "Installed and closed. Open with -On."

## Add the PC to your SSH configuration

On Linux, add a host entry to `~/.ssh/config`. Replace `<pc-address>` with
the PC's address:

```text
Host oscyl-win
    HostName <pc-address>
    User oscyl-test
    IdentityFile ~/.ssh/id_oscyl_windows
    IdentitiesOnly yes
    StrictHostKeyChecking accept-new
```

## Open the door, test, close it

1. On the PC, in an administrator PowerShell, open the door:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\windows-test-host.ps1 -On -Hours 3
   ```

   The door stays open for three hours unless you give another `-Hours`.
   A scheduled task closes it then.

2. On Linux, build the command line tool for Windows and copy it to the PC.
   The build needs `mingw-w64-gcc` and the Rust target
   `x86_64-pc-windows-gnu`:

   ```sh
   cargo build --release -p oscyl-cli --target x86_64-pc-windows-gnu
   scp target/x86_64-pc-windows-gnu/release/oscyl.exe oscyl-win:
   ```

3. Run it on the PC, against the synth:

   ```sh
   ssh oscyl-win oscyl.exe ports
   ssh oscyl-win oscyl.exe info
   ```

4. Close the door when you are done. On the PC, run the script with `-Off`,
   or with no argument:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\windows-test-host.ps1 -Off
   ```

   `-Off` stops the SSH server, ends every session of `oscyl-test` and turns
   the firewall rule off.

For the checks to run on the synth, see
[Check a driver on hardware](hardware-check.md).

## Remove the door

```powershell
powershell -ExecutionPolicy Bypass -File .\windows-test-host.ps1 -Uninstall
```

`-Uninstall` closes the door, then removes the user and its profile, the
key, the firewall rule and OpenSSH Server. It puts back the `sshd_config`
from before `-Install`.
