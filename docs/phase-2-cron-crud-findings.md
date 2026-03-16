# Phase 2 Findings — Cron CRUD Completion

This document captures the main findings, implementation notes, and verification results for Phase 2 of the OpenFang 12-phase roadmap.

Phase:
- Phase 2: Cron CRUD Completion

Roadmap source:
- `docs/roadmap-12-phases.md`

Date:
- 2026-03-16

## 1. Outcome

Phase 2 is functionally complete.

Implemented:
- `PUT /api/cron/jobs/{id}`
- validated in-place cron job updates in the kernel scheduler
- CLI cron surface aligned to the current API
- API documentation updated
- backend, unit, and end-to-end verification completed

## 2. What Was Added

Backend:
- `PUT /api/cron/jobs/{id}` now updates cron jobs in place
- supported patch fields:
  - `name`
  - `enabled`
  - `agent_id`
  - `schedule`
  - `action`
  - `delivery`

Scheduler behavior:
- updates are patch-based rather than full replace
- validation happens before the updated job is committed
- invalid updates do not mutate the existing persisted job
- enabling or rescheduling recomputes `next_run`
- re-enabling resets consecutive error count

CLI:
- `openfang cron update <id>` was added
- `openfang cron list` now reads the real wrapped API response shape
- `openfang cron enable` / `disable` now use the actual `PUT /api/cron/jobs/{id}/enable` endpoint with JSON body
- `openfang cron create` and `cron update --agent` now resolve agent names to UUIDs so the CLI matches its own help text

## 3. Important Findings

### Finding 1: CLI drift already existed before Phase 2

The CLI cron surface was not only missing `update`.

It had already drifted from the live API in several ways:
- `cron list` expected a flat array instead of `{ "jobs": [...], "total": ... }`
- `cron list` expected legacy fields such as `cron_expr` and `prompt`
- `cron create` expected a top-level `id` in the response
- `cron enable` / `disable` expected legacy path semantics instead of the current enable endpoint with request body

Practical conclusion:
- Phase 2 could not be considered complete at product level without also repairing the CLI surface

### Finding 2: API response shape is still inconsistent across cron endpoints

`POST /api/cron/jobs` currently returns:
- `{ "result": "<stringified json>" }`

while other cron endpoints return normal structured JSON bodies.

Practical conclusion:
- the CLI now tolerates the current create response
- but this API shape should be normalized later for consistency

This is not a blocker for Phase 2 completion, but it is a small API cleanup item.

### Finding 3: Agent reference ergonomics matter operationally

The CLI advertised "agent name or ID" for cron creation, but the backend path effectively needed a UUID.

Practical conclusion:
- resolving names in the CLI is the correct short-term fix
- this keeps the user-facing contract intact without forcing a backend change right now

## 4. Verification Completed

Formatting:

```bash
cargo fmt --all
```

Targeted backend/API verification:

```bash
cargo test -p openfang-api --test api_integration_test
cargo test -p openfang-kernel update_job
```

CLI verification:

```bash
cargo test -p openfang-cli
```

Results:
- API integration tests passed
- scheduler unit tests for cron update behavior passed
- CLI unit tests passed

## 5. End-to-End CLI Verification

A real isolated daemon-backed CLI flow was run against a temporary OpenFang home directory and a dedicated localhost port.

Verified flow:
1. daemon start
2. `openfang cron create`
3. `openfang cron list --json`
4. `openfang cron update`
5. `openfang cron enable`
6. `openfang cron delete`
7. final `openfang cron list --json`

Observed result:
- create succeeded
- list reflected the new job
- update changed name, schedule, prompt, and enabled state
- enable re-enabled the updated job
- delete removed the job
- final list returned zero jobs

Practical conclusion:
- the CLI is now aligned with the daemon-backed cron API in real usage, not only in unit tests

## 6. Files Touched for Phase 2

Main implementation files:
- `crates/openfang-kernel/src/cron.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/src/server.rs`
- `crates/openfang-cli/src/main.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `docs/api-reference.md`

This findings document:
- `docs/phase-2-cron-crud-findings.md`

## 7. Recommendation for Future Documentation

This kind of material is better kept in a phase-specific findings document than appended into the session handoff.

Reason:
- the session handoff is time-scoped
- phase findings are workstream-scoped
- implementation discoveries such as CLI/API drift are durable engineering knowledge, not just session context

Recommended pattern going forward:
- roadmap stays strategic
- session handoff stays operational
- phase findings docs capture implementation discoveries, verification, and cleanup notes

## 8. Next Suggested Step

At the time Phase 2 completed, the next suggested step was Phase 3:
- rich `comms_send`
- add `thread_id`
- add attachments

That Phase 3 work has since been completed.

Current next suggested step:
- Phase 4: MCP server CRUD API

Small cleanup item still worth keeping in mind later:
- normalize cron create response shape so it matches the structured response style used by other cron endpoints
