# ADR 0058: Complete CLI Authorization History Windows

`av history --since` must return every retained record in the requested window.
The previous 1 MiB result cap was a precaution against oversized XPC replies,
not an authorization boundary or measured platform limit. It made ordinary
history windows unusable and provided no way to retrieve the omitted interval.
This decision supersedes that part of ADR 0047.

The `history-read` operation uses the existing Verified Launcher checks,
Authorization History Access or Approval, display-safe redaction, and verified
record-before-disclosure requirement. It prepares one immutable JSON snapshot,
including the current read, and transfers it in chunks of at most 256 KiB.
The CLI fetches every chunk on the same XPC connection and validates byte offsets
and total length before decoding or displaying anything. A record larger than a
chunk is split across chunks without truncation. JSON output remains one array.

`history-next` can only advance through that connection's already-authorized
snapshot. It cannot select records, widen the window, start another read, or
reuse another connection's authorization. Normal live Gate Client and Launcher
Bundle integrity checks still run. Completion and connection cancellation release
the snapshot. New writes and retention pruning cannot alter an in-flight snapshot.
Older clients retain their original bounded protocol; new clients require an
updated helper and never fall back to a partial result.

The snapshot and CLI result are held in memory, proportional to the retained
window. This deliberately reuses the existing full-window read and avoids a
long-lived database transaction, plaintext temporary file, or repeated store
scans. If measured memory pressure warrants it, replace snapshot materialization
with an authenticated streaming design while preserving this contract.

Human-readable terminal output uses `/usr/bin/less` with secure mode, a cleared
environment except for explicit pager settings and TERM, and no shell dispatch.
Pager hooks, shell escapes, files, and command history are disabled. `--no-pager`,
JSON, and nonterminal destinations bypass it. Failure to start the pager falls
back to direct output; choosing to quit the pager early is successful.
