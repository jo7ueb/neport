# neport-mcp

Standalone Python MCP server for Neport. It uses the official Python MCP SDK
and communicates with Neport over its REST API; no Neport repository checkout is
needed by the user.

## Install and run

```bash
# Download the wheel from the GitHub Release, then:
pipx install neport_mcp-0.1.0-py3-none-any.whl
neport-mcp
```

Required environment variables:

```text
NEPORT_API_URL=https://neport.yendo.info
NEPORT_API_TOKEN=npt_...
```

Optional: `NEPORT_MCP_TIMEOUT_MS` (default `15000`) and
`NEPORT_MCP_LOG_LEVEL` (default `WARNING`). Logs go to stderr; stdout is
reserved for MCP stdio messages.

Create the token from an authenticated Neport session at
`POST /api/v1/auth/tokens`. Use `read` for queries and add `write` only when
the assistant must create or change data.
