# `av dotenv`

Encrypted dotenv files with approval-gated secret injection.

> [!NOTE]
> This is a README-style draft for the proposed `av dotenv` feature. The full
> design lives in [`dotenv-feature-spec.md`](./dotenv-feature-spec.md).

`av dotenv` is for projects that already use `.env` files and do not want to
turn every local process, editor plugin, test runner, and agent session into a
secret reader.

It keeps the migration boring:

```sh
$ npm run dev
# reads plaintext .env today

$ av dotenv ingest .
# rewrites .env files with dotenvx-compatible encrypted values
# stores the private decryption key in Keychain

$ av dotenv run -- npm run dev
# decrypts in memory, asks for approval when needed, then runs your command
```

No shell-wide magic. No ecosystem guessing. No ambient authority cosplay.

## The Common Path

Start with the `.env` files you already have:

```sh
$ av dotenv ingest .
found .env
encrypted OPENAI_API_KEY
encrypted DATABASE_URL
stored project decryption key in Keychain
initialized dotenv execution policy
```

The resulting `.env` can be committed:

```dotenv
# AUTOMIC VAULT MANAGED ENVIRONMENT
#
# Secrets are encrypted using dotenvx-compatible encryption.

OPENAI_API_KEY="encrypted:ZXlKaGJHY2lPa..."
DATABASE_URL="encrypted:ZXlKaGJHY2lPa..."
```

Run the project through `av dotenv run`:

```sh
$ av dotenv run -- npm run dev
Automic Vault wants to inject 2 dotenv secrets into npm
approved
> app@dev
> vite
```

The child process gets the environment it expected. The shell does not.

## Trust Executables Explicitly

`av dotenv` does not decide that `node`, `python`, `cargo`, or `docker` are
safe because they look familiar. You approve the tools that may receive
project secrets:

```sh
$ av dotenv allow node
allowed node for this project

$ av dotenv allow /opt/homebrew/bin/python
allowed /opt/homebrew/bin/python for this project
```

Approvals are scoped to execution policy, not vibes. The proposed modes are:

- `ONCE`: allow this run
- `ALWAYS`: allow future matching runs
- `IF UNCHANGED`: allow future runs while the executable, arguments, working
  directory, entrypoint, and requested secret set still match

You can take trust back:

```sh
$ av dotenv revoke node
revoked node for this project
```

## Why This Exists

Plaintext dotenv is convenient because everything can read it.

That is also the bug.

```txt
.env on disk
  -> every process with file access
  -> every child process with inherited environment
  -> every agent/tooling layer that can inspect the workspace
```

`av dotenv` changes the shape:

```txt
encrypted .env in git
  -> private key in Keychain
  -> explicit command through av dotenv run
  -> approved executable
  -> plaintext only in process memory
```

It is not perfect isolation. It is a better default for the workflow developers
already use.

## Compatibility

The encrypted values use the `dotenvx` format by design. That keeps the files
portable across development, CI, and production systems that already understand
dotenvx-style encryption.

Initial runtime support is intentionally explicit:

- `node`: `npm`, `pnpm`, `yarn`, `tsx`, `vite`, `next`
- `python`: `python`, `uv`, `poetry`
- `cargo`
- `go run`
- `ruby`: `bundler`, `rails`
- `php`: `composer`
- `java`: `gradle`, `maven`
- `docker`: compose and local container execution

> [!IMPORTANT]
> Unsupported tools should fail closed. If a runtime needs a shim, wrapper, or
> special environment handling, Automic Vault should know that before secrets
> are injected.

## Inspect the Project Policy

Use `info` when you need to see what Automic Vault thinks is managed:

```sh
$ av dotenv info
managed files:
  .env

approved executables:
  node
  /opt/homebrew/bin/python

policy:
  mode: if-unchanged
  last access: 2026-05-23T14:12:08Z
```

For the full command surface and security invariants:

```sh
$ av dotenv --help
```

Until the feature ships, read the design spec instead.

## What This Does Not Try To Do

`av dotenv` is not a production secret manager. Use your production secret
manager in production.

It also does not promise to prevent every exfiltration path. Once a trusted
program receives a secret, that program can misuse it. Automic Vault can reduce
casual leakage, require human approval, monitor stdout/stderr, and make access
visible. It cannot make arbitrary code morally upright.

> [!WARNING]
> The invariant is stricter on disk than at runtime: no plaintext dotenv files,
> no plaintext temp files, no decrypted cache. Plaintext exists only in memory
> during approved execution.

## Temporary Reveals

Some workflows need a single value briefly:

```sh
$ av inject +OPENAI_API_KEY
Automic Vault wants to reveal OPENAI_API_KEY
approved
```

This should require human approval unless the request is already trusted.

Use it sparingly. If a command needs the project environment, prefer:

```sh
$ av dotenv run -- command-that-needs-env
```

## Implementation Notes

Runtime interception may use installed shims, wrappers, or a dynamically
modified `PATH` during `av dotenv run`.

Whatever mechanism wins, it must be reversible, predictable, and compatible
with developer tooling. If that sounds less glamorous than transparent magic,
good.

See [`dotenv-feature-spec.md`](./dotenv-feature-spec.md) for the complete draft:
encryption model, approval identity, leakage detection, supported runtimes, and
future capability-tracked secret access.
