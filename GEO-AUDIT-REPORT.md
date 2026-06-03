# GEO Audit Report: Automic Vault

**Audit Date:** May 31, 2026
**URL:** https://www.automicvault.com/
**Business Type:** SaaS / open-source developer security product
**Pages Analyzed:** 30 live URL fetches, 26 HTML pages scored

---

## Executive Summary

**Overall GEO Score: 72/100 (Fair, close to Good)**

Automic Vault has strong technical GEO foundations: AI crawler access is explicit, `llms.txt` exists, the sitemap is broad, pages render as static HTML, schema coverage is unusually good, and the package catalog creates a large crawlable surface. The main gap is not infrastructure. It is authority and message freshness: the live site still presents the older "From the creator of Homebrew" homepage, while the local site and brand work now say "A new kind of package manager for a new kind of threat model." AI systems will understand the product, but may cite it inconsistently until the sharper positioning, fresh version metadata, and stronger external entity signals are deployed.

### Score Breakdown

| Category | Score | Weight | Weighted Score |
|---|---:|---:|---:|
| AI Citability | 72/100 | 25% | 18.0 |
| Brand Authority | 60/100 | 20% | 12.0 |
| Content E-E-A-T | 71/100 | 20% | 14.2 |
| Technical GEO | 88/100 | 15% | 13.2 |
| Schema & Structured Data | 84/100 | 10% | 8.4 |
| Platform Optimization | 57/100 | 10% | 5.7 |
| **Overall GEO Score** | | | **72/100** |

### Live Site Delta

The live production page is behind the current local landing-page work.

- Live homepage H1: `Automic Vault`
- Live title: `Automic Vault | From the creator of Homebrew`
- Live `llms.txt` positioning: `From the creator of Homebrew`
- Current local H1: `A new kind of package manager for a new kind of threat model`
- Current local `llms.txt` positioning: `A new kind of package manager for a new kind of threat model`

Deploying the local landing page, `llms.txt`, translations, and sitemap changes is the fastest GEO improvement because it aligns human copy, AI-readable copy, and schema around the same product promise.

---

## Critical Issues (Fix Immediately)

None found.

The live site is indexable, has no domain-level `noindex`, returns 200 for key pages, serves static HTML, exposes `llms.txt`, and explicitly allows major AI crawlers in `robots.txt`.

---

## High Priority Issues

### 1. Live production copy is stale relative to current positioning

**Pages:** `/`, `/llms.txt`, homepage schema
**Evidence:** The live homepage still uses the generic H1 `Automic Vault` and the title `Automic Vault | From the creator of Homebrew`. The current local repo has the stronger Homebrew/dev-tool hazard positioning.

**Why it matters:** AI answer engines prefer stable, repeated entity descriptions. Right now the live homepage, live `llms.txt`, GitHub README, and current local landing page are not fully aligned.

**Fix:** Deploy the current landing page and mirror copy so these surfaces all say the same thing:

- Homepage H1 and title
- Meta description
- JSON-LD `WebPage.headline`
- `SoftwareApplication.description`
- `llms.txt`
- `llms-full.txt`
- README one-line description

### 2. Product-level brand authority is still mostly owned-channel authority

**Pages/platforms:** Google/Bing visible results, GitHub, mxcl.dev, X
**Evidence:** Search results for `"Automic Vault"` show the official site, package pages, mxcl.dev, GitHub, and many irrelevant "Automic" or "Atomic Vault" results. GitHub currently shows 18 stars and 1 fork. Founder authority is strong, but product authority is still early.

**Why it matters:** AI systems need entity disambiguation. The product name collides with Broadcom Automic, Automic Group, and generic "atomic vault" results.

**Fix:** Build more third-party entity anchors:

- Create and complete LinkedIn, GitHub org, X, and possibly YouTube profiles with the same description.
- Publish one founder-authored launch/explainer post on mxcl.dev that links to the live site and GitHub.
- Seed developer-community discussion where appropriate, especially around Homebrew, local agent secrets, and command approval gates.
- Add `sameAs` links only to profiles that are live and complete.

### 3. Live software version metadata appears stale

**Pages:** `/`, `/llms.txt`, `SoftwareApplication` schema
**Evidence:** Live homepage schema says `softwareVersion: 1.11.0`; live GitHub shows latest release `Automic Vault 1.13.0` dated May 30, 2026.

**Why it matters:** AI systems use freshness signals when recommending security tools. Version drift makes the site look less current than the repository.

**Fix:** Ensure deployment stamps the current release version into homepage schema, `llms.txt`, download page, and any generated package/catalog metadata.

---

## Medium Priority Issues

### 1. Several high-intent pages are thin

