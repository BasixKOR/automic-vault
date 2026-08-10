# git-credential-fill Detector

## Trigger Conditions

- Git config delegates GitHub credentials to an untrusted `gh auth
  git-credential` helper.
- `git credential fill` returns a GitHub password or token for github.com.

A GitHub helper is not a Finding when an empty helper first resets inherited
helpers and every effective helper is an absolute path to the signed Automic
Vault `gh` Isotope. That helper requests the token through the `gh` Secret Gate
instead of making it ambient authority.

## Sensitive Files

- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`

## Mitigation

For an untrusted helper, remove the helper that returned the GitHub token,
reject the cached credential, and move GitHub remotes to SSH. A signed Automic
Vault `gh` Isotope may remain when it is the complete effective helper chain.

## Confirm the Finding

Ask Git what it can return without a prompt:

```sh
printf 'protocol=https\nhost=github.com\n\n' |
  GIT_TERMINAL_PROMPT=0 GCM_INTERACTIVE=never git credential fill
```

Any non-empty `password=` line confirms the finding. Clean output has no
`password=` line. Git may print the username or report that it could not read a
password.

Do not paste this command's output into an issue or chat. It may contain the
token.

Git does not identify the helper that supplied a live credential. Inspect the
affected Git config when `av scan` reports a file and line. Otherwise, review
the configured helpers. `av doctor gh` verifies whether `gh` resolves to the
signed Isotope:

```sh
git config --global --get-all credential.helper
git config --global --get-all credential.https://github.com.helper
av doctor gh
```

## Remove GitHub HTTPS Credential Access

Reject GitHub's cached credential:

```sh
printf 'protocol=https\nhost=github.com\n\n' | git credential reject
```

Remove a GitHub CLI helper from global config:

```sh
git config --global --unset-all credential.https://github.com.helper
```

If Git still returns a password, open the affected config:

```sh
git config --global --edit
```

Delete untrusted helper lines that provide GitHub HTTPS credentials, including:

```gitconfig
[credential "https://github.com"]
  helper = !gh auth git-credential
```

Keychain Access may also contain a Git or GitHub Internet password. Search for
`github.com`, remove the item used by Git, and run the credential-fill check
again.

## Move GitHub Remotes to SSH

Create a passphrase-protected key if needed, then add its passphrase to the
macOS Keychain:

```sh
ssh-keygen -t ed25519 -C "$(git config --global user.email)"
ssh-add --apple-use-keychain ~/.ssh/id_ed25519
```

Configure the Apple SSH agent:

```sshconfig
Host github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519
  AddKeysToAgent yes
  UseKeychain yes
```

Set `~/.ssh/config` to mode `0600`:

```sh
chmod 600 ~/.ssh/config
```

Add `~/.ssh/id_ed25519.pub` to your GitHub account, then convert the current
checkout:

```sh
git remote set-url origin "$(git remote get-url origin |
  sed -E 's#https://github.com/#git@github.com:#')"
```

## Verify

```sh
ssh -T git@github.com
git fetch
git push --dry-run
printf 'protocol=https\nhost=github.com\n\n' |
  GIT_TERMINAL_PROMPT=0 GCM_INTERACTIVE=never git credential fill
av scan
```

The last credential-fill command must not print `password=`.

## Caveats

This detector queries `github.com`. A live finding may have no affected file
because Git does not report which helper returned the credential. SSH agent
access remains available to processes in your login session after you unlock
the key, but Git no longer exposes a reusable HTTPS token through its helper
protocol.

The signed-Isotope exception fails closed. Relative helper commands, a missing
reset, other effective helpers, config includes, an invalid signature, or any
other uncertainty preserve the live probe or Finding.
