# Automic Vault SEO Content Gap Recommendations

## Context

Automic Vault is a new site with only two indexed sitemap URLs:

- `https://www.automicvault.com/`
- `https://www.automicvault.com/docs/`

The Infisical keyword and subpage exports show useful adjacent demand around secrets management, dotenv, API key management, HashiCorp Vault, MCP secrets, and privileged access management. Automic Vault should not copy generic secrets-management content. The best angle is agent-specific runtime control: secrets, command approval, package/tool execution, and preventing autonomous agents from reading or misusing credentials.

## Build These First

| Priority | New Automic page | Why |
|---|---|---|
| P0 | `/secrets-manager-for-ai-agents/` | Infisical gets traffic from `secrets management tools`, `secret manager`, and `open source secret manager`. Automic needs a literal product-category page. |
| P0 | `/stop-ai-agents-reading-env-files/` | Infisical's dotenv page is one of its best: `dotenv` 3,600 volume, `python dotenv` 1,500, `dotenv npm` 480. Automic has a sharper angle: agents should not read `.env` files. |
| P0 | `/api-key-management-for-ai-agents/` | Infisical ranks for `api key management`, `secure api key`, and `api key security`. Automic can make this agent-specific and more conversion-oriented. |
| P0 | `/hashicorp-vault-for-ai-agents/` | Infisical's Vault pricing and alternatives pages drive meaningful traffic. Position Automic honestly as the local agent-runtime layer, not a full Vault replacement. |
| P0 | `/mcp-secrets-management/` | Infisical already has a "Managing Secrets in MCP Servers" page. Low traffic now, but high relevance and likely early-category demand. |
| P1 | `/privileged-access-management-for-ai-agents/` | Infisical gets traffic from PAM pages. Automic can translate PAM into approval gates for autonomous tool use. |
| P1 | `/ai-agent-approval-gates/` | Core product page. Infisical data does not prove this directly, but Automic's homepage already sells this. It deserves its own page. |
| P1 | `/secure-aws-cli-credentials-ai-agents/` | Strong product fit because Automic already shows AWS credential protection. Tie into AWS Secrets Manager, HashiCorp Vault, and API key terms. |
| P1 | `/github-cli-token-security-ai-agents/` | Good use-case page for agent access to GitHub tokens, CLI credentials, and package publishing. |
| P2 | `/secret-scanning-vs-agent-secret-protection/` | Explain why GitGuardian/TruffleHog-style scanning is after-the-fact, while Automic prevents exposure at runtime. |

## Avoid For Now

| Infisical page type | Reason |
|---|---|
| SSH certificates | Good traffic, but only build if Automic can clearly gate or protect SSH usage. |
| OAuth 2.0 implementation | Tutorial traffic, weak product fit. |
| Azure Key Vault pricing | Searchers want cloud pricing, not local agent security. |
| Kubernetes secrets | Useful later if Automic supports server or Kubernetes workflows. |
| Certificate manager | Too far from current Automic positioning. |

## Recommended First Five Page Titles

1. Secrets Manager for AI Agents
2. Stop AI Coding Agents from Reading Your `.env` Files
3. API Key Management for AI Coding Agents
4. HashiCorp Vault vs Automic Vault for AI Agent Security
5. MCP Secrets Management: How to Give Tools Access Without Giving Models Secrets

## Positioning Rule

Use Infisical's proven demand categories, but make every page agent-specific.

Do not write generic "secrets management" content. Automic Vault's wedge is runtime control: secrets, command approval, and tool execution at the layer where agents actually act.

## Suggested Internal Linking

- Homepage -> all P0 pages.
- Docs -> `/secrets-manager-for-ai-agents/`, `/api-key-management-for-ai-agents/`, and `/ai-agent-approval-gates/`.
- `/secrets-manager-for-ai-agents/` -> dotenv, API key management, MCP secrets, HashiCorp Vault comparison.
- `/stop-ai-agents-reading-env-files/` -> API key management and approval gates.
- `/mcp-secrets-management/` -> secrets manager and approval gates.

## SE Ranking Follow-Up

Track these keyword groups in SE Ranking:

- `secrets manager for ai agents`
- `ai agent secrets manager`
- `ai coding agent secrets`
- `dotenv ai agents`
- `prevent ai agents reading env files`
- `api key management for ai agents`
- `mcp secrets management`
- `mcp secrets manager`
- `ai agent approval gates`
- `human approval for ai agent commands`
- `hashicorp vault ai agents`
- `github cli token security`
- `aws cli credentials security`

