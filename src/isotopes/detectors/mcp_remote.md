# mcp-remote Radioisotope

mcp-remote bridges local stdio MCP clients to remote MCP servers and handles
OAuth for clients that do not yet support the remote authorization flow.

The stock package stores OAuth token and client registration JSON under
`~/.mcp-auth`, or under `MCP_REMOTE_CONFIG_DIR` when that environment variable
is set. Those files can include access tokens, refresh tokens, ID tokens, and
static client secrets.

This radioisotope migrates credential-bearing auth files into Automic
Vault-backed keychain storage. The installed launcher restores those files into
a temporary `MCP_REMOTE_CONFIG_DIR` for the lifetime of each `mcp-remote`
process.

Runtime token refreshes happen in the temporary directory and are not written
back to the keychain. If a remote server forces a new OAuth flow, rerun the
migration after completing that flow.
