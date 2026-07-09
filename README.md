# Automic Vault

The world is wild with plaintext secrets.

The shadows are full of supply chain attacks that feast upon them.

Your workshop shelves are filled with powerful tools that can unbridle kingdoms.

From heaven descended agents, you gave them your tools and they broke prod for
you had no real guardrails.

> You need Automic Vault.

Bringing operating system security primitives and approval gates to the wild
west of developer tooling. From the creator of Homebrew.

---

```sh
$ av scan aws
🚨 plaintext credentials found in: ~/.aws/credentials

$ av harden aws
1. Would import `~/.aws/credentials` into the macOS keychain
2. Would delete plaintext keys from `~/.aws/credentials`
3. Would `brew install aws-vault`
4. Would stub `~/.local/bin/aws` to invoke `aws-vault` with hardened approval
   gates

Proceed? [y/N] y

# …

$ cat ~/.local/bin/aws
#!/bin/bash

set -seo pipefail

if [ "$@" = "s3 sync" ]; then
  av gate `Sensitive action detected`
  # - human in the loop, agents have to get your approval to do aws mutations
  # - to avoid all approval gates for your shell use Vaultty or `av repl`
  #   agents are invited to ask for a temporary token so they can perform
  #   multiple tasks
  # - scripts can get pre-approved for specific actions using YAML frontmatter
fi

# snip…

exec /usr/local/bin/av \
  inject +AWS_ACCESS_KEY_ID +AWS_SECRET_ACCESS_KEY \
  aws-vault exec default -- "$@"
# ^^ the first time av inject will ask you to approve-always for this script SHA
# you will get a warning every time until you also harden `brew` since the
# target aws is not immutably installed
```

- We do the minimum changes
- But: the minimum that is as secure as possible

For example here we insist on aws-vault because it converts your too powerful
AWS keys into short-lived session tokens for every invocation.


```sh
$ av verify aws
- checking ~/.aws/credentials
- file doesn’t exist, good
```
