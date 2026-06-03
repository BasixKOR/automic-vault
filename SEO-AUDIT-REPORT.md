# SEO Audit Report: Automic Vault

**Audit Date:** June 1, 2026
**URL:** https://www.automicvault.com/
**Site Type:** Open-source macOS developer security product with SaaS-style documentation and programmatic package pages
**Scope:** Technical SEO, on-page SEO, content quality, international SEO, and rendered schema checks
**Pages Crawled:** 50 live URLs

## Executive Summary

**Overall SEO Health: 78/100**

Automic Vault has a strong technical SEO base. The live site is fast to fetch, static HTML is accessible, all sampled pages returned `200`, canonicals are self-referencing, `robots.txt` allows crawlers, XML sitemaps are present, hreflang clusters are reciprocal in the sampled pages, and rendered JSON-LD is present.

The main SEO problems are not crawlability blockers. They are page quality and prioritization problems: production still uses older homepage positioning, many localized pages are extremely thin, some titles and meta descriptions are weak or out of range, and the package catalog is large enough to dominate search results unless core commercial pages are strengthened.

### Top Priorities

1. Deploy the latest local landing-page copy and metadata to production.
2. Expand or withhold thin localized pages from the sitemap until they contain useful content.
3. Tighten titles and meta descriptions on core pages.
4. Add more internal links from package catalog pages back into the core SEO pages.
5. Fix minor technical issues: one redirect chain, missing logo/icon alt text, stale sitemap `lastmod`, and PageSpeed visibility.

## Methodology

Data collected:

- Live crawl of 50 URLs from `https://www.automicvault.com/`
- `robots.txt`, root sitemap, package sitemap index, and five package sitemap files
- Redirect checks for HTTP, non-www HTTPS, and non-www HTTP
- Rendered schema checks in the browser for homepage, docs, one package page, and Japanese homepage
- Search result spot check for `site:automicvault.com Automic Vault` and exact-brand queries
- PageSpeed Insights request attempted for mobile and desktop; both returned HTTP `429`, so no PSI lab or field data is included

No Google Search Console, Bing Webmaster Tools, analytics, backlink database, or real Core Web Vitals field data was available.

## Quick Wins

1. Deploy the current local homepage and `llms.txt` so production uses `A new kind of package manager for a new kind of threat model` consistently.
2. Change the live homepage H1 from `Automic Vault` to a keyword-bearing H1.
3. Update production sitemap `lastmod` values from `2026-05-24` to the actual deployment date when the new pages ship.
4. Add descriptive alt text to the logo/icon image or mark it consistently decorative with `aria-hidden` where appropriate.
5. Make `/download/`, `/pricing/`, GitHub CLI, AWS CLI, secret scanner, and `av trace` pages at least 500 words with concrete examples.

## Technical SEO Findings

### Finding 1: Crawlability and indexability are strong

**Impact:** Positive

**Evidence:** All 50 sampled URLs returned `200`. The root sitemap returned `200` and listed 104 URLs. The package sitemap index returned `200` and pointed to five package sitemaps:

- `/pkg/sitemap-hubs.xml`: 23 URLs
- `/pkg/sitemap-brew.xml`: 6,969 URLs
- `/pkg/sitemap-cask.xml`: 55 URLs
- `/pkg/sitemap-npm.xml`: 601 URLs
- `/pkg/sitemap-pip.xml`: 2 URLs

`robots.txt` allows `*`, `GPTBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `anthropic-ai`, `Google-Extended`, and `Bingbot`, and declares both sitemap locations.

**Fix:** No blocking fix needed. Keep monitoring sitemap validity after each package catalog generation.

**Priority:** Low

### Finding 2: HTTP non-www has a redirect chain

**Impact:** Low to Medium

**Evidence:**

- `http://www.automicvault.com/` -> `301` -> `https://www.automicvault.com/`
- `https://automicvault.com/` -> `301` -> `https://www.automicvault.com/`
- `http://automicvault.com/` -> `301` -> `https://automicvault.com/` -> `301` -> `https://www.automicvault.com/`

