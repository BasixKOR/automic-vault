# Securing Git on macOS

Use SSH transport with a passphrase-protected key stored in the macOS Keychain.

> [!IMPORTANT]
> If Git can fetch or push over HTTPS without prompting a human, some credential
> helper can probably return a plaintext token to `git credential fill`.
> That is convenient. It is also ambient authority.

`av scan` reports Git configurations that expose credentials to ordinary
same-user processes, including agent subprocesses.

Detected hazards:

- `~/.git-credentials`
- global `credential.helper = store --file ...` paths
- `printf 'protocol=https\nhost=github.com\n\n' | git credential fill`
  returning a non-empty `password=` for `github.com`
- Git config that delegates GitHub credentials to `gh auth git-credential`
- Git config that enables `git-credential-oauth`
- plaintext `oauthClientSecret` values in Git config

The live `git credential fill` finding may not have a file and line. Git does
not always say which helper returned the token. File-backed findings do include
the affected path and line.


The fix is not "use a better HTTPS credential helper". On macOS, if Git can ask
a helper for an HTTPS token non-interactively, an agent command can ask too.

Use SSH.

## Check What Git Can Read

Start with the boring check:

```sh
$ av scan
Automic Vault scan
Findings:
1. high isotope:git - Git credential store contains plaintext credentials
   /Users/you/.git-credentials:1
   Read more: https://github.com/automic-vault/automic-vault/main/docs/securing-git.md
```

Then check what Git itself can retrieve for GitHub:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential fill
protocol=https
host=github.com
username=x-access-token
password=github_pat_...
# ^^ bad: Git can retrieve a token without you approving this command
```

Clean output is no `password=` line:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential fill
protocol=https
host=github.com
username=
# ^^ fine; exact output varies, but there should be no password
```

Also check for plaintext credential-store files:

```sh
$ test -f ~/.git-credentials && sed -n '1,3p' ~/.git-credentials
https://user:token@example.com/repo.git
# ^^ bad: plaintext token on disk
```

Do not paste the output anywhere. It may be the secret.

## The Safe Target State

The target state on macOS:

- repository remotes use SSH URLs, eg. `git@github.com:user/repo.git`
- your SSH private key has a real passphrase
- macOS stores that passphrase in the Keychain after you enter it once
- Git does not have a useful HTTPS token available through `git credential fill`
- no `credential.helper = store` plaintext files contain tokens

This is the practical boundary:

- the SSH private key file is encrypted at rest by its passphrase
- the passphrase is mediated by the macOS Keychain
- Git does not need an HTTPS token
- agent shell commands cannot ask Git for an HTTPS token and get one back

No magic. Just fewer plaintext secrets lying around.

## Create A Passphrase-Protected SSH Key

If you already have a passphrase-protected SSH key, skip to adding it to the
Keychain.

```sh
$ ssh-keygen -t ed25519 -C "$(git config --global user.email)"
Generating public/private ed25519 key pair.
Enter file in which to save the key (/Users/you/.ssh/id_ed25519):
Enter passphrase (empty for no passphrase):
Enter same passphrase again:
```

Do not use an empty passphrase.

> [!NOTE]
> An unencrypted SSH private key is just another plaintext secret. Different
> file, same problem.

Lock the file permissions down:

```sh
$ chmod 700 ~/.ssh
$ chmod 600 ~/.ssh/id_ed25519
$ chmod 644 ~/.ssh/id_ed25519.pub
```

## Store The SSH Passphrase In The macOS Keychain

Add the key to the Apple SSH agent and store the passphrase in Keychain:

```sh
$ ssh-add --apple-use-keychain ~/.ssh/id_ed25519
Enter passphrase for /Users/you/.ssh/id_ed25519:
Identity added: /Users/you/.ssh/id_ed25519
```

Teach SSH to use the Keychain:

```sh
$ mkdir -p ~/.ssh
$ chmod 700 ~/.ssh
$ cat >> ~/.ssh/config <<'EOF'
Host github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519
  AddKeysToAgent yes
  UseKeychain yes
EOF
$ chmod 600 ~/.ssh/config
```

For GitLab:

```sshconfig
Host gitlab.com
  HostName gitlab.com
  User git
  IdentityFile ~/.ssh/id_ed25519
  AddKeysToAgent yes
  UseKeychain yes
```

## Add The Public Key To Your Git Host

Copy the public key:

```sh
$ pbcopy < ~/.ssh/id_ed25519.pub
```

Add it to your Git host:

- GitHub: Settings -> SSH and GPG keys -> New SSH key
- GitLab: Preferences -> SSH Keys
- Bitbucket: Personal settings -> SSH keys

Then test it:

```sh
$ ssh -T git@github.com
Hi USERNAME! You've successfully authenticated, but GitHub does not provide shell access.
```

For GitLab:

