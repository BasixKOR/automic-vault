# OpenVPN Radioisotope

Detect-only coverage for user-level OpenVPN client secrets.

OpenVPN profiles can contain private keys or reference plaintext
`auth-user-pass` files. This radioisotope reports those local files without
changing VPN profile semantics.
