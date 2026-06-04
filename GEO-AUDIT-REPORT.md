# GEO Audit Report: Automic Vault

- **Audit date:** June 4, 2026
- **Primary URL:** https://www.automicvault.com/
- **Business type:** open-source macOS developer security product / local agent runtime boundary / package security catalog
- **Scope:** live production site, live package origin, package sitemaps, `llms.txt`, `llms-full.txt`, GitHub release metadata, and current web-search visibility samples
- **Pages sampled:** 37 live HTML pages: 16 main-site pages, 21 package catalog pages, including 12 individual package detail pages

## Executive Summary

**Overall GEO score: 82/100 (Good)**

Automic Vault's live site has materially improved since the prior reports. The production homepage now uses the stronger "Secure the tools you brew install" positioning, `llms.txt` resolves the current version to `1.17.0`, `llms-full.txt` is live, and the package catalog is now a major AI-search surface rather than a secondary artifact.

The package pages are the strongest GEO asset on the site. The live package sitemap index exposes 14,113 package URLs across hub, Homebrew, cask, npm, and pip sitemaps. Sampled package detail pages had clean titles, descriptions, canonicals, one H1, parseable JSON-LD, markdown alternates, source trails, install commands, risk signals, and freshness sections.

The main remaining constraint is authority, not crawlability. Owned surfaces are strong; third-party product-level references are still sparse. To move from "good" to "very strong", prioritize external entity anchors and deeper package-specific answer blocks for the highest-risk and highest-volume package pages.

## Score Breakdown

| Category | Score | Weight | Weighted |
|---|---:|---:|---:|
| AI Citability and Visibility | 87/100 | 25% | 21.75 |
| Brand Authority Signals | 64/100 | 20% | 12.80 |
| Content Quality and E-E-A-T | 82/100 | 20% | 16.40 |
| Technical Foundations | 92/100 | 15% | 13.80 |
| Structured Data | 93/100 | 10% | 9.30 |
| Platform Optimization | 80/100 | 10% | 8.00 |
| **Overall GEO Score** | | | **82.25 -> 82/100** |

## What Changed Since The Previous Report

- The live homepage now uses the current product story: `Secure the tools you brew install .`
- `llms.txt` now says `Current version: 1.17.0`; the old unresolved version placeholder is gone.
- `llms-full.txt` is live and returns about 413 KB of concatenated crawlable site text.
- The latest GitHub release is `v1.17.0`, published June 4, 2026, matching `llms.txt`.
- The live package sitemap index is served by `av-web` and exposes 5 sitemap shards.
- Package catalog URL count is now 14,113 live sitemap URLs, not just a local generated surface.
- Sampled main and package pages no longer had sub-350-word thin pages.

## Critical Issues

None found.

The live site is indexable, static or server-rendered, technically crawlable, AI-crawler friendly, schema-rich, and aligned around the current product positioning.

## Package Catalog Findings

### Sitemap Coverage

| Live package sitemap | URL count | Last modified |
|---|---:|---|
| `/pkg/sitemap-hubs.xml` | 27 | June 4, 2026 20:14:50 UTC |
| `/pkg/sitemap-brew.xml` | 6,858 | June 4, 2026 20:14:50 UTC |
| `/pkg/sitemap-cask.xml` | 6,485 | June 4, 2026 20:14:50 UTC |
| `/pkg/sitemap-npm.xml` | 741 | June 4, 2026 20:14:50 UTC |
| `/pkg/sitemap-pip.xml` | 2 | June 4, 2026 20:14:50 UTC |
| **Total package URLs** | **14,113** | |

The package sitemap index is live at `https://www.automicvault.com/pkg/sitemap.xml` and is also advertised in `robots.txt`. Sampled package sitemap XML includes `hreflang` alternates, which helps multilingual package-page discovery.

### Sampled Package Page Quality

| Sample group | Count | Min words | Median words | Max words | Issues found |
|---|---:|---:|---:|---:|---|
| Package pages and hubs | 21 | 639 | 1,402 | 2,121 | 0 missing titles, descriptions, canonicals, H1s, or valid JSON-LD |
| Individual package detail pages | 12 | 639 | 1,578 | 2,121 | 0 missing markdown alternates or source/risk/freshness sections |
| Package hubs | 8 | 1,137 | 1,402 | 1,444 | Hub pages have no markdown alternates |

