# GEO Audit Report: Automic Vault

**Audit Date:** 2026-05-24  
**URL:** https://www.automicvault.com/  
**Business Type:** Hybrid - open-source developer security software with package registry/content publisher surface  
**Pages Analyzed:** 33 HTML pages plus `robots.txt`, `llms.txt`, `llms-full.txt`, `sitemap.xml`, `pkg/sitemap.xml`, and `pricing.md`

---

## Executive Summary

**Overall GEO Score: 75/100 (Good)**

Automic Vault now has a strong GEO foundation: the deployed site is static, indexable, schema-rich, AI-crawler friendly, and exposes both concise and full-text LLM entry points. The biggest constraints are no longer crawlability; they are authority depth, stale product version signals, and thin proof-oriented content on security, terms, privacy, and comparison pages.

### Score Breakdown

| Category | Score | Weight | Weighted Score |
|---|---:|---:|---:|
| AI Citability | 78/100 | 25% | 19.50 |
| Brand Authority | 63/100 | 20% | 12.60 |
| Content E-E-A-T | 72/100 | 20% | 14.40 |
| Technical GEO | 91/100 | 15% | 13.65 |
| Schema & Structured Data | 88/100 | 10% | 8.80 |
| Platform Optimization | 60/100 | 10% | 6.00 |
| **Overall GEO Score** | | | **75/100** |

---

## Implementation Follow-Up (2026-05-24)

Implemented from this report with local source data:

- Product version fields are now deploy-templated from `Cargo.toml`, so static schema and `llms.txt` stamp to the current app version during deploy.
- `llms-full.txt` generation now excludes the full package corpus.
- Added `/pricing/` as an HTML pricing page while retaining `pricing.md` as the markdown/plain-text variant.
- Added `/.well-known/security.txt`.
- Expanded `/security/`, `/privacy/`, and `/terms/` with direct-answer blocks and more trust/privacy detail.
- Added semantic comparison tables to the HashiCorp Vault, secret-scanning, privacy, terms, pricing, and security pages.
- Aligned visible freshness and sitemap lastmod values to May 24, 2026 for the touched top-level pages.
- Added package category hubs for cloud CLIs, source-control tools, package publishers, MCP tools, and secret-risk packages.
- Added reviewer metadata to static topic-page schema and generated package/hub schema.
- Expanded generated package-page Twitter Card metadata with title, description, and image fields.
- Added FAQ blocks and `FAQPage` schema to the strongest AI-agent secrets, API key, MCP, approval-gate, and comparison pages.

Still not fully addressed:

- Product-level third-party authority and platform presence remain external growth work.
- npm/PyPI package-page enrichment parity remains a separate data-generation phase.

---

## Critical Issues (Fix Immediately)

No critical issues were found.

The deployed site is crawlable static HTML, has canonical tags, uses structured data, exposes sitemaps, and explicitly allows major AI crawlers in `robots.txt`.

## High Priority Issues

1. **Product version signals were inconsistent in the deployed audit sample.**  
   The audit found stale `1.6.0` values in `www/llms.txt` and homepage `SoftwareApplication` schema while `Cargo.toml` and public Git tags showed `1.9.0`. The worktree now templates these fields from `Cargo.toml` during deploy.

2. **Brand authority is still founder-led rather than product-led.**  
   The Max Howell/Homebrew graph is strong and useful, but third-party signals for Automic Vault itself are still sparse. Search results include the site, GitHub, and `mxcl.dev`, but also collide with unrelated Automic/Broadcom and Automic Group entities.

3. **Trust pages are too thin for a security product.**  
   `/security/` is 265 words, `/privacy/` is 237 words, and `/terms/` is 178 words in the deployed output. They are crawlable and well structured, but too brief to carry security, privacy, disclosure, notarization, signing, data handling, and open-source trust claims in AI answers.

4. **Most non-package topic pages need more quotable direct-answer blocks.**  
   The pages are clear, but many sit around 300-380 words. For competitive queries such as "secrets manager for AI agents" or "MCP secrets management", add 80-140 word answer blocks that define the problem, say who the page is for, state how Automic Vault works, and name concrete alternatives or complements.

