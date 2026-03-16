# Integration Install & Reconnect Smoke

This playbook verifies the lifecycle of a bundled integration through the live API:
- discover
- install
- health check
- reconnect
- reload
- remove

Date:
- 2026-03-16

## 1. Purpose

Use this playbook when changing:
- bundled integration templates
- integration install/remove flows
- integration registry behavior
- extension MCP reconnect behavior
- integration health reporting

This is a generic playbook. It works best with a bundled integration that is easy to connect locally, such as `chrome-devtools`.

## 2. Prerequisites

Required:
- running OpenFang daemon
- one bundled integration that can realistically connect on the current machine

Recommended:
- use an isolated `OPENFANG_HOME`

## 3. Verify Template Discovery

```bash
curl -s http://127.0.0.1:4200/api/integrations/available
```

Expected:
- target integration appears in `integrations`
- metadata is present:
  - `id`
  - `name`
  - `category`
  - `required_env`
  - `setup_instructions`

## 4. Install Integration

Example:

```bash
curl -s -X POST http://127.0.0.1:4200/api/integrations/add \
  -H 'content-type: application/json' \
  -d '{"id":"chrome-devtools"}'
```

Expected:
- HTTP `201`
- response contains:
  - `"status":"installed"`
  - `"connected": true` or `false` depending on environment

## 5. Verify Installed View

```bash
curl -s http://127.0.0.1:4200/api/integrations
```

Expected:
- integration appears in `installed`
- state reflects the newly installed integration

## 6. Verify Health

```bash
curl -s http://127.0.0.1:4200/api/integrations/health
```

Expected:
- integration entry is present
- `status` is meaningful (`Ready`, `Error`, or equivalent serialized form)
- `tool_count` is populated when connected

## 7. Verify MCP Surface

```bash
curl -s http://127.0.0.1:4200/api/mcp/servers
```

Expected:
- connected integrations appear in `connected`
- tool list is non-empty for a successful connection

## 8. Reconnect Integration

```bash
curl -s -X POST http://127.0.0.1:4200/api/integrations/chrome-devtools/reconnect
```

Expected:
- reconnect endpoint returns success
- health and MCP state remain consistent afterward

Re-check:

```bash
curl -s http://127.0.0.1:4200/api/integrations/health
curl -s http://127.0.0.1:4200/api/mcp/servers
```

## 9. Reload Integrations

```bash
curl -s -X POST http://127.0.0.1:4200/api/integrations/reload
```

Expected:
- reload succeeds
- installed integration remains visible
- MCP connection is still present or reconnects cleanly

## 10. Remove Integration

```bash
curl -s -X DELETE http://127.0.0.1:4200/api/integrations/chrome-devtools
```

Expected:
- integration disappears from installed list
- health entry is removed or no longer active
- MCP connected entry disappears

## 11. Failure Triage

If install succeeds but the integration never connects:
- check required binaries and env vars
- inspect daemon logs for MCP startup errors

If reconnect fails but install succeeded:
- inspect `GET /api/integrations/health`
- inspect `GET /api/mcp/servers`
- confirm the integration's runtime dependencies are still present

If removal succeeds but the MCP server remains listed:
- this indicates a live connection cleanup regression
- inspect integration reload and effective MCP reconciliation