Sampled detail pages included `gh`, `awscli`, `curl`, `git`, `node`, `uv`, `kubernetes-cli`, `fd`, `starship`, `monero`, `codex`, and `visual-studio-code`.

### Package Strengths

- Individual package pages expose `SoftwareApplication`, `TechArticle`, `HowTo`, `BreadcrumbList`, `Organization`, `Person`, and `WebSite` schema.
- Package detail pages have markdown alternates, which is especially useful for AI ingestion.
- Pages separate package metadata, install commands, security posture, executable facts, freshness, and source trail.
- High-intent package pages create exact-match AI-answer targets for queries such as "is gh safe for AI agents", "install awscli with approval gates", and "package security for codex".
- Package hubs are substantial: sampled hubs had 81 to 102 internal links and 1,137 to 1,444 words.
- Package origin assets are live from the Atlas origin: `/pkg/styles.css`, `/pkg/search.js`, and `/pkg/search.json` all returned 200.

### Package Weaknesses

1. **Package detail pages are structurally strong but still often feel generated.**
   The source trail is valuable, but top package pages should add short human-authored answer blocks: what the package can read, what it can mutate, what secrets it commonly touches, and whether unattended agent use should require approval.

2. **Package hubs do not have markdown alternates.**
   Detail pages do, which is good. Add `.md` alternates for package hubs such as `/pkg/cloud-clis/`, `/pkg/mcp-tools/`, `/pkg/package-publishers/`, and `/pkg/secret-risk-packages/` so AI systems can ingest category summaries as clean text.

3. **Cask/desktop app package pages are thinner than Homebrew formula pages.**
   Sampled cask pages were still acceptable, but `visual-studio-code` had 639 words and `codex` had 726. These deserve stronger package-specific security notes because IDEs and agent tools are high-authority local software.

## High Priority Issues

### 1. Product-level authority is still early

**Evidence:** GitHub reported 21 stars, 1 fork, 0 open issues, and latest release `v1.17.0` during this audit. Current search visibility is mostly owned or founder-owned: the official site, localized package pages, GitHub, and `mxcl.dev`. Search samples still surface unrelated "Automic" and "Atomic Vault" entities.

**Why it matters:** AI systems need corroboration outside the product's own domain. The founder/Homebrew signal is strong, but the product entity still needs independent references to reduce ambiguity and increase citation confidence.

**Fix:**

- Publish or syndicate the founder note on developer-visible channels.
- Create complete product profiles on GitHub org, X, LinkedIn, YouTube, and Product Hunt-style launch surfaces where appropriate.
- Encourage third-party posts around AI agents reading local secrets, Homebrew-era trust assumptions, and command approval gates.
- Add `sameAs` links only after profiles are complete and stable.

### 2. Highest-value package pages need direct-answer blocks

**Evidence:** Sampled package pages are technically strong, but many answer in generated metadata sections rather than a concise editorial paragraph. The best candidates are `gh`, `awscli`, `git`, `node`, `uv`, `kubernetes-cli`, `codex`, `visual-studio-code`, package publisher tools, cloud CLIs, MCP tools, and source-control tools.

**Why it matters:** ChatGPT, Perplexity, Gemini, and Google AI Overviews prefer passages that can be lifted into an answer. A package page should directly answer: "What risk does this tool create for an AI agent run, and how does Automic Vault control it?"

**Fix:** Add a short "Agent safety answer" block to the top 100 package pages:

- Whether the package can read credentials.
- Whether it can mutate remote state.
- Whether it publishes artifacts or code.
- Whether it has postinstall, daemon, plugin, or credential-helper behavior.
- The recommended Automic Vault control: scanner, protected credential helper, approval gate, or hardened install.

### 3. Some main use-case pages are just above minimum depth

**Evidence:** Sampled low-end pages were still indexable and above 350 words, but close to the floor: `/stop-ai-agents-reading-env-files/` at 357 words, `/ai-agent-approval-gates/` at 374, `/mcp-secrets-management/` at 416, and `/api-key-management-for-ai-agents/` at 432.

