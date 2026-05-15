# GEO Audit Report: Automic Vault

**Audit Date:** 2026-05-15  
**URL:** https://www.automicvault.com/  
**Business Type:** Hybrid - open-source developer security software with SaaS-style product, docs, and topic pages  
**Pages Analyzed:** 17 sitemap entries: 14 HTML pages plus `llms.txt`, `pricing.md`, and `robots.txt`

---

## Executive Summary

**Overall GEO Score: 62/100 (Fair)**

Automic Vault has a strong technical GEO foundation: static HTML rendering, crawlable pages, explicit AI crawler access, a useful `llms.txt`, canonical URLs, and consistent JSON-LD across the site. The strongest underused asset is founder authority: Max Howell's public association with creating Homebrew in 2009 is sourceable and highly relevant to package-manager and developer-tooling credibility, but the Automic Vault site does not yet expose that authority through an about page, `Person` schema, citations, or expanded `sameAs` links. The remaining gaps are content depth, visible freshness, citations, and richer software/article/FAQ structured data.

### Score Breakdown

| Category | Score | Weight | Weighted Score |
|---|---:|---:|---:|
| AI Citability | 67/100 | 25% | 16.75 |
| Brand Authority | 46/100 | 20% | 9.20 |
| Content E-E-A-T | 62/100 | 20% | 12.40 |
| Technical GEO | 84/100 | 15% | 12.60 |
| Schema & Structured Data | 61/100 | 10% | 6.10 |
| Platform Optimization | 48/100 | 10% | 4.80 |
| **Overall GEO Score** | | | **62/100** |

---

## Audit Scope and Boundaries

- **User-facing surface changed:** none. This is an additive audit report only.
- **Runtime boundary audited:** static website under `www/`, served publicly at `https://www.automicvault.com/` via S3/CloudFront.
- **Persistence boundary audited:** none. The site is static; no datastore or app persistence is involved.
- **Change type:** additive documentation.

## Critical Issues (Fix Immediately)

No critical issues were found.

AI crawlers are not blocked, the homepage and key pages return indexable HTML, no domain-level `noindex` directive was found, key pages do not return 5xx errors, and structured data is present on all HTML pages.

## High Priority Issues

1. **Founder authority is strong but under-modeled on the site.**  
   Public sources verify that Max Howell created Homebrew in 2009, which is highly relevant authority for a macOS package-manager and developer-security product. Automic Vault mentions this on the homepage, but the site lacks an `/about/` page, `Person` schema, founder links, source citations, and expanded `sameAs` connections that would make this signal easier for AI systems to resolve.

2. **Article and docs pages lack author and freshness metadata.**  
   SEO pages declare `Article`, and docs declare `TechArticle`, but the JSON-LD lacks `author`, `headline`, `datePublished`, `dateModified`, and `image`. Visible "Last updated" text is only present in `www/pricing.md`.

3. **No dedicated trust or entity pages exist.**  
   The site has no `/about/`, `/security/`, `/privacy/`, `/terms/`, contact page, disclosure policy, founder bio page, or team/maintainer page. The homepage contains useful Max Howell/founder authority, but it is not formalized or linked as a durable entity signal.

4. **High-intent topic pages are thin for AI citation.**  
   Most SEO topic pages are clear but short. They define a problem and product angle, then stop before deeper threat models, implementation examples, limitations, citations, FAQs, or source-backed comparisons. This makes them less likely to be cited by answer engines when competing with deeper vendor and documentation pages.

## Medium Priority Issues

1. **`SoftwareApplication` schema is valid but incomplete.**  
   Current schema includes `name`, `applicationCategory`, `operatingSystem`, `url`, `description`, and `publisher`. It should also include fields already known to the site: `softwareVersion`, `downloadUrl`, `codeRepository`, `license`, `offers`, `featureList`, and `screenshot`.

2. **Docs FAQ is visible but not represented as `FAQPage` schema.**  
   `www/docs/index.html` has a real FAQ section, but no corresponding structured data. This is a clean extraction win for ChatGPT, Perplexity, Gemini, and Copilot even if FAQ rich-result display is platform-dependent.

3. **Sitemap lacks `<lastmod>` and includes low-value URLs.**  
   `www/sitemap.xml` lists key pages, but has no freshness metadata. It also includes `robots.txt`, which is low-value in an XML sitemap. `llms.txt` can remain if intentional for AI discovery; `pricing.md` would be stronger as an HTML page.

4. **Evidence density is low.**  
   Several claims are strong but unsupported by citations, benchmarks, source links, or external references. Comparison pages should cite primary sources such as vendor docs, platform docs, relevant standards, and Automic Vault source files.

