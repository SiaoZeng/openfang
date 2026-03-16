# Local Webhook Channel Smoke

This playbook verifies the daemon-backed local webhook smoke path.

Date:
- 2026-03-16

## 1. Purpose

Use this playbook when changing:
- channel adapter delivery behavior
- `comms_send` channel delivery
- webhook adapter config
- attachment/thread delivery plumbing
- daemon-backed local smoke infrastructure

This is the preferred local real-stack smoke path because it avoids external provider dependencies.

## 2. Script

Reference script:
- `scripts/webhook-smoke.sh`

## 3. Basic Run

```bash
./scripts/webhook-smoke.sh
```

Expected:
- the script builds or reuses the current CLI binary
- an isolated daemon starts successfully
- a local callback receiver is started
- `comms_send` to the webhook channel succeeds
- the signed callback is received and verified

Expected terminal result:
- `Webhook smoke test passed.`

## 4. Useful Variants

Preserve temp files for inspection:

```bash
KEEP_WEBHOOK_SMOKE_TMP=1 ./scripts/webhook-smoke.sh
```

Skip rebuild if the binary is already current:

```bash
SKIP_WEBHOOK_SMOKE_BUILD=1 ./scripts/webhook-smoke.sh
```

## 5. What This Proves

- real daemon startup
- real HTTP routing
- real channel adapter startup
- real `comms_send` channel delivery path
- real webhook signing behavior

## 6. What This Does Not Prove

- external provider/API behavior
- OAuth flows
- production-grade adapter quirks like Telegram topics or Slack thread semantics

Use the Telegram playbook for a real external adapter check.

## 7. Failure Triage

If the daemon never becomes ready:
- inspect the daemon log preserved by `KEEP_WEBHOOK_SMOKE_TMP=1`

If the callback is never received:
- inspect callback listener log
- confirm webhook listen/callback ports are free
- inspect the send response body and daemon log

If signature verification fails:
- inspect the configured shared secret
- inspect webhook adapter request headers and callback payload
