# Codex Automations

Use native Codex/ChatGPT scheduled automations for the publishing cadence.
Do not use `launchd` for this repo-owned cadence.

## Hourly Database Publish

Schedule: every hour at minute 0.

Prompt:

```text
In /Users/mxcl/src/automic-vault, run the hourly Automic Vault database publish.

Use this command:
scripts/automation-runner.sh db

After it finishes, inspect cache/automation/db.status.json and the tail of cache/automation/db.log. If it failed or timed out, diagnose the failure, make the smallest safe fix in the repo, run the relevant tests, commit at a sensible interval, and then rerun scripts/automation-runner.sh db once. Preserve public /db.json schema compatibility: do not bump scripts/build-db.py SCHEMA_VERSION or src/lib/rs/lib.rs DB_SCHEMA_VERSION for additive fields.
```

## Daily Package-Origin Publish

Schedule: daily at 03:15 local time.

Prompt:

```text
In /Users/mxcl/src/automic-vault, run the daily Automic Vault package-origin publish.

Use this command:
scripts/automation-runner.sh pkg-origin

After it finishes, inspect cache/automation/pkg-origin.status.json and the tail of cache/automation/pkg-origin.log. If it failed or timed out, diagnose the failure, make the smallest safe fix in the repo, run the relevant tests, commit at a sensible interval, and then rerun scripts/automation-runner.sh pkg-origin once. Remember that package catalog routes are served by av-web from cache/pkg.sqlite locally and /var/lib/automic-vault-web/pkg.sqlite on Atlas; keep public /db.json backward-compatible.
```

## Health Check

Schedule: every day at 08:00 local time.

Prompt:

```text
In /Users/mxcl/src/automic-vault, check the Automic Vault Codex automation status.

Use this command:
scripts/codex-automation-status.sh

If either job is failed, timed out, stale, or currently running far beyond its expected cadence, inspect the relevant log under cache/automation/, diagnose the issue, make the smallest safe fix in the repo, run the relevant tests, commit at a sensible interval, and rerun only the affected scripts/automation-runner.sh job once.
```
