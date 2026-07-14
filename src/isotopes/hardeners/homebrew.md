# Homebrew

## Summary

- Only `brew` can alter `/opt/homebrew`
- Approval gates can be configured to stop agents installing things behind your
  back.

## What it Does

Installs `/usr/local/bin/brew` as a small setuid/setgid Automic Vault launcher
for `/opt/homebrew/bin/brew`.

The root phase creates the `automic` user and `vault` group when needed, owns
`/opt/homebrew` as `automic:vault`, and installs the launcher as
`06755 automic:vault`.

## Details

- This targets Apple Silicon Homebrew at `/opt/homebrew`.
- Existing `/usr/local/bin/brew` files are left alone unless they are already
  the Automic Vault brew stub.
- Hardening copies missing files from the invoking user's `~/.homebrew` into
  the hardened account, preserving configuration already created there. This
  includes Homebrew's tap trust store.
- The invoking user's `~/Library/Caches/Homebrew` contents are merged into the
  hardened cache and removed from their original location instead of being
  downloaded again.
- `/usr/local/bin` must precede `/opt/homebrew/bin` in `PATH`. After hardening,
  run `hash -r` or start a new shell so it does not keep using a cached path to
  the original `brew` executable.
- Every launcher invocation is authorized by the menu bar app before Homebrew
  runs. Read Only Access approves known inspection commands automatically and
  prompts for writes or unknown commands; Read & Update Access additionally
  approves `brew update` and is the default; No Access prompts for every
  command, while Full Access approves every command automatically.
- The launcher fails closed when the approval service is unavailable.
- The stub clears the environment, restores only safe terminal/locale values,
  and executes `/opt/homebrew/bin/brew` directly.

## Cask Caveats

Cask support is best-effort because Homebrew runs as the `automic` account
after hardening:

- Apps installed in `/Applications` are owned by `automic:vault`. Most apps
  run normally, but an in-app updater may request authentication or fail if it
  expects the invoking user to own the app bundle.
- Homebrew sees `HOME` as `/opt/homebrew/var/automic`. Cask artifacts that
  default to `~/Library`, including preference panes, fonts, Quick Look
  plugins, and services, are installed under that home instead of the invoking
  user's home.
- Non-sudo cask installer scripts run as `automic`. Package installers run as
  root but receive `automic` as `USER`, `LOGNAME`, and `USERNAME`. Casks that
  configure the current user's Keychain, LaunchAgents, login items, or
  preferences may install into the wrong account or fail.
- Artifact destinations can be overridden explicitly, for example with
  `--prefpanedir=/Library/PreferencePanes`. This does not change the identity
  or `HOME` seen by installer scripts, so it is not a general workaround.
