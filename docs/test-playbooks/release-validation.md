# Release Validation

This playbook is the operator-facing runtime companion to `docs/production-checklist.md`.

Date:
- 2026-03-16

## 1. Purpose

Use this playbook around a release candidate or a real release when you need a concise validation run that checks the shipped system rather than only build-time prerequisites.

This playbook should be run:
- before tagging a release candidate
- after the GitHub release workflow completes
- after any significant packaging or installer change

## 2. Pre-Tag Validation

Confirm locally:

```bash
cargo fmt --all
cargo test
cargo build --bin openfang
```

Run focused smoke checks:
- local webhook smoke
- one MCP/browser smoke

Recommended:

```bash
./scripts/webhook-smoke.sh
```

And:
- follow [Chrome DevTools MCP Smoke](chrome-devtools-mcp-smoke.md)

## 3. Packaging Validation

Check:
- CLI binary starts
- daemon starts
- dashboard is reachable

Example:

```bash
./target/debug/openfang --version
./target/debug/openfang doctor
```

If Docker is part of the release:

```bash
docker build -t openfang:local .
docker run --rm openfang:local --version
```

## 4. Integration Surface Validation

On a release candidate or built binary, verify:
- bundled integrations list is present
- at least one high-value integration installs and connects
- MCP server listing works

Recommended:
- follow [Integration Install & Reconnect Smoke](integration-install-reconnect-smoke.md)

## 5. Post-Release GitHub Validation

After the workflow completes, verify:
- release artifacts are present
- checksums are present
- desktop bundles exist for expected targets
- CLI archives exist for expected targets

Also verify:
- `latest.json` exists
- updater signatures are populated

This should be run together with:
- `docs/production-checklist.md`

## 6. Runtime Validation On Released Artifacts

For at least one released binary:
- start daemon
- hit `/api/health`
- hit `/api/status`
- verify a simple agent interaction path

If browser/MCP features are release-relevant:
- run the Chrome DevTools MCP smoke against the released binary

If channel delivery is release-relevant:
- run the Telegram smoke manually against a real test chat

## 7. Failure Triage

If CI artifacts are correct but runtime smoke fails:
- treat this as a packaging/runtime regression, not just a docs gap

If release binaries start but integrations fail:
- inspect bundled integration templates
- inspect dependency expectations (`npx`, Chrome, OAuth, env vars)

If updater artifacts are missing or unsigned:
- block the release
- resolve signing configuration before announcing availability
