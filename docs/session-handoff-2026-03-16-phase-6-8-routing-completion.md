# Session Handoff - 2026-03-16 Phase 6-8 Routing Completion

This document captures the work completed after the Phase 5 follow-up session, specifically:
- Phase 6 completion for deterministic Hand identity
- Phase 7 completion for the native builtin router
- Phase 8 completion for the routing capability registry surface

This document should be read together with:
- `docs/session-handoff-2026-03-16-phase-5-followups.md`
- `docs/roadmap-12-phases.md`
- `docs/architecture.md`

## 1. Scope of This Session

This session focused on:
- finishing the remaining operational work for deterministic Hand identity
- making Hand pause/resume semantics restart-stable
- adding a native `builtin:router` kernel execution path
- wiring the existing `orchestrator` template onto that native router path
- introducing an explicit routing capability registry surface for Hands, workflows, and agents
- exposing that routing capability surface over the admin API

## 2. What Was Found

The practical findings were:
- Hand agent identity was already partly deterministic, but Hand `instance_id` was still random
- API/UI/TUI Hand lifecycle actions depend on `instance_id`, so restart drift there still mattered
- paused Hands were not persisted, so restart semantics were softer than the UI implied
- the first native router implementation needed to avoid falling back into the normal LLM loop
- a router-only internal catalog was not enough for Phase 8; the router needed an explicit capability surface it could consume

Practical conclusion:
- Phase 6 was not only about stable `agent_id`; it also needed stable `instance_id` and restart-safe pause semantics
- Phase 7 could be landed OpenFang-natively without copying LibreFang logic directly
- Phase 8 should be implemented as an explicit routing registry layer rather than more router-local heuristics

## 3. What Was Fixed

### 3.1 Phase 6: Deterministic Hand Identity Was Completed

Hands now preserve stable identity semantics across restart and restore.

Important result:
- Hand agents use deterministic `agent_id` values derived from `hand_id`
- Hand instances now also use deterministic `instance_id` values
- `hand_state.json` persists `instance_id`
- restore reuses persisted Hand identity instead of minting new instance IDs
- cron and trigger reassignment logic still protects older persisted state during restart

Main files:
- `crates/openfang-hands/src/lib.rs`
- `crates/openfang-hands/src/registry.rs`
- `crates/openfang-kernel/src/kernel.rs`

### 3.2 Hand Pause / Resume Semantics Were Hardened

Pause semantics are now explicit and restart-stable.

Important result:
- paused Hands are persisted in `hand_state.json`
- paused Hands restore as `Paused` instead of silently becoming active
- paused Hands do not restart autonomous background loops on boot
- `resume_hand` only succeeds from the `Paused` state
- direct `send_message` calls remain allowed while autonomous execution stays paused

Practical product decision:
- `paused` was kept as a hard autonomous stop
- it was intentionally not turned into an auto-wake or “soft sleep” mode
- any future wake-on-message behavior should be modeled as a distinct `standby`-style concept, not by weakening `pause`

Main files:
- `crates/openfang-hands/src/registry.rs`
- `crates/openfang-kernel/src/kernel.rs`

### 3.3 Phase 7: Native Builtin Router Was Added

OpenFang now has a true native router execution path.

Important result:
- `builtin:router` is dispatched natively in the kernel
- router execution is deterministic and rule-based
- router decisions no longer require the normal LLM agent loop
- the router can target:
  - Hands
  - workflows
  - specialist agents
  - assistant fallback
- the existing `orchestrator` template now uses `module = "builtin:router"`

Implementation notes:
- the router supports explicit selectors such as `agent:...`, `hand:...`, and `workflow:...`
- direct mentions and keyword heuristics are still supported
- async recursion was explicitly broken with boxed futures at the router dispatch boundary so the kernel could delegate safely without Rust async type recursion

Main files:
- `crates/openfang-kernel/src/router.rs`
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-kernel/src/lib.rs`
- `agents/orchestrator/agent.toml`
- `crates/openfang-types/src/agent.rs`

### 3.4 Phase 8: Routing Capability Registry Was Added

The router now consumes an explicit capability-oriented registry surface.

Important result:
- Hands, workflows, and agents are represented as routing capabilities
- routing capabilities carry:
  - kind
  - identifier
  - name
  - description
  - tags
  - routing keywords
  - target record
- the router builds its execution catalog from that routing capability layer instead of directly from raw registries
- a read-only admin API endpoint now exposes routing capabilities at:
  - `GET /api/routing/capabilities`

Why this matters:
- routing is no longer only name-based or template-based
- Phase 8 now has a concrete substrate Phase 9 can use when deciding whether an existing capability fits a goal

Main files:
- `crates/openfang-kernel/src/routing_capabilities.rs`
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/tests/api_integration_test.rs`

