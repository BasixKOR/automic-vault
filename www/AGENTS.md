# Automic Vault Website

## Scope

This directory is the static Automic Vault website.

Work here should be limited to:
- HTML pages
- CSS stylesheets
- static copy
- images and other public assets
- static metadata files such as sitemap, robots, llms, text, markdown, and JSON
  representations of website content

Do not add application code, server code, package manager behavior, helper
behavior, or product runtime guidance to this directory.

## Deployment Safety

This `AGENTS.md` file is local repository guidance only. It must not be synced
or uploaded to S3.

## Editing Principles

Prioritize:
- clear page purpose
- readable HTML
- small CSS changes that fit the existing stylesheet structure
- stable URLs
- accessible text and image alternatives
- low-surprise changes

Do not optimize for novelty. Preserve the existing shape of the website unless
the task explicitly asks for a redesign.

## File Layout

- Keep human-facing pages at stable, readable paths.
- Keep shared visual styling in existing CSS files when practical.
- Keep static assets under `assets/` unless the existing page pattern uses a
  different location.
- Keep generated search/package/pagefind output in its existing generated
  locations.
- Do not create new top-level directories inside `www/` unless the page group is
  real and persistent.

## HTML

- Prefer semantic elements and readable document structure.
- Keep headings in logical order.
- Preserve canonical links, metadata, Open Graph, and structured content unless
  the change intentionally updates them.
- Keep internal links stable and use trailing slash conventions consistently
  with nearby pages.
- Avoid inline style churn when the existing stylesheet can own the change.

## CSS

- Reuse existing custom properties, spacing, type, and layout patterns.
- Keep responsive behavior explicit and test narrow widths when changing layout.
- Avoid one-off visual hacks unless the issue is truly isolated.
- Keep text legible against backgrounds and images.
- Avoid decorative changes that make the product harder to understand.

## Content

- Keep copy concrete, product-specific, and concise.
- Avoid unsupported claims.
- Preserve security-sensitive wording when editing pages about secrets,
  credentials, approvals, or agent access.
- Keep SEO-facing alternate formats aligned with the canonical page content when
  editing them together.

## Verification

Before finishing substantial website work, run the smallest relevant check:
- inspect the changed page locally when layout changes
- check links or generated files when URL/content indexes change
- run existing generation checks when generated website artifacts are involved

Commit as Codex after each completed job.