```sh
$ ssh -T git@gitlab.com
Welcome to GitLab, @USERNAME!
```

The first connection may ask you to trust the host key. That is normal. The
passphrase prompt should happen once, then Keychain should handle future use.

## Convert Existing Checkouts To SSH

Check the current remote:

```sh
$ git remote -v
origin  https://github.com/user/repo.git (fetch)
origin  https://github.com/user/repo.git (push)
```

Switch GitHub HTTPS remotes to SSH:

```sh
$ git remote set-url origin "$(git remote get-url origin | sed -E 's#https://github.com/#git@github.com:#')"
```

Check it:

```sh
$ git remote -v
origin  git@github.com:user/repo.git (fetch)
origin  git@github.com:user/repo.git (push)
```

For GitLab:

```sh
$ git remote set-url origin "$(git remote get-url origin | sed -E 's#https://gitlab.com/#git@gitlab.com:#')"
```

For one-off remotes, set the URL explicitly:

```sh
$ git remote set-url origin git@github.com:user/repo.git
```

Now test normal Git:

```sh
$ git fetch
$ git push --dry-run
```

## Remove HTTPS Token Exposure

Once SSH works, remove the HTTPS credentials. This will probably break HTTPS
pushes and pulls. Good. That is the point.

Reject the GitHub credential from Git's helper chain:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential reject
```

Verify it is gone:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential fill
```

There should be no `password=` line.

If it still returns a password, open Keychain Access:

1. Search for `github.com`.
2. Delete Git or GitHub Internet password items used by Git.
3. Run the `git credential fill` check again.

Remove plaintext credential-store files if they exist:

```sh
$ rm -i ~/.git-credentials
```

Check for configured plaintext stores:

```sh
$ git config --global --get-all credential.helper
store --file ~/.custom-git-credentials
```

If you see `store`, remove it:

```sh
$ git config --global --unset-all credential.helper
```

If you need to remove one specific helper and keep another, edit the config:

```sh
$ git config --global --edit
```

Delete lines like:

```gitconfig
[credential]
  helper = store
  helper = store --file ~/.custom-git-credentials
```

Then delete the custom store file after confirming it contains only Git
credentials you no longer need:

```sh
$ rm -i ~/.custom-git-credentials
```

## Remove `gh auth git-credential` Exposure

The GitHub CLI can act as a Git credential helper:

```gitconfig
[credential "https://github.com"]
  helper = !gh auth git-credential
```

That lets Git ask `gh` for a token. It also lets any same-user command ask Git
for the same token.

If your remotes use SSH, remove the helper:

```sh
$ git config --global --unset-all credential.https://github.com.helper
```

If that does not remove it, edit the file:

```sh
$ git config --global --edit
```

Delete the `helper = !gh auth git-credential` line.

Then verify:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential fill
```

Again: no `password=`.

## Remove `git-credential-oauth` Exposure

`git-credential-oauth` may appear as:

```gitconfig
[credential]
  helper = oauth -device
  oauthClientSecret = ...
```

Remove it if you want the SSH-only state:

```sh
$ git config --global --unset-all credential.helper
$ git config --global --unset-all credential.oauthClientSecret
```

If the config is more complex:

```sh
$ git config --global --edit
```

Delete the OAuth helper and any plaintext `oauthClientSecret`.

## Verify Everything

Run the scanner:

```sh
$ av scan
Automic Vault scan
No problems found.
```

Check GitHub HTTPS credential fill:

```sh
$ printf 'protocol=https\nhost=github.com\n\n' | git credential fill
```

There should be no `password=`.

Check remotes:

```sh
$ git remote -v
origin  git@github.com:user/repo.git (fetch)
origin  git@github.com:user/repo.git (push)
```

Check SSH authentication:

```sh
$ ssh -T git@github.com
Hi USERNAME! You've successfully authenticated, but GitHub does not provide shell access.
```

Check real Git operations:

```sh
$ git fetch
$ git push --dry-run
```

Check that no plaintext store helper remains globally:

```sh
$ git config --global --get-all credential.helper
```

No output is ideal for SSH-only Git.

## Caveats

SSH with a Keychain-stored passphrase does not mean "no secrets exist". It means
Git no longer needs an HTTPS token that can be returned as plaintext by
`git credential fill`.

After the SSH key is unlocked, your login session can use it through the SSH
agent. That is still ambient authority. It is just a better macOS boundary than
leaving GitHub tokens in files or helper APIs.

If you use multiple Git hosts, repeat the SSH setup and credential cleanup per
host.

If your company requires HTTPS Git credentials, you cannot reach the SSH-only
state. Your best option is to limit token scope and lifetime, then keep agent
work away from shells that can call `git credential fill`.

For the rest:

- [GitHub: Connecting to GitHub with SSH][github-ssh]

[github-ssh]: https://docs.github.com/en/authentication/connecting-to-github-with-ssh