## 4. Verification Completed

Formatting:

```bash
cargo fmt --all
```

Targeted Phase 6 verification:

```bash
cargo test -p openfang-hands persist_and_load_state_round_trip_preserves_identity
cargo test -p openfang-hands load_state_accepts_legacy_entries_without_instance_id
cargo test -p openfang-hands hand_instance_identity_is_stable_per_hand
cargo test -p openfang-kernel test_hand_identity_survives_restart_restore
```

Targeted pause/resume verification:

```bash
cargo test -p openfang-hands persist_and_load_state_round_trip_preserves_paused_status
cargo test -p openfang-kernel test_paused_hand_survives_restart_without_background_restart
cargo test -p openfang-kernel test_resume_paused_hand_restarts_background_loop
cargo test -p openfang-kernel test_paused_hand_still_allows_direct_messages
```

Targeted Phase 7 / 8 router verification:

```bash
cargo test -p openfang-kernel explicit_agent_route_strips_selector_from_forward_message
cargo test -p openfang-kernel direct_workflow_mention_beats_fallback
cargo test -p openfang-kernel browser_keywords_route_to_browser_hand
cargo test -p openfang-kernel falls_back_to_assistant_when_no_match_exists
cargo test -p openfang-kernel test_builtin_router_routes_to_explicit_python_agent
cargo test -p openfang-kernel test_routing_capability_registry_includes_hands_workflows_and_agents
cargo test -p openfang-api --test api_integration_test test_list_routing_capabilities_endpoint
```

Results:
- deterministic Hand identity tests passed
- restart-safe paused Hand tests passed
- router rule tests passed
- native router end-to-end dispatch test passed
- routing capability registry test passed
- routing capability API integration test passed

## 5. Current Practical State After This Session

Completed:
- Phase 1
- Phase 2
- Phase 3
- Phase 3 follow-ups
- Phase 4
- Phase 5
- Phase 5 follow-up stabilization pass
- Phase 6
- Phase 7
- Phase 8

Practical summary:
- Hands now have stable restart semantics at both agent and instance level
- Hand pause/resume behavior now matches the operator-facing meaning of the UI
- OpenFang has a native system router instead of relying on a prompt-only orchestrator
- the router now consumes an explicit capability-oriented view of the system

## 6. Important Practical Notes For The Next Session

### Note 1: `paused` is intentionally hard for autonomous work, but not a full disconnect

Current behavior:
- paused Hands do not auto-run in background
- paused Hands remain paused across restart
- direct user/system messages are still allowed

This means:
- `paused` is not a future `standby` feature
- if wake-on-message semantics are wanted later, model them as a separate state or policy

### Note 2: The router is deterministic by design

Current behavior:
- explicit selectors win
- direct name mentions are supported
- hand/agent keyword routing is rule-based
- assistant fallback is explicit

This is intentional:
- the native router should be explainable and testable
- more semantic or fuzzy matching should be layered onto a capability model carefully, not by sneaking LLM behavior back into the basic dispatch path

### Note 3: Phase 8 is a first capability layer, not the final semantic registry

The new routing capability records are sufficient for deterministic selection, but they are still lightweight.

What they currently capture:
- type/kind
- name and description
- tags
- routing keywords
- target linkage

What they do not yet capture:
- richer structured skill semantics
- confidence or ranking metadata
- “can create missing capability” builder hooks

That is acceptable for Phase 8 and is the natural handoff point into Phase 9.

## 7. Recommended Immediate Next Step

Proceed to Phase 9:
- User-in-the-Loop Agent Builder

Why:
- the system now has a native router
- the router now has an explicit capability registry surface
- the next strategic step is deciding what to do when no existing capability is a good fit

Practical builder direction:
- detect “no good existing capability”
- produce a reviewable draft for:
  - agent
  - Hand
  - workflow
- require user approval before creation or activation

## 8. Resume Checklist