This is not a ranking blocker, but direct one-hop canonical redirects are cleaner and reduce crawler work.

**Fix:** Redirect `http://automicvault.com/` directly to `https://www.automicvault.com/`.

**Priority:** Low

### Finding 3: Root sitemap `lastmod` is stale relative to local work

**Impact:** Medium

**Evidence:** The live root sitemap reports `2026-05-24` for homepage, localized pages, docs, pricing, download, topic pages, `llms.txt`, and `security.txt`. The local repo has newer landing-page and `llms.txt` changes from May 31 and June 1.

**Fix:** When deploying the new landing page, regenerate and deploy `www/sitemap.xml` so `lastmod` reflects the actual updated content. Avoid changing `lastmod` without content changes.

**Priority:** Medium

### Finding 4: Rendered schema exists and is visible

**Impact:** Positive

**Evidence:** Browser-rendered checks found one JSON-LD script on sampled pages:

- Homepage: `Organization`, `Person`, `WebSite`, `SoftwareApplication`, `Offer`, `WebPage`, `BreadcrumbList`
- Docs: `TechArticle`, `FAQPage`, `Question`, `Answer`, plus organization/software schema
- Package page `/pkg/brew/awscli/`: `SoftwareApplication`, `TechArticle`, `HowTo`, `HowToStep`, `BreadcrumbList`
- Japanese homepage: `WebPage`, `WebSite`

**Fix:** Keep using rendered browser checks for schema validation. Do not rely only on text fetches when auditing schema.

**Priority:** Low

### Finding 5: PageSpeed Insights could not be measured

**Impact:** Unknown

**Evidence:** PageSpeed Insights returned `429` for both mobile and desktop homepage tests. The live fetch crawl itself was fast, with an average fetch time of 145 ms across sampled pages, and the rendered homepage had no horizontal overflow at the tested 1293 px viewport. This is not a substitute for Core Web Vitals.

**Fix:** Run PageSpeed Insights or CrUX through Search Console outside this session. Track mobile LCP, INP, and CLS for homepage, docs, download, and package pages.

**Priority:** Medium

## International SEO Findings

### Finding 6: Hreflang structure is technically sound in the sample

**Impact:** Positive

**Evidence:** Sampled English, Japanese, German, French, Simplified Chinese, and package pages had six hreflang entries: `en`, `ja`, `de`, `fr`, `zh-Hans`, and `x-default`. The sampled pages had self-referencing canonical URLs and no cross-locale canonicals.

**Fix:** Continue validating this in sitemap and HTML outputs. Keep `x-default` on all clusters.

**Priority:** Low

### Finding 7: Many localized pages are very thin

**Impact:** High

**Evidence:** Sampled localized support pages are indexable but extremely thin:

- `/ja/docs/`: 31 words
- `/zh-hans/docs/`: 31 words
- `/ja/about/`: 40 words
- `/zh-hans/about/`: 41 words
- `/ja/security/`: 35 words
- `/zh-hans/security/`: 38 words
- `/ja/privacy/`: 30 words
- `/zh-hans/privacy/`: 30 words
- `/ja/terms/`: 34 words
- `/zh-hans/terms/`: 34 words

Thin localized pages can reduce perceived site quality and are unlikely to rank for meaningful localized queries.

**Fix:** Either expand localized pages to provide real localized value or do not publish locale variants for low-value support pages until they are ready. Recommended minimum for indexable localized docs/about/security pages: 300-500 useful words with translated headings, examples, and links.

**Priority:** High

### Finding 8: Localized titles and descriptions need normalization

**Impact:** Medium

**Evidence:** Several localized titles are too short, such as `Automic Vault 文档` at 16 characters and `关于 Automic Vault` at 16 characters. French homepage title is long at 68 characters. Many localized meta descriptions are below 110 characters or above 165 characters.

**Fix:** Create localized title and description templates by page type. Keep core page titles descriptive rather than only brand + generic page type.

**Priority:** Medium

## On-Page SEO Findings

### Finding 9: Live homepage is behind current positioning

**Impact:** High

**Evidence:** Live homepage:

- Title: `Automic Vault | From the creator of Homebrew`
- H1: `Automic Vault`
- Meta description: 174 characters and based on older "protects AI agent runs" language

Current local work has stronger positioning:

- H1: `Secure the tools you brew install.`
- Tagline: `A new kind of package manager for a new kind of threat model`
- Founder quote: `Stop agents, malware, and compromised tools from accessing secrets or performing sensitive actions without approval.`
- Founder quote moved into a second-to-last section

**Fix:** Deploy the local landing-page changes and ensure the homepage title, H1, meta description, Open Graph, Twitter description, JSON-LD `WebPage.description`, `llms.txt`, and sitemap all update together.

**Priority:** High

### Finding 10: Several title tags are too short

**Impact:** Medium

**Evidence:** Examples:

- `/docs/`: `Automic Vault CLI Docs` - 22 characters
- `/security/`: `Security | Automic Vault` - 24 characters
- `/privacy/`: `Privacy | Automic Vault` - 23 characters
- `/terms/`: `Terms | Automic Vault` - 21 characters
- `/pricing/`: `Pricing | Automic Vault` - 23 characters

These are not broken, but they waste SERP real estate and understate the page query target.

**Fix:** Rewrite core titles around search intent:

- Docs: `Automic Vault CLI Docs for Agent Secrets and Approval Gates`
- Security: `Automic Vault Security Model for Local AI Agent Runs`
- Pricing: `Automic Vault Pricing - Free Open Source Agent Security`
- Download: `Download Automic Vault for macOS - Secure Homebrew Tools`

**Priority:** Medium

### Finding 11: Meta descriptions need tuning

**Impact:** Medium

**Evidence:** Homepage live description is 174 characters, German homepage is 173, and French homepage is 198. Download page is short at 91. Many localized support pages have descriptions under 80 characters.

**Fix:** Use 130-160 character descriptions for English core pages. For CJK pages, use localized norms, but keep the descriptions useful rather than generic. Lead with the search problem and the product action.

**Priority:** Medium

### Finding 12: Important high-intent pages are thin

**Impact:** High

**Evidence:** Sampled thin English pages:

- `/download/`: 146 words
- `/pricing/`: 263 words
- `/github-cli-token-security-ai-agents/`: 282 words
- `/secure-aws-cli-credentials-ai-agents/`: 318 words
- `/secret-scanner-for-ai-agents/`: 332 words
- `/av-trace/`: 342 words

These pages target commercial and high-intent queries but do not yet provide enough depth to consistently outrank stronger content.

**Fix:** Add focused sections to each:

- What this protects
- Why agents change the risk
- What changes after install
- Example command or workflow
- When to use Automic Vault with 1Password, HashiCorp Vault, AWS, GitHub, npm, or Homebrew
- FAQ block with schema where relevant

**Priority:** High

### Finding 13: Core site search visibility is being diluted by package pages

**Impact:** Medium

**Evidence:** Search result spot check for `site:automicvault.com Automic Vault` returned many package pages, including `qalculate-gtk`, `oh-my-opencode-linux-x64-musl`, `lua`, and `openclaw`, before core pages in the observed result set. This shows the package catalog is crawlable and being indexed, which is good for long-tail SEO, but it can also drown out core product pages if internal authority is not managed.

**Fix:**

- Add stronger internal links from package pages and package hubs to core pages.
- Add a small "Why secure this package with Automic Vault?" block on package pages that links to relevant guide pages.
- Use package hub pages as topical bridges: cloud CLIs -> AWS credentials page, source-control tools -> GitHub token page, package publishers -> approval gates.
- Consider stronger homepage and docs links in package page headers/footers.

**Priority:** Medium

## Content Quality Findings

### Finding 14: E-E-A-T foundation is good, but author/provenance cues can be stronger

**Impact:** Medium

**Evidence:** About, security, privacy, terms, docs, and security.txt exist. Founder `Person` schema is present, and Homebrew creator context is clear. However, most guide pages do not show a visible author/reviewer or last-reviewed date.

**Fix:** Add a compact provenance block to key pages:

- Maintained by Max Howell, creator of Homebrew
- Last reviewed date
- Source repository link
- Security disclosure link