**Why it matters:** These pages target valuable AI-answer queries. They already have FAQ/schema support on several routes, but would be stronger with more concrete examples.

**Fix:** Add compact sections:

- "Before / after"
- "What the agent can no longer read"
- "Command example"
- "What this does not replace"
- "When to pair this with 1Password, HashiCorp Vault, AWS, GitHub, or MCP config"

### 4. Package hub markdown alternates are missing

**Evidence:** Sampled individual package detail pages had markdown alternates. Sampled package hubs did not.

**Why it matters:** Hub pages are category-level explainers with strong internal linking. Clean text alternates would help AI ingestion for category queries such as "cloud CLI security for AI agents" and "MCP tool package risks".

**Fix:** Generate hub-level `.md` alternates and link them with `rel="alternate" type="text/markdown"` or `type="text/plain"` consistently with detail pages.

## Medium Priority Issues

### 1. Homepage extracted H1 has a spacing artifact

The HTML/text extraction returns `Secure the tools you brew install .` with a space before the period. This is minor, but AI systems can copy extracted text literally. Tighten the generated markup so the text extracts as `Secure the tools you brew install.`

### 2. Security-product trust headers are good, but CSP remains permissive

The site has HSTS, `frame-ancestors 'none'`, `nosniff`, referrer policy, and permissions policy. CSP still allows `unsafe-inline` and `wasm-unsafe-eval`. This is not a GEO blocker, but for a security product it is worth tightening when implementation allows.

### 3. Package search JSON is functional but not itself an AI surface

`/pkg/search.json` returns 200 from `av-web`, but an empty query returns an empty result payload. That is fine for UI behavior; the searchable GEO value lives in static package URLs and sitemaps, not in the JSON endpoint. If search-result pages are introduced later, make them canonical and indexable only when they represent durable category pages.

## Category Deep Dives

### AI Citability and Visibility: 87/100

Strengths:

- Static or server-rendered HTML is directly extractable.
- `robots.txt` allows `*`, `GPTBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `anthropic-ai`, `Google-Extended`, and `Bingbot`.
- `robots.txt` advertises both the main sitemap and package sitemap index.
- `llms.txt` is live, current, and product-specific.
- `llms-full.txt` is live and exposes the crawlable site text.
- Package pages provide exact long-tail citation targets.

Weaknesses:

- External corroboration is thin.
- Some use-case pages need more concrete examples.
- Package pages should add stronger direct-answer passages for priority packages.

Best citation targets today:

- `/docs/`
- `/security/whitepaper/`
- `/secret-scanner-for-ai-agents/`
- `/secure-aws-cli-credentials-ai-agents/`
- `/github-cli-token-security-ai-agents/`
- `/pkg/brew/gh/`
- `/pkg/brew/awscli/`
- `/pkg/brew/git/`
- `/pkg/brew/node/`
- `/pkg/cask/codex/`

### Brand Authority Signals: 64/100

Strengths:

- Founder authority is strong and now tied to a current founder note on `mxcl.dev`.
- GitHub is active, public, and current.
- The official site, package pages, and localized pages are now appearing in search samples.
- `Organization` and `Person` schema help entity association.

Weaknesses:

- Product-level third-party authority is still sparse.
- Search result ambiguity remains with unrelated Automic/Broadcom, Automic Group, and generic "atomic vault" entities.
- GitHub adoption signals are still early: 21 stars and 1 fork at audit time.

### Content Quality and E-E-A-T: 82/100

Strengths:

- The main positioning is now sharper and consistently reflected in `llms.txt`.
- Security whitepaper and docs are substantial.
- Use-case pages are clear and schema-backed.
- Package catalog content is source-backed and transparent.

Weaknesses:

- Some use-case pages need more worked examples.
- Generated package pages need editorial direct-answer blocks for the most important packages.
- Package hub pages should be made available as clean markdown/text alternates.

### Technical Foundations: 92/100

Strengths:

- Main static origin and package Atlas origin both return fast 200s in live samples.
- Main sitemap has 120 URLs and fresh June 4 metadata.
- Package sitemap index has 5 shards and fresh June 4 metadata.
- Core AI files and security metadata return 200: `/robots.txt`, `/llms.txt`, `/llms-full.txt`, and `/.well-known/security.txt`.
- Security headers are broadly strong.

Weaknesses:

- CSP permissiveness is a trust polish issue.
- Package search JSON should not become an accidental indexable thin surface if query pages are added later.

### Structured Data: 93/100

Strengths:

- Sampled pages had no JSON-LD parse errors.
- Main pages use `SoftwareApplication`, `FAQPage`, `Article`, `TechArticle`, `BreadcrumbList`, `Organization`, `Person`, and `WebSite` schema where appropriate.
- Package details use `SoftwareApplication`, `TechArticle`, `HowTo`, `HowToStep`, and `BreadcrumbList`.
- Package hubs use `CollectionPage` and `ItemList`.

Weaknesses:

- Priority package pages would benefit from richer package-specific facts in visible text, not only schema and tables.

### Platform Optimization: 80/100

| Platform | Readiness | Notes |
|---|---:|---|
| ChatGPT / browsing LLMs | High | `llms.txt`, `llms-full.txt`, markdown package alternates, static text, and package pages are strong. |
| Perplexity | High | Strong sitemap and exact package URLs. More third-party citations would help. |
| Google AI Overviews | Medium-high | Technical and schema foundations are strong. Needs more external authority and direct-answer passages. |
| Claude / ClaudeBot | High | Robots allowlist, clean text, and markdown alternates are favorable. |
| Bing Copilot | Medium-high | Crawlability and schema are good. Product entity authority should be strengthened. |

## Recommended Action Plan

### Fix This Week

1. Remove the homepage H1 spacing artifact so extracted text reads `Secure the tools you brew install.`
2. Generate markdown alternates for package hub pages.
3. Add direct-answer blocks to the top 25 package detail pages, starting with `gh`, `awscli`, `git`, `node`, `uv`, `kubernetes-cli`, `codex`, and `visual-studio-code`.
4. Add 100 to 200 words of concrete examples to the four thinnest use-case pages.

### Package Page Sprint

1. Build a priority package list from install volume, risk level, protected-tool coverage, approval-gate coverage, cloud/source-control/publisher behavior, and AI-agent relevance.
2. Add "Agent safety answer" blocks for the top 100 packages.
3. Add cask-specific security treatment for IDEs, agent apps, terminal apps, browser automation tools, and credential-bearing desktop apps.
4. Track which package pages have human-authored guidance versus generated-only summaries.

### Authority Sprint

1. Use the founder note as the canonical external explainer and link it prominently from the site and GitHub.
2. Publish one technical walkthrough showing Automic Vault protecting a real AI-agent run involving GitHub CLI, AWS CLI, `.env`, and a package install.
3. Seek third-party developer mentions from channels AI systems commonly cite: GitHub discussions, Hacker News, Reddit, YouTube demos, newsletters, and comparison posts.
4. Keep the same product definition across site, GitHub, `llms.txt`, mxcl.dev, social profiles, and package pages.

## Evidence Sources

- Live homepage: https://www.automicvault.com/
- Main sitemap: https://www.automicvault.com/sitemap.xml
- Package sitemap index: https://www.automicvault.com/pkg/sitemap.xml
- Robots file: https://www.automicvault.com/robots.txt
- LLM summary: https://www.automicvault.com/llms.txt
- Full LLM text: https://www.automicvault.com/llms-full.txt
- Package catalog: https://www.automicvault.com/pkg/
- Sample package pages: https://www.automicvault.com/pkg/brew/gh/, https://www.automicvault.com/pkg/brew/awscli/, https://www.automicvault.com/pkg/brew/git/, https://www.automicvault.com/pkg/cask/codex/
- GitHub repo and release metadata: https://github.com/automic-vault/automic-vault
- Founder context: https://mxcl.dev/automic-vault/

## Bottom Line

The live site is now in good GEO shape. The package catalog is a real competitive advantage: it gives Automic Vault thousands of exact, structured, source-backed pages for AI-search queries around developer tools and local agent risk. The next gains will come from making the most important package pages less generic, giving package hubs clean text alternates, and building enough third-party product authority that AI systems treat Automic Vault as a distinct entity rather than only a founder-associated owned-domain project.
