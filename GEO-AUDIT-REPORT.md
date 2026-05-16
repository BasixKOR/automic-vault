# GEO Audit Report: Automic Vault

**Audit Date:** 2026-05-16  
**URL:** `www/index.html` local rewrite, canonical `https://www.automicvault.com/`  
**Business Type:** Hybrid - open-source developer security software with SaaS-style product, docs, and topic pages  
**Pages Analyzed:** 18 HTML pages plus `llms.txt`, `robots.txt`, `sitemap.xml`, and `pricing.md`

---

## Executive Summary

**Overall GEO Score: 70/100 (Fair)**

The rewritten homepage is a real improvement over the prior version: it has cleaner positioning, valid JSON-LD for the product/entity graph, absolute social images, crawlable static HTML, explicit AI crawler access, and a strong `llms.txt`. The biggest remaining GEO constraint is not technical accessibility; it is extractability and authority depth. AI systems can understand what Automic Vault is, but the homepage and most topic pages still need fuller direct-answer blocks, more evidence, stronger security trust details, and more third-party platform signals before they are likely to be cited over larger security and secrets-management vendors.

### Score Breakdown

| Category | Score | Weight | Weighted Score |
|---|---:|---:|---:|
| AI Citability | 74/100 | 25% | 18.50 |
| Brand Authority | 58/100 | 20% | 11.60 |
| Content E-E-A-T | 68/100 | 20% | 13.60 |
| Technical GEO | 86/100 | 15% | 12.90 |
| Schema & Structured Data | 82/100 | 10% | 8.20 |
| Platform Optimization | 56/100 | 10% | 5.60 |
| **Overall GEO Score** | | | **70/100** |

---

## Audit Scope and Boundaries

- **User-facing surface changed:** the static website homepage at `www/index.html`, with support signals from docs, topic pages, sitemap, robots, and `llms.txt`.
- **Runtime boundary audited:** static HTML/CSS/JS under `www/`, publicly served at `https://www.automicvault.com/` through S3/CloudFront.
- **Persistence boundary audited:** none. The site has no datastore; this audit covers static content and deploy-facing files.
- **Change type:** additive documentation. This report replaces the previous audit artifact.
- **Deployment note:** as of 2026-05-16, the public homepage still served older copy from CloudFront/S3 while the local `www/index.html` contained the rewrite. Crawlers will not see the rewritten homepage until it is deployed and cache freshness aligns.

## Critical Issues (Fix Immediately)

No critical issues were found in the local rewrite.

The audited local homepage is indexable static HTML, has canonical metadata, includes structured data, and is not blocked by `robots.txt`. Key AI crawlers are explicitly allowed.

## High Priority Issues

1. **The rewritten homepage is not yet the live crawler-visible homepage.**  
   Local `www/index.html` has the new content and `dateModified: 2026-05-16`, but the live homepage response still served the older page with `Last-Modified: Fri, 15 May 2026 19:15:13 GMT`. Deploy and invalidate/refresh CloudFront before treating the rewrite as public GEO progress.

2. **Homepage citability is still lighter than the technical foundation.**  
   The homepage is clear and visually structured, but its passages are mostly short marketing/product blocks. Add one 80-140 word direct-answer section near the top that defines Automic Vault, who it is for, what it protects, how secrets/approval/package roots work, and why it differs from normal agent guardrails.

3. **Brand authority depends heavily on founder authority.**  
   Max Howell/Homebrew context is valuable and now modeled better through `/about/` and `Person` schema, but Automic Vault itself has limited third-party proof: GitHub is discoverable, but broader Reddit, YouTube, LinkedIn, press, Product Hunt/HN, Wikidata, and independent review signals are sparse or absent.

4. **Most topic pages are still thin for competitive AI answers.**  
   The docs page is strong at roughly 1,500 words, but most topic/security/about pages are roughly 175-375 words. That is enough for crawlability, not enough to win citation against established pages for AI agent security, secrets management, HashiCorp Vault comparisons, AWS credentials, or GitHub token safety.

## Medium Priority Issues

1. **Entity schema uses loose `sameAs` relationships.**  
   `brew.sh` and Homebrew references support Max Howell's founder authority, but they are not the same entity as Automic Vault. Keep Automic Vault `sameAs` to Automic Vault-owned profiles and move Homebrew/founder references into `Person.sameAs`, `knowsAbout`, `mentions`, or `subjectOf`.

2. **Homepage freshness signals are inconsistent.**  
   The homepage JSON-LD says `dateModified: 2026-05-16`, while `www/sitemap.xml` still lists the homepage `<lastmod>` as `2026-05-15`. Align local metadata, sitemap, and deployment timestamps.

