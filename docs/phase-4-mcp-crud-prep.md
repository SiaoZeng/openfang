# Phase 4 Preparation - MCP Server CRUD API

This document captures the implementation-ready preparation for Phase 4 of the OpenFang 12-phase roadmap.

Phase:
- Phase 4: MCP Server CRUD API

Roadmap source:
- `docs/roadmap-12-phases.md`

Date:
- 2026-03-16

## 1. Preparation Outcome

Phase 4 is not implemented yet, but the repo is now prepared with:
- a clear gap analysis of the current MCP admin surface
- direct LibreFang source references for the CRUD port
- OpenFang-specific corrections that should be made during the port
- a concrete execution and verification checklist

## 2. Current OpenFang Baseline

Already present:
- `GET /api/mcp/servers`
- `GET /api/agents/{id}/mcp_servers`
- `PUT /api/agents/{id}/mcp_servers`
- kernel MCP connection management
- effective MCP server merging for manual config plus extension-installed integrations
- config reload planning that detects MCP server list changes

Important practical baseline:
- OpenFang already knows how to parse MCP server config entries
- OpenFang already knows how to connect MCP servers at boot
- OpenFang already exposes connected MCP tools through API, CLI, and MCP HTTP transport

What is missing for Phase 4:
- `POST /api/mcp/servers`
- `PUT /api/mcp/servers/{name}`
- `DELETE /api/mcp/servers/{name}`
- reliable persistence of `[[mcp_servers]]` entries through the admin API
- reliable runtime application semantics after config mutation

## 3. Main Findings From Preparation

### Finding 1: LibreFang already has the CRUD slice to port

Direct reference files:
- `/home/jan/gh/librefang/crates/librefang-api/src/server.rs`
- `/home/jan/gh/librefang/crates/librefang-api/src/routes.rs`

Relevant LibreFang handlers and helpers:
- `add_mcp_server`
- `update_mcp_server`
- `delete_mcp_server`
- `upsert_mcp_server_config`
- `remove_mcp_server_config`

This is a strong direct port candidate for the API layer and TOML persistence helpers.

### Finding 2: `reload_config()` currently overstates MCP hot-reload success

OpenFang already marks MCP server list changes as `HotAction::ReloadMcpServers`.

But in `crates/openfang-kernel/src/kernel.rs`, `apply_hot_actions()` currently only applies a small subset of hot actions in practice. `ReloadMcpServers` is only logged as "not yet auto-applied".

Practical conclusion:
- a naive CRUD port from LibreFang would save config and report reload success
- but the running MCP connections would not reliably match the new config after the write

This is the most important implementation correction for Phase 4.

### Finding 3: the public docs currently have MCP response-shape drift

Current code in OpenFang returns this structure from `GET /api/mcp/servers`:
- `configured`
- `connected`
- `total_configured`
- `total_connected`

But `docs/api-reference.md` still shows an older shape:
- `servers`
- `total`

Practical conclusion:
- Phase 4 should update the docs while touching this surface anyway
- otherwise the CRUD work will land on top of already stale MCP API docs

### Finding 4: extension-managed MCP servers should remain a separate admin path

OpenFang already has integration endpoints and extension MCP reload flows.

The manual Phase 4 CRUD endpoints should operate on `config.toml` `[[mcp_servers]]` entries, not on extension-installed integrations.

Practical conclusion:
- Phase 4 should manage manual MCP server config
- integrations remain managed by the integration subsystem
- this distinction should be explicit in handler logic and docs

## 4. Recommended Implementation Scope

Minimum Phase 4 API surface:
- `POST /api/mcp/servers`
- `PUT /api/mcp/servers/{name}`
- `DELETE /api/mcp/servers/{name}`

Expected request model:
- reuse `openfang_types::config::McpServerConfigEntry`

Expected validation:
- non-empty `name`
- required `transport`
- serde validation through `McpServerConfigEntry`
- duplicate-name rejection on create
- `404` on update/delete for unknown manual server entries

