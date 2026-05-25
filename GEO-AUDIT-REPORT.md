# GEO Audit Report: Automic Vault

**Audit date:** 2026-05-25  
**Primary site:** https://www.automicvault.com/  
**Primary focus:** current generated package pages under `/pkg/`  
**Business type:** open-source macOS developer security tool with a large package intelligence/catalog surface

## Executive Summary

**Overall GEO score: 77/100**

Automic Vault has a strong technical GEO foundation: the site is static, crawlable, schema-rich, and explicitly AI-crawler friendly. The new package catalog is the strongest surface: it publishes 9,008 package HTML pages, 9,006 markdown alternates, 22 category hubs, valid package sitemaps, canonical URLs, `SoftwareApplication` schema, `TechArticle` schema, `HowTo` install schema, breadcrumbs, Open Graph, and Twitter metadata.

The main constraints are no longer basic crawlability. The highest-leverage fixes are trust and answer-engine precision: restore the missing `llms-full.txt`, replace the unresolved version placeholder in `llms.txt`, avoid putting inferred install commands into citation-grade HowTo schema, improve freshness signals, and build more third-party product authority so AI systems can disambiguate Automic Vault from unrelated "Automic" entities.

## Score Breakdown

| Category | Score | Weight | Weighted |
|---|---:|---:|---:|
| AI Citability and Visibility | 82/100 | 25% | 20.50 |
| Brand Authority Signals | 58/100 | 20% | 11.60 |
| Content Quality and E-E-A-T | 76/100 | 20% | 15.20 |
| Technical Foundations | 90/100 | 15% | 13.50 |
| Structured Data | 91/100 | 10% | 9.10 |
| Platform Optimization | 69/100 | 10% | 6.90 |
| **Overall GEO Score** | | | **76.80 -> 77/100** |

## Audit Scope

Local generated files inspected:

- `www/sitemap.xml`
- `www/robots.txt`
- `www/llms.txt`
- `www/pkg/sitemap.xml`
- all HTML pages under `www/pkg/`
- package markdown alternates under `www/pkg/`
- generated package manifest `www/pkg/.manifest.json`

External visibility checked through web search:

- `https://www.automicvault.com/`
- `https://www.automicvault.com/pkg/`
- `https://mxcl.dev/`
- brand queries for `"Automic Vault"` and `automicvault.com`

## Package Catalog Findings

Package generation is technically strong.

| Check | Result |
|---|---:|
| Package HTML pages | 9,008 |
| Package hub/index HTML pages | 23 |
| Indexable package pages | 9,006 |
| Noindex package pages | 2 |
| Markdown package alternates | 9,006 |
| Package sitemap URLs | 9,029 |
| Missing package sitemap targets | 0 |
| Missing package titles/descriptions/canonicals | 0 |
| Package canonical mismatches | 0 |
| Invalid JSON-LD blocks | 0 |
| Duplicate package descriptions | 0 |
| Duplicate package titles | 0 |
| Missing asset references | 0 |

Structured data coverage is broad:

- 9,008 `SoftwareApplication` nodes
- 9,008 `TechArticle` nodes
- 9,008 `HowTo` nodes
- 9,030 `BreadcrumbList` nodes
- 23 `CollectionPage` nodes
- 22 hub `ItemList` collections

## High Priority Issues

1. **`llms-full.txt` is referenced but missing.**  
   `www/sitemap.xml` and `www/llms.txt` both point to `https://www.automicvault.com/llms-full.txt`, but no `www/llms-full.txt` file exists locally. This is the clearest GEO defect because it advertises a high-value AI ingestion artifact that will resolve as missing after deploy unless generated elsewhere.

2. **`llms.txt` still contains an unresolved version placeholder.**  
   The file says `Current version: __AUTOMIC_VAULT_VERSION__`. LLM-facing facts should never contain deploy placeholders; it reduces citation confidence and can be copied into AI answers.

3. **Inferred install commands are present in `HowTo` schema on 8,385 package pages.**  
   The visible UI labels these commands as inferred, but schema consumers may treat HowTo steps as verified instructions. For package pages, only source-backed install commands should be emitted in `HowTo`; inferred commands should remain visible as lower-confidence UI content outside citation-grade structured data.

4. **All 9,008 package pages show unknown upstream freshness.**  
   The freshness section consistently reports upstream release/tag data as unknown. This is transparent, but it weakens the package pages for AI answers that need current-version confidence. Prioritize upstream release/tag enrichment for high-risk and high-volume packages.

5. **Brand authority remains founder-led instead of product-led.**  
   Search finds Automic Vault on owned properties and `mxcl.dev`, but third-party product mentions are sparse. The Max Howell/Homebrew association is valuable, yet the product needs independent references on developer platforms to avoid blending with unrelated Automic/Broadcom and Automic Group results.

## Medium Priority Issues