When resuming:
1. read `docs/roadmap-12-phases.md`
2. read `docs/session-handoff-2026-03-16-phase-6-8-routing-completion.md`
3. inspect:
   - `crates/openfang-kernel/src/router.rs`
   - `crates/openfang-kernel/src/routing_capabilities.rs`
   - `crates/openfang-kernel/src/kernel.rs`
4. inspect the new routing capability API:
   - `GET /api/routing/capabilities`
5. if Phase 9 starts, define the builder’s “no suitable capability found” threshold before writing code

## 9. Addendum: What Happened After The Original Phase 8 Handoff

Sections 5 through 8 above describe the state at the end of Phase 8.
They are now historically accurate but no longer the current head state.
This addendum supersedes the old “Phase 9 is next” recommendation.

## 10. Phase 9 Was Completed

Phase 9 is now implemented as a native, approval-backed capability builder.

Current result:
- the router can detect a real capability gap and return a reviewable draft instead of silently falling through to assistant fallback
- proposals can be generated as:
  - `agent`
  - `workflow`
  - `hand`
- proposal application is approval-backed and tracked as a job
- proposal jobs are queryable over the admin API
- the flow is now available in:
  - API
  - dashboard builder page
  - CLI
  - TUI

Main APIs:
- `GET /api/routing/capabilities`
- `POST /api/routing/proposals`
- `POST /api/routing/proposals/apply`
- `GET /api/routing/proposals/jobs`
- `GET /api/routing/proposals/jobs/{id}`

Main files:
- `crates/openfang-kernel/src/capability_builder.rs`
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/src/types.rs`
- `crates/openfang-api/static/js/pages/builder.js`
- `crates/openfang-cli/src/main.rs`
- `crates/openfang-cli/src/tui/screens/builder.rs`
- `crates/openfang-cli/src/tui/event.rs`
- `crates/openfang-cli/src/tui/mod.rs`

## 11. Stabilization Passes After Phase 9

After the builder landed, a larger stabilization pass fixed both real breaks and softer UI/API contract drift.

### 11.1 Real Breaks That Were Fixed

Important fixes:
- builder-created Hands are now persisted on disk and survive restart
- builder proposal jobs are now persisted and included in backup/restore
- unfinished proposal jobs are restored deterministically as failed after restart instead of silently disappearing
- builder workflow drafts no longer point their steps back at the router/orchestrator path
- duplicate workflow names are rejected on apply instead of creating nondeterministic routing
- the in-process TUI builder submit path no longer depends on a missing Tokio runtime

Main files:
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-kernel/src/capability_builder.rs`
- `crates/openfang-kernel/src/backup.rs`
- `crates/openfang-api/tests/api_integration_test.rs`

### 11.2 Soft Break / Drift Fixes That Were Landed

Important fixes:
- dashboard builder now shows explicit load errors instead of degrading to fake empty states
- last selected builder job restore now actually restores the current job
- TUI builder keeps job selection on refresh, clears stale post-approval state, and renders workflow proposal previews
- Comms page now uses the correct authenticated SSE path and closes streams on page leave
- Hands, scheduler, settings, and extensions pages now surface partial/fetch failures instead of silently rendering misleading empty states
- TUI parser drift was fixed for sessions, memory KV, skills, audit wrappers, channels, workflows, triggers, usage wrappers, settings wrappers, peers, and security
- the TUI security screen now preserves its curated 15-feature catalog while still applying live status from `/api/security`
- `/api/usage` now includes `cost_usd` per agent so the TUI by-agent usage view no longer silently renders zero cost