**Pages:** `/download/` (151 words), `/pricing/` (267), `/github-cli-token-security-ai-agents/` (292), `/secure-aws-cli-credentials-ai-agents/` (328), `/secret-scanner-for-ai-agents/` (339)

**Why it matters:** These pages target valuable AI-answer queries, but many have fewer than 350 words. They are readable but not yet the best citation target.

**Fix:** Add compact, concrete sections:

- "What it protects"
- "What changes after install"
- "Command example"
- "What Automic Vault is not"
- "When to use this with 1Password/HashiCorp Vault/GitHub/AWS"

### 2. FAQ schema is present, but not broad enough

**Pages with FAQ schema:** `/docs/`, `/secrets-manager-for-ai-agents/`, `/api-key-management-for-ai-agents/`, `/mcp-secrets-management/`, `/ai-agent-approval-gates/`, `/secret-scanning-vs-agent-secret-protection/`, `/hashicorp-vault-for-ai-agents/`

**Pages missing FAQ schema:** `/`, `/security/`, `/download/`, `/pricing/`, `/stop-ai-agents-reading-env-files/`, `/secure-aws-cli-credentials-ai-agents/`, `/github-cli-token-security-ai-agents/`, `/secret-scanner-for-ai-agents/`, `/av-trace/`

**Fix:** Add 3 to 5 natural Q&A blocks to pages where users would ask comparison or use-case questions.

### 3. The homepage is citable, but not the strongest answer page

The live homepage has 607 words, good schema, and good internal links. Its first answer, however, is broad: "A hardened package manager and secrets boundary for the tools AI agents run on your Mac." The local rewrite is stronger because it ties the risk to `brew install` and visible local dev-tool state.

**Fix:** Deploy the newer hero, then add one citable paragraph directly under the hero:

> If you install developer tools with Homebrew, npm, PyPI, or MCP servers, those tools can leave credentials in files an agent can read. Automic Vault finds those paths, moves supported credentials out of plaintext storage, and asks before sensitive commands use them.

### 4. Package pages are strong but internally isolated

Sample package pages had strong word count and HowTo schema, but only one internal link was detected on each package page. They link richly outward to package sources but do not create enough internal topic authority.

**Fix:** Add related internal links from package pages to:

- `/secret-scanner-for-ai-agents/`
- `/github-cli-token-security-ai-agents/`
- `/secure-aws-cli-credentials-ai-agents/`
- `/ai-agent-approval-gates/`
- package family or risk category hubs

### 5. `llms.txt` is good, but production should match the new brand system

The live `llms.txt` is useful and includes product facts, pages for AI systems to cite, and query topics. It should now be updated to match the new Homebrew/dev-tool hazard story and current release metadata.

---

## Low Priority Issues

### 1. Some security headers are permissive for static-site convenience

The live site has strong HSTS, frame-ancestor blocking, `nosniff`, referrer policy, and permissions policy. CSP still allows `unsafe-inline` and `wasm-unsafe-eval`.

This is not a GEO blocker, but tightening it would strengthen trust signals for a security product.

### 2. Download and pricing pages use generic H1 patterns

`/download/` uses `Automic Vault` as the H1. It would be stronger as:

`Download Automic Vault for macOS`

### 3. Product comparison pages could cite external sources

Comparison pages such as `/hashicorp-vault-for-ai-agents/` are useful, but AI systems trust comparisons more when they link to authoritative product docs for both sides.

---

## Category Deep Dives

### AI Citability (72/100)

Strengths:

- Static HTML is directly extractable.
- Pages have clear H1s, titles, meta descriptions, and canonical URLs.
- The site publishes both `llms.txt` and `llms-full.txt`.
- The package catalog creates many specific, query-addressable pages.
- Docs page is substantial at about 1,510 words and includes FAQ schema.

Weaknesses:

- Live homepage positioning is generic compared with the current local copy.
- Several SEO landing pages are under 350 words.
- Some pages state the claim but do not include enough concrete before/after examples.

Best citation targets today:

- `/docs/`
- `/secrets-manager-for-ai-agents/`
- `/api-key-management-for-ai-agents/`
- `/ai-agent-approval-gates/`
- `/pkg/brew/awscli/`
- `/pkg/brew/gh/`
- `/pkg/brew/curl/`

Rewrite priority:

Start with homepage, download, GitHub CLI, AWS CLI, secret scanner, and `.env` pages.

### Brand Authority (60/100)

Strengths:

- The founder claim is well supported by Homebrew's own site, Wikipedia, GitHub, and developer media.
- mxcl.dev links the founder, Homebrew, and Automic Vault together.
- GitHub repo is public and active.
- Organization schema uses `sameAs` links to GitHub, X, mxcl.dev, and brew.sh.

Weaknesses:

