# ADR 0011: Allow Path Execution for Snapshot-Incompatible Interpreters

- Status: Accepted
- Date: 2026-08-11

## Context

ADR 0005 requires Blessed Scripts to execute from a verified snapshot so file
edits cannot race authorization. Some interpreters, including `uv run --script`,
cannot execute the `/dev/fd/N` snapshot. Sending the script through standard
input would consume input that belongs to the script. Treating the invocation as
a direct injection discards the user’s Blessing and its narrower authority.

## Decision

Automic Vault may execute a Blessed Script from its canonical path when its
interpreter appears on the snapshot-incompatible list. The blessing review
warns that another same-user process can change the script after verification
and before the interpreter opens it. The user overrides the snapshot requirement
by choosing **Bless Anyway**. Automic Vault verifies the exact contents against
the Blessing for every authorization request and prints the same warning before
every execution. The Blessing records this override. Existing Blessings do not
gain it during an upgrade and must be reviewed again.

Automic Vault preserves standard input and does not transform the interpreter
command. Interpreters not on the compatibility list continue to receive the
verified `/dev/fd/N` snapshot.

## Consequences

- The Blessing still binds the canonical path, exact contents, Script
  Declaration, capabilities, and Launcher Endorsements.
- Path execution has a time-of-check-to-time-of-use window that the verified
  snapshot normally removes.
- Adding an interpreter to the compatibility list requires a reproduction that
  demonstrates snapshot execution cannot work and tests for both warnings.
