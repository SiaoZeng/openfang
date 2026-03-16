# Phase 4 Findings - MCP Server CRUD API

This document captures the implementation notes, important findings, and verification results for Phase 4 of the OpenFang 12-phase roadmap.

Phase:
- Phase 4: MCP Server CRUD API

Roadmap source:
- `docs/roadmap-12-phases.md`

Date:
- 2026-03-16

## 1. Outcome

Phase 4 is functionally complete.

Implemented:
- `POST /api/mcp/servers`
- `PUT /api/mcp/servers/{name}`
- `DELETE /api/mcp/servers/{name}`
- persistent `[[mcp_servers]]` config mutation through the API
- runtime MCP reload after create, update, and delete
- MCP API documentation updates
- integration and kernel regression coverage

## 2. What Was Added

API surface:
- manual MCP server create/update/delete endpoints were added
- `GET /api/mcp/servers` now reflects the current disk-backed manual MCP config instead of relying only on stale boot-time config

Persistence:
- MCP server entries are written to and removed from `config.toml`
- array-of-table style `[[mcp_servers]]` persistence now works through the admin API

Runtime behavior:
- manual MCP changes now rebuild the live MCP connection set
- `mcp_connections`, `mcp_tools`, and `effective_mcp_servers` are reconciled after changes
- `config_reload` now triggers actual MCP runtime reconciliation when MCP config changes are detected

Documentation:
- MCP endpoint docs now match the real response shape
- CRUD endpoints are documented in the API docs

## 3. Important Findings

### Finding 1: the original gap was not just CRUD, but truthful runtime apply behavior

Before this phase:
- MCP config changes could be detected by reload planning
- but MCP runtime reconciliation was not actually being applied

Practical conclusion:
- Phase 4 had to include both admin endpoints and real runtime MCP rebuild behavior
- otherwise the API would have reported successful config mutation without matching live behavior

### Finding 2: extension-managed and manual MCP servers must stay separate concepts

OpenFang already had:
- integration install/remove flows
- extension MCP reload/reconnect logic

Phase 4 added:
- CRUD for manual `[[mcp_servers]]` entries only

Practical conclusion:
- this separation is correct and should be preserved
- integrations remain one-click packaged MCP setup
- Phase 4 manual CRUD remains the lower-level admin path

### Finding 3: hyphenated MCP server names exposed a real tool-filtering regression

The new bundled `chrome-devtools` integration surfaced an important bug:
- MCP tool names are normalized with underscores
- server IDs can contain hyphens
- naive server-prefix extraction and allowlist matching can silently drop tools

Observed practical effect during live validation:
- the `browser-hand` had `chrome-devtools` assigned correctly
- the MCP server connected correctly
- but the MCP tools were initially missing from the agent's actual tool selection

Practical conclusion:
- Phase 4 needed one more normalization fix beyond the CRUD port
- hyphenated MCP server names must always be resolved against known configured server names, not only by first-underscore heuristics

### Finding 4: the browser-hand E2E check was valuable because unit and API tests would not have shown the whole failure

The live smoke run proved:
- integration install worked
- MCP server connected
- hand activation assigned the right MCP allowlist

But it also revealed:
- the final agent tool selection still dropped the MCP tools until the normalization bug was fixed

Practical conclusion:
- for MCP-heavy capability paths, a targeted smoke test is worth keeping even when API and kernel tests are green

## 4. Verification Completed

Formatting:

```bash
cargo fmt --all
```

API integration verification:

```bash
cargo test -p openfang-api --test api_integration_test
```

Extensions + kernel verification:

```bash
cargo test -p openfang-extensions -p openfang-kernel
```

Binary build:

```bash
cargo build --bin openfang
```

## 5. End-to-End Smoke Validation Completed

An isolated local daemon-backed smoke run was performed for the new bundled `chrome-devtools` integration and the MCP-first `browser-hand`.

Verified:
- `chrome-devtools` appears in `/api/integrations/available`
- integration install succeeds through `/api/integrations/add`
- `/api/mcp/servers` shows a live `chrome-devtools` connection with real tools
- `browser-hand` activates successfully
- `/api/agents/{id}/mcp_servers` shows `assigned = ["chrome-devtools"]`
- the live `Tools selected for LLM request` log now includes `mcp_chrome_devtools_*` tools after the normalization fix

Important note:
- the test model provider in the isolated environment was intentionally not functional
- so the final agent turn failed at the LLM call layer
- this did not invalidate the MCP wiring check because tool selection had already been verified before the LLM failure

## 6. Files Touched For Phase 4

Main implementation files:
- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `docs/api-reference.md`
- `docs/mcp-a2a.md`

Supporting MCP/browser follow-up files:
- `crates/openfang-extensions/integrations/chrome-devtools.toml`
- `crates/openfang-extensions/src/bundled.rs`
- `crates/openfang-extensions/src/registry.rs`
- `crates/openfang-extensions/src/installer.rs`
- `crates/openfang-hands/bundled/browser/HAND.toml`
- `crates/openfang-hands/bundled/browser/SKILL.md`
- `crates/openfang-cli/src/tui/event.rs`

This findings document:
- `docs/phase-4-mcp-crud-findings.md`

## 7. Recommended Next Step

Phase 4 is now in a good stopping state.

Recommended next roadmap move:
- Phase 5: Backup and Restore

Reason:
- MCP admin workflows are now first-class
- runtime reload behavior is no longer misleading
- the next high-value operational feature is system backup and restore