5. **Platform-specific indexing hooks are incomplete.**  
   The site has good crawler access, but no IndexNow key/workflow was found for Bing/Copilot freshness. Search Console/Bing Webmaster submission cannot be verified from the repo.

6. **Live security headers are missing.**  
   Public responses confirm HTTPS and CloudFront delivery, but do not include HSTS, CSP, `X-Content-Type-Options`, `Referrer-Policy`, or frame-control headers. This is not a direct AI citation blocker, but it affects technical trust and browser security posture.

## Low Priority Issues

1. **Homepage `og:image` is relative.**  
   The homepage uses `/preview.jpg`; use an absolute URL for Open Graph consistency.

2. **Twitter card metadata is inconsistent.**  
   The homepage has title, description, and image tags. Most topic pages only declare `twitter:card`.

3. **Some copy reduces professional citability.**  
   Phrases such as "Agents and malware can suck it", "not vibes", and unsupported absolutes like "cannot be bypassed" are memorable, but less likely to be quoted in enterprise/security answers. There is also a typo: "intimiately".

4. **Some image and mobile polish remains.**  
   The homepage hero grid image lacks explicit dimensions, below-fold images could use `loading="lazy"`, and docs copy buttons appear below standard mobile tap-target guidance.

---

## Category Deep Dives

### AI Citability (67/100)

**Strengths**

- `www/llms.txt` is unusually useful: it states product category, current version, license, source repository, pricing, use cases, non-goals, and recommended descriptions in an extractable format.
- The site has clear topical pages for high-intent queries: AI agent secrets, `.env` exposure, API keys, MCP secrets, privileged access, approval gates, AWS CLI credentials, GitHub CLI tokens, secret scanning, and shell installer tracing.
- Pages are server-rendered static HTML. Titles, meta descriptions, H1s, links, and body copy are visible without JavaScript.
- Most pages use direct explanatory copy and clear H2/H3 structures.

**Gaps**

- Topic pages need more direct answer blocks that can stand alone in AI responses.
- Claims need citations and examples. AI systems prefer passages that state what, why, limitation, and source basis in one compact block.
- The docs FAQ should be exposed as structured data.
- Content repeats some phrases across pages, especially "Close the next credential path", which makes the cluster feel templated.

**Rewrite Pattern**

Add a short "Direct answer" block near the top of each topic page:

> Automic Vault is a local macOS security layer for AI coding agents. It keeps secrets out of files and model context, then injects approved credentials only into trusted command-line tools at runtime. This differs from central vaults such as HashiCorp Vault because it controls the final local execution step where an agent can read files, call CLIs, or expose tokens.

### Brand Authority (46/100)

**Strengths**

- The brand is coherent on owned surfaces: homepage, docs, `llms.txt`, GitHub, and X.
- `mxcl.dev`, Homebrew, Wikipedia, and other indexed references provide sourceable founder authority: Max Howell is publicly associated with creating Homebrew in 2009.
- This authority is directly relevant to Automic Vault's positioning around package management, macOS developer tooling, installer tracing, and agent-era execution boundaries.
- The public GitHub repo is discoverable and linked from the site.

**Gaps**

- No LinkedIn company page, Wikipedia/Wikidata entity, YouTube demo, Product Hunt/HN launch thread, Reddit discussion, or third-party reviews were found during the audit.
- `sameAs` only lists GitHub and X; it does not include `mxcl.dev`, a founder profile, Homebrew references, LinkedIn, or other stable entity anchors.
- The homepage founder section is not connected to structured data with `founder`, `Person`, `knowsAbout`, or authoritative profile links.
- Search results collide with Broadcom Automic Automation and generic Vault/Nucleus entities.

**Priority Direction**

Convert the existing founder authority into a durable entity graph: add a founder/about page, cite Homebrew and independent references, add `Person` schema for Max Howell, add `Organization.founder`, add `knowsAbout` terms for package management and AI agent security, then reflect stable URLs in `sameAs` and `llms.txt`.

### Content E-E-A-T (62/100)

**Strengths**

- Docs are the strongest expertise asset. They explain runtime boundaries, secret injection, containment, package installs, and operational trust notes.
- The homepage communicates a specific point of view and connects the product to package-manager expertise.
- Public founder credentials are strong and relevant: Homebrew's creation by Max Howell in 2009 is externally verifiable.
- Pricing is transparent and current in `pricing.md`.

**Gaps**