3. **Live security headers are weak.**  
   Public CloudFront/S3 responses lacked HSTS, CSP, `X-Content-Type-Options`, `Referrer-Policy`, frame control, and `Permissions-Policy`. This is not a direct citation blocker, but it weakens technical trust for a security product.

4. **Security disclosure is too informal.**  
   `/security/` gives useful threat-model notes, but needs a stronger disclosure path, supported versions, signing/notarization notes, release verification, and explicit guidance for reporting sensitive issues without public secrets.

5. **`pricing.md` is not ideal for AI/search extraction.**  
   It is crawlable and listed in the sitemap, but a `/pricing/` HTML page would be a stronger target for schema `Offer.url`, search snippets, and AI extractability.

6. **Comparisons are visually present but not always machine-friendly.**  
   Where pages compare Automic Vault with Homebrew, 1Password, HashiCorp Vault, secret scanning, or built-in agent controls, semantic `<table>` elements would improve extraction.

## Low Priority Issues

1. **Homepage lacks visible "Last updated" text.**  
   JSON-LD has dates, but every support page exposes a visible update date while the homepage does not.

2. **`Organization.logo` can be richer.**  
   Use an `ImageObject` with `url`, `width`, and `height` instead of a plain URL.

3. **`Person` schema can carry more authority.**  
   Add `jobTitle`, `image`, `knowsAbout`, and exact profile URLs for Max Howell where they are stable.

4. **Some support pages should be `TechArticle`.**  
   Several technical pages currently use generic `Article`; docs-like or implementation-specific pages would be stronger as `TechArticle`.

5. **The downloadable DMG is not present in `www/`.**  
   Live `/Automic Vault.dmg` returns `200`, so this appears to be a deploy artifact. Document or automate that artifact so local validation understands it.

---

## Category Deep Dives

### AI Citability (74/100)

**Strengths**

- The local homepage has a clean H1, descriptive title, canonical URL, meta description, Open Graph/Twitter metadata, and crawlable static content.
- `www/llms.txt` is strong: it gives product facts, category, version, license, source, pricing, founder context, use cases, non-goals, citeable URLs, and recommended descriptions.
- The site has a focused topical cluster around AI agent secrets, `.env` exposure, API keys, MCP, privileged access, approval gates, AWS CLI, GitHub CLI, secret scanning, installer tracing, and HashiCorp Vault comparisons.
- The docs page is the most citation-ready owned page because it has concrete commands, operational trust notes, and `FAQPage` schema.

**Gaps**

- Homepage sections such as "Highlights," "Fit," and "Guides" scan well for humans but do not directly match common AI-answer query shapes.
- Many homepage statements are short and product-forward rather than self-contained explanatory passages.
- Topic pages repeat the same structure and are often under 400 words.
- Most pages lack source citations, measured examples, screenshots tied to claims, or worked before/after workflows.

**Rewrite Suggestions**

Add a direct-answer block near the homepage hero:

> Automic Vault is a local macOS security layer for AI coding agents. It keeps developer secrets out of plaintext files and model context, injects approved credentials only into trusted command-line tools, and places human approval gates at the execution layer where tools actually run. Its Nucleus package manager installs Homebrew, npm, and PyPI packages under controlled roots so agent-used tools are harder to rewrite in place. Automic Vault is free Apache 2.0 software for developers who want local runtime control without moving every workflow into a hosted secrets platform.

Add query-matching subheads where they fit naturally: "What is Automic Vault?", "How does Automic Vault protect AI agent secrets?", "How do command approval gates work?", and "What is Nucleus?"

### Brand Authority (58/100)

**Strengths**