Expected response semantics:
- create returns `201 Created`
- update returns `200 OK`
- delete returns `200 OK`
- responses should include both operation status and reload/apply status

## 5. OpenFang-Specific Implementation Notes

### 5.1 Route wiring

Planned API wiring in `crates/openfang-api/src/server.rs`:
- extend `/api/mcp/servers` from `GET` to `GET + POST`
- add `/api/mcp/servers/{name}` for `PUT + DELETE`

### 5.2 Persistence helpers

Recommended location:
- `crates/openfang-api/src/routes.rs`

Recommended helper functions:
- `upsert_mcp_server_config(...)`
- `remove_mcp_server_config(...)`

Implementation approach:
- parse `config.toml` into `toml::Value` or `toml::value::Table`
- mutate the `mcp_servers` array
- write back with `toml::to_string_pretty(...)`

### 5.3 Runtime apply behavior

Phase 4 should not stop at `reload_config()` alone.

Recommended correction:
- add a dedicated runtime apply path for manual MCP server changes
- make the running `mcp_connections`, `mcp_tools`, and `effective_mcp_servers` reflect the new config after create/update/delete

At minimum, one of these must happen:
- implement `HotAction::ReloadMcpServers` for real in the kernel
- or add a dedicated kernel method for rebuilding manual MCP connections and call it from the new API handlers

Preferred direction:
- implement the kernel-side MCP reload correctly so the reload plan and the real behavior agree

### 5.4 Effective vs manual server lists

The new CRUD endpoints should target manual config entries from `kernel.config.mcp_servers`.

But any runtime reconciliation step must continue to respect:
- manual config entries
- extension-installed MCP entries
- duplicate-name precedence rules already used by `effective_mcp_servers`

### 5.5 Audit logging

Phase 4 should mirror LibreFang here:
- record config-change audit entries for add/update/delete actions

## 6. Suggested Execution Order

1. Port the API route wiring from LibreFang into OpenFang.
2. Port the three handlers plus the two TOML helper functions.
3. Replace LibreFang-specific types and audit paths with OpenFang equivalents.
4. Implement real runtime MCP reload behavior in the kernel.
5. Update MCP API docs to the current response shape and new CRUD endpoints.
6. Add integration coverage for create, update, and delete flows.

## 7. Verification Checklist

Minimum code verification:

```bash
cargo fmt --all
cargo test -p openfang-api --test api_integration_test
```

Recommended targeted coverage:
- create MCP server persists a new `[[mcp_servers]]` entry
- duplicate create returns `409`
- update replaces an existing entry by name
- delete removes an existing entry by name
- invalid request bodies return `400`
- runtime server listing reflects the config mutation after apply/reload

Recommended doc verification:

```bash
rg -n "GET /api/mcp/servers|POST /api/mcp/servers|PUT /api/mcp/servers/\\{name\\}|DELETE /api/mcp/servers/\\{name\\}" docs/api-reference.md docs/mcp-a2a.md
```

## 8. Porting References

LibreFang route registration:
- `/home/jan/gh/librefang/crates/librefang-api/src/server.rs`

LibreFang handler block:
- `/home/jan/gh/librefang/crates/librefang-api/src/routes.rs`

OpenFang target files:
- `/home/jan/gh/openfang/crates/openfang-api/src/server.rs`
- `/home/jan/gh/openfang/crates/openfang-api/src/routes.rs`
- `/home/jan/gh/openfang/crates/openfang-kernel/src/kernel.rs`
- `/home/jan/gh/openfang/docs/api-reference.md`
- `/home/jan/gh/openfang/docs/mcp-a2a.md`

## 9. Recommended Start Point For The Next Session

Begin in this order:
1. read `docs/phase-4-mcp-crud-prep.md`
2. port the LibreFang MCP CRUD handlers into OpenFang API routes
3. fix the runtime MCP reload semantics before calling the phase complete

That order keeps Phase 4 from landing as "config file CRUD only" without matching live daemon behavior.
