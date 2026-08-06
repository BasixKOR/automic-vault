This is a security product. Changes must not make anything less secure. Every
change must be thoroughly considered for security implications before enacting.

Dont rush to fix something without first considering if the issue at hand is
even a bug in a security-first-mindset.

## Canonical Domain Language

`docs/domain-language.md` and `docs/architecture.md` are authoritative across
the endorsed Automic Vault ecosystem.

Before changing product language, security concepts, authorization policy, UI
copy, CLI vocabulary, or public documentation, read both files. Use their terms
and security boundaries. Update them before introducing or renaming a domain
concept. Record an architectural decision in `docs/adr/` when the change alters
a security boundary, authority model, or system invariant.

Endorsed properties must adopt and link to these definitions rather than keep a
competing copy. Persisted values, wire fields, and compatibility flags may keep
legacy names when changing them would break compatibility; user-facing language
must use the canonical term.
