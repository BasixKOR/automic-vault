# Isotope Scope Audit

Automic Vault's narrowed scope is secret exposure detection plus minimally
invasive workarounds. Package replacement and package binary mutation are out
of scope.

Prefer upstream configuration or credential-helper hooks. If a package cannot
be configured directly, create an Automic Vault-owned stub that runs `av inject`
and calls the unmodified package executable.

This audit uses the strict current-state criterion: an isotope or radioisotope
cannot work this way currently if it depends on Automic Vault replacing or
modifying a package, or if it only detects exposure and has no current
workaround.

## Findings

- Total integrations reviewed: 132.
- Fork/replacement isotopes to retire or redesign: 2.
- Radioisotopes that mutate `/opt/...` launchers today: 100.
- Detect-only integrations with no current workaround: 30.

`aws-cli` should be split: keep the `credential_process` config migration, and
remove package launcher patching unless the narrowed scope later explicitly
includes command gating.

## Fork/replacement isotopes

These currently replace packages and need to be retired or redesigned:

- `gh`
- `supabase`

## Detect-only integrations

These currently detect exposure but do not provide a minimally invasive
workaround:

- `atuin`
- `azure-cli`
- `bash`
- `certbot`
- `cloudflare-wrangler`
- `cloudflared`
- `curl`
- `databricks`
- `docker`
- `docker-machine`
- `fastlane`
- `git`
- `httpie`
- `mongodb-atlas-cli`
- `opencode`
- `openssh`
- `openssl@3`
- `openvpn`
- `perl`
- `pianobar`
- `poetry`
- `rsync`
- `ruby`
- `stripe-cli`
- `tailscale`
- `vercel-cli`
- `wget`
- `wget2`
- `yt-dlp`
- `zsh`

## Package launcher mutators

These currently apply the workaround by mutating an installed package launcher
under `/opt/...`. Convert each one to package configuration or an Automic
Vault-owned stub:

- `acli`
- `akamai`
- `algolia`
- `aliyun-cli`
- `ansible`
- `argocd`
- `ast-cli`
- `astra`
- `aws-cli`
- `bitwarden-cli`
- `buf`
- `censys`
- `checkov`
- `circleci`
- `civo`
- `cloudsmith-cli`
- `composer`
- `dcos-cli`
- `doctl`
- `dropbox-uploader`
- `fastly`
- `fauna-shell`
- `firebase-cli`
- `flyctl`
- `gallery-dl`
- `gcli`
- `glab`
- `goat`
- `gotify`
- `gptcommit`
- `grafanactl`
- `graphite`
- `hcloud`
- `helm`
- `heroku`
- `huggingface-cli`
- `imap-backup`
- `jfrog-cli`
- `k6`
- `kubernetes-cli`
- `luarocks`
- `maestro`
- `mariadb`
- `maven`
- `mcp-remote`
- `mercurial`
- `midnight-commander`
- `minio-mc`
- `mkcert`
- `mycli`
- `mysql`
- `mysql-client`
- `mysql@8.0`
- `mysql@8.4`
- `netlify-cli`
- `node`
- `node@18`
- `nuget`
- `oci-cli`
- `openhue-cli`
- `openstackclient`
- `opentofu`
- `ordercli`
- `ossutil`
- `oxide-cli`
- `phylum-cli`
- `plumber`
- `pnpm`
- `podman`
- `pulumi`
- `qwen-code`
- `railway`
- `rclone`
- `runpodctl`
- `rust`
- `s3cmd`
- `sbt`
- `sentry-cli`
- `shodan`
- `skopeo`
- `snowflake-cli`
- `snyk`
- `soracom-cli`
- `sqlcmd`
- `sslmate`
- `talosctl`
- `terraform`
- `terraform-core`
- `todoist-cli`
- `transifex-cli`
- `travis`
- `twine`
- `uaa-cli`
- `uv`
- `vagrant`
- `vault`
- `virustotal-cli`
- `vultr`
- `wakatime-cli`
- `wsk`

## Out of scope

Package integrity verification and injection approval helper details are
intentionally out of scope for this audit.
