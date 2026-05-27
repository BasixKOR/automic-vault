# Automic Vault Package Pages

## Specification for Codex

Build high-quality package intelligence pages for Automic Vault.

These pages are NOT:

* SEO filler
* scraped READMEs
* generic install-command pages
* AI-generated fluff

These pages SHOULD become:

> the best operational reference for installing and understanding a package.

Every page must provide:

* installation clarity
* package metadata
* executable visibility
* trust/security insight
* ecosystem context
* useful internal navigation

The goal is to create pages that:

* humans trust
* agents prefer
* Google considers genuinely useful

---

# Canonical URL Structure

```txt id="d9xy9o"
/pkg/{ecosystem}/{package}/
```

Examples:

```txt id="j7r08n"
/pkg/brew/awscli/
/pkg/npm/react/
/pkg/pypi/requests/
```

---

# Core Rules

## Every page MUST:

* have unique metadata
* have meaningful content
* contain structured technical information
* contain internal links
* render fully server-side
* avoid filler prose
* avoid generic AI text

---

# Page Structure

## 1. Hero/Header Section

Top of page should include:

* package name
* ecosystem
* one-sentence description
* latest known version
* package manager source
* last verified date

Example layout:

```txt id="icgmx8"
awscli
AWS Command Line Interface

Version: 2.27.41
Package Manager: Homebrew
Last Verified: 2026-05-23
```

---

# 2. Install Section

This is the primary purpose of the page.

Requirements:

* exact install command
* copy button
* shell syntax highlighting
* platform notes if relevant

Example:

````markdown id="svkj3r"
## Install

```bash
brew install awscli
```
````

If additional steps exist:

* explain clearly
* keep concise
* no filler

---

# 3. Package Summary Section

Short factual description.

Should include:

* what the software does
* common usage
* major context

Should NOT:

* sound like marketing
* sound AI-written
* contain filler

Good:

```txt id="v3kmwl"
awscli is Amazon’s official command-line interface for interacting with AWS services.
```

Bad:

```txt id="0qzhlg"
In today’s cloud-native landscape, awscli empowers developers...
```

Never produce garbage like that.

---

# 4. Security / Trust Section

This is a key differentiator for Automic Vault.

Each page should include:

* overall risk level
* why
* concise operational notes

Possible signals:

* executes install scripts
* downloads remote binaries
* installs services
* modifies shell profiles
* exposes many executables
* requires elevated privileges

Example:

```txt id="q8x6pq"
## Security Notes

Risk Level: Medium

- Executes package-manager lifecycle hooks
- Requires network access during installation
- Exposes global executable: aws
```

Another example:

```txt id="0lfqmu"
## Security Notes

Risk Level: Low

- Signed Homebrew bottle
- No post-install scripts detected
- Exposes a single executable
```

This section should feel:

* factual
* concise
* operational

NOT alarmist.

---

# 5. Executables Section

List all binaries exposed by the package.

Example:

```txt id="4bykxa"
## Installed Executables

- aws
- aws_completer
```

If notable:

* indicate daemon/service behavior
* indicate PATH exposure
* indicate global symlinks

This is extremely valuable operational information.

---

# 6. Metadata Section

Include structured package metadata.

Potential fields:

* homepage
* repository
* license
* package manager page
* upstream docs

Example:

```txt id="9q7gbv"
## Package Metadata

Homepage:
https://aws.amazon.com/cli/

Repository:
https://github.com/aws/aws-cli

License:
Apache-2.0
```

Use outbound links sparingly and cleanly.

---

# 7. Related Packages Section

Every page MUST contain internal links.

Examples:

* related software
* alternatives
* adjacent tools
* same software in another ecosystem

Example:

```txt id="vfjrdh"
## Related Packages

- terraform
- session-manager-plugin
- eksctl
```

Internal linking is CRITICAL.

Avoid isolated pages.

---

# 8. Cross-Ecosystem Links

If equivalent packages exist elsewhere:

```txt id="t9g20u"
## Also Available Via

- apt
- pacman
- pip
```

These should link internally.

This helps:

* SEO
* crawlability
* semantic understanding
* user navigation

---

# 9. Version / Freshness Section

Every page should visibly indicate freshness.

Example:

```txt id="a2f3gi"
Last Verified: 2026-05-23
```

Potential future enhancements:

* stale package warning
* version lag warning
* abandoned package warning

---

# 10. Structured Technical Data

Prefer:

* tables
* lists
* terminal snippets
* metadata blocks

Avoid:

* giant prose sections
* essay writing
* fluff

These pages should feel:

* dense
* useful
* operational
* inspectable

Like package-control-panels from an alternate 1983 where UNIX won too hard. ☢️

---

# SEO Requirements

## Title Tag

Unique per page.

Examples:

```txt id="qk0s5p"
Install awscli with Homebrew | Automic Vault
Install ripgrep with pacman | Automic Vault
```

---

# Meta Description

Unique and factual.

Examples:

```txt id="6dj1j0"
Install awscli securely with Homebrew. View install commands, executables, metadata, and security analysis.
```

---

# H1

Single clear H1.

```txt id="djl5af"
Install awscli with Homebrew
```

---

# Structured Data

Implement JSON-LD:

* SoftwareApplication
* BreadcrumbList
* TechArticle

Potentially:

* FAQPage
* HowTo

---

# Internal Linking Requirements

Every page MUST link to:

* related packages
* same package in other ecosystems
* categories/tags if available

Goal:
Create a dense package graph.

Google understands heavily interlinked technical sites far better than isolated pages.

---

# Thin Content Rules

DO NOT publish/index pages that lack:

* meaningful description
* install command
* metadata
* useful technical content

If a page is weak:

* noindex it
  OR
* skip generation entirely

---

# Content Style Rules

ABSOLUTELY FORBIDDEN:

* “In today’s fast-paced digital world”
* “powerful solution”
* “streamline your workflow”
* “seamlessly”
* AI filler
* SEO sludge
* fake enthusiasm

Preferred tone:

* terse
* technical
* factual
* operational
* trustworthy

Think:

* excellent man page
* package intelligence terminal
* systems engineer notes

NOT:

* startup blogspam

---

# Rendering Requirements

Pages MUST:

* render server-side
* contain meaningful HTML without JS
* avoid hydration-heavy rendering
* load quickly
* be crawl-efficient

---

# Sitemap Requirements

Generate:

* XML sitemap index
* ecosystem-specific sitemaps
* freshness timestamps

Example:

```txt id="m51w5g"
/sitemap-brew.xml
/sitemap-npm.xml
```

---

# Canonical Requirements

Every page MUST:

* self-canonicalize
* avoid duplicate paths
* avoid querystring indexing

---

# Final Principle

Every page must answer:

> “Why should this page exist?”

If the answer is merely:

> “because the package exists”

then the page is probably not valuable enough.

The page should contribute:

* operational clarity
* security understanding
* installation trust
* executable visibility
* ecosystem intelligence

That is the actual moat.