Main files:
- `crates/openfang-api/static/js/pages/builder.js`
- `crates/openfang-api/static/js/pages/comms.js`
- `crates/openfang-api/static/js/pages/hands.js`
- `crates/openfang-api/static/js/pages/scheduler.js`
- `crates/openfang-api/static/js/pages/settings.js`
- `crates/openfang-api/static/js/pages/agents.js`
- `crates/openfang-api/static/js/app.js`
- `crates/openfang-api/static/index_body.html`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-cli/src/tui/event.rs`
- `crates/openfang-cli/src/tui/mod.rs`
- `crates/openfang-cli/src/tui/screens/builder.rs`
- `crates/openfang-cli/src/tui/screens/security.rs`
- `crates/openfang-cli/src/tui/screens/settings.rs`

### 11.3 One Confirmed Zombie Path Was Removed

Removed:
- the unused `KernelHandle::hand_install()` path was deleted after confirming that the real hand-install flow no longer used it

Main files:
- `crates/openfang-runtime/src/kernel_handle.rs`
- `crates/openfang-kernel/src/kernel.rs`

## 12. Verification Added During The Continuation Sessions

Targeted builder and stabilization verification included:

```bash
cargo test -p openfang-kernel test_router_returns_builder_draft_for_missing_capability_gap -- --nocapture
cargo test -p openfang-kernel capability_builder::tests -- --nocapture
cargo test -p openfang-kernel test_builder_hand_persists_across_restart -- --nocapture
cargo test -p openfang-kernel test_builder_workflow_does_not_bind_steps_to_router -- --nocapture
cargo test -p openfang-kernel test_builder_workflow_rejects_duplicate_names -- --nocapture
cargo test -p openfang-kernel test_submit_capability_proposal_without_runtime_still_completes -- --nocapture
cargo test -p openfang-kernel test_pending_proposal_job_restored_as_failed_after_restart -- --nocapture
cargo test -p openfang-kernel create_and_restore_backup_round_trip -- --nocapture
cargo test -p openfang-api --test api_integration_test test_create_routing_proposal_endpoint -- --nocapture
cargo test -p openfang-api --test api_integration_test test_apply_routing_proposal_creates_workflow_after_approval -- --nocapture
cargo test -p openfang-api --test api_integration_test test_apply_routing_proposal_fails_for_duplicate_workflow_name -- --nocapture
cargo test -p openfang-api --test api_integration_test test_usage_stats_includes_agent_costs -- --nocapture
cargo test -p openfang-cli parse_security_features_preserves_builtin_catalog -- --nocapture
cargo test -p openfang-cli --no-run
cargo check -p openfang-api -p openfang-cli
```

Practical verification that was also run:
- real CLI builder end-to-end against an isolated daemon
- PTY TUI smoke test for the Builder tab
- browser smoke tests for builder error-state handling and current-job restore
- browser sanity checks for Comms live/offline state

## 13. Current Practical State After The Continuation Sessions

Completed now:
- Phase 1
- Phase 2
- Phase 3
- Phase 3 follow-ups
- Phase 4
- Phase 5
- Phase 5 follow-up stabilization pass
- Phase 6
- Phase 7
- Phase 8
- Phase 9

Practical summary:
- the routing stack is now native, deterministic, and capability-backed
- the system can propose, approve, and apply new capabilities when routing finds a genuine gap
- the builder flow is operator-usable from API, dashboard, CLI, and TUI
- the major restart, backup, proposal-job, and self-routing failure modes found after Phase 9 have been fixed
- the most obvious UI/API contract drift in the current builder/comms/settings/security/usage surfaces has been corrected

## 14. What Is Still Left / Recommended Next Work

### 14.1 Immediate Remaining Work

Still worth doing:
- add broader automated integration coverage for the new builder/UI/TUI paths instead of relying mainly on smoke tests
- decide whether `/api/audit/recent` should expose a stable human-readable agent display field; the TUI currently still shows `agent_id` because that is the only stable API field
- run a broader workspace-level regression pass when the current change stack settles

### 14.2 Recommended Next Feature Phase

According to `docs/roadmap-12-phases.md`, the next roadmap phase after the now-completed builder work is:
- Phase 10: Extensions API

Practical recommendation:
- if feature work resumes immediately, start Phase 10
- if stabilization takes priority, first add deeper regression coverage around builder jobs, approvals, dashboard error states, and TUI response-shape parsing

## 15. Updated Resume Checklist

When resuming from the current head:
1. read `docs/roadmap-12-phases.md`
2. read this handoff file, including Sections 9 through 15
3. inspect the Phase 9 core:
   - `crates/openfang-kernel/src/capability_builder.rs`
   - `crates/openfang-kernel/src/kernel.rs`
   - `crates/openfang-api/src/routes.rs`
4. inspect the operator surfaces:
   - `crates/openfang-api/static/js/pages/builder.js`
   - `crates/openfang-cli/src/main.rs`
   - `crates/openfang-cli/src/tui/screens/builder.rs`
   - `crates/openfang-cli/src/tui/event.rs`
5. decide whether the next session is:
   - Phase 10 feature work
   - or a wider regression/integration-test pass