**Priority:** Medium

### Finding 15: FAQ schema exists on some pages but not all high-intent pages

**Impact:** Medium

**Evidence:** FAQ schema was found on docs and several guide/comparison pages, but not on pages such as `/download/`, `/pricing/`, `/github-cli-token-security-ai-agents/`, `/secure-aws-cli-credentials-ai-agents/`, `/secret-scanner-for-ai-agents/`, and `/av-trace/`.

**Fix:** Add visible FAQs and JSON-LD FAQPage schema where the content naturally answers questions. Avoid schema-only FAQs that are not visible on the page.

**Priority:** Medium

### Finding 16: Logo/icon alt text is missing on many pages

**Impact:** Low

**Evidence:** Many pages had one image missing alt text, usually the logo/icon image. Other content images had alt text.

**Fix:** If the logo conveys brand identity, use `alt="Automic Vault"`. If it is purely decorative next to adjacent visible brand text, mark it decorative consistently and avoid counting it as content.

**Priority:** Low

## Prioritized Action Plan

### Critical Fixes

No critical crawl/indexation blockers were found.

### High-Impact Improvements

1. Deploy the current local homepage and metadata.
2. Expand thin high-intent English pages to 500+ useful words.
3. Expand localized docs/about/security/privacy/terms pages or stop publishing those locale variants until they are useful.
4. Add internal link bridges from package pages to core SEO pages.

### Quick Wins

1. Rewrite short titles on docs, security, pricing, privacy, and terms.
2. Tune meta descriptions to the 130-160 character range where appropriate.
3. Add missing logo/icon alt handling.
4. Fix the non-www HTTP redirect chain.
5. Run PageSpeed Insights once rate limiting clears.

### Long-Term Recommendations

1. Build topical clusters around `AI agent secret scanner`, `GitHub CLI token security`, `AWS CLI credential security`, `MCP secrets management`, and `Homebrew package security`.
2. Create comparison pages for 1Password, HashiCorp Vault, prompt-only agent controls, and traditional secret scanners.
3. Use package catalog data to produce curated, high-value hub pages rather than relying only on individual package pages.
4. Monitor Search Console coverage for package pages, locale pages, duplicate canonicals, and crawled-not-indexed patterns.

## Appendix: Sampled Page Metrics

| URL | Status | Words | Title Length | Description Length | H1 |
|---|---:|---:|---:|---:|---|
| `/` | 200 | 599 | 44 | 174 | Automic Vault |
| `/docs/` | 200 | 1506 | 22 | 115 | Operate from the terminal. Keep authority gated. |
| `/about/` | 200 | 344 | 50 | 134 | From the creator of Homebrew, built for local agent work |
| `/security/` | 200 | 697 | 24 | 146 | Agent security belongs at the local runtime boundary |
| `/pricing/` | 200 | 263 | 23 | 145 | Free open-source software for local agent security |
| `/download/` | 200 | 146 | 32 | 91 | Automic Vault |
| `/github-cli-token-security-ai-agents/` | 200 | 282 | 55 | 141 | GitHub CLI token security for AI agents |
| `/secure-aws-cli-credentials-ai-agents/` | 200 | 318 | 56 | 127 | Secure AWS CLI credentials before an AI agent can read them |
| `/secret-scanner-for-ai-agents/` | 200 | 332 | 39 | 137 | The secret scanner for AI agent machines |
| `/pkg/` | 200 | 705 | 40 | 152 | Package security catalog |
| `/ja/docs/` | 200 | 31 | 20 | 67 | CLI とローカル実行境界 |
| `/zh-hans/docs/` | 200 | 31 | 16 | 55 | CLI 与本地运行边界 |

## Sources Checked

- https://www.automicvault.com/
- https://www.automicvault.com/robots.txt
- https://www.automicvault.com/sitemap.xml
- https://www.automicvault.com/pkg/sitemap.xml
- https://www.automicvault.com/docs/
- https://www.automicvault.com/pkg/
- https://www.automicvault.com/ja/
- https://www.automicvault.com/pkg/brew/awscli/
