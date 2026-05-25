# `av dotenv` Implementation

Ship the first usable runtime-secret slice: Rust CLI, local daemon, macOS
approval UI, and the Node SDK package `@automic/av`.

> [!IMPORTANT]
> This is the build plan, not the full feature spec. Build Node first. Python,
> Rust, Go, and Ruby SDKs can wait their turn like everyone else.

The promise:

```sh
$ av dotenv ingest .
found .env
encrypted OPENAI_API_KEY
encrypted DATABASE_URL
stored project decryption key in Keychain
wrote .config/automic-vault.json
# ^^ committed metadata, encrypted dotenv, private key outside the repo

$ av dotenv serve
listening on ~/Library/Application Support/Automic Vault/dotenv.sock
```

Application code stops reading ambient environment variables:

```js
import { secret } from "@automic/av"

const apiKey = await secret("OPENAI_API_KEY")
```

Local development asks the daemon. Production, CI, and tests read `getenv`.
Unexpected callsites warn; they do not break deploys.

## Build This Slice

Add `av dotenv` with four commands:

```sh
$ av dotenv ingest [path] [--json]
$ av dotenv info [--json]
$ av dotenv revoke [SECRET]
$ av dotenv serve [--socket <path>]
```

`ingest` discovers `.env` and `.env.*` files, excluding `.env.example`,
`.env.sample`, `.env.template`, `.env.keys`, `.git`, and dependency
directories. It encrypts every nonempty value that does not already start with
`encrypted:`, prepends `DOTENV_PUBLIC_KEY`, and stores the private key in
Keychain.

`info` prints managed files, known secret names, project hash, and expected
callsite count. `revoke` removes approve-always baselines for one secret or all
secrets. It does not delete keys or rewrite dotenv files. `serve` runs the
local Unix socket daemon.

> [!NOTE]
> The v1 daemon lifecycle is manual. Users run `av dotenv serve`. App-managed
> startup is a later feature, not a reason to make this slice larger.

## Write The Right State

Use native Rust code for dotenvx-compatible encryption:

- parse dotenv files with `dotenvy`, or a small wrapper around it, while
  preserving comments, blank lines, and assignments
- encrypt/decrypt with `ecies` using pure Rust AES support
- add direct `base64` and `hex` dependencies if the implementation needs them
- write values as `encrypted:` plus base64 ECIES ciphertext

Keychain:

```txt
service: com.automicvault.dotenv
account: av.dotenv.project.<project_hash>.privatekey
```

Committed metadata lives in `.config/automic-vault.json`:

```json
{
  "schema": 1,
  "project_hash": "...",
  "managed_files": [".env"],
  "public_key": "...",
  "known_secrets": ["OPENAI_API_KEY", "DATABASE_URL"],
  "expected_callsites": []
}
```

No plaintext values. Secret names and callsite fingerprints are acceptable in
committed metadata. Plaintext is not.

> [!WARNING]
> Never write plaintext secrets to disk: no decrypted temp files, no plaintext
> caches, no logs, no shell history. Secrets are plaintext only in memory and
> only after an approved request.

## Ask Before New Access

Reuse the file-backed approval shape from `av inject` and `av gate`.

Pending request:

```txt
~/Library/Application Support/Automic Vault/dotenv/pending-approval.json
```

Decisions:

```txt
~/Library/Application Support/Automic Vault/dotenv/decisions/
```

Notification:

```txt
com.automicvault.dotenv-approval.pending-changed
```

The macOS app gets `DotenvApprovalStore` and a compact approval view showing:

- secret name
- project identity
- runtime and executable
- normalized backtrace
- approve once, approve always, deny

Approve-once writes only the decision file for the waiting request.
Approve-always appends the normalized callsite baseline to project metadata.

## Speak One JSON Line

The Node SDK sends one JSON line to the daemon:

```json
{
  "type": "secret_request",
  "id": "uuid",
  "secret": "OPENAI_API_KEY",
  "cwd": "/repo",
  "runtime": "node",
  "pid": 12345,
  "mode": "development",
  "backtrace": ["src/lib/secrets.ts:12", "src/routes/chat.ts:48"]
}
```

The daemon resolves metadata from `cwd`, verifies the secret exists, normalizes
the backtrace, checks the baseline, prompts if needed, decrypts in memory, and
returns:

```json
{"type":"secret_response","id":"uuid","value":"..."}
```

Errors are also JSON lines:

```json
{"type":"error","id":"uuid","code":"missing_secret","message":"..."}
```

## Ship `@automic/av`

Create `sdk/node` as package `@automic/av`.

The public API is deliberately small:

```ts
export async function secret(name: string): Promise<string>
```

Mode precedence:

1. `AUTOMIC_VAULT_ENV`
2. `NODE_ENV`
3. `CI`
4. default `development`

In `development`, connect to the daemon. If it is not available, throw a clear
error that tells the user to run:

```sh
$ av dotenv serve
```

In `production`, `ci`, and `testing`, read `process.env[name]`. Capture the
stack, compare it against `.config/automic-vault.json`, and emit a structured
stderr warning for an unknown callsite. Return the value anyway.

Missing values throw `AutomicVaultMissingSecretError`.

No remote reporting transport in v1. Local first. Boring on purpose.

## Test The Contract

Rust:

```sh
$ cargo test dotenv
$ cargo test isotope
```

Cover CLI parsing, dotenv discovery exclusions, rewrite preservation,
Keychain through an in-memory fake, metadata read/write/revoke behavior, a
dotenvx decrypt fixture generated by `@dotenvx/dotenvx`, approval request and
decision flow, and daemon socket success/error cases.

Swift:

```sh
$ cd src/gui
$ swift test
```

Cover Codable compatibility for request/decision structs, approval store paths,
and decision cleanup.

Node:

```sh
$ cd sdk/node
$ npm test
```

Use `node:test` for mode precedence, development daemon requests with captured
backtraces, `process.env` reads outside development, warning-on-unknown
baseline behavior, expected baseline behavior, and the named missing-secret
error.

## Do Not Accidentally Build

- Do not implement transparent support for unmodified dotenv applications.
- Do not build Python, Rust, Go, or Ruby SDKs in this slice.
- Do not replace production secret managers.
- Do not add remote reporting.
- Do not make ciphertext byte-stable; ECIES is randomized.

## References

- [`@dotenvx/dotenvx`](https://www.npmjs.com/package/@dotenvx/dotenvx)
- [`eciesjs`](https://github.com/ecies/js)
- [`ecies` Rust crate](https://crates.io/crates/ecies)
- [`dotenv-feature-spec.md`](./dotenv-feature-spec.md)
- [`dotenv-feature-readme.md`](./dotenv-feature-readme.md)