1. **386 descriptions end with a hard ellipsis.**  
   Many generated meta/schema descriptions are truncated mid-thought. This is not a crawl blocker, but it makes snippets and JSON-LD descriptions feel less authoritative. Prefer sentence-boundary truncation for hub and package descriptions.

2. **638 package pages explicitly say no radioisotope coverage was found.**  
   That is honest and useful, but for high-risk packages it can read as a coverage gap. Prioritize radioisotope or approval-gate coverage for packages that also have postinstall hooks, cloud credentials, publishing authority, source-control authority, or secret access.

3. **566 npm package pages include postinstall risk signals.**  
   These are exactly the pages that should receive deeper, package-specific security treatment: lifecycle script summary, executable surface, credential paths, registry authority, and recommended approval policies.

4. **Package pages are highly structured, but many are generated summaries rather than expert analyses.**  
   The source trail is clear, which helps. Still, AI answers prefer pages that contain concise, direct guidance. Add short direct-answer blocks to the most important package pages, especially `awscli`, `gh`, `git`, `docker`, `uv`, `node`, `claude-code`, `codex`, `terraform`-family tools, and package publishers.

5. **External platform footprint is thin.**  
   The site and GitHub are discoverable, but there is not enough third-party product context from Reddit, Hacker News, YouTube demos, Product Hunt, Wikidata/Wikipedia, or comparison articles.

## Strengths

- `robots.txt` explicitly allows `GPTBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `anthropic-ai`, `Google-Extended`, and `Bingbot`.
- `robots.txt` advertises both the main sitemap and package sitemap index.
- The package sitemap index cleanly splits hubs, brew, cask, npm, and pip URLs.
- Main sitemap has 24 URLs; the only missing local target is `llms-full.txt`.
- All non-404 HTML pages have titles, meta descriptions, canonicals, and exactly one `h1`.
- All checked JSON-LD parses cleanly.
- Package pages provide markdown alternates, which is useful for AI ingestion.
- Package pages disclose their generated source trail.
- Package pages have strong internal linking through category hubs and related package sections.
- Search pages, package hubs, and individual package pages create a large long-tail query surface around "install X safely", "X package security", and "AI agent package risk".

## Platform Notes

| Platform | Readiness | Notes |
|---|---:|---|
| ChatGPT / browsing LLMs | High | Static HTML, markdown alternates, `llms.txt`, package pages, strong schema. Missing `llms-full.txt` hurts. |
| Perplexity | High | Good sitemap structure and direct package pages. Needs more third-party corroboration. |
| Google AI Overviews | Medium-high | Strong technical SEO and topic pages. Needs stronger authority and direct-answer blocks on top package pages. |
| Claude / ClaudeBot | High | Robots allowlist and markdown alternates are good. Fix unresolved LLM-facing placeholders. |
| Bing Copilot | Medium-high | Crawlable and schema-rich. More external product mentions would help entity recognition. |

## Recommended Actions

### Fix Now

1. Generate or remove `llms-full.txt` references so `www/sitemap.xml` and `www/llms.txt` do not point to a missing AI artifact.
2. Replace `__AUTOMIC_VAULT_VERSION__` in `www/llms.txt` during generation or deployment.
3. Restrict package `HowTo` schema to verified install commands only.
4. Add a generation check that fails if sitemap URLs or `llms.txt` links point at missing local files.

### Package Page Sprint

1. Add package-specific direct-answer blocks for the top 50 security-sensitive packages.
2. Add upstream release/tag enrichment to the freshness pipeline for high-volume packages.
3. Sentence-truncate meta descriptions instead of ending mid-sentence with an ellipsis.
4. Prioritize radioisotope or approval-gate coverage for postinstall packages and cloud/source-control/publisher tools.
5. Add schema/UI separation for verified versus inferred install commands.

### Authority Sprint

1. Publish a technical launch post that demonstrates Automic Vault protecting real agent workflows.
2. Create product profiles or launch pages on developer-visible platforms.
3. Encourage third-party writeups around AI agent secrets, approval gates, and package install control.
4. Add comparison pages for "Automic Vault vs secret scanning", "Automic Vault vs HashiCorp Vault for AI agents", and "Automic Vault vs package manager sandboxing" if not already indexed strongly enough.

## Suggested Checks To Add

- `scripts/generate-llms-full.mjs` output existence check in deploy.
- sitemap-to-local-file validation for `www/sitemap.xml` and all package sitemap files.
- JSON-LD validation for generated pages.
- package schema validation that rejects inferred install commands inside `HowTo`.
- meta description test that avoids hard ellipsis endings.
- high-risk package completeness report covering postinstall, executable, repository, homepage, freshness, approval-gate, and radioisotope fields.

## Bottom Line

The package catalog is already valuable for GEO: it is large, crawlable, well linked, schema-rich, and source-backed. The next gains come from trust precision. Fix the missing LLM artifact and placeholder version first, then tighten package schema so AI systems distinguish verified facts from inferred convenience commands. After that, build external product authority and add direct-answer treatment to the most important package pages.