## Medium Priority Issues

1. **`pricing.md` is a weaker citation target than `/pricing/`.**  
   It is valid crawlable markdown and linked from `llms.txt`, but schema `Offer.url` and AI/search snippets would be stronger with an HTML `/pricing/` page that also links the markdown/plain-text variant.

2. **Package pages are strong for Homebrew but thinner for npm and PyPI.**  
   Homebrew pages now include version, license, dependencies, bottle/source/install behavior, and richer security notes. npm/PyPI pages still frequently show "No radioisotope coverage found yet" and lack equivalent registry depth.

3. **Schema is broad, but article authorship is under-modeled.**  
   The site has Organization, Person, WebSite, SoftwareApplication, WebPage, Article, TechArticle, FAQPage, HowTo, and BreadcrumbList. Add explicit `author`, `reviewedBy`, `dateModified`, and `about` links on topic pages where the founder/security expertise is part of the trust argument.

4. **Comparison content is mostly prose, not extractable tables.**  
   Pages comparing Automic Vault with HashiCorp Vault, secret scanning, agent controls, and package managers would be easier for AI systems to quote if the key distinctions were repeated in semantic tables.

5. **The package-page index is huge but not summarized by category hubs.**  
   `pkg/sitemap.xml` lists 8,789 package URLs. Add crawlable hubs for high-value groups such as cloud CLIs, package publishers, source-control tools, MCP tools, and secret-risk packages.

## Low Priority Issues

1. **Package page Twitter Card metadata is lighter than top-level pages.**  
   Package pages include `twitter:card` but not the fuller title/description/image set used on top-level pages.

2. **Some pages could expose visible freshness.**  
   The content has schema dates and `llms.txt` facts, but visible "Last updated" lines would improve citation confidence.

3. **No dedicated `security.txt` was observed in the deployed artifact.**  
   The `/security/` page has reporting guidance, but a root `/.well-known/security.txt` would be a useful trust signal.

---

## Category Deep Dives

### AI Citability (78/100)

Strengths:

- Homepage has concise, quotable lines: "A hardened package manager and secrets boundary for the tools AI agents run on your Mac."
- `llms.txt` gives AI systems a clean product summary, use cases, exclusions, high-value links, and a pointer to `llms-full.txt`.
- Package pages now provide consistent install commands, package metadata, security notes, and install behavior.
- `llms-full.txt` is a strong ingestion artifact for non-browsing LLM workflows.

Gaps:

- Many topic pages are still short. They explain the idea but do not yet dominate the answer for broad category queries.
- Some claims need explicit evidence blocks: "how it works", "what data stays local", "what an approved tool receives", "what the model cannot read".
- Top pages should add short "answer box" sections that can stand alone when extracted.

### Brand Authority (63/100)

Strengths:

- Public GitHub repo is discoverable, has Apache-2.0 licensing, release tags through `v1.9.0`, and a public source graph AI systems can corroborate.
- `mxcl.dev` connects Max Howell, Homebrew, and Automic Vault in a coherent public identity graph.
- Site schema connects Automic Vault to Organization, SoftwareApplication, WebSite, Person, and founder context.

Gaps:

- Product-level mentions outside owned properties are still limited.
- Search results for "Automic Vault" include unrelated Automic/Broadcom/Automic Group results, so entity disambiguation matters.
- No strong evidence found for Wikipedia/Wikidata/Product Hunt/Hacker News/Reddit/YouTube/LinkedIn company coverage for the product itself.

### Content E-E-A-T (72/100)

Strengths:

- Founder authority is unusually strong for a developer tool because the Homebrew connection is explicit and externally verifiable.
- Docs page is substantial at about 1,570 words and explains command surfaces, runtime boundaries, and operational trust notes.
- Topic pages are focused and avoid generic SEO filler.

Gaps:

- Security and legal/trust pages are short for a security product.
- Add explicit release verification, signing/notarization, disclosure, supported versions, threat model, and local-data statements.
- Add author/reviewer attribution to technical articles.

