# Session Handoff - 2026-03-16 Phase 5 Follow-ups

This document captures the work completed after Phase 5 implementation, specifically the repo-wide semantic review, the follow-up fixes, and the new dedicated SDK/TUI test harnesses.

This document should be read together with:
- `docs/session-handoff-2026-03-16-phase-4-completion.md`
- `docs/phase-5-backup-restore-findings.md`
- `docs/roadmap-12-phases.md`

## 1. Scope of This Session

This session focused on:
- repo-wide semantic review of the current worktree
- fixing real breaks and drift discovered during that review
- tightening backup/restore semantics after Phase 5
- fixing comms/attachment contract mismatches
- fixing CLI/TUI MCP and comms regressions
- adding dedicated JavaScript and Python SDK test harnesses
- wiring those new SDK tests into CI and contributor docs

## 2. What Was Found

The semantic review surfaced nine meaningful issues across backend, CLI/TUI, SDKs, and docs.

Most important categories:
- backup/restore config-path drift when the daemon is booted from a non-default config path
- partial restore being surfaced as unconditional success
- agent-to-agent comms silently dropping non-image attachments while reporting success
- CLI/TUI still parsing the older `/api/mcp/servers` response shape
- TUI comms modal mishandling spaces in the message field
- JavaScript and Python SDK upload helpers not preserving the real upload contract and auth headers

Practical conclusion:
- Phase 5 was functionally complete, but the surrounding operational surfaces still needed a stabilization pass
- the semantic review was worth doing because broad `cargo test` and `cargo build` were green even though several runtime-contract issues remained

## 3. What Was Fixed

### 3.1 Backup / Restore Stabilization

The kernel now tracks the effective config source path instead of assuming `home_dir/config.toml`.

Important result:
- backup includes the real active config file even when the daemon was booted from a custom path
- restore writes `config.toml` payloads back to the real active config path
- MCP config reload and CRUD helpers now read/write against the same config source the daemon is actually using
- the config hot-reload watcher also follows that same source path

Restore semantics were also tightened:
- partial restore now returns an error response at the API layer instead of a silent `200 OK`
- the response still includes `restored_files`, `errors`, and `manifest` so operators can inspect what failed
- audit logging now distinguishes clean restore from partial restore

Main files:
- `crates/openfang-kernel/src/kernel.rs`
- `crates/openfang-kernel/src/backup.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/src/server.rs`

### 3.2 Comms / Attachment Contract Fixes

Agent-to-agent comms no longer silently discards non-image attachments.

Important result:
- agent delivery now explicitly rejects non-image attachments with `400 Bad Request`
- `attachments_received` now reflects what was actually injected
- the API docs were updated to make the current image-only agent-delivery behavior explicit

Main files:
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `docs/api-reference.md`

### 3.3 CLI / TUI Drift Fixes

Two TUI/CLI regressions were fixed:
- the TUI comms send modal now accepts spaces correctly in the message field
- the CLI/TUI MCP server views now understand the current wrapped `/api/mcp/servers` response shape

Important result:
- the TUI MCP screen no longer renders an empty list against the new daemon response
- `openfang doctor` reports MCP configured/connected counts again

Main files:
- `crates/openfang-cli/src/tui/screens/comms.rs`
- `crates/openfang-cli/src/tui/event.rs`
- `crates/openfang-cli/src/main.rs`

### 3.4 Dedicated SDK Test Harnesses Were Added

New dedicated harnesses now exist for the REST SDKs:
- JavaScript SDK via `node:test`
- Python SDK via `unittest`

What they currently cover:
- upload helper behavior
- header preservation
- auth propagation for upload requests

These were also wired into CI and contributor docs.

Main files:
- `sdk/javascript/test/index.test.js`
- `sdk/javascript/package.json`
- `sdk/python/tests/test_openfang_client.py`
- `.github/workflows/ci.yml`
- `CONTRIBUTING.md`
- `docs/README.md`

## 4. Verification Completed

Formatting:

```bash
cargo fmt --all
```

Targeted backend verification:

```bash
cargo test -p openfang-kernel create_and_restore_backup_round_trip
cargo test -p openfang-kernel backup_and_restore_follow_custom_config_path
cargo test -p openfang-api --test api_integration_test test_backup_restore_endpoints_round_trip
cargo test -p openfang-api --test api_integration_test test_restore_partial_failure_returns_500
cargo test -p openfang-api --test api_integration_test test_comms_send_agent_rejects_non_image_attachments
```

Broad backend verification:

```bash
cargo test -p openfang-api -p openfang-kernel
cargo build --bin openfang
```

New dedicated SDK / CLI harness verification:

```bash
cargo test -p openfang-cli
npm --prefix sdk/javascript test
python -m unittest discover -s sdk/python/tests
```

Results:
- targeted backup/restore tests passed
- targeted comms attachment rejection test passed
- openfang-api tests passed
- openfang-kernel tests passed
- openfang-cli tests passed
- JavaScript SDK harness passed
- Python SDK harness passed
- binary build passed

## 5. Current Practical State After This Session

Completed:
- Phase 1
- Phase 2
- Phase 3
- Phase 3 follow-ups
- Phase 4
- Phase 5

Also completed:
- Phase 5 stabilization follow-up from repo-wide semantic review
- dedicated JS/Python SDK test harness baseline
- CI wiring for those SDK harnesses

## 6. Important Practical Notes For The Next Session

### Note 1: Phase 5 is now in a much cleaner stopping state

The highest-signal follow-up issues discovered after the initial Phase 5 implementation were fixed in the same day.

The key operational bug to remember is:
- config-path correctness matters across backup, restore, MCP CRUD, and config reload

That area is now materially safer than the initial Phase 5 landing state.

### Note 2: Agent attachment semantics are intentionally still narrower than channel delivery

Current behavior:
- channel delivery supports generic files
- agent delivery supports image attachments only

This is now explicit instead of silently lossy, but it is still a product limitation worth revisiting later if richer inter-agent file handoff becomes important.

### Note 3: The SDK harnesses are intentionally lightweight

The new JS/Python suites are not full end-to-end daemon tests.

They are currently meant to guard:
- request construction
- header propagation
- upload contract compatibility

If SDK surface area grows, these harnesses should probably be extended before the next major SDK-facing feature lands.

## 7. Recommended Immediate Next Step

Proceed to Phase 6:
- Deterministic Hand Identity

Why:
- backup/restore now has both implementation and stabilization follow-up completed
- the next stability-sensitive operational gap is preserving Hand identity semantics across restart and restore

Secondary worthwhile thread after that:
- broaden the dedicated SDK harnesses beyond uploads into comms/task/session helper coverage

## 8. Resume Checklist

When resuming:
1. read `docs/roadmap-12-phases.md`
2. read `docs/phase-5-backup-restore-findings.md`
3. read `docs/session-handoff-2026-03-16-phase-5-followups.md`
4. review the new SDK harnesses:
   - `sdk/javascript/test/index.test.js`
   - `sdk/python/tests/test_openfang_client.py`
5. if Phase 6 starts, inspect Hand persistence / restoration code paths before changing IDs
