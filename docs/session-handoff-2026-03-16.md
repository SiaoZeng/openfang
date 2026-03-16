# Session Handoff — 2026-03-16

This document captures the main findings, decisions, and completed work from the current OpenFang session so the next session can resume with minimal context loss.

## 1. Repository Context

Primary repository:
- ` /home/jan/gh/openfang `

Comparison repository:
- ` /home/jan/gh/librefang `

Active branch during this session:
- OpenFang: `main`
- LibreFang: `main`

## 2. High-Level Understanding of OpenFang

OpenFang is a Rust monorepo for an Agent OS. The practical architecture is:

- `openfang-types`
- `openfang-memory`
- `openfang-runtime`
- `openfang-kernel`
- `openfang-api`
- `openfang-cli`
- `openfang-desktop`
- supporting crates for channels, hands, skills, migrate, wire, extensions

Key architectural understanding established in this session:

- The kernel is not itself an LLM agent.
- The kernel is the orchestration and runtime coordination layer.
- The kernel holds the global default model/driver, but only certain agent execution paths use it.
- Agent lifecycle is separate from execution style.
- An agent can exist as a managed runtime unit without being a normal LLM chat agent.

Important distinction clarified:

- `Kernel` = orchestrator, registry owner, capability gatekeeper, scheduler, memory owner, bridge to tools/channels/MCP
- `Default model` = the standard LLM configuration the kernel provides to LLM-backed agents
- `Agent` = a managed runtime entity with manifest, lifecycle, permissions, and execution module

Execution model summary:

- `builtin:chat` -> normal LLM agent loop
- `python:*` -> Python runtime path
- `wasm:*` -> WASM runtime path
- LibreFang additionally introduces `builtin:router`

## 3. LibreFang Comparison Findings

LibreFang is not a clean superset of OpenFang.

Direct git comparison established:

- LibreFang has `302` commits not in local OpenFang
- OpenFang has `45` commits not in local LibreFang

Interpretation:

- LibreFang is ahead in several areas
- but the branches have already diverged significantly
- LibreFang should be used as a design and selective code reference, not as a patch stream to apply blindly

Important qualitative conclusion:

- For early phases, LibreFang is a strong reference implementation
- For later strategic features, especially router/builder work, LibreFang is better treated as an architectural template than a direct code source

## 4. Strategic Direction Agreed in This Session

We aligned on a stronger Agent OS interpretation for OpenFang:

- A user should be able to describe a goal without pre-selecting the right agent.
- The system should route that goal to an existing capability.
- If no fitting capability exists, the system should help design a new capability with the user in the loop.
- Workflows are not just automation glue; they are reusable compositions of system capabilities.

Important product conclusion:

- The router is not a side feature.
- The router is a core Agent OS primitive.
- The longer-term goal is not only "more agents" but "capability routing + capability creation + workflow composition".

## 5. LibreFang Router Understanding

LibreFang's router was analyzed and the following was established:

- It is modeled as an agent in lifecycle terms.
- It is not a normal LLM chat agent.
- It uses `module = "builtin:router"`.
- The kernel dispatches it onto a dedicated native execution path.
- Its job is to route a request to:
  - a Hand
  - a specialist template/agent
  - a fallback path

This led to an important design idea for OpenFang:

- OpenFang should likely eventually add:
  - a builtin router
  - then a user-in-the-loop agent builder

The proposed builder concept:

- detect that no current capability matches the goal
- research what capability is missing
- design a new agent/hand/workflow with user confirmation
- scaffold a draft
- require approval before activation

Important guardrail agreed:

- the builder must not autonomously create and activate broad-capability agents without explicit approval

## 6. Porting Strategy Agreed

We explicitly agreed on the following strategy for LibreFang-derived work:

- use LibreFang as a reference
- do not blindly copy patches
- use it first as:
  - design reference
  - structure reference
  - selective code reference

Porting tiers agreed conceptually:

- directly portable
- partially portable
- conceptually portable

