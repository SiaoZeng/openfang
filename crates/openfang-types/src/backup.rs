//! Shared backup and restore types.

use serde::{Deserialize, Serialize};

/// Metadata stored inside every backup archive as `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    /// Backup manifest format version.
    pub format_version: u32,
    /// Product identifier for archive validation.
    pub product: String,
    /// Timestamp the archive was created.
    pub created_at: String,
    /// Best-effort hostname from the creating machine.
    pub hostname: String,
    /// OpenFang version that created the archive.
    pub openfang_version: String,
    /// Human-readable component names included in the archive.
    #[serde(default)]
    pub components: Vec<String>,
    /// Components intentionally omitted from the archive.
    #[serde(default)]
    pub omitted_components: Vec<String>,
    /// Allowed archive file paths for safe restore validation.
    #[serde(default)]
    pub archive_files: Vec<String>,
    /// Allowed archive directory roots for safe restore validation.
    #[serde(default)]
    pub archive_directories: Vec<String>,
}

/// Backup archive summary returned by listing endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupSummary {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub created_at: Option<String>,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub omitted_components: Vec<String>,
    pub openfang_version: Option<String>,
}

/// Response from backup creation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateBackupResponse {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub omitted_components: Vec<String>,
}

/// Response from backup listing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListBackupsResponse {
    #[serde(default)]
    pub backups: Vec<BackupSummary>,
    pub total: usize,
}

/// Response from backup deletion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteBackupResponse {
    pub deleted: String,
}

/// Request body for restoring a backup archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreBackupRequest {
    pub filename: String,
}

/// Response from backup restore.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreBackupResponse {
    pub restored_files: usize,
    #[serde(default)]
    pub errors: Vec<String>,
    pub manifest: BackupManifest,
    pub restart_required: bool,
    pub message: String,
}
