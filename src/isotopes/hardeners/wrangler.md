# Wrangler

Uses the `automic-vault/isotopes/wrangler-isotope` formula as a constrained
update manifest, then downloads and verifies the signed Wrangler Isotope before
placing its complete runtime under `/opt/av/wrangler`. It does not install a
Homebrew keg. The protected runtime is required because ordinary Node processes
cannot be Gate Clients for Wrangler Credentials.

Before switching, use upstream Wrangler to log out of every auth profile. Then
run `av harden wrangler` and `wrangler login`. Login stores the complete OAuth
Credential in Automic Vault; every use requires Approval. Only Global Values
are supported initially. Installation does not migrate existing credentials or
resolve existing Detector Findings. Library consumers do not inherit the
Isotope's credential authority.
