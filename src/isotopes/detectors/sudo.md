# sudo Detector

## Trigger Conditions

- sudo is not configured to offer Touch ID through `pam_tid.so`.
- sudoers does not set `Defaults timestamp_timeout=0`.
- sudoers sets a non-zero `timestamp_timeout`.

## Sensitive Files

- `/etc/pam.d/sudo`
- `/etc/pam.d/sudo_local`
- `/etc/sudoers`
- `/etc/sudoers.d/*`
