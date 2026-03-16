# Phase 5 Findings - Backup and Restore

This document captures the implementation notes, important findings, and verification results for Phase 5 of the OpenFang 12-phase roadmap.

Phase:
- Phase 5: Backup and Restore

Roadmap source:
- `docs/roadmap-12-phases.md`

Date:
- 2026-03-16

## 1. Outcome

Phase 5 is functionally complete.

Implemented:
- `POST /api/backup`
- `GET /api/backups`
- `DELETE /api/backups/{filename}`
- `POST /api/restore`
- OpenFang-native backup manifests embedded as `manifest.json`
- kernel-owned backup/archive logic with API handlers layered on top
- unit and API integration coverage for the main round-trip path
- operator-facing documentation for backup and restore behavior

## 2. What Was Added

API surface:
- backup creation now produces timestamped `.zip` archives in `~/.openfang/backups/`
- backup archives are listable and removable via the admin API
- restore accepts a previously created archive filename and writes persisted state back into `home_dir`

Archive format:
- every backup archive now includes a `manifest.json`
- the manifest records:
  - archive format version
  - product identifier (`openfang`)
  - creation time
  - hostname
  - OpenFang version
  - included components
  - omitted components
  - allowed archive file and directory roots for safe restore validation

Kernel behavior:
- backup and restore logic lives in `openfang-kernel`, not only in API route handlers
- restore validates archive metadata before extraction
- restore only writes manifest-approved, home-relative paths
- restore explicitly reports `restart_required = true` instead of pretending to hot-apply all restored runtime state

## 3. Important Findings

### Finding 1: restore should be honest about runtime semantics

OpenFang persists important state across:
- `config.toml`
- SQLite/data files
- workflow files
- workspace files
- cron and hand state files

Practical conclusion:
- restore should write the persisted files back to disk
- but it should not claim that the running daemon has fully reconciled every restored subsystem
- the API now explicitly tells operators to restart the daemon after restore

### Finding 2: backup scope must follow configured persistence paths, but only when they remain safely home-relative

Some persistent paths are configurable:
- `data_dir`
- `workspaces_dir`
- `workflows_dir`
- optional custom SQLite path

Practical conclusion:
- home-relative configured paths are included
- external paths outside `home_dir` are intentionally omitted
- those omissions are recorded in `omitted_components` so the archive is not misleading

### Finding 3: restore should validate archive intent, not only zip path traversal

Using `enclosed_name()` protects against basic zip-slip issues, but that alone still allows unexpected files inside the OpenFang home directory.

Practical conclusion:
- restore validates the archive manifest
- restore only accepts entries that match manifest-declared file and directory roots
- this keeps the restore flow closer to an application-level import than a blind unzip

## 4. Verification Completed

Formatting:

```bash
cargo fmt --all
```

Kernel verification:

```bash
cargo test -p openfang-kernel create_and_restore_backup_round_trip
```

API integration verification:

```bash
cargo test -p openfang-api --test api_integration_test test_backup_restore_endpoints_round_trip
```

## 5. Files Touched For Phase 5

Main implementation files:
- `crates/openfang-types/src/backup.rs`
- `crates/openfang-types/src/lib.rs`
- `crates/openfang-kernel/src/backup.rs`
- `crates/openfang-kernel/src/error.rs`
- `crates/openfang-kernel/src/lib.rs`
- `crates/openfang-api/src/routes.rs`
- `crates/openfang-api/src/server.rs`
- `crates/openfang-api/tests/api_integration_test.rs`
- `docs/api-reference.md`
- `docs/production-checklist.md`
- `docs/roadmap-12-phases.md`

This findings document:
- `docs/phase-5-backup-restore-findings.md`

## 6. Recommended Next Step

Phase 5 is now in a good stopping state.

Recommended next roadmap move:
- Phase 6: Deterministic Hand Identity

Reason:
- backup and restore now cover the operational recovery baseline
- the next stability gap is preserving Hand identity semantics cleanly across restart and restoration flows
