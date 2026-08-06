# Homebrew

## Summary

- Only `brew` can alter `/opt/homebrew`.
- Hardened Homebrew manages formulae and a narrow class of CLI-only casks.
- Approval gates can be configured to stop agents installing things behind your
  back.

**Homebrew shell completions are unavailable while hardened.** Completion files
remain protected inside `/opt/homebrew`; the launcher never copies them into a
user-owned location or weakens shell ownership checks.

## What it Does

Installs `/usr/local/bin/brew` as a small setuid/setgid Automic Vault launcher
for `/opt/homebrew/bin/brew`.

The root phase creates the `automic` user and `vault` group when needed, owns
`/opt/homebrew` as `automic:vault`, and installs the launcher as
`06755 automic:vault`.

## Rationale

Modern macOS has numerous protections to prevent malware or agents from
altering installed sofware.

These protections apply to `.apps` and other bundle types, not to command line
tools. Command line tools are protected by their parent `.app` which is often
a Terminal but nowadays is often an Agent Harness.

Thus we need to apply UNIX security permissions to our command line tools to
ensure what is installed *remains what is installed*. Automic Vault hardening
is that solution.

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
- Shell startup must evaluate the hardened launcher's environment instead of
  invoking `/opt/homebrew/bin/brew` directly:

  ```sh
  eval "$(/usr/local/bin/brew shellenv)"
  ```

  For compatibility with existing startup files, `brew shellenv zsh` is
  normalized to generic shell output and does not add Homebrew's protected zsh
  completion directory to `fpath`.
- Every launcher invocation is authorized by the menu bar app before Homebrew
  runs. Read Only Access approves known inspection commands automatically and
  prompts for writes or unknown commands; Read & Update Access additionally
  approves `brew update` and is the default; No Access prompts for every
  command, while Full Access approves every command automatically.
- The launcher fails closed when the approval service is unavailable.
- The stub clears the environment, restores only safe terminal/locale values,
  and executes `/opt/homebrew/bin/brew` directly.
- Do not add `/opt/homebrew/share/zsh/site-functions` to `fpath` or bypass zsh's
  ownership checks. Older user-owned Automic Vault completion mirrors are no
  longer updated and may be removed.

## Casks

**Application and installer casks are categorically incompatible with this
hardener.** A normal cask is not confined to the Homebrew prefix: it may modify
`/Applications`, `/Library`, launch services, system plugins, privileged
packages, and user data. Running that package manager as the protected
`automic` account also makes its nested `sudo` operations authenticate the
wrong identity. Pretending this is the same ownership model as a formula
weakens the security guarantee and still fails for ordinary casks.

The sole exception is a CLI-only cask from the official `homebrew/cask`
repository. It must declare one or more `binary` artifacts whose sources remain
inside its staged Caskroom and whose targets are directly inside
`/opt/homebrew/bin`. Generated shell completions may remain protected inside the
Homebrew prefix but are not added to shell paths. `zap` metadata may be present,
but `--zap` is rejected and never runs. Cask dependencies and every app,
package, installer, script, flight block, service, plugin, arbitrary artifact,
completion-file, manpage, or external target are rejected.

Cask mutations must use `--cask` and name every cask explicitly. The launcher
checks Homebrew's effective JSON metadata after approval and validates the
protected installation receipt before upgrades, reinstalls, or removals.
Homebrew's own in-process forbidden-artifact check is also enabled for the
actual installation. Path-based casks, custom destination flags, bulk cask
upgrades, and `brew bundle` are unavailable. Commands without `--cask` remain
pinned to `--formula`.

Hardening refuses to proceed while `/opt/homebrew/Caskroom` contains anything
other than validated CLI-only casks. For an existing hardened installation, run
`sudo av unharden brew`, remove or migrate incompatible casks using
`/opt/homebrew/bin/brew`, then run `av harden brew` again. Homebrew is
user-writable between those commands; do not run hardened tools or expose
credentials through them during that migration window.
