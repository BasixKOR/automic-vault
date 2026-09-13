# ADR 0047: Encrypted Rolling Authorization History

Status: accepted

## Context

Authorization History was one Keychain item containing the newest 50 records.
That kept sensitive request metadata inside the Data Protection Keychain, but a
busy developer could lose useful operational context quickly. Increasing the
array bound would make every allowed Secret Use rewrite and verify an
ever-growing Keychain value.

Authorization History remains local operational history. Longer retention must
not imply an append-only audit trail or move readable Secret Names, commands,
working directories, and software identities into an ordinary same-user file.

## Decision

The menu bar app stores Authorization Records as independently authenticated
AES-GCM ciphertext rows in Application Support. A random 256-bit root key is
stored in the app's Data Protection Keychain access group with After First
Unlock availability. HKDF derives separate encryption and retention-bucket
keys. Record timestamps remain inside the ciphertext; keyed hourly retention
buckets permit expiry without disclosing activity times. Each row's opaque ID
and retention bucket are authenticated with its ciphertext.

The store prunes expired records transactionally on each read or write and caps
encrypted record payloads at 25 MiB, deleting the oldest records first. A
dormant database may retain expired ciphertext until its next access. The
database is excluded from backup. SQLite provides synchronous transactions; an
allowed Secret Use succeeds only after
its complete record is committed, read back, authenticated, decoded, and
compared with the expected record.

On first use, the app imports existing Keychain and older UserDefaults history,
verifies every imported record and rechecks both legacy sources before committing
the import, then applies retention and removes the legacy items. A changed
source rolls back the import without deleting it. A database without its
encryption key is unavailable
and never receives a replacement key.

The dashboard continues to show the newest 50 records. `av history` returns the
newest 50 by default; `--since <duration>` may request a narrower time window up
to 30 days. A single reply exceeding 1 MiB fails rather than truncating the
result; a narrower `--since` window can be requested. Filtering occurs inside
the menu bar app before disclosure. CLI
formats receive only display-safe commands as defined by ADR 0046.
An explicit window uses the dedicated `history-window` XPC operation with the
signed menu helper. An older helper rejects that operation rather than silently
ignoring `--since` and returning its default 50-record view.

## Consequences

- Longer history does not make Authorization History tamper-proof, complete, or
  suitable as forensic evidence. Same-user software can still damage or delete
  the encrypted database.
- The database exposes its existence, record count, insertion order, and
  approximate storage volume. Equal bucket values reveal which records share a
  retention hour, but not that hour or record contents without the
  Keychain-held key.
- There is one durable history store after migration and no background writer.
- GUI export remains unnecessary while the attended, authorized CLI can emit
  JSON for an explicit time window.
