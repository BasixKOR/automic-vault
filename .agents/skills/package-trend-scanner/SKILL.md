---
name: package-trend-scanner
description: Discover trending packages for Automic Vault/Nucleus by scanning package registries, repository activity, launch/discussion surfaces, and ecosystem analytics; normalize candidates to installable package specs such as brew:foo, cask:bar, npm:@scope/name, or pip:name; emit trending.json with per-package source evidence for automations and app display.
metadata:
  short-description: Build Automic Vault package trend feeds
---

# Package Trend Scanner

Use this skill when producing or reviewing the Automic Vault trending package
feed, especially the automation that publishes `https://automicvault.com/trending.json`.

The job is not to list popular packages in general. The job is to discover
current, installable, agent-relevant packages and attach enough source evidence
that the app can explain why each package is being shown.

## Boundary

- Runtime boundary: this runs as an offline automation or one-shot research job,
  not inside the app request path.
- Persistence boundary: the only output is the JSON feed requested by the job.
  Do not change Nucleus package roots, registry endpoints, or app persistence.
- Product surface: Automic Vault.app displays the feed. Keep fields stable,
  compact, and directly useful for UI.
- Change shape: feed generation is additive. Do not introduce new package
  qualifiers unless the repo already supports them.

## Supported Package Specs

Emit only these package spec prefixes unless the user explicitly expands support:

| Ecosystem | Package spec | Notes |
| --- | --- | --- |
| Homebrew formula | `brew:<formula>` | Use core formula names. Do not emit tapped formula paths unless current Nucleus support is confirmed. |
| Homebrew cask | `cask:<cask>` | Use cask tokens from Homebrew cask metadata. |
| npm | `npm:<package>` | Preserve npm scope, for example `npm:@anthropic-ai/claude-code`. Do not include versions in trend feed specs. |
| PyPI | `pip:<project>` | Normalize PyPI project names per PEP 503: lowercase and collapse runs of `-`, `_`, `.` to `-`. Do not emit `pypi:`. |

If a candidate cannot be resolved to one of these specs with high confidence,
exclude it or put it in a private notes section outside the published JSON.

## Source Strategy

Prefer machine-readable, first-party sources. Use social and launch surfaces for
discovery, then verify every candidate against a package registry.

Primary registry and ecosystem sources:

- Homebrew Formulae JSON APIs:
  - formula metadata: `https://formulae.brew.sh/api/formula.json`
  - cask metadata: `https://formulae.brew.sh/api/cask.json`
  - formula install-on-request analytics:
    `https://formulae.brew.sh/api/analytics/install-on-request/homebrew-core/30d.json`
  - cask install analytics:
    `https://formulae.brew.sh/api/analytics/cask-install/homebrew-cask/30d.json`
- npm registry:
  - search API: `https://registry.npmjs.org/-/v1/search`
  - package metadata: `https://registry.npmjs.org/<package>`
  - downloads API: `https://api.npmjs.org/downloads/point/last-week/<package>`
- PyPI:
  - project JSON: `https://pypi.org/pypi/<project>/json`
  - simple JSON index when needed: request `application/vnd.pypi.simple.v1+json`
  - PyPI download stats may be used as a supplemental source when available,
    but never as the only verification that a project exists.

Discovery sources:

- GitHub repository search for recently created or recently pushed repositories
  with relevant topics and keywords.
- GitHub Trending pages when the API misses a language/community signal.
- Hacker News Algolia API for recent "Show HN", launch, and discussion threads.
- Reddit JSON/RSS for focused communities such as command-line, self-hosted,
  devops, kubernetes, local-first, Python, JavaScript, Rust, Go, and AI agent
  tooling.
- Product launch and changelog surfaces only when they provide package or repo
  links that can be verified elsewhere.

Relevant keyword clusters:

- agent, agents, coding agent, AI CLI, LLM CLI, MCP, local AI
- secrets, sandbox, permissions, approvals, containment, credentials
- terminal, shell, command line, developer tools, package manager
- kubernetes, containers, cloud, infrastructure, deploy, CI, release
- macOS developer tools and security tooling

## Discovery Workflow

1. Define the window.
   - Default social/GitHub window: last 7 days.
   - Default registry analytics window: closest available 7-30 day window.
   - Record `generatedAt`, `window.startedAt`, and `window.endedAt` in UTC.
