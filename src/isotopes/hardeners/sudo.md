## What It Does

`av harden sudo` checks whether Touch ID is already enabled for sudo. If it is not, it prints the command to enable it through `/etc/pam.d/sudo_local`.

The hardener does not edit PAM files itself. It leaves the root-owned change to the user.

## Caveats

- macOS must include `/etc/pam.d/sudo_local` from `/etc/pam.d/sudo`.
- Touch ID sudo still falls back to password authentication when biometrics are unavailable.