- No formal author, maintainer, reviewer, or founder metadata appears on HTML content pages.
- No visible update dates on HTML pages.
- No citations to source code, security docs, platform docs, or third-party references.
- No trust pages: privacy, terms, security, contact, disclosure, or about.
- Most high-intent topic pages need more first-hand implementation details, screenshots tied to claims, limitations, and examples.

### Technical GEO (84/100)

**Strengths**

- `robots.txt` explicitly allows GPTBot, ChatGPT-User, PerplexityBot, ClaudeBot, anthropic-ai, Google-Extended, and Bingbot.
- Sitemap exists and is declared from `robots.txt`.
- Canonicals are present on every HTML page.
- Raw HTML contains the primary content and metadata.
- Live responses confirm HTTPS, S3/CloudFront delivery, compression-capable static serving, and long-lived immutable asset caching.
- URL structure is clean and stable.

**Gaps**

- No security response headers were observed on the live homepage response.
- Sitemap has no `<lastmod>`.
- Sitemap includes `robots.txt`.
- Some page-speed polish remains: explicit dimensions/aspect-ratio for the hero grid image and lazy loading for below-fold images.

### Schema & Structured Data (61/100)

**Strengths**

- All HTML pages include parseable JSON-LD.
- The schema graph consistently uses `Organization`, `SoftwareApplication`, and either `WebPage`, `Article`, or `TechArticle`.
- `Organization.sameAs` includes GitHub and X.

**Gaps**

- `SoftwareApplication` lacks rich software fields: `offers`, `softwareVersion`, `downloadUrl`, `codeRepository`, `license`, `featureList`, and `screenshot`.
- Article pages lack `headline`, `author`, `datePublished`, `dateModified`, and `image`.
- Docs FAQ lacks `FAQPage`.
- Topic pages lack `BreadcrumbList`.
- Homepage lacks a `WebSite` node.
- Organization lacks `description`, `founder`, `foundingDate`, `knowsAbout`, and `contactPoint` if a real support channel exists.

### Platform Optimization (48/100)

**Google AI Overviews**

Good crawlability and static HTML help. Missing visible dates, deeper citations, stronger author/entity schema, and FAQ/table structures limit eligibility for concise AI Overview extraction.

**ChatGPT**

`llms.txt`, GitHub, docs, static pages, and founder authority are strong. Brand/entity confidence is still limited because the site does not yet connect Automic Vault, Max Howell, Homebrew, and third-party references in structured data.

**Perplexity**

Perplexity heavily rewards sourceable, citation-friendly pages. Current pages need more primary-source links, dated updates, external references, and comparison tables.

**Gemini**

The site has good Google-crawl fundamentals, but needs stronger E-E-A-T, schema richness, and helpful-content depth.

**Bing Copilot**

Bingbot is allowed. Add IndexNow and Bing Webmaster submission workflow for freshness, especially because the site is new and changes often.

---

## Quick Wins (Implement This Week)

1. Add visible "Last updated" dates and JSON-LD `dateModified` to the homepage, docs, and all topic pages.
2. Enrich `SoftwareApplication` JSON-LD with `softwareVersion`, `downloadUrl`, `codeRepository`, `license`, `offers.price: 0`, `featureList`, and `screenshot`.
3. Add `FAQPage` schema to `www/docs/index.html` using the existing visible FAQ.
4. Add `<lastmod>` to `www/sitemap.xml` and remove `robots.txt` from it.
5. Create an `/about/` page with Max Howell founder credentials, Homebrew creation context, GitHub/profile links, product rationale, and structured `Person`/`Organization` links.
6. Add one 120-160 word direct-answer block and 4-6 FAQs to the highest-value topic pages.
7. Add citations to primary sources on comparison and credential pages: HashiCorp Vault docs, AWS credential docs, GitHub CLI auth docs, MCP docs, macOS Keychain docs, and Automic Vault source files.
8. Configure security response headers in the CloudFront deployment path.
9. Replace relative homepage `og:image` with `https://www.automicvault.com/preview.jpg`.
10. Fix the typo "intimiately" and soften unsupported absolutes unless they are backed by source-level evidence.

## 30-Day Action Plan

### Week 1: Metadata and Crawl Signals

- [ ] Add `lastmod` values to every sitemap URL.
- [ ] Remove `robots.txt` from the sitemap.
- [ ] Add visible "Last updated" dates to HTML pages.
- [ ] Add `dateModified`, `datePublished`, `headline`, `author`, and `image` to Article/TechArticle JSON-LD.
- [ ] Add richer `SoftwareApplication` schema fields.
- [ ] Add `FAQPage` schema for the docs FAQ.
- [ ] Make homepage `og:image` absolute.

### Week 2: Authority and Trust

