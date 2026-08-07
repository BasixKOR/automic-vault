# Direct Secret Access

Direct Secret Access lets one Verified Launcher use one exact Secret Name in
future direct `av inject` requests without asking for Approval each time.

This is intentionally not the preferred way to use Automic Vault. The Launcher
may select any Target and arguments, and the Target receives the Secret. Code
signing establishes the Launcher's identity and integrity; it does not prove the
Launcher's intent or make the selected Target trustworthy.

## Safer alternatives

Choose the narrowest option that works:

1. **Harden the Tool.** A Tool-specific Secret Gate recognizes its Target and
   operations, allowing policy to distinguish read-only work, writes, elevated
   credentials, and Secret Disclosure.
2. **Bless an exact script.** A Blessing binds the script’s canonical path,
   contents, declared Secret Names, Target, injection options, and Gate
   capabilities. A Launcher Endorsement can then authorize that reviewed script.
3. **Approve each request.** Approval binds one complete request and live process
   and creates no durable delegation.

Direct Secret Access is appropriate only when commands must be selected
dynamically, no suitable Hardener exists, and an exact Blessed Script is too
restrictive.

## What a rule permits

A Direct Access Rule binds:

- one exact Secret Name;
- one designated requirement for a Verified Launcher; and
- direct `av inject` Secret Application.

It does not permit listing Secret Names, reading a raw Secret from Automic Vault,
changing or deleting Secrets, using sibling Launchers, or bypassing another Gate
Client’s policy. A request for several Secrets is automically authorized only
when the same Verified Launcher has a rule for every requested Secret Name.

The live Launcher must continue to pass code-signature, identity, Hardened
Runtime, and entitlement checks. A path, filename, icon, process identifier, or
bundle display name is not an identity.

## Adding and removing access

Select a Secret in the Automic Vault app and use **Allow Launcher** under Direct
Secret Access. The app requires a fresh acknowledgement every time, verifies the
selected signed app or executable, and shows the identity before saving it.

Remove a Launcher from the same Secret to revoke the rule. Renaming or deleting
the Secret also revokes every Direct Access Rule for that Secret Name.

All allowed uses still require a persisted Authorization Record before Automic
Vault releases the Secret.