2. Gather candidates from multiple source families.
   - Registry analytics identify already-popular packages.
   - GitHub/HN/Reddit identify rising packages before registry analytics catch up.
   - Keep raw source URLs and observed metrics as you go.
3. Resolve candidates to package specs.
   - For Homebrew, match against formula/cask API tokens.
   - For npm, verify package metadata and use the registry `name` exactly.
   - For PyPI, verify project metadata, then emit normalized `pip:<name>`.
   - For GitHub-only projects, inspect package manifests, release docs, README
     install commands, Homebrew formula metadata, npm package metadata, and PyPI
     metadata. Require a clear package identity before inclusion.
4. Filter for Automic Vault relevance.
   - Prefer CLIs, developer tools, agent tools, security tools, and operational
     tools.
   - Exclude pure libraries unless the surrounding evidence clearly shows a
     developer-facing tool or agent workflow.
   - Exclude packages with suspicious names, empty metadata, unclear ownership,
     obvious typosquatting, or unresolved installability.
5. Score and rank.
   - Reward independent source families, recent velocity, registry popularity,
     installability confidence, and Automic Vault relevance.
   - Penalize missing metadata, single-source hype, stale repositories, unclear
     maintainers, and packages with no executable/tool surface.
   - Keep the final list concise. A useful feed is usually 10-30 packages.
6. Emit JSON only for the feed artifact.
   - No Markdown, comments, trailing commas, or prose inside the feed file.

## Output Contract

Write valid UTF-8 JSON with this shape:

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-05-10T15:04:05Z",
  "window": {
    "startedAt": "2026-05-03T00:00:00Z",
    "endedAt": "2026-05-10T00:00:00Z",
    "label": "7d"
  },
  "packages": [
    {
      "pkgspec": "brew:uv",
      "ecosystem": "brew",
      "name": "uv",
      "displayName": "uv",
      "description": "Fast Python package and project manager",
      "homepage": "https://github.com/astral-sh/uv",
      "repository": "https://github.com/astral-sh/uv",
      "score": 92,
      "reasons": [
        "High recent Homebrew install-on-request volume",
        "Active GitHub discussion in developer tooling"
      ],
      "sources": [
        {
          "id": "homebrew-install-on-request-30d",
          "type": "registry-analytics",
          "name": "Homebrew install-on-request analytics",
          "url": "https://formulae.brew.sh/api/analytics/install-on-request/homebrew-core/30d.json",
          "observedAt": "2026-05-10T15:04:05Z",
          "metric": "installs_30d",
          "value": 12345,
          "rank": 12,
          "evidence": "Homebrew analytics ranked uv among recent explicitly requested formula installs."
        }
      ]
    }
  ]
}
```

Field rules:

- `pkgspec` is the stable primary key and must be unique.
- `ecosystem` is one of `brew`, `cask`, `npm`, or `pip`.
- `name` is the unqualified package name as used by the registry.
- `score` is an integer from 0 to 100.
- `reasons` contains short UI-ready strings. Do not mention internal scraping
  mechanics.
- `sources` must contain at least one item. Prefer two or more independent
  source families for top-ranked packages.
- `sources[].evidence` should be concise and factual. Avoid unverifiable claims.
- Omit a field rather than emitting `null`, unless the consuming code explicitly
  asks for nullable fields.

## Dedupe Rules

- Dedupe by `pkgspec` first.
- If Homebrew, npm, and PyPI package the same upstream project, keep separate
  entries only when each package is independently useful to install.
- If multiple specs point to the same CLI, prefer the package source that the
  upstream install docs recommend for macOS users, then Homebrew, then npm, then
  pip.
- Preserve all relevant source evidence on the retained entry.

## Verification

Before publishing or handing off the feed:

```bash
jq -e '
  .schemaVersion == 1
  and (.generatedAt | type == "string")
  and (.window.startedAt | type == "string")
  and (.window.endedAt | type == "string")
  and (.packages | type == "array")
  and ([
    .packages[].pkgspec
  ] | length == unique | length)
  and all(.packages[];
    (.pkgspec | test("^(brew|cask|npm|pip):[^[:space:]]+$"))
    and (.score | type == "number" and . >= 0 and . <= 100)
    and (.sources | type == "array" and length > 0)
  )
' trending.json
```

Also spot-check the top packages by opening their registry pages or metadata
URLs. The feed should never contain a package that cannot be installed by the
current Nucleus package spec conventions.
