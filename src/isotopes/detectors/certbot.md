# Certbot Radioisotope

Detect-only coverage for Certbot key material.

Certbot's ACME account keys and certificate private keys are service
deployment state. This radioisotope reports unencrypted user-level keys without
attempting to move Certbot's renewal-managed files.
