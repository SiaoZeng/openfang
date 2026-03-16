# OpenFang Documentation

Welcome to the OpenFang documentation. OpenFang is the open-source Agent Operating System -- 14 Rust crates, 40 channels, 60 skills, 20 LLM providers, 170+ API endpoints, and 16 security systems in a single binary.

---

## Getting Started

| Guide | Description |
|-------|-------------|
| [Getting Started](getting-started.md) | Installation, first agent, first chat session |
| [Configuration](configuration.md) | Complete `config.toml` reference with every field |
| [CLI Reference](cli-reference.md) | Every command and subcommand with examples |
| [Troubleshooting](troubleshooting.md) | Common issues, FAQ, diagnostics |

## Core Concepts

| Guide | Description |
|-------|-------------|
| [Architecture](architecture.md) | 14-crate structure, kernel boot, agent lifecycle, memory substrate |
| [Agent Templates](agent-templates.md) | 30 pre-built agents across 4 performance tiers |
| [Workflows](workflows.md) | Multi-agent pipelines with branching, fan-out, loops, and triggers |
| [API Reference](api-reference.md#routing--builder-endpoints) | Routing capability registry, goal-gap analysis, and approval-backed builder jobs |
| [Security](security.md) | 16 defense-in-depth security systems |

## Integrations

| Guide | Description |
|-------|-------------|
| [Channel Adapters](channel-adapters.md) | 40 messaging channels -- setup, configuration, custom adapters |
| [LLM Providers](providers.md) | 20 providers, 51 models, 23 aliases -- setup and model routing |
| [Skills](skill-development.md) | 60 bundled skills, custom skill development, FangHub marketplace |
| [MCP & A2A](mcp-a2a.md) | Model Context Protocol and Agent-to-Agent protocol integration |

## Reference

| Guide | Description |
|-------|-------------|
| [API Reference](api-reference.md) | All 177 REST/WS/SSE endpoints with request/response examples |
| [Desktop App](desktop.md) | Tauri 2.0 native app -- build, features, architecture |

## Release & Operations

| Guide | Description |
|-------|-------------|
| [Production Checklist](production-checklist.md) | Every step before tagging v0.1.0 -- signing keys, secrets, verification |
| [12-Phase Roadmap](roadmap-12-phases.md) | Strategic roadmap for evolving OpenFang into a stronger Agent OS |
| [Session Handoff 2026-03-16](session-handoff-2026-03-16.md) | Detailed handoff notes from the roadmap, LibreFang comparison, and Phase 1/Phase 2/Phase 3 work |
| [Session Handoff 2026-03-16 Follow-up](session-handoff-2026-03-16-phase-3-followups.md) | Follow-up handoff covering Phase 3 UI aftercare, local smoke testing, and real adapter smoke-test preparation |
| [Session Handoff 2026-03-16 Phase 4 Completion](session-handoff-2026-03-16-phase-4-completion.md) | Follow-up handoff covering Phase 4 MCP CRUD completion, Chrome DevTools integration, browser-hand changes, and the new test playbooks |
| [Session Handoff 2026-03-16 Phase 5 Follow-ups](session-handoff-2026-03-16-phase-5-followups.md) | Follow-up handoff covering the repo-wide semantic review after Phase 5, the stabilization fixes, and the new dedicated SDK test harnesses |
| [Session Handoff 2026-03-16 Phase 6-8 Routing Completion](session-handoff-2026-03-16-phase-6-8-routing-completion.md) | Follow-up handoff covering deterministic Hand identity, hard pause/resume semantics, the native builtin router, and the routing capability registry |
| [Phase 2 Findings](phase-2-cron-crud-findings.md) | Phase 2 implementation notes, CLI/API drift findings, and verification results for cron CRUD completion |
| [Phase 3 Findings](phase-3-comms-findings.md) | Phase 3 implementation notes, rich comms_send findings, and verification results for thread-aware and attachment-aware communication |
| [Phase 4 Preparation](phase-4-mcp-crud-prep.md) | Phase 4 implementation prep for MCP server CRUD, including LibreFang port references and the required OpenFang hot-reload correction |
| [Phase 4 Findings](phase-4-mcp-crud-findings.md) | Phase 4 implementation notes, MCP CRUD runtime findings, and verification results including the hyphenated MCP allowlist fix |
| [Channel Adapter Test Concept](channel-adapter-test-concept.md) | Test strategy for maintained mock adapters, daemon-backed integration, and real external adapter smoke tests |
| [Test Playbooks](test-playbooks/README.md) | Operator-style smoke guides for browser/MCP, external integrations, and other non-deterministic validation paths |
| [Chrome DevTools MCP Smoke](test-playbooks/chrome-devtools-mcp-smoke.md) | End-to-end operator smoke for the bundled Chrome DevTools integration and MCP-first browser hand |
| [Integration Install & Reconnect Smoke](test-playbooks/integration-install-reconnect-smoke.md) | Manual validation flow for discovering, installing, reconnecting, reloading, and removing integrations |
| [Local Webhook Channel Smoke](test-playbooks/webhook-channel-smoke.md) | Daemon-backed local channel smoke using the webhook adapter and signed callback verification |
| [Telegram Channel Smoke](test-playbooks/telegram-channel-smoke.md) | Manual or nightly real-adapter smoke for Telegram text, thread, and attachment delivery |
| [Release Validation](test-playbooks/release-validation.md) | Concise operator runbook for validating release candidates and shipped artifacts at runtime |

## Additional Resources

| Resource | Description |
|----------|-------------|
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Development setup, code style, PR guidelines |
| [MIGRATION.md](../MIGRATION.md) | Migrating from OpenClaw, LangChain, or AutoGPT |
| [SECURITY.md](../SECURITY.md) | Security policy and vulnerability reporting |
| [CHANGELOG.md](../CHANGELOG.md) | Release notes and version history |

## SDK Test Harnesses

The REST SDKs have dedicated lightweight test harnesses alongside the Rust workspace:

```bash
npm --prefix sdk/javascript test
python -m unittest discover -s sdk/python/tests
```

---

## Quick Reference

### Start in 30 Seconds

```bash
export GROQ_API_KEY="your-key"
openfang init && openfang start
# Open http://127.0.0.1:4200
```

### Key Numbers

| Metric | Count |
|--------|-------|
| Crates | 14 |
| Agent templates | 30 |
| Messaging channels | 40 |
| Bundled skills | 60 |
| Built-in tools | 38 |
| LLM providers | 20 |
| Models in catalog | 51 |
| Model aliases | 23 |
| API endpoints | 76 |
| Security systems | 16 |
| Tests | 967 |

### Important Paths

| Path | Description |
|------|-------------|
| `~/.openfang/config.toml` | Main configuration file |
| `~/.openfang/data/openfang.db` | SQLite database |
| `~/.openfang/skills/` | Installed skills |
| `~/.openfang/daemon.json` | Daemon PID and port info |
| `agents/` | Agent template manifests |

### Key Environment Variables

| Variable | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic (Claude) |
| `OPENAI_API_KEY` | OpenAI (GPT-4o) |
| `GEMINI_API_KEY` | Google Gemini |
| `GROQ_API_KEY` | Groq (fast Llama/Mixtral) |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `XAI_API_KEY` | xAI (Grok) |

Only one provider key is needed to get started. Groq offers a free tier.