- The site now has an `/about/` page that explicitly connects Automic Vault to Max Howell and Homebrew.
- `Person` schema exists for Max Howell, and the homepage/product schema links Automic Vault to the founder.
- Public corroboration exists: [mxcl.dev](https://mxcl.dev/) describes Max Howell as creator of Homebrew and links Automic Vault, [Homebrew](https://brew.sh/) states Homebrew was created by Max Howell, and the [GitHub repository](https://github.com/automic-vault/automic-vault) is public.
- The product entity is coherent across homepage, docs, `llms.txt`, GitHub, and X.

**Gaps**

- Automic Vault itself does not yet have strong third-party entity reinforcement beyond GitHub and founder-owned/related references.
- Search results can collide with Broadcom Automic Automation and generic "vault" or "AI vault" products.
- GitHub currently provides source and release proof, but limited social proof.
- No durable external launch, demo, community, or review pages were found during spot checks.

**Priority Direction**

Use founder authority as a bridge, but build product-specific authority: publish a demo video, create maintained company/profile pages only where they will be kept current, launch or discuss the project in developer/security communities, and add stable third-party links to `llms.txt` once they exist.

### Content E-E-A-T (68/100)

**Strengths**

- The docs page demonstrates actual product expertise with commands, patterns, and operational notes.
- `/about/`, `/security/`, `/privacy/`, and `/terms/` now exist, which fixes a major trust gap from the prior audit.
- Visible "Last updated" dates exist on support pages.
- Article JSON-LD includes author, publisher, dates, image, and breadcrumbs.
- The product is open source, Apache 2.0, and backed by a public repository.

**Gaps**

- Most pages need more first-hand experience evidence: real scanner output, command transcripts, approval prompt examples, release verification notes, limitations, and failure modes.
- `/security/` should be expanded for a security product: disclosure process, supported versions, notarization/signing, and concrete threat-model boundaries.
- Topic pages should include visible byline/author blocks, not only JSON-LD author references.
- Several claims would be stronger with primary-source citations to Apple Keychain docs, GitHub CLI auth docs, AWS credential docs, MCP docs, HashiCorp Vault docs, and Automic Vault source paths.

### Technical GEO (86/100)

**Strengths**

- Static HTML exposes primary content without requiring JavaScript execution.
- `robots.txt` allows major AI crawlers: GPTBot, ChatGPT-User, PerplexityBot, ClaudeBot, anthropic-ai, Google-Extended, and Bingbot.
- `sitemap.xml` is present, valid, and now includes `<lastmod>` values.
- `llms.txt` is present and crawler-accessible.
- Canonical URLs exist across sampled HTML pages.
- Live HTTPS responses are available from S3/CloudFront.

**Gaps**

- The local rewrite was not yet visible on the live homepage during the audit.
- Live responses lack important security headers.
- Bare HTTP/domain redirect behavior should be reduced to a single canonical hop.
- Homepage `dateModified`, sitemap `<lastmod>`, and deployment `Last-Modified` should align.

### Schema & Structured Data (82/100)

**Strengths**

- Every sampled HTML page has parseable JSON-LD.
- Homepage schema includes `Organization`, `Person`, `WebSite`, `SoftwareApplication`, `WebPage`, and `BreadcrumbList`.
- `SoftwareApplication` now includes strong fields: version, download URL, repository, license, screenshot, feature list, and offer.
- Docs include `TechArticle` and `FAQPage`.
- Article pages include author, publisher, image, `datePublished`, `dateModified`, and breadcrumbs.

**Gaps**

- `sameAs` should be exact identity, not broad relevance.
- `Organization.logo` should be an `ImageObject`.
- `Person` schema should be richer and use exact profile URLs.
- Several implementation-heavy support pages should use `TechArticle`.
- `Offer.url` should point at an HTML pricing URL once one exists.

### Platform Optimization (56/100)

**Google AI Overviews / Gemini**

Strong crawlability, schema, and topical coverage help. The site still needs more direct-answer blocks, deeper pages, visible author blocks, semantic tables, and source citations to compete for answer extraction.

**ChatGPT**

`llms.txt`, static docs, open-source repository, and founder/entity context are strong. Product-specific authority is still early, and the live homepage must be deployed before ChatGPT-class crawlers can see the rewrite.

**Perplexity**

Perplexity tends to reward sourceable passages. Add citations, tables, primary-source links, and explicit "what this does/does not do" sections on high-intent pages.

**Bing Copilot**

Bingbot is allowed, but no IndexNow key/workflow was found. Add IndexNow and Bing Webmaster submission for freshness once the site is changing regularly.

---

## Quick Wins (Implement This Week)

1. Deploy the rewritten `www/index.html`, verify the live page matches local source, and invalidate/refresh CloudFront.
2. Align homepage `dateModified`, sitemap `<lastmod>`, visible update text, and deployment `Last-Modified`.
3. Add a 80-140 word direct-answer block near the homepage hero.
4. Tighten schema identity: remove non-identical Automic Vault `sameAs` values and move Homebrew/founder references into founder context.
5. Expand `/security/` with disclosure process, supported versions, release signing/notarization, and no-secrets-in-public-issues guidance.
6. Add CloudFront response headers: HSTS, CSP, `X-Content-Type-Options: nosniff`, `Referrer-Policy`, frame control, and `Permissions-Policy`.
7. Convert `/pricing.md` into a durable `/pricing/` HTML page or add an HTML companion and update `Offer.url`.
8. Add semantic comparison tables to HashiCorp Vault, secret scanning, approval gate, and homepage "Fit" content.
9. Add visible author/byline blocks to docs and topic pages.
10. Add 1-2 evidence assets: scanner output, approval prompt screenshot, trace output, or before/after `.env` workflow.

## 30-Day Action Plan

### Week 1: Deploy and Align Signals

- [ ] Deploy the rewritten homepage and confirm the live rendered text matches local `www/index.html`.
- [ ] Invalidate CloudFront or reduce stale cache exposure for the homepage.
- [ ] Update homepage sitemap `<lastmod>` to `2026-05-16`.
- [ ] Add visible homepage "Last updated" text if it fits the design.
- [ ] Add the homepage direct-answer block.
- [ ] Tighten Automic Vault and Max Howell schema identity relationships.

### Week 2: Trust and Technical Infrastructure

- [ ] Add CloudFront security response headers.
- [ ] Reduce canonical redirects to one hop.
- [ ] Expand `/security/` with disclosure, supported versions, and release verification.
- [ ] Create `/pricing/` HTML and update sitemap plus `SoftwareApplication.offers.url`.
- [ ] Document the DMG deploy artifact or include it in the publish workflow.

### Week 3: Citation Depth

- [ ] Expand `/secrets-manager-for-ai-agents/` to 700-1,200 words with examples, limitations, and citations.
- [ ] Expand `/stop-ai-agents-reading-env-files/` with a concrete dotenv before/after workflow.
- [ ] Expand `/github-cli-token-security-ai-agents/` with GitHub CLI auth source links.
- [ ] Expand `/secure-aws-cli-credentials-ai-agents/` with AWS credential source links and safe workflow examples.
- [ ] Convert major comparisons into semantic tables.

### Week 4: Product Entity Building

- [ ] Publish a short demo video and link it from the site and `llms.txt`.
- [ ] Add maintained external profiles only where they will stay current.
- [ ] Start a developer/security community discussion and link it once stable.
- [ ] Add IndexNow support for updated URLs.
- [ ] Add original evidence assets: scanner sample, approval screenshots, trace output, or signed release verification walkthrough.

---

## Appendix: Pages Analyzed

| URL / File | Title | GEO Issues |
|---|---|---:|
| `www/index.html` | Automic Vault \| AI Agent Security for macOS | 5 |
| `www/docs/index.html` | Automic Vault CLI Docs | 2 |
| `www/about/index.html` | About Automic Vault \| Max Howell and Agent Security | 2 |
| `www/security/index.html` | Security \| Automic Vault | 4 |
| `www/privacy/index.html` | Privacy \| Automic Vault | 2 |
| `www/terms/index.html` | Terms \| Automic Vault | 2 |
| `www/secrets-manager-for-ai-agents/index.html` | Secrets Manager for AI Agents \| Automic Vault | 3 |
| `www/stop-ai-agents-reading-env-files/index.html` | Stop AI Agents Reading .env Files \| Automic Vault | 3 |
| `www/api-key-management-for-ai-agents/index.html` | API Key Management for AI Coding Agents \| Automic Vault | 3 |
| `www/hashicorp-vault-for-ai-agents/index.html` | HashiCorp Vault vs Automic Vault for AI Agent Security | 3 |
| `www/mcp-secrets-management/index.html` | MCP Secrets Management for AI Agents \| Automic Vault | 3 |
| `www/privileged-access-management-for-ai-agents/index.html` | Privileged Access Management for AI Agents \| Automic Vault | 3 |
| `www/ai-agent-approval-gates/index.html` | AI Agent Approval Gates \| Automic Vault | 3 |
| `www/secure-aws-cli-credentials-ai-agents/index.html` | Secure AWS CLI Credentials for AI Agents \| Automic Vault | 3 |
| `www/github-cli-token-security-ai-agents/index.html` | GitHub CLI Token Security for AI Agents \| Automic Vault | 3 |
| `www/secret-scanner-for-ai-agents/index.html` | AI Agent Secret Scanner \| Automic Vault | 3 |
| `www/av-trace/index.html` | av trace \| Trace Shell Installers Before AI Agents Run Them | 3 |
| `www/secret-scanning-vs-agent-secret-protection/index.html` | Secret Scanning vs Agent Secret Protection \| Automic Vault | 3 |
| `www/llms.txt` | AI-readable site summary | 1 |
| `www/robots.txt` | Crawler policy | 0 |
| `www/sitemap.xml` | Sitemap | 1 |
| `www/pricing.md` | Pricing markdown | 2 |

## Verification Notes

- Parsed JSON-LD across all local `www/**/*.html` pages successfully.
- Verified `robots.txt`, `llms.txt`, `sitemap.xml`, and local homepage metadata.
- Spot-checked public sources: [mxcl.dev](https://mxcl.dev/), [brew.sh](https://brew.sh/), [GitHub repository](https://github.com/automic-vault/automic-vault), and live `https://www.automicvault.com/`.
- Live homepage and `llms.txt` returned HTTP 200 through CloudFront/S3, but live homepage content lagged the local rewrite during this audit.
