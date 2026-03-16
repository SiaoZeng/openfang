# Chrome DevTools MCP Smoke

This playbook verifies the bundled `chrome-devtools` integration and the MCP-first `browser-hand` flow against a real local OpenFang daemon.

Date:
- 2026-03-16

## 1. Purpose

This is a manual or operator-run smoke test.

It should prove:
- the `chrome-devtools` integration is available
- it can be installed successfully
- the MCP server connects and exposes tools
- `browser-hand` activates with `chrome-devtools` assigned
- the hand agent receives `mcp_chrome_devtools_*` tools in its live tool set

This playbook does not require a successful LLM response to prove the MCP path is wired correctly.

## 2. Prerequisites

Required:
- built `openfang` binary
- Node.js with `npx`
- Google Chrome or Chromium

Recommended:
- a working local or remote model provider if you want to see the hand actually execute a browser task

## 3. Isolated Test Home

Use an isolated `OPENFANG_HOME` so the smoke test does not mutate your normal environment.

Example:

```bash
mkdir -p /tmp/openfang-e2e-chrome/data
cat > /tmp/openfang-e2e-chrome/config.toml <<'EOF'
api_listen = "127.0.0.1:4310"

[default_model]
provider = "ollama"
model = "test-model"
api_key_env = "OLLAMA_API_KEY"
EOF
```

## 4. Start Daemon

```bash
OPENFANG_HOME=/tmp/openfang-e2e-chrome ./target/debug/openfang start --yolo
```

Expected signals in logs:
- `Extension registry: 26 templates available`
- API server listening on `127.0.0.1:4310`

## 5. Verify Integration Is Available

```bash
curl -s http://127.0.0.1:4310/api/integrations/available
```

Expected:
- `count` includes the bundled set
- one entry has `"id":"chrome-devtools"`

## 6. Install Integration

```bash
curl -s -X POST http://127.0.0.1:4310/api/integrations/add \
  -H 'content-type: application/json' \
  -d '{"id":"chrome-devtools"}'
```

Expected response:

```json
{
  "id": "chrome-devtools",
  "status": "installed",
  "connected": true
}
```

Expected daemon log:
- `MCP server connected server=chrome-devtools`

## 7. Verify Live MCP State

```bash
curl -s http://127.0.0.1:4310/api/mcp/servers
```

Expected:
- one connected server named `chrome-devtools`
- `tools_count` is non-zero
- tool names include `mcp_chrome_devtools_new_page`

## 8. Verify Browser Hand Definition

```bash
curl -s http://127.0.0.1:4310/api/hands/browser
```

Expected:
- requirements include `npx`
- tool list includes `mcp_chrome_devtools_*`
- browser hand requirements are satisfied on the current machine

## 9. Activate Browser Hand

```bash
curl -s -X POST http://127.0.0.1:4310/api/hands/browser/activate \
  -H 'content-type: application/json' \
  -d '{}'
```

Capture the returned `agent_id`.

Expected:
- activation succeeds
- active hand instance appears in `/api/hands/active`

## 10. Verify MCP Assignment On Agent

Replace `<agent_id>` with the actual ID.

```bash
curl -s http://127.0.0.1:4310/api/agents/<agent_id>/mcp_servers
```

Expected:
- `assigned` contains `chrome-devtools`
- `available` contains `chrome-devtools`
- `mode` is `allowlist`

## 11. Verify Live Tool Selection

Send a simple message to force a real tool-selection pass.

```bash
curl -s -X POST http://127.0.0.1:4310/api/agents/<agent_id>/message \
  -H 'content-type: application/json' \
  -d '{"message":"Open a page and inspect it."}'
```

If the model provider is offline, the HTTP request may fail with a model-driver error. That is acceptable for this smoke test.

What matters is the daemon log line:
- `Tools selected for LLM request`

Expected:
- the listed tools include `mcp_chrome_devtools_*`
- examples:
  - `mcp_chrome_devtools_new_page`
  - `mcp_chrome_devtools_take_snapshot`
  - `mcp_chrome_devtools_click`

## 12. Optional Full Browser Check

If a working model provider is available, send a concrete browser task:

```bash
curl -s -X POST http://127.0.0.1:4310/api/agents/<agent_id>/message \
  -H 'content-type: application/json' \
  -d '{"message":"Open example.com, take a snapshot, and summarize the page title."}'
```

Success criteria:
- no MCP connection errors
- browser-hand uses the MCP browser tools
- response mentions `example.com` and the page title

## 13. Cleanup

Stop the daemon with `Ctrl+C`.

If you want a clean rerun:

```bash
rm -rf /tmp/openfang-e2e-chrome
```

## 14. Failure Triage

If `chrome-devtools` is missing from available integrations:
- confirm the binary was rebuilt after integration changes

If install succeeds but `connected` is false:
- confirm `npx` is available
- confirm Chrome/Chromium is installed
- inspect daemon logs for MCP startup errors

If the browser hand activates but MCP tools do not appear in live tool selection:
- inspect agent MCP allowlist via `/api/agents/<agent_id>/mcp_servers`
- inspect daemon logs for `Tools selected for LLM request`
- this indicates a tool-filtering regression, not an integration install problem

If the agent crashes during execution:
- separate model-provider errors from MCP/browser errors
- provider errors do not invalidate the MCP wiring check
