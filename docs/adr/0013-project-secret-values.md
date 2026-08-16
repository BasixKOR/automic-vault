# ADR 0013: Select Secret Values by Project Directory

- Status: Accepted
- Date: 2026-08-14

## Context

Credentials such as `OPENAI_API_KEY` and `DOTENV_PRIVATE_KEY` commonly differ
between projects, while Tools require their established environment variable
names. Storing those credentials under invented Secret Names is incompatible
with transparent injection and obscures the capability the user is granting.

A working directory is controlled by the Launcher. It cannot safely distinguish
authorized from unauthorized code running as the same user. Treating it as an
authority boundary would weaken the product's name-based policy model.

## Decision

A Secret is one name-based capability with one optional Global Value and zero
or more Project Values. Authorization policy, Direct Access Rules, Blessings,
availability, and Secret mutation operate on the Secret Name and cover all its
Values.

For each requested Secret Name, the menu bar app selects the nearest Project
Value at or above the request's physical canonical working directory. Traversal
stops before crossing a filesystem boundary. If no Project Value matches, the
Global Value is selected. If neither exists, the Secret is missing. A failure to
read the selected Value denies the request without fallback.

Selection models only Git's fundamental physical parent traversal. It does not
invoke Git or inspect `.git`, Git environment variables, configuration, worktree
metadata, ownership, or the user's home directory.

Project Directories are canonical absolute path strings, not filesystem
identities. They must exist and be directories when a Value is created, and a
filesystem or mount root cannot be used. Moving a directory does not move its
Value; recreating the same path makes the Value selectable again.

The app resolves selected Values once before authorization, includes their
sources in the immutable Authorization Request and Authorization Record, and
loads those exact Keychain items after authorization succeeds. The working
directory and Project Directory are displayed as context, never as proof of
authority.

Availability and rename are Secret-level mutations that update all Values.
Multi-item rename, availability, and whole-Secret deletion use a persisted
forward-repair journal. Interrupted operations resume in the forward direction,
and affected names deny Secret Use until repair completes. Deleting one Value
retains Direct Access Rules while another Value remains; deleting the last
Value revokes them.

## Consequences

- Existing name-based grants intentionally cover later Project Values of the
  same Secret.
- A Launcher authorized for a Secret Name may select among its Values by
  choosing a working directory.
- Nested Project Values inherit other Secret Names independently from ancestors
  or their Global Values.
- Directory deletion makes a Project Value inactive without deleting it, and
  path reuse reactivates it.
- No Git dependency, repository registry, or filesystem-identity persistence is
  introduced.