### Technical GEO (91/100)

Strengths:

- `robots.txt` explicitly allows `GPTBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `anthropic-ai`, `Google-Extended`, and `Bingbot`.
- `robots.txt` lists both `sitemap.xml` and `pkg/sitemap.xml`.
- `sitemap.xml` has 21 top-level URLs; `pkg/sitemap.xml` has 8,789 package URLs.
- The deploy config attaches HSTS, CSP, `X-Content-Type-Options`, `X-Frame-Options: DENY`, `Referrer-Policy`, XSS protection, and a `Permissions-Policy`.
- The site is static HTML with canonical URLs, Open Graph tags, and crawlable text.

Gaps:

- Direct header verification from this environment was blocked by local direct-curl policy, so header findings are based on deployed script configuration and deploy success output.
- Consider adding `/.well-known/security.txt`.

### Schema & Structured Data (88/100)

Strengths:

- Homepage includes Organization, Person, WebSite, SoftwareApplication, WebPage, and BreadcrumbList.
- Docs include FAQPage and TechArticle.
- Package pages include WebSite, SoftwareApplication, TechArticle, BreadcrumbList, and HowTo.
- Schema parsing found no invalid JSON-LD in the audited pages.

Gaps:

- Product version is stale in schema.
- Article pages need richer author/reviewer/publisher relationships.
- Package pages could add `softwareRequirements`, `programmingLanguage`, `codeRepository`, and stronger `sameAs` where known.

### Platform Optimization (60/100)

Strengths:

- GitHub repository and `mxcl.dev` are strong developer-platform signals.
- X profile is linked in site schema.
- Package pages cover high-intent install queries across Homebrew, npm, and PyPI.

Gaps:

- No dedicated LinkedIn company page, YouTube demos, Product Hunt launch, Wikipedia/Wikidata entity, or strong Reddit/HN discussion surface was observed.
- Product brand is still new relative to broader "Automic" entity collisions.

---

## Quick Wins (Implement This Week)

1. Update all site/LLM/schema version facts from `1.6.0` to the current release, or automate version injection from release metadata.
2. Add a `/pricing/` HTML page and keep `pricing.md` as the AI/plain-text variant.
3. Expand `/security/` with disclosure, supported versions, signing/notarization, release verification, and local data handling.
4. Add visible "Last updated" lines to homepage, docs, and topic pages.
5. Add semantic comparison tables to HashiCorp Vault, secret scanning, approval gates, and package manager pages.

## 30-Day Action Plan

### Week 1: Freshness and Trust

- [ ] Fix version drift across homepage schema, `llms.txt`, `llms-full.txt`, download page, and GitHub release references.
- [ ] Add `/pricing/` HTML and update schema `Offer.url`.
- [ ] Add `/.well-known/security.txt`.
- [ ] Expand `/security/` to at least 700-1,000 words with concrete reporting and verification details.

### Week 2: Citability Blocks

- [ ] Add a direct-answer block to each top-level topic page.
- [ ] Add "Automic Vault vs ..." comparison tables where the page already implies a comparison.
- [ ] Add FAQ blocks to the strongest category pages and mirror them in FAQPage schema.
- [ ] Add author/reviewer metadata to article schemas.

### Week 3: Product Authority

- [ ] Create or complete public product profiles on LinkedIn, YouTube, Product Hunt, and relevant developer directories.
- [ ] Publish one technical launch/demo post that other sites can cite.
- [ ] Add a concise press/about kit page with product facts, founder facts, screenshots, and canonical descriptions.
- [ ] Disambiguate Automic Vault from unrelated Automic/Broadcom/Automic Group entities in schema and copy.

### Week 4: Package Content Hubs

- [ ] Add package hubs for cloud CLIs, source-control tools, package publishers, MCP tools, and secret-risk packages.
- [ ] Add npm/PyPI enrichment parity planning pages or start an equivalent registry-depth pass.
- [ ] Add "Top secured packages" and "Known risky command families" pages.
- [ ] Add internal links from relevant topic pages into package hubs and high-risk package pages.

---

## Appendix: Pages Analyzed

| URL | Title | GEO Issues |
|---|---|---:|
| https://www.automicvault.com/ | Automic Vault | From the creator of Homebrew | 1 |
| https://www.automicvault.com/docs/ | Automic Vault CLI Docs | 1 |
| https://www.automicvault.com/about/ | About Automic Vault | From the creator of Homebrew | 1 |
| https://www.automicvault.com/security/ | Security | Automic Vault | 3 |
| https://www.automicvault.com/privacy/ | Privacy | Automic Vault | 2 |
| https://www.automicvault.com/terms/ | Terms | Automic Vault | 2 |
| https://www.automicvault.com/download/ | Download Automic Vault for macOS | 1 |
| https://www.automicvault.com/secrets-manager-for-ai-agents/ | Secrets Manager for AI Agents | Automic Vault | 1 |
| https://www.automicvault.com/stop-ai-agents-reading-env-files/ | Stop AI Agents Reading .env Files | Automic Vault | 1 |
| https://www.automicvault.com/api-key-management-for-ai-agents/ | API Key Management for AI Coding Agents | Automic Vault | 1 |
| https://www.automicvault.com/hashicorp-vault-for-ai-agents/ | HashiCorp Vault vs Automic Vault for AI Agent Security | 2 |
| https://www.automicvault.com/mcp-secrets-management/ | MCP Secrets Management for AI Agents | Automic Vault | 1 |
| https://www.automicvault.com/privileged-access-management-for-ai-agents/ | Privileged Access Management for AI Agents | Automic Vault | 1 |
| https://www.automicvault.com/ai-agent-approval-gates/ | AI Agent Approval Gates | Automic Vault | 1 |
| https://www.automicvault.com/secure-aws-cli-credentials-ai-agents/ | Secure AWS CLI Credentials for AI Agents | Automic Vault | 1 |
| https://www.automicvault.com/github-cli-token-security-ai-agents/ | GitHub CLI Token Security for AI Agents | Automic Vault | 1 |
| https://www.automicvault.com/secret-scanner-for-ai-agents/ | AI Agent Secret Scanner | Automic Vault | 1 |
| https://www.automicvault.com/av-trace/ | av trace | Trace Shell Installers Before AI Agents Run Them | 1 |
| https://www.automicvault.com/secret-scanning-vs-agent-secret-protection/ | Secret Scanning vs Agent Secret Protection | Automic Vault | 1 |
| https://www.automicvault.com/pkg/brew/awscli/ | Install awscli with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/curl/ | Install curl with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/docker/ | Install docker with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/gh/ | Install gh with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/git/ | Install git with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/node/ | Install node with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/openssh/ | Install openssh with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/brew/ripgrep/ | Install ripgrep with Homebrew | Automic Vault | 0 |
| https://www.automicvault.com/pkg/npm/tsx/ | Install tsx with npm | Automic Vault | 1 |
| https://www.automicvault.com/pkg/npm/vercel/ | Install vercel with npm | Automic Vault | 1 |
| https://www.automicvault.com/pkg/npm/vite/ | Install vite with npm | Automic Vault | 1 |
| https://www.automicvault.com/pkg/npm/wrangler/ | Install wrangler with npm | Automic Vault | 1 |
| https://www.automicvault.com/pkg/pip/pgcli/ | Install pgcli with PyPI | Automic Vault | 1 |
| https://www.automicvault.com/pkg/pip/psycopg2/ | Install psycopg2 with PyPI | Automic Vault | 1 |

## Evidence Notes

- Live homepage was fetched through web search/open and showed deployed copy with package count, guides, and product claims.
- Live indexed search found Automic Vault topic pages, the public GitHub repository, and `mxcl.dev` founder/entity support.
- Local deployed artifact analysis covered `www/` after deployment because direct `curl` is blocked by local policy in this workspace.
- Generated asset counts observed locally: 8,808 `index.html` files, 21 top-level sitemap URLs, and 8,789 package sitemap URLs.