This principle is now reflected in the strategic roadmap document.

## 7. Feature Prioritization Agreed

The following items were classified as immediate/high-ROI:

- Backup/Restore API
- MCP server CRUD via API
- `comms_send` with `thread_id` and attachments
- Cron job update endpoint
- single-item detail endpoints for Tool / Profile / A2A agent
- deterministic Hand identities across restarts

The following were classified as medium priority / conditional:

- API versioning
- Extensions API
- local TOML-based provider catalog
- stable prefix mode / provider prompt caching
- multi-bot routing

The following were classified as later work:

- builtin router
- user-in-the-loop agent builder
- decision trace layer as the latest/later introspection item

## 8. 12-Phase Roadmap Created

A new strategic roadmap document was created:

- `docs/roadmap-12-phases.md`

It differs from the old launch roadmap in that it focuses on evolving OpenFang into a stronger Agent OS.

The 12 phases are:

1. API detail endpoints
2. Cron CRUD completion
3. Rich `comms_send`
4. MCP server CRUD API
5. Backup and restore
6. Deterministic Hand identity
7. Builtin router
8. Capability registry for routing
9. User-in-the-loop agent builder
10. Extensions API
11. Provider catalog modernization
12. Performance and scale layer

The roadmap also now contains:

- affected modules per phase
- porting approach per phase

Deliberate documentation decision:

- risks were intentionally left out to keep the roadmap focused
- detailed code-level acceptance criteria were intentionally not embedded in the roadmap
- best practice chosen:
  - roadmap stays strategic
  - technical acceptance belongs in phase plans, issues, and PRs

## 9. Best Practice Decision About Acceptance Criteria

We aligned on this best practice:

- Roadmap documents should define:
  - goal
  - scope
  - affected modules
  - porting approach
  - behavioral done criteria

- Roadmaps should not over-specify:
  - exact methods
  - exact internal helper names
  - exact low-level implementation structure

- Code-level acceptance should instead live in:
  - phase detail docs
  - issue descriptions
  - PR review criteria

The guiding rule agreed:

- prefer externally observable and testable behavior over implementation-shaped requirements

## 10. Phase 1 Started and Completed

Phase 1 goal:

- add single-item detail endpoints for:
  - profiles
  - tools
  - discovered external A2A agents

Implemented endpoints:

- `GET /api/profiles/{name}`
- `GET /api/tools/{name}`
- `GET /api/a2a/agents/{id}`

Behavior of the new A2A detail endpoint:

- lookup by index
- lookup by URL
- lookup by name

Main files changed for Phase 1:

- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `docs/api-reference.md`

## 11. Phase 1 Verification

Formatting run:

```bash
cargo fmt --all
```

Test run:

```bash
cargo test -p openfang-api --test api_integration_test
```

Result:

- test suite passed
- 20 integration tests passed
- newly added Phase 1 endpoint tests passed

During the first test attempt, Cargo dependency resolution hit sandbox network limits. The test was then rerun with approved elevated execution using the saved `cargo test` prefix rule.

## 12. Phase 2 Started and Completed

Phase 2 goal:

- add `PUT /api/cron/jobs/{id}`
- complete the editable cron management surface

Implemented work:

- added `PUT /api/cron/jobs/{id}` to the API
- added validated in-place cron job update support in the scheduler
- supported patch fields:
  - `name`
  - `enabled`
  - `agent_id`
  - `schedule`
  - `action`
  - `delivery`

Important implementation finding:

- the CLI cron surface had already drifted from the current cron API before Phase 2
- Phase 2 therefore required not only backend work but also CLI alignment to be truly complete at product level

CLI work completed:

- added `openfang cron update <id>`
- fixed `openfang cron list` to read the wrapped cron API response
- fixed `openfang cron enable` / `disable` to use the actual enable endpoint shape
- fixed cron create/update agent handling so agent names are resolved to UUIDs

New phase-specific document created:

- `docs/phase-2-cron-crud-findings.md`