- Product-level search results are still thin and noisy.
- GitHub authority is early: 18 stars and 1 fork at audit time.
- There is no obvious product Wikipedia, Reddit, YouTube, or strong LinkedIn entity footprint.
- The brand collides with unrelated "Automic Automation", "Automic Group", and "Atomic Vault" entities.

Action:

Anchor the product entity with a small number of high-quality external profiles and one founder-owned canonical explainer.

### Content E-E-A-T (71/100)

Strengths:

- Founder context is strong and relevant.
- About, security, privacy, terms, docs, and pricing pages exist.
- The site is transparent about being free open-source software.
- Security disclosure path exists through `security.txt`.

Weaknesses:

- Most pages do not have visible author attribution or "last reviewed" metadata.
- Claims are rarely backed by outbound citations, examples, or command transcripts.
- Current production version metadata appears behind GitHub.

Action:

Add compact provenance blocks to key pages:

- "Maintained by Max Howell, creator of Homebrew"
- "Last reviewed May 31, 2026"
- "Source: GitHub repository"
- "Security disclosure: security.txt"

### Technical GEO (88/100)

Strengths:

- `robots.txt` explicitly allows `GPTBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `anthropic-ai`, `Google-Extended`, and `Bingbot`.
- `robots.txt` declares both the site sitemap and package sitemap index.
- Root sitemap has 104 URLs.
- Package sitemap index exposes 5 package sitemap files.
- Pages are static HTML with canonical URLs.
- Open Graph and Twitter metadata are present across pages.
- Security headers are strong for a static site.
- No missing image alt text was found in the sampled HTML pages.

Weaknesses:

- Production freshness is behind the repo and GitHub release.
- One manually sampled npm package URL returned 404 and was excluded from scoring. This was not discovered from the sitemap, so it is not treated as a live internal-link defect.

Action:

Keep the deployment pipeline stamping current release, date, `llms.txt`, and sitemap metadata together.

### Schema & Structured Data (84/100)

Schema found in the sample:

- `Organization`
- `Person`
- `WebSite`
- `SoftwareApplication`
- `Offer`
- `WebPage`
- `Article`
- `TechArticle`
- `FAQPage`
- `BreadcrumbList`
- `CollectionPage`
- `HowTo`
- `HowToStep`

Strengths:

- Homepage schema is comprehensive.
- Package pages include `HowTo`.
- Docs include `TechArticle` and `FAQPage`.
- Founder `Person` schema exists and is connected to the organization.

Weaknesses:

- FAQ schema coverage is uneven.
- Software version appears stale on production.
- Package pages could add stronger `about`, `mentions`, and internal topical relationships.

Action:

Add FAQ schema to the homepage and thin high-intent pages. Keep `SoftwareApplication.softwareVersion` synchronized at deploy time.

### Platform Optimization (57/100)

Strengths:

- GitHub repository exists and is public.
- X profile is referenced in schema.
- mxcl.dev provides an external founder/entity anchor.
- Homebrew creator association is supported by high-authority sources.

Weaknesses:

- Product-level profiles are sparse.
- No obvious Reddit, YouTube, LinkedIn company, Hacker News, Stack Overflow, or Wikipedia footprint for Automic Vault itself.
- Search results mix product pages with unrelated Automic/Atomic entities.

Action:

Do not try to manufacture volume. Create a small number of credible, consistent profiles and link them from schema only after they contain complete product descriptions.

---

## Quick Wins (Implement This Week)

1. Deploy the current local landing page, `llms.txt`, translations, and sitemap updates so production matches the new brand direction.
2. Stamp live schema and `llms.txt` with the current release version.
3. Change the live homepage H1/title/schema headline to "A new kind of package manager for a new kind of threat model."
4. Add FAQ blocks to `/download/`, `/pricing/`, `/github-cli-token-security-ai-agents/`, `/secure-aws-cli-credentials-ai-agents/`, and `/secret-scanner-for-ai-agents/`.
5. Add internal related links from package pages to the relevant guide pages.

---

## 30-Day Action Plan

### Week 1: Align Production Sources

- [ ] Deploy the local landing-page copy and generated mirrors.
- [ ] Regenerate and deploy `llms-full.txt`.
- [ ] Verify homepage schema `softwareVersion`, `dateModified`, and description match the current release.
- [ ] Confirm Search Console can fetch `/robots.txt`, `/sitemap.xml`, `/pkg/sitemap.xml`, `/llms.txt`, and `/llms-full.txt`.

### Week 2: Make Key Pages More Citable

- [ ] Expand `/download/` with requirements, installation steps, release source, and verification commands.
- [ ] Expand `/pricing/` with what is free, what data stays local, and what enterprise buyers may ask.
- [ ] Add concise command examples to AWS, GitHub CLI, `.env`, and secret scanner pages.
- [ ] Add FAQ schema to the pages missing it.

### Week 3: Strengthen Entity Authority

- [ ] Publish a canonical mxcl.dev post that introduces Automic Vault and links to the site, GitHub, docs, and download page.
- [ ] Complete product profiles on GitHub org, X, LinkedIn, and YouTube if those channels will be maintained.
- [ ] Add only completed profiles to `Organization.sameAs`.
- [ ] Create one comparison page that clearly says how Automic Vault differs from 1Password, HashiCorp Vault, and prompt-only agent controls.

### Week 4: Improve Package Catalog Authority

- [ ] Add topic backlinks from package pages to relevant guide pages.
- [ ] Add package risk category hubs such as "GitHub token risks", "AWS credential risks", and "dotenv/plaintext risks".
- [ ] Sample 25 package pages from each sitemap and validate canonical, schema, indexability, and internal links.
- [ ] Add package page snippets to `llms-full.txt` or a package-specific AI index if the full file becomes too large.

---

## Appendix: Pages Analyzed

| URL | Title | GEO Issues |
|---|---|---:|
| https://www.automicvault.com/ | Automic Vault \| From the creator of Homebrew | 3 |
| https://www.automicvault.com/docs/ | Automic Vault CLI Docs | 1 |
| https://www.automicvault.com/about/ | About Automic Vault \| From the creator of Homebrew | 2 |
| https://www.automicvault.com/security/ | Security \| Automic Vault | 1 |
| https://www.automicvault.com/privacy/ | Privacy \| Automic Vault | 0 |
| https://www.automicvault.com/terms/ | Terms \| Automic Vault | 0 |
| https://www.automicvault.com/pricing/ | Pricing \| Automic Vault | 2 |
| https://www.automicvault.com/download/ | Download Automic Vault for macOS | 2 |
| https://www.automicvault.com/secrets-manager-for-ai-agents/ | Secrets Manager for AI Agents \| Automic Vault | 1 |
| https://www.automicvault.com/stop-ai-agents-reading-env-files/ | Stop AI Agents Reading .env Files \| Automic Vault | 2 |
| https://www.automicvault.com/api-key-management-for-ai-agents/ | API Key Management for AI Coding Agents \| Automic Vault | 1 |
| https://www.automicvault.com/mcp-secrets-management/ | MCP Secrets Management for AI Agents \| Automic Vault | 1 |
| https://www.automicvault.com/privileged-access-management-for-ai-agents/ | Privileged Access Management for AI Agents \| Automic Vault | 2 |
| https://www.automicvault.com/ai-agent-approval-gates/ | AI Agent Approval Gates \| Automic Vault | 1 |
| https://www.automicvault.com/secure-aws-cli-credentials-ai-agents/ | Secure AWS CLI Credentials for AI Agents \| Automic Vault | 2 |
| https://www.automicvault.com/github-cli-token-security-ai-agents/ | GitHub CLI Token Security for AI Agents \| Automic Vault | 2 |
| https://www.automicvault.com/secret-scanner-for-ai-agents/ | AI Agent Secret Scanner \| Automic Vault | 2 |
| https://www.automicvault.com/av-trace/ | av trace \| Trace Shell Installers Before AI Agents Run Them | 1 |
| https://www.automicvault.com/secret-scanning-vs-agent-secret-protection/ | Secret Scanning vs Agent Secret Protection \| Automic Vault | 1 |
| https://www.automicvault.com/hashicorp-vault-for-ai-agents/ | HashiCorp Vault vs Automic Vault for AI Agent Security | 1 |
| https://www.automicvault.com/pkg/ | Package security catalog \| Automic Vault | 1 |
| https://www.automicvault.com/pkg/brew/awscli/ | Install awscli \| Automic Vault | 1 |
| https://www.automicvault.com/pkg/brew/gh/ | Install gh \| Automic Vault | 1 |
| https://www.automicvault.com/pkg/brew/curl/ | Install curl \| Automic Vault | 1 |
| https://www.automicvault.com/pkg/brew/docker/ | Install docker \| Automic Vault | 1 |

Non-HTML and support files checked:

- https://www.automicvault.com/robots.txt
- https://www.automicvault.com/sitemap.xml
- https://www.automicvault.com/pkg/sitemap.xml
- https://www.automicvault.com/llms.txt
- https://www.automicvault.com/llms-full.txt
- https://www.automicvault.com/.well-known/security.txt

External entity signals reviewed:

- https://github.com/automic-vault/automic-vault
- https://mxcl.dev/
- https://brew.sh/
- https://en.wikipedia.org/wiki/Homebrew_(package_manager)
