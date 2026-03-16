# Channel Adapter Test Concept

This document defines how OpenFang should test channel-delivery behavior for features such as:
- `thread_id`
- file/media attachments
- generic `comms_send` channel delivery
- `channel_send` runtime tool behavior

Date:
- 2026-03-16

## 1. Why This Exists

Phase 3 added richer `comms_send` behavior, including:
- thread-aware channel delivery
- attachment-aware channel delivery

The current API integration test uses a mock channel adapter.

That is necessary, but not sufficient on its own.

Reason:
- a mock adapter proves the generic API/kernel contract
- it does not prove that a real external adapter still behaves correctly
- adapter-specific semantics can drift over time even while contract tests stay green

Practical conclusion:
- OpenFang needs both contract tests and real-adapter smoke tests

## 2. Testing Layers

### Layer A: Contract tests with a maintained mock adapter

Purpose:
- fast CI feedback
- deterministic tests
- no network, secrets, or external service dependency

What this layer should prove:
- `comms_send` requires exactly one delivery target
- `thread_id` is forwarded into threaded delivery calls
- attachments are forwarded through the expected file/media path
- generic response shape stays stable
- attachment upload references are resolved correctly

This is the current baseline layer.

### Layer B: Daemon-backed local integration tests

Purpose:
- exercise the real daemon
- exercise HTTP + routing + kernel + adapter plumbing end to end
- avoid external provider dependency where possible

Recommended form:
- start an isolated daemon
- register a local test adapter or webhook-backed adapter
- drive requests through real CLI/API calls

Reference implementation:
- `scripts/webhook-smoke.sh`

How to run it locally:

```bash
./scripts/webhook-smoke.sh
```

Useful options:

```bash
KEEP_WEBHOOK_SMOKE_TMP=1 ./scripts/webhook-smoke.sh
SKIP_WEBHOOK_SMOKE_BUILD=1 ./scripts/webhook-smoke.sh
```

What this layer should prove:
- daemon-backed send path works as expected
- upload + delivery path works under the daemon, not only in in-process tests
- thread behavior is preserved through the full stack

### Layer C: Real external adapter smoke tests

Purpose:
- prove that at least one production-grade adapter still behaves correctly
- catch drift that mocks cannot detect

Recommended initial adapter:
- Telegram

Why Telegram:
- supports real outbound thread handling
- supports file/media delivery better than most adapters
- is a strong reference adapter for Phase 3 semantics

Recommended execution mode:
- manual or nightly
- not required for every PR

## 3. Mock Adapter Policy

The mock adapter must be treated as a maintained contract adapter, not a disposable fake.

That means:
- it must evolve when the generic channel-delivery contract evolves
- it must not silently lag behind the production adapter surface
- test coverage should explicitly reflect the intended contract, not just current implementation accidents

## 4. When the Mock Adapter Must Be Reviewed

Whenever any of the following change, the mock adapter tests must be reviewed:
- `ChannelAdapter` trait
- `send_in_thread` behavior
- `ChannelContent` variants
- upload reference shape
- `comms_send` request or response shape
- `channel_send` runtime tool semantics
- kernel channel-delivery helpers
- attachment handling rules

Practical rule:
- any PR touching channel-delivery contracts must also confirm whether the mock adapter still represents the intended generic behavior
- if the change touches daemon-backed channel delivery, run `./scripts/webhook-smoke.sh`

## 5. Required Contract Cases

The maintained mock adapter test suite should always cover at least:
- plain text send
- threaded text send
- file attachment send
- mixed message + attachment send
- invalid target shape
- missing recipient/default recipient behavior
- attachment load failure
- adapter error propagation

Recommended additional split:
- one adapter mock that overrides `send_in_thread`
- one adapter mock that falls back to plain `send()`

Why:
- this makes thread-capable vs non-thread-capable behavior explicit
- it prevents generic code from accidentally assuming capabilities every adapter does not have

## 6. Real Adapter Smoke Test Strategy

### Recommended baseline

Run a real Telegram smoke test outside normal PR CI.

Suggested scope:
1. start isolated daemon
2. use dedicated Telegram bot token
3. send text to a dedicated test chat/topic
4. send threaded reply into a known topic
5. send a file attachment
6. verify success responses and basic delivery evidence

### What this should not try to be

It should not be:
- a full cross-channel certification suite
- a mandatory blocking PR test
- a brittle end-to-end acceptance system with many external dependencies

It should be:
- a narrow confidence test for one real, high-value adapter

## 7. CI / Execution Recommendation

Recommended split:

- PR CI:
  - contract tests with maintained mock adapter
  - daemon-backed local integration tests

- Nightly or manual:
  - real external adapter smoke test

This keeps:
- developer feedback fast
- secrets out of normal CI where possible
- real-world confidence available when needed

## 8. Ownership Recommendation

The mock adapter should have explicit ownership in review terms.

Recommended ownership rule:
- the person changing generic channel-delivery behavior is responsible for updating the contract mock tests

Recommended reviewer check:
- ask whether the change affects:
  - thread semantics
  - attachment semantics
  - fallback behavior
  - error propagation

If yes:
- the mock adapter coverage must be revisited in the same change

## 9. Suggested Future Work

Good next testing steps:
- add a second non-thread-capable mock adapter path
- extend the daemon-backed local webhook adapter smoke test as needed
- use `scripts/telegram-smoke.sh` as the manual or nightly Telegram smoke script baseline
- document required test credentials and cleanup rules

## 10. Summary

OpenFang should use a layered strategy:
- maintained mock adapter for contract coverage
- daemon-backed local integration for real stack coverage
- selective real external adapter smoke testing for production confidence

The key rule is:
- the mock adapter is valuable only if it is actively maintained as a contract test whenever channel-delivery semantics change