- [ ] Add `/about/` with Max Howell founder, Homebrew, maintainer, and product authority signals.
- [ ] Add `/security/` with responsible disclosure, threat model summary, and source links.
- [ ] Add `/privacy/` and `/terms/` if downloads, analytics, or support flows collect user data.
- [ ] Add a contact/support path.
- [ ] Add `founder`, `Person`, `knowsAbout`, and expanded `sameAs` schema for real profiles and stable public references.

### Week 3: Content Depth

- [ ] Expand `/secrets-manager-for-ai-agents/` to 800-1,200 words with direct answer, threat model, examples, limitations, and citations.
- [ ] Expand `/stop-ai-agents-reading-env-files/` with concrete before/after dotenv workflows.
- [ ] Expand `/github-cli-token-security-ai-agents/` with GitHub CLI auth source links and safe workflow examples.
- [ ] Expand `/hashicorp-vault-for-ai-agents/` with a sourced comparison table and clear "use both" guidance.
- [ ] Add page-specific conclusions instead of repeated generic endings.

### Week 4: External Entity Building

- [ ] Polish the GitHub org profile and repo topics.
- [ ] Publish a short demo video and link it from the site.
- [ ] Create or complete LinkedIn company/profile references if appropriate.
- [ ] Launch a discussion/post on a relevant developer/security venue and link it from `llms.txt` when stable.
- [ ] Add IndexNow key and submission workflow for updated URLs.
- [ ] Submit the sitemap in Google Search Console and Bing Webmaster Tools.

---

## Appendix: Pages Analyzed

| URL | Title | GEO Issues |
|---|---|---:|
| https://www.automicvault.com/ | Automic Vault \| AI Agent Security for macOS | 8 |
| https://www.automicvault.com/docs/ | Automic Vault CLI Docs | 6 |
| https://www.automicvault.com/secrets-manager-for-ai-agents/ | Secrets Manager for AI Agents \| Automic Vault | 6 |
| https://www.automicvault.com/stop-ai-agents-reading-env-files/ | Stop AI Agents Reading .env Files \| Automic Vault | 6 |
| https://www.automicvault.com/api-key-management-for-ai-agents/ | API Key Management for AI Coding Agents \| Automic Vault | 6 |
| https://www.automicvault.com/hashicorp-vault-for-ai-agents/ | HashiCorp Vault vs Automic Vault for AI Agent Security | 7 |
| https://www.automicvault.com/mcp-secrets-management/ | MCP Secrets Management for AI Agents \| Automic Vault | 6 |
| https://www.automicvault.com/privileged-access-management-for-ai-agents/ | Privileged Access Management for AI Agents \| Automic Vault | 6 |
| https://www.automicvault.com/ai-agent-approval-gates/ | AI Agent Approval Gates \| Automic Vault | 6 |
| https://www.automicvault.com/secure-aws-cli-credentials-ai-agents/ | Secure AWS CLI Credentials for AI Agents \| Automic Vault | 6 |
| https://www.automicvault.com/github-cli-token-security-ai-agents/ | GitHub CLI Token Security for AI Agents \| Automic Vault | 6 |
| https://www.automicvault.com/secret-scanner-for-ai-agents/ | AI Agent Secret Scanner \| Automic Vault | 6 |
| https://www.automicvault.com/av-trace/ | av trace \| Trace Shell Installers Before AI Agents Run Them | 6 |
| https://www.automicvault.com/secret-scanning-vs-agent-secret-protection/ | Secret Scanning vs Agent Secret Protection \| Automic Vault | 6 |
| https://www.automicvault.com/llms.txt | AI summary file | 1 |
| https://www.automicvault.com/pricing.md | Markdown pricing page | 3 |
| https://www.automicvault.com/robots.txt | Robots policy | 0 |

## Sources Consulted

- Local source files under `www/`, especially `index.html`, `docs/index.html`, `robots.txt`, `sitemap.xml`, `llms.txt`, and `pricing.md`.
- Live response headers from `https://www.automicvault.com/` and `https://www.automicvault.com/assets/icon@2x.webp`.
- GitHub repository metadata for `https://github.com/automic-vault/automic-vault`.
- Search result evidence for `https://mxcl.dev/` and exact brand/domain queries.
- Public references for Max Howell and Homebrew: `https://brew.sh/`, `https://en.wikipedia.org/wiki/Homebrew_(package_manager)`, and `https://workbrew.com/homebrew-turns-15`.
- Google Search Central: `SoftwareApplication`, `Article`, `Organization`, and `robots.txt` documentation.
- IndexNow documentation for URL update submission and key verification.
