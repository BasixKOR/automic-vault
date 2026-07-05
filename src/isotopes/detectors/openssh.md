# OpenSSH Radioisotope Detector

This detector reports unencrypted SSH private keys in `~/.ssh` and explicit
`IdentityFile` paths from `~/.ssh/config`.

It does not currently migrate keys, wrap `ssh`, or manage ssh-agent state.

## Fix macOS Keychain Passphrase Support

Encrypt the private key with a passphrase, then let Apple's OpenSSH build store
that passphrase in the macOS Keychain. The key file still stays on disk, but it
is no longer a reusable plaintext credential that local tools can read directly.

First add a passphrase to any unencrypted private key that the detector reports:

```sh
$ ssh-keygen -p -f ~/.ssh/id_ed25519
```

Then enable Keychain-backed passphrase lookup in `~/.ssh/config`:

```sshconfig
Host *
  AddKeysToAgent yes
  UseKeychain yes
```

Finally, add the encrypted key to the agent and save its passphrase in the
Keychain:

```sh
$ ssh-add --apple-use-keychain ~/.ssh/id_ed25519
```

Repeat those commands for each private key path that Automic Vault reports. On
older macOS releases you may see examples using `ssh-add -K`; prefer
`--apple-use-keychain` because `-K` is now also an upstream OpenSSH option.
