# ADR 0032: Route Secret Application with positive command catalogs

Status: accepted

## Context

An environment wrapper that requests its protected Secret for every invocation
unnecessarily exposes that Secret to local inspection commands and arbitrary
scripts. Treating unrecognized invocations as Unknown at the Secret Gate still
creates a Secret Application request and lets one Approval expose the Secret.

## Decision

A command-aware wrapper may positively enumerate the Tool operations that can
legitimately use its protected Secret. Only those invocations request Secret
Application. Every other invocation executes with that Secret removed from its
environment and does not enter an Authorization Gate.

The npm wrapper follows this rule for exact reviewed npm commands and documented
aliases that may authenticate to a registry. Unknown commands, commands added by
a future npm release, npm's dynamic abbreviations, login commands that obtain a
new credential, and arbitrary package scripts run without `NODE_AUTH_TOKEN`.
Users may make an explicit `av inject` request when an unsupported operation
really needs the protected Secret; that request remains subject to the ordinary
Unknown-operation Approval rule.

## Consequences

Future npm commands fail authentication rather than silently gaining a protected
credential. The positive catalog must be reviewed when npm adds or changes
registry-authenticating commands. Unknown Secret Application still cannot be
automically authorized under ADR 0002 because tokenless routing occurs before a
Secret Use request exists.
