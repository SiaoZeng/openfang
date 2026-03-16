# Telegram Channel Smoke

This playbook verifies the real Telegram adapter using a dedicated bot and test chat or topic.

Date:
- 2026-03-16

## 1. Purpose

Use this playbook when changing:
- Telegram adapter behavior
- `thread_id` handling
- attachment delivery semantics
- real external channel delivery behavior

This is a manual or nightly smoke path. It should not be a normal blocking per-PR test.

## 2. Prerequisites

Required:
- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

Optional:
- `TELEGRAM_THREAD_ID` for topic/thread validation

Recommended:
- a dedicated test chat or forum topic

## 3. Script

Reference script:
- `scripts/telegram-smoke.sh`

## 4. Basic Run

```bash
TELEGRAM_BOT_TOKEN=... \
TELEGRAM_CHAT_ID=... \
./scripts/telegram-smoke.sh
```

Expected:
- isolated daemon starts
- Telegram adapter config is applied
- outbound `comms_send` request is accepted
- the message arrives in the target Telegram chat

Expected terminal result:
- `Telegram smoke request accepted.`
- plus manual verification in Telegram itself

## 5. Thread/Topic Run

```bash
TELEGRAM_BOT_TOKEN=... \
TELEGRAM_CHAT_ID=... \
TELEGRAM_THREAD_ID=... \
./scripts/telegram-smoke.sh
```

Expected:
- request body includes `thread_id`
- message appears in the target Telegram topic/thread

## 6. Attachment Run

Attachments are enabled by default.

The script:
- uploads a small text file through the agent upload endpoint
- sends it through `/api/comms/send`

Expected:
- message plus attachment is accepted by the API
- attachment is visible in the Telegram chat/topic

To disable attachment send:

```bash
SEND_ATTACHMENT=0 ./scripts/telegram-smoke.sh
```

## 7. Useful Variants

Preserve temp files:

```bash
KEEP_TELEGRAM_SMOKE_TMP=1 ./scripts/telegram-smoke.sh
```

Skip rebuild:

```bash
SKIP_TELEGRAM_SMOKE_BUILD=1 ./scripts/telegram-smoke.sh
```

## 8. What This Proves

- real external channel adapter acceptance
- real outbound delivery behavior
- real thread/topic handling
- real attachment flow

## 9. Failure Triage

If the daemon starts but Telegram delivery does not happen:
- verify bot token
- verify the bot has access to the target chat/topic
- verify `TELEGRAM_CHAT_ID` and `TELEGRAM_THREAD_ID`

If the API accepts the request but nothing arrives:
- inspect daemon log
- verify the bot is not rate-limited or restricted
- verify the topic still exists and the bot can post there

If attachments fail but text succeeds:
- inspect upload response
- inspect adapter-specific file handling
- confirm Telegram-side file restrictions are not being hit
