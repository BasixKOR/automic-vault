## What it Does

`av harden supabase` installs the signed Supabase Isotope from the Automic Vault
tap when Homebrew is available. Without Homebrew it installs `supabase` and
`supabase-go` in `/usr/local/bin`; `av doctor supabase` reports direct install
updates. It then moves legacy Supabase CLI access tokens into Automic Vault and
removes the plaintext fallback token files used by older Supabase CLI releases.
