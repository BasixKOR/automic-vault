---
name: github-isotope-creator
description: Create Automic Vault isotope forks from upstream GitHub repositories. Use when Codex needs to fork a GitHub repo into github.com/automic-vault, prepare a local data/isotopes clone, add automic-vault.yml packaging metadata, write detect.rs hazard detection, patch upstream code for Automic Vault security behavior, document fork notes in README.md, verify, commit, and push the isotope branch.
---

# GitHub Isotope Creator

Create a source-based Automic Vault isotope from an upstream GitHub repository.
An isotope is a nested fork under `data/isotopes/<name>` that builds a signed,
safer replacement package while preserving upstream shape where practical.

## Boundaries

- User-facing surface: the isotope fork repository, its README fork notes, and
  Automic Vault package metadata.
- Runtime boundary: the upstream tool being rebuilt. Keep Automic Vault-specific
  behavior behind build tags, feature flags, subcommands, or narrow patches.
- Persistence boundary: isotope source lives in the nested git repo under
  `data/isotopes`; Automic Vault detectors are `detect.rs` files compiled by the
  parent repo build script.
- Change shape: prefer additive patches over broad upstream rewrites. Preserve
  upstream defaults unless the isotope build explicitly opts into new behavior.

## Quick Setup

Use the helper for repeatable GitHub fork and clone setup:

```bash
python3 .agents/skills/github-isotope-creator/scripts/prepare_isotope_fork.py \
  supabase/cli \
  --fork-name supabase-cli
```

The helper creates or reuses `github.com/automic-vault/<fork-name>`, clones the
upstream repo into `data/isotopes/<fork-name>`, sets:

- `origin` to `git@github.com:automic-vault/<fork-name>.git`
- `upstream` to `https://github.com/<owner>/<repo>.git`
- local branch to `trunk`

If the upstream repo name is generic, such as `cli`, choose a descriptive fork
name like `supabase-cli`.

## Workflow

1. Confirm the target and risk.
   - Identify the upstream GitHub repo, the package being replaced, and the
     security reason for the isotope.
   - If the package source is Homebrew, npm, or PyPI, verify the installed
     package points at that GitHub repo before patching.
2. Prepare the fork.
   - Use `prepare_isotope_fork.py` or perform equivalent `gh repo fork`,
     `git clone`, remote setup, and `trunk` branch setup manually.
   - Do not commit nested isotope changes in the parent repo unless the parent
     intentionally tracks that nested clone.
3. Read local patterns.
   - Prefer matching existing source isotopes in `data/isotopes`, especially
     `gh-cli` for Keychain/security work and README fork notes.
   - Check `data/radioisotopes/AGENTS.md` when writing `automic-vault.yml` or
     `detect.rs` because those contracts also apply to source isotopes.
4. Patch upstream narrowly.
   - Keep upstream default behavior intact outside the isotope build.
   - Use platform/build tags when possible, for example `darwin && automicvault`
     for macOS-only isotope behavior.
   - Add tests around the patch seam. If upstream tests use a mock backend,
     preserve deterministic test behavior under isotope build tags.
5. Add `automic-vault.yml`.
   - Use `name: isotope:<name>`.
   - Use `replaces:` for a full replacement package, usually `brew:<formula>`.
   - Use `modifies:` only for radioisotope-style post-install modifications.
   - Include a build script that signs all delivered binaries with
     `$CODESIGN_IDENTITY`.
   - Prefer stable release tags when beta and stable tags share a commit:

```bash
version="$(git tag --points-at HEAD | sed -n 's/^v\([0-9][0-9.]*\)$/\1/p' | sort -V | tail -1)"
if [ -z "$version" ]; then
  version="$(git describe --tags --abbrev=0 | sed 's/^v//')"
fi
```

6. Add `detect.rs`.
   - Export `pub fn install_is_insecure() -> Result<bool, String>`.
   - Prefer also exporting `pub fn install_insecurity_reasons() -> Result<Vec<String>, String>`
     so the app can explain hazards.
   - Detect concrete insecure installed state: plaintext token files, unsafe
     Keychain ACLs, world-readable secrets, or other package-specific hazards.
   - In tests, never commit token-shaped literals that trigger GitHub push
     protection. Build fixtures at runtime, for example with `format!`.
7. Put fork notes at the top of `README.md`.
   - Match the `gh-cli` shape:
     `# Automic Vault Fork Notes`, short Automic Vault explanation, bullet list
     of behavior added on top of upstream, separator, then the original upstream
     README.
8. Verify.
   - Run the smallest relevant upstream tests for the patch.
   - Run `cargo test --lib <isotope-name-fragment>` from the parent repo to
     compile and exercise `detect.rs`.
   - Run `git diff --check`.
   - Run the isotope build if feasible and not excessively expensive.
9. Commit and push.
   - Commit inside the nested isotope repo.
   - If local GPG signing fails because a Keychain item is missing, use
     `git -c commit.gpgsign=false commit ...` and mention that in the result.
   - Push `trunk` to `origin`.
   - If GitHub auth or push protection blocks the push, fix the local issue
     when safe; otherwise report the exact blocker and leave the local commit.

## Metadata Template

```yaml
name:
  isotope:example-cli

replaces:
  brew:example

build: |
  version="$(git tag --points-at HEAD | sed -n 's/^v\([0-9][0-9.]*\)$/\1/p' | sort -V | tail -1)"
  if [ -z "$version" ]; then
    version="$(git describe --tags --abbrev=0 | sed 's/^v//')"
  fi
  make
  codesign --force --options runtime --timestamp \
    --sign "$CODESIGN_IDENTITY" \
    bin/example
  tar czf out.tgz bin

justification:
  title:
    Short User-Facing Risk
  detail: |
    Explain what the factory release exposes and how this isotope changes that
    behavior.

caveats:
  - Existing insecure credentials are detected but not migrated automatically.
```

## README Fork Notes Template

```markdown
# Automic Vault Fork Notes

This repository is the Automic Vault fork of <Tool>.

Automic Vault is a macOS-first secret and execution control system that
keeps sensitive credentials behind explicit human approval in the Automic
Vault GUI app instead of exposing them directly to terminal tools.

This fork currently adds the following behavior on top of upstream
`owner/repo`:

- ...

The remainder of this README is the original upstream <Tool> README.

---
```
