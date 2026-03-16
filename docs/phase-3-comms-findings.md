# Phase 3 Findings — Rich `comms_send`

This document captures the main findings, implementation notes, and verification results for Phase 3 of the OpenFang 12-phase roadmap.

Phase:
- Phase 3: Rich `comms_send`

Roadmap source:
- `docs/roadmap-12-phases.md`

Date:
- 2026-03-16

## 1. Outcome

Phase 3 is complete at the API and SDK level.

Implemented:
- `comms_send` now supports `thread_id`
- `comms_send` now supports file attachments via uploaded file references
- `comms_send` now supports channel-facing delivery in addition to agent-to-agent delivery
- JavaScript and Python SDK surfaces now expose comms endpoints
- API documentation updated

## 2. What Was Added

Request shape:
- `from_agent_id`
- `to_agent_id` for agent delivery
- `channel` and `recipient` for channel delivery
- `message`
- `thread_id`
- `attachments`

Attachment model:
- attachments use uploaded file references
- shape:
  - `file_id`
  - `filename`
  - `content_type`

Delivery modes:
- agent mode:
  - sends a message to another OpenFang agent
  - uploaded image attachments are injected into the recipient session before text delivery
- channel mode:
  - sends to a configured outbound channel adapter
  - applies `thread_id` when the adapter supports threaded replies
  - sends uploaded attachments as file data through the channel adapter path

SDK updates:
- JavaScript SDK:
  - added `client.comms.send(...)`
  - added `client.comms.task(...)`
  - added `client.comms.topology()`
  - added `client.comms.events(...)`
- Python SDK:
  - added `client.comms.send(...)`
  - added `client.comms.task(...)`
  - added `client.comms.topology()`
  - added `client.comms.events(...)`
  - added `client.agents.upload(...)` so attachment refs can be created from Python

## 3. Important Findings

### Finding 1: `comms_send` was previously not channel-facing at all

Before this phase, `/api/comms/send` only supported:
- `from_agent_id`
- `to_agent_id`
- `message`

It validated agent IDs and then directly called agent message delivery.

Practical conclusion:
- `thread_id` and attachments could not be added meaningfully without broadening the delivery model
- Phase 3 therefore required `comms_send` to support channel delivery targets, not only agent targets

### Finding 2: existing thread support already lived in channel delivery primitives

The kernel/channel adapter path already had:
- `send_channel_message(..., thread_id)`
- `send_channel_media(..., thread_id)`
- `send_channel_file_data(..., thread_id)`

Practical conclusion:
- the right implementation path was to reuse the existing channel adapter delivery surface
- Phase 3 mainly needed API and wire-shape work, not a brand-new threading subsystem

### Finding 3: attachment support is asymmetric across targets

For agent delivery:
- uploaded image attachments can be injected into the recipient session

For channel delivery:
- uploaded attachments are sent as file data through the adapter path

Practical conclusion:
- `comms_send` now supports attachments in a real and useful way
- but attachment behavior is still target-dependent because agent-session media handling and channel-adapter media handling are different subsystems

### Finding 4: not every channel adapter supports threaded replies or file uploads equally

`comms_send` now exposes `thread_id` and attachments through the generic channel path, but actual behavior still depends on adapter capability.

Practical conclusion:
- Phase 3 is complete at the generic `comms_send` surface
- adapter-specific richness still varies and should be treated as channel capability variance, not as a blocker for the phase

### Finding 5: first-party comms UIs still use the legacy 3-field send shape

The existing web/TUI comms send flows remain backward compatible because the new API shape is additive.

Practical conclusion:
- the phase is complete at API/SDK level
- richer first-party comms UI affordances are a follow-up UX task, not a blocker for this phase

## 4. Verification Completed

Formatting:

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

Results:
- API integration tests passed
- new `comms_send` integration coverage passed
- Python SDK file compiled cleanly
- JavaScript SDK file parsed cleanly

## 5. Integration Coverage Added

The API integration test suite now verifies:
- `comms_send` rejects requests without exactly one delivery target
- channel delivery works through a mock channel adapter
- `thread_id` is forwarded into threaded adapter sends
- uploaded attachments are delivered through the channel file-data path

Practical conclusion:
- the new `comms_send` behavior is covered beyond pure deserialization tests

## 6. Files Touched for Phase 3

Main implementation files:
- `crates/openfang-types/src/comms.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `sdk/javascript/index.d.ts`
- `sdk/javascript/index.js`
- `sdk/python/openfang_client.py`
- `docs/api-reference.md`

This findings document:
- `docs/phase-3-comms-findings.md`

## 7. Recommendation for Future Work

Good follow-up items, but not blockers for Phase 3 completion:
- expose rich `comms_send` fields in the web comms page and TUI comms screen
- normalize attachment handling further across agent-target and channel-target flows
- improve adapter-level support where thread or file handling is still shallow
- keep the mock channel adapter current as a maintained contract test; see `docs/channel-adapter-test-concept.md`

## 8. Next Suggested Step

Proceed to Phase 4:
- MCP server CRUD API

Why:
- it remains one of the agreed high-ROI API/admin completion items
- the current roadmap sequence already places it next
