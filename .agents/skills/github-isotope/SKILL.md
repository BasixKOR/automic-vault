---
name: github-isotope
description: Create or maintain an Automic Vault isotope from an upstream GitHub repository fork, including the fork setup, manifest, README notes, detector, build validation, and updater compatibility checks.
metadata:
  short-description: Create Automic Vault GitHub isotopes
---

# GitHub Isotope

Use this skill when turning an upstream GitHub repository into an
`automic-vault` isotope or updating an existing isotope fork.

## Workflow

1. Work in `/Users/mxcl/src/automic-vault/data/isotopes/<repo>`.
2. Fork or verify the fork at `github.com/automic-vault/<repo>`, with `origin`
   set to the Automic fork and `upstream` set to the original repo.
3. Put all isotope commits on `trunk`, and make the fork default branch `trunk`:

   ```bash
   gh api -X PATCH /repos/automic-vault/<repo> -f default_branch=trunk
   git checkout trunk
   git remote set-head origin -a
   ```

   This matters because `scripts/build-isotopes.sh` clones the fork default
   branch, then rebases the current branch onto the upstream default branch
   before building. If the fork default stays as upstream `main`, `master`, or
   `develop`, the updater may build from an upstream-only branch and report a
   missing `automic-vault.yml` or missing `build` field.

4. Add or preserve root-level `automic-vault.yml`. Full GitHub-fork isotopes
   must include at least:

   ```yaml
   name:
     isotope:<package-name>

   replaces:
     brew:<formula-name>

   build: |
     # build commands that create out.tgz or isotopes/<repo>/out.tgz

   justification:
     title:
       Short Risk Title
     detail: |
       Explain the upstream weakness and what this isotope changes.
   ```

   Add `migrate`, `caveats`, or `homebrewFormula` only when needed. Use
   existing manifests such as `data/isotopes/gh-cli/automic-vault.yml` as the
   local reference.

5. Make the source changes narrowly. For keychain fixes, prefer a signed binary
   that creates Keychain items trusted by the isotope executable itself instead
   of allowing `/usr/bin/security` broad read access.
6. Add `detect.rs` when the isotope needs to detect existing unsafe state.
7. Add an "Automic Vault isotope" section at the top of upstream `README.md`
   summarizing what changed and any caveats.
8. Commit the isotope files and source changes to `trunk`, push `trunk`, and
   verify the fork default branch is still `trunk`.

## Validation

Before finishing, run:

```bash
git -C data/isotopes/<repo> show HEAD:automic-vault.yml >/dev/null
gh api /repos/automic-vault/<repo> --jq .default_branch
scripts/build-isotopes.sh --repo <repo> --dry-run
```

If the dry run says `Missing manifest` or `Missing required manifest field
'build'`, first check that the local isotope cache is on `trunk` and that the
fork default branch is `trunk`. Existing updater caches do not automatically
switch branches after the GitHub default branch changes.
