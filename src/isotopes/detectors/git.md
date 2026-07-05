# Git Radioisotope Detector

This detector reports plaintext Git credential-store files and GitHub tokens
that Git credential helpers can return non-interactively. It does not currently
migrate credentials or modify Git.

Detected hazards:

- `~/.git-credentials`
- global `credential.helper = store --file ...` paths
- `printf 'protocol=https\nhost=github.com\n\n' | git credential fill`
  returning a non-empty `password=` for github.com

## How to fix GitHub Keychain exposure

Delete the exposed GitHub entry from the Keychain-backed Git credential helper:

```sh
printf 'protocol=https\nhost=github.com\n\n' | git credential reject
```

If that does not remove it, open Keychain Access, search for `github.com`, and
delete the Git or GitHub Internet password item.

Verify the exposure is gone:

```sh
printf 'protocol=https\nhost=github.com\n\n' | git credential fill
```

The command should not return a `password=` line.

> [!IMPORTANT]
>
> This will almost certainly break pushing and pulling with git to GitHub!
> The safe way to authenticate your Git with GitHub is via passphrase protected
> SSH.
>
> See [GitHub's documentation on SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh) for more information.
>
> On macOS the passphrase is stored in the keychain after you enter it once.
> Thus your ssh keyfile is safe and your passphrase has no exposure.
>
> Once you have switched to SSH you can switch your checkout’s remote to SSH
> with `git remote set-url origin git@github.com:user/repo.git`. Here’s a
> one-liner:
>
> ```sh
> git remote set-url origin "$(git remote get-url origin | sed -E 's#https://github.com/#git@github.com:#')"
> ```
