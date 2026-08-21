# Secret Proxy

Secret Proxy lets a Target use named Secrets in HTTP/S requests without putting
the raw values in its environment or process memory through Automic
Vault.

```sh
av proxy +GITHUB_TOKEN +API_KEY -- node app.js
```

The Target receives a random Secret Reference for each name, plus standard
proxy and per-session CA environment variables. When a reference appears in an
HTTP/S URL path or query, header, or bounded body, Automic Vault asks whether to
apply the corresponding Secret to that exact destination. The choices are
Deny, Allow Once, and Allow for Session. Session approval binds the exact
canonical origin and exact Secret Names. Automic Vault never persists that
approval.

Every session start requires Approval. Secret Proxy has no Launcher rule,
durable destination rule, or project-directory policy for Secret Proxy.

## Security boundary

The proxy is a separately signed, sandboxed, Hardened Runtime helper without
Keychain authority. The Keychain-owning app records an allowed use before it
returns only the Secret values required for that request. Automic Vault never
gives the launched Target raw Secret values.

Secret References and the Proxy Credential are bearer values. Code that can
inspect an unhardened Target may steal them and reuse an origin already allowed
for the session. It still cannot ask the proxy to send the Secret to a different
origin without another Approval. The approval window reports weak Target
runtime protection; it does not block common interpreters such as Node.

The helper rejects private and reserved destinations, ambiguous DNS, invalid
upstream TLS, protocol upgrades, uninspectable content encodings, bodies over
10 MiB, and transactions exceeding 30 seconds. It never installs a CA in the
system trust store. Authorization History omits query values.

## Compatibility

The Target must use the standard proxy variables and accept one of the scoped
CA variables. Automic Vault supplies upper- and lower-case `HTTP_PROXY`,
`HTTPS_PROXY`, and `ALL_PROXY`, clears `NO_PROXY`, and supplies:

- `SSL_CERT_FILE`
- `NODE_EXTRA_CA_CERTS`
- `REQUESTS_CA_BUNDLE`
- `CURL_CA_BUNDLE`
- `GIT_SSL_CAINFO`
- `AWS_CA_BUNDLE`

`av proxy` refuses an existing proxy or CA environment unless
`--replace-existing-env` is supplied. Software that ignores proxy variables,
pins certificates, uses Security.framework trust, or uses a custom
network stack may fail or bypass the proxy. A bypass sends only inert Secret
References, not raw Secrets.

Credential transformations are not supported. If a Target hashes, signs,
encrypts, chunks, or otherwise transforms a Secret Reference before the proxy
sees it, Automic Vault releases no Secret and the upstream request fails. This
includes schemes such as AWS SigV4 unless the reference itself remains visible
in the request.

The helper inspects responses and replaces direct echoes of a Secret used for
that request with its Secret Reference. This is defense in depth, not a general
output-redaction guarantee; the helper cannot recognize transformed output.

Active sessions and their statistics appear under **Active Proxies**. Ending a
session terminates only the proxy helper, not the Target. Its records remain in
Authorization History, which has one global 50-entry cap.

See [Canonical Domain Language](domain-language.md),
[Architecture](architecture.md), and
[ADR 0017](adr/0017-process-bound-secret-proxy.md) for the authoritative model.
