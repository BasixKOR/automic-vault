# Homebrew

## Summary

- Only `brew` can alter `/opt/homebrew`.
- Hardened Homebrew manages formulae only. Homebrew Casks are not supported.
- Approval gates can be configured to stop agents installing things behind your
  back.

Zsh completions are copied into a user-owned mirror after Homebrew runs. This
keeps `/opt/homebrew` protected, satisfies zsh's ownership checks, and avoids a
`sudo` prompt when Homebrew regenerates completion files.

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
- Zsh startup must evaluate the hardened launcher's shell environment before
  `compinit`:

  ```zsh
  eval "$(/usr/local/bin/brew shellenv zsh)"
  autoload -Uz compinit
  compinit
  ```

  Replace any startup invocation of `/opt/homebrew/bin/brew shellenv` with the
  `/usr/local/bin/brew shellenv zsh` command above.
- Every launcher invocation is authorized by the menu bar app before Homebrew
  runs. Read Only Access approves known inspection commands automatically and
  prompts for writes or unknown commands; Read & Update Access additionally
  approves `brew update` and is the default; No Access prompts for every
  command, while Full Access approves every command automatically.
- The launcher fails closed when the approval service is unavailable.
- The stub clears the environment, restores only safe terminal/locale values,
  and executes `/opt/homebrew/bin/brew` directly.
- Homebrew's zsh completions remain `automic:vault`. After each invocation, the
  launcher permanently drops to the configured desktop UID and copies protected
  regular completion files into
  `~/.local/share/automic-vault/homebrew/zsh/site-functions`. The mirror is
  replaced atomically and the original Homebrew completion directory is removed
  from `fpath` by `brew shellenv zsh`. Completion symlinks that resolve outside
  the protected Homebrew prefix are omitted with a warning.
- A failed refresh leaves the previous mirror intact. It warns without changing
  the result of an ordinary Homebrew command; `brew shellenv zsh` fails instead
  of emitting an unsafe `fpath` when no valid mirror can be published.

## Casks

**Homebrew Casks are categorically incompatible with this hardener.** A cask is
not confined to the Homebrew prefix: it may modify `/Applications`, `/Library`,
launch services, system plugins, privileged packages, and user data. Running
that package manager as the protected `automic` account also makes its nested
`sudo` operations authenticate the wrong identity. Pretending this is the same
ownership model as a formula weakens the security guarantee and still fails for
ordinary casks.

The launcher therefore pins `install`, `reinstall`, `upgrade`, `uninstall`,
`remove`, and `rm` to `--formula` before requesting approval. Explicit cask
mutations are rejected. `brew bundle` is rejected because a Brewfile may contain
casks. Read-only commands such as `brew list --cask` remain available for
migration and inspection.

Hardening refuses to proceed while `/opt/homebrew/Caskroom` contains installed
casks. For an existing hardened installation, run `sudo av unharden brew`,
remove or migrate every cask using `/opt/homebrew/bin/brew`, then run
`sudo av harden brew` again. Homebrew is user-writable between those commands;
do not run hardened tools or expose credentials through them during that
migration window.
