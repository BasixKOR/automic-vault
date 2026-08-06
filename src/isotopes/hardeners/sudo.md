## What It Does

`av harden sudo` previews the PAM change, then directs you to rerun it with
sudo. The privileged run appends:

```sh
echo 'auth sufficient pam_tid.so' >> /etc/pam.d/sudo_local
```

## Caveats

- macOS must include `/etc/pam.d/sudo_local` from `/etc/pam.d/sudo`.
- Touch ID sudo still falls back to password authentication when biometrics are
  unavailable.