## 13. Phase 2 Verification

Formatting run:

```bash
cargo fmt --all
```

Backend/API verification:

```bash
cargo test -p openfang-api --test api_integration_test
cargo test -p openfang-kernel update_job
```

CLI verification:

```bash
cargo test -p openfang-cli
```

End-to-end verification:

- ran an isolated daemon-backed CLI flow against a temporary OpenFang home directory
- verified:
  - cron create
  - cron list
  - cron update
  - cron enable
  - cron delete
- end-to-end cron flow passed

## 14. Phase 3 Started and Completed

Phase 3 goal:

- extend `comms_send`
- add `thread_id`
- add attachments

Implemented work:

- expanded `comms_send` request shape
- added channel-facing delivery mode to `/api/comms/send`
- added `thread_id` support for channel-target deliveries
- added attachment support using uploaded file references
- updated JavaScript and Python SDKs to expose comms endpoints

Important implementation findings:

- the old `comms_send` endpoint was agent-to-agent only and could not support realistic threaded/channel delivery without broadening the target model
- the kernel already had reusable channel send primitives with thread support, so the clean implementation path was to reuse those instead of inventing a second delivery stack
- attachment behavior is now real but still target-dependent:
  - agent-target attachments use session injection for uploaded images
  - channel-target attachments use outbound file delivery through channel adapters

New phase-specific document created:

- `docs/phase-3-comms-findings.md`

## 15. Phase 3 Verification

Formatting run:

```bash
cargo fmt --all
```

Backend/API verification:

```bash
cargo test -p openfang-api --test api_integration_test
```

SDK syntax verification:

```bash
python3 -m py_compile sdk/python/openfang_client.py
node -c sdk/javascript/index.js
```

Result:

- API integration tests passed
- new `comms_send` tests passed
- Python SDK syntax check passed
- JavaScript SDK syntax check passed

## 15.1 GraphRAG Note

The session findings were also written into the local GraphRAG MCP memory.

Important separation decision:

- detailed GraphRAG RCA and repair notes were moved into a separate handoff document because they do not belong to the OpenFang project handoff

Separate GraphRAG handoff:

- `/home/jan/.local/share/graphrag/session-handoff-2026-03-16-domain-filter-rca.md`

Practical rule for future OpenFang work:

- use this OpenFang handoff and the OpenFang roadmap as the primary source
- treat GraphRAG as a secondary memory/recovery layer

## 16. Important Files Created or Updated This Session

Created:

- `docs/roadmap-12-phases.md`
- `docs/session-handoff-2026-03-16.md`
- `docs/phase-2-cron-crud-findings.md`
- `docs/phase-3-comms-findings.md`

Updated:

- `docs/README.md`
- `docs/api-reference.md`
- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `crates/openfang-kernel/src/cron.rs`
- `crates/openfang-cli/src/main.rs`
- `crates/openfang-types/src/comms.rs`
- `sdk/javascript/index.d.ts`
- `sdk/javascript/index.js`
- `sdk/python/openfang_client.py`

## 17. Recommended Immediate Next Step

Resume with Phase 4:

- MCP server CRUD API

Why this should be next:

- it is already in the agreed roadmap
- it remains one of the agreed immediate/high-ROI API/admin items
- it continues the operational API-surface completion track
- it is the next sequential phase now that Phases 1 through 3 are complete

## 18. Resume Checklist for Next Session

When resuming:

1. read `docs/roadmap-12-phases.md`
2. read this handoff file
3. read `docs/phase-2-cron-crud-findings.md`
4. read `docs/phase-3-comms-findings.md`
5. confirm Phases 1 through 3 are complete and green
6. begin Phase 4 implementation
7. continue using LibreFang as a selective reference only

## 19. Core Strategic Memory

The most important durable conclusion from this session is:

- OpenFang should evolve from "many agents in one system" toward "an Agent OS that routes goals to capabilities and helps create missing capabilities safely".

That conclusion should guide future design decisions more than individual endpoint work.
