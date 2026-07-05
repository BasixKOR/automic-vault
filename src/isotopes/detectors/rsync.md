# rsync Detector

## Trigger Conditions

- rsync password file contains plaintext credentials.

## Sensitive Files

- `~/.rsync_pass`
- `~/.rsync-password`
- `~/.rsync.pass`
- `~/.rsyncd.conf`
- `~/.config/rsync/rsyncd.conf`
- `secrets files referenced by scanned rsync config`
