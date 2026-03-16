# Test Playbooks

This directory contains operator-style test playbooks for scenarios that are valuable but not fully covered by deterministic automated tests.

Use a playbook when the check depends on one or more of:
- local GUI/browser state
- external services
- OAuth or secrets
- interactive/manual verification
- operational setup rather than pure code behavior

Do not create a playbook for normal unit or CI-stable integration tests.

## Current Playbooks

- [Chrome DevTools MCP Smoke](chrome-devtools-mcp-smoke.md)
- [Integration Install & Reconnect Smoke](integration-install-reconnect-smoke.md)
- [Local Webhook Channel Smoke](webhook-channel-smoke.md)
- [Telegram Channel Smoke](telegram-channel-smoke.md)
- [Release Validation](release-validation.md)

## Scope

These playbooks cover the main classes of non-deterministic validation currently needed in OpenFang:
- MCP/browser integration smoke
- integration lifecycle smoke
- local daemon-backed channel smoke
- real external channel smoke
- release-time operational verification
