//! Backup and restore helpers for kernel state persisted under `home_dir`.

use crate::error::KernelResult;
use crate::kernel::OpenFangKernel;
use openfang_types::backup::{
    BackupManifest, BackupSummary, CreateBackupResponse, DeleteBackupResponse, ListBackupsResponse,
    RestoreBackupRequest, RestoreBackupResponse,
};
use openfang_types::error::OpenFangError;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const BACKUP_FORMAT_VERSION: u32 = 1;
const BACKUP_PRODUCT: &str = "openfang";

#[derive(Clone, Copy)]
enum ArchiveItemKind {
    File,
    Directory,
}

struct ArchiveItem {
    component: &'static str,
    source: PathBuf,
    archive_path: String,
    kind: ArchiveItemKind,
}

impl OpenFangKernel {
    /// Create a timestamped backup archive under `<home_dir>/backups/`.
    pub fn create_backup(&self) -> KernelResult<CreateBackupResponse> {
        let backups_dir = self.config.home_dir.join("backups");
        std::fs::create_dir_all(&backups_dir).map_err(OpenFangError::Io)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("openfang_backup_{timestamp}.zip");
        let backup_path = backups_dir.join(&filename);

        let file = File::create(&backup_path).map_err(OpenFangError::Io)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let (items, omitted_components) = self.backup_items();
        let mut components = Vec::new();
        let mut archive_files = Vec::new();
        let mut archive_directories = Vec::new();

        for item in items {
            match item.kind {
                ArchiveItemKind::File => {
                    match add_file_to_zip(&mut zip, &item.source, &item.archive_path, options) {
                        Ok(()) => {
                            components.push(item.component.to_string());
                            archive_files.push(item.archive_path);
                        }
                        Err(e) => {
                            tracing::warn!(
                                component = item.component,
                                source = %item.source.display(),
                                "Backup: skipping file: {e}"
                            );
                        }
                    }
                }
                ArchiveItemKind::Directory => {
                    match add_directory_to_zip(&mut zip, &item.source, &item.archive_path, options)
                    {
                        Ok(count) if count > 0 => {
                            components.push(item.component.to_string());
                            archive_directories.push(item.archive_path);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!(
                                component = item.component,
                                source = %item.source.display(),
                                "Backup: skipping directory: {e}"
                            );
                        }
                    }
                }
            }
        }

        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            product: BACKUP_PRODUCT.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            hostname: hostname_string(),
            openfang_version: env!("CARGO_PKG_VERSION").to_string(),
            components: components.clone(),
            omitted_components: omitted_components.clone(),
            archive_files,
            archive_directories,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| {
            OpenFangError::Serialization(format!("Failed to encode backup manifest: {e}"))
        })?;
        zip.start_file("manifest.json", options)
            .map_err(|e| OpenFangError::Internal(format!("Failed to start manifest entry: {e}")))?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(OpenFangError::Io)?;
        zip.finish().map_err(|e| {
            OpenFangError::Internal(format!("Failed to finalize backup archive: {e}"))
        })?;

        let size_bytes = std::fs::metadata(&backup_path)
            .map(|m| m.len())
            .unwrap_or(0);

        tracing::info!(
            filename,
            size_bytes,
            components = components.len(),
            omitted = omitted_components.len(),
            "Backup created"
        );
        self.audit_log.record(
            "system",
            openfang_runtime::audit::AuditAction::ConfigChange,
            format!("Backup created: {filename}"),
            "completed",
        );

        Ok(CreateBackupResponse {
            filename,
            path: backup_path.to_string_lossy().to_string(),
            size_bytes,
            created_at: manifest.created_at,
            components,
            omitted_components,
        })
    }

    /// List backup archives stored under `<home_dir>/backups/`.
    pub fn list_backups(&self) -> KernelResult<ListBackupsResponse> {
        let backups_dir = self.config.home_dir.join("backups");
        if !backups_dir.exists() {
            return Ok(ListBackupsResponse {
                backups: Vec::new(),
                total: 0,
            });
        }

        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&backups_dir).map_err(OpenFangError::Io)? {
            let entry = entry.map_err(OpenFangError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("zip") {
                continue;
            }

            let metadata = std::fs::metadata(&path).map_err(OpenFangError::Io)?;
            let modified_at = metadata.modified().ok().map(|ts| {
                let dt: chrono::DateTime<chrono::Utc> = ts.into();
                dt.to_rfc3339()
            });
            let manifest = read_backup_manifest(&path);

            backups.push(BackupSummary {
                filename: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified_at,
                created_at: manifest.as_ref().map(|m| m.created_at.clone()),
                components: manifest
                    .as_ref()
                    .map(|m| m.components.clone())
                    .unwrap_or_default(),
                omitted_components: manifest
                    .as_ref()
                    .map(|m| m.omitted_components.clone())
                    .unwrap_or_default(),
                openfang_version: manifest.as_ref().map(|m| m.openfang_version.clone()),
            });
        }

        backups.sort_by(|a, b| b.filename.cmp(&a.filename));
        let total = backups.len();
        Ok(ListBackupsResponse { backups, total })
    }

    /// Delete a single backup archive by filename.
    pub fn delete_backup(&self, filename: &str) -> KernelResult<DeleteBackupResponse> {
        validate_backup_filename(filename)?;
        let backup_path = self.config.home_dir.join("backups").join(filename);
        if !backup_path.exists() {
            return Err(OpenFangError::InvalidInput("Backup not found".to_string()).into());
        }

        std::fs::remove_file(&backup_path).map_err(OpenFangError::Io)?;
        self.audit_log.record(
            "system",
            openfang_runtime::audit::AuditAction::ConfigChange,
            format!("Backup deleted: {filename}"),
            "completed",
        );

        Ok(DeleteBackupResponse {
            deleted: filename.to_string(),
        })
    }

    /// Restore a backup archive into `<home_dir>`.
    ///
    /// This overwrites persisted files on disk. The daemon should be restarted
    /// afterwards so all restored state is reloaded consistently.
    pub fn restore_backup(&self, req: RestoreBackupRequest) -> KernelResult<RestoreBackupResponse> {
        validate_backup_filename(&req.filename)?;
        let backup_path = self.config.home_dir.join("backups").join(&req.filename);
        if !backup_path.exists() {
            return Err(OpenFangError::InvalidInput("Backup file not found".to_string()).into());
        }

        let file = File::open(&backup_path).map_err(OpenFangError::Io)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| OpenFangError::InvalidInput(format!("Invalid backup archive: {e}")))?;

        let manifest = read_manifest_from_archive(&mut archive)?;
        validate_manifest(&manifest)?;

        let mut restored_files = 0usize;
        let mut errors = Vec::new();

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(entry) => entry,
                Err(e) => {
                    errors.push(format!("Failed to read archive entry {i}: {e}"));
                    continue;
                }
            };

            let entry_name = match entry.enclosed_name() {
                Some(name) => name.to_path_buf(),
                None => {
                    errors.push(format!("Skipped unsafe archive entry at index {i}"));
                    continue;
                }
            };

            if entry_name == Path::new("manifest.json") {
                continue;
            }
            if !is_safe_relative_path(&entry_name) {
                errors.push(format!(
                    "Skipped invalid archive path {}",
                    entry_name.display()
                ));
                continue;
            }
            if !entry_allowed_by_manifest(&entry_name, &manifest) {
                errors.push(format!(
                    "Skipped unexpected archive path {}",
                    entry_name.display()
                ));
                continue;
            }

            let target = if entry_name == Path::new("config.toml") {
                self.config_path().to_path_buf()
            } else {
                self.config.home_dir.join(&entry_name)
            };

            if entry.is_dir() {
                if let Err(e) = std::fs::create_dir_all(&target) {
                    errors.push(format!("mkdir {}: {e}", entry_name.display()));
                }
                continue;
            }

            if let Some(parent) = target.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    errors.push(format!("mkdir parent for {}: {e}", entry_name.display()));
                    continue;
                }
            }

            let mut data = Vec::new();
            if let Err(e) = entry.read_to_end(&mut data) {
                errors.push(format!("read {}: {e}", entry_name.display()));
                continue;
            }
            if let Err(e) = std::fs::write(&target, &data) {
                errors.push(format!("write {}: {e}", entry_name.display()));
                continue;
            }
            restored_files += 1;
        }

        let audit_status = if errors.is_empty() {
            "completed".to_string()
        } else {
            format!("partial: {} error(s)", errors.len())
        };
        self.audit_log.record(
            "system",
            openfang_runtime::audit::AuditAction::ConfigChange,
            format!("Backup restored: {} ({restored_files} files)", req.filename),
            audit_status,
        );

        Ok(RestoreBackupResponse {
            restored_files,
            errors,
            manifest,
            restart_required: true,
            message: "Restore complete. Restart the daemon for all changes to take effect."
                .to_string(),
        })
    }

    fn backup_items(&self) -> (Vec<ArchiveItem>, Vec<String>) {
        let home_dir = &self.config.home_dir;
        let mut items = Vec::new();
        let mut omitted = Vec::new();

        push_file_item(
            &mut items,
            "config",
            self.config_path().to_path_buf(),
            "config.toml",
        );
        push_file_item(
            &mut items,
            "cron_jobs",
            home_dir.join("cron_jobs.json"),
            "cron_jobs.json",
        );
        push_file_item(
            &mut items,
            "hand_state",
            home_dir.join("hand_state.json"),
            "hand_state.json",
        );
        push_file_item(
            &mut items,
            "proposal_jobs",
            home_dir.join("proposal_jobs.json"),
            "proposal_jobs.json",
        );
        push_file_item(
            &mut items,
            "custom_models",
            home_dir.join("custom_models.json"),
            "custom_models.json",
        );
        push_file_item(
            &mut items,
            "integrations",
            home_dir.join("integrations.toml"),
            "integrations.toml",
        );
        push_file_item(
            &mut items,
            "secrets",
            home_dir.join("secrets.env"),
            "secrets.env",
        );
        push_file_item(&mut items, "dotenv", home_dir.join(".env"), ".env");
        push_file_item(&mut items, "vault", home_dir.join("vault.enc"), "vault.enc");
        push_dir_item(&mut items, "agents", home_dir.join("agents"), "agents");
        push_dir_item(&mut items, "hands", home_dir.join("hands"), "hands");
        push_dir_item(&mut items, "skills", home_dir.join("skills"), "skills");

        push_home_relative_item(
            &mut items,
            &mut omitted,
            home_dir,
            "data",
            &self.config.data_dir,
            ArchiveItemKind::Directory,
        );

        if let Some(sqlite_path) = self.config.memory.sqlite_path.as_ref() {
            if !sqlite_path.starts_with(&self.config.data_dir) {
                push_home_relative_item(
                    &mut items,
                    &mut omitted,
                    home_dir,
                    "memory_db",
                    sqlite_path,
                    ArchiveItemKind::File,
                );
            }
        }

        let workflows_dir = self
            .config
            .workflows_dir
            .clone()
            .unwrap_or_else(|| home_dir.join("workflows"));
        push_home_relative_item(
            &mut items,
            &mut omitted,
            home_dir,
            "workflows",
            &workflows_dir,
            ArchiveItemKind::Directory,
        );

        let workspaces_dir = self.config.effective_workspaces_dir();
        push_home_relative_item(
            &mut items,
            &mut omitted,
            home_dir,
            "workspaces",
            &workspaces_dir,
            ArchiveItemKind::Directory,
        );

        (items, omitted)
    }
}

fn push_file_item(
    items: &mut Vec<ArchiveItem>,
    component: &'static str,
    source: PathBuf,
    archive_path: &str,
) {
    if source.exists() && source.is_file() {
        items.push(ArchiveItem {
            component,
            source,
            archive_path: archive_path.to_string(),
            kind: ArchiveItemKind::File,
        });
    }
}

fn push_dir_item(
    items: &mut Vec<ArchiveItem>,
    component: &'static str,
    source: PathBuf,
    archive_path: &str,
) {
    if source.exists() && source.is_dir() {
        items.push(ArchiveItem {
            component,
            source,
            archive_path: archive_path.to_string(),
            kind: ArchiveItemKind::Directory,
        });
    }
}

fn push_home_relative_item(
    items: &mut Vec<ArchiveItem>,
    omitted: &mut Vec<String>,
    home_dir: &Path,
    component: &'static str,
    source: &Path,
    kind: ArchiveItemKind,
) {
    if !source.exists() {
        return;
    }

    match relative_archive_path(home_dir, source) {
        Some(archive_path) => items.push(ArchiveItem {
            component,
            source: source.to_path_buf(),
            archive_path,
            kind,
        }),
        None => omitted.push(component.to_string()),
    }
}

fn relative_archive_path(home_dir: &Path, source: &Path) -> Option<String> {
    let relative = source.strip_prefix(home_dir).ok()?;
    if relative.as_os_str().is_empty() || !is_safe_relative_path(relative) {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn add_file_to_zip(
    zip: &mut zip::ZipWriter<File>,
    source: &Path,
    archive_path: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    let data = std::fs::read(source).map_err(|e| format!("read {}: {e}", source.display()))?;
    zip.start_file(archive_path, options)
        .map_err(|e| format!("start {archive_path}: {e}"))?;
    zip.write_all(&data)
        .map_err(|e| format!("write {archive_path}: {e}"))?;
    Ok(())
}

fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<File>,
    source: &Path,
    archive_root: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<u64, String> {
    let mut count = 0u64;
    for entry in walkdir::WalkDir::new(source)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(source)
            .map_err(|e| format!("strip prefix {}: {e}", source.display()))?;
        let archive_path = Path::new(archive_root).join(relative);
        let archive_path = archive_path.to_string_lossy().replace('\\', "/");
        add_file_to_zip(zip, path, &archive_path, options)?;
        count += 1;
    }
    Ok(count)
}

fn validate_backup_filename(filename: &str) -> KernelResult<()> {
    if filename.is_empty()
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || !filename.ends_with(".zip")
    {
        return Err(OpenFangError::InvalidInput("Invalid backup filename".to_string()).into());
    }
    Ok(())
}

fn validate_manifest(manifest: &BackupManifest) -> KernelResult<()> {
    if manifest.product != BACKUP_PRODUCT {
        return Err(OpenFangError::InvalidInput(format!(
            "Backup archive is for '{}' not '{}'",
            manifest.product, BACKUP_PRODUCT
        ))
        .into());
    }
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(OpenFangError::InvalidInput(format!(
            "Unsupported backup format version {}",
            manifest.format_version
        ))
        .into());
    }

    for root in &manifest.archive_files {
        if !is_safe_relative_path(Path::new(root)) {
            return Err(OpenFangError::InvalidInput(format!(
                "Backup manifest contains invalid file root '{root}'"
            ))
            .into());
        }
    }
    for root in &manifest.archive_directories {
        if !is_safe_relative_path(Path::new(root)) {
            return Err(OpenFangError::InvalidInput(format!(
                "Backup manifest contains invalid directory root '{root}'"
            ))
            .into());
        }
    }
    Ok(())
}

fn read_manifest_from_archive<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> KernelResult<BackupManifest> {
    let mut entry = archive.by_name("manifest.json").map_err(|_| {
        OpenFangError::InvalidInput("Backup archive is missing manifest.json".to_string())
    })?;
    let mut content = String::new();
    entry
        .read_to_string(&mut content)
        .map_err(OpenFangError::Io)?;
    serde_json::from_str(&content).map_err(|e| {
        OpenFangError::Serialization(format!("Failed to decode backup manifest: {e}")).into()
    })
}

fn read_backup_manifest(path: &Path) -> Option<BackupManifest> {
    let file = File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    read_manifest_from_archive(&mut archive).ok()
}

fn entry_allowed_by_manifest(entry_name: &Path, manifest: &BackupManifest) -> bool {
    manifest
        .archive_files
        .iter()
        .map(Path::new)
        .any(|path| entry_name == path)
        || manifest
            .archive_directories
            .iter()
            .map(Path::new)
            .any(|path| entry_name.starts_with(path))
}

fn is_safe_relative_path(path: &Path) -> bool {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }

    path.components().all(|component| match component {
        Component::Normal(_) => true,
        Component::CurDir => false,
        Component::ParentDir => false,
        Component::RootDir => false,
        Component::Prefix(_) => false,
    })
}

fn hostname_string() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openfang_types::config::KernelConfig;

    #[test]
    fn create_and_restore_backup_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let home_dir = tmp.path().join("openfang-home");
        std::fs::create_dir_all(&home_dir).unwrap();

        let config = KernelConfig {
            home_dir: home_dir.clone(),
            data_dir: home_dir.join("data"),
            ..KernelConfig::default()
        };
        let kernel = OpenFangKernel::boot_with_config(config).unwrap();

        std::fs::write(home_dir.join("config.toml"), "log_level = \"debug\"\n").unwrap();
        std::fs::write(home_dir.join("custom_models.json"), "{\"models\":[]}").unwrap();
        std::fs::create_dir_all(home_dir.join("agents").join("assistant")).unwrap();
        std::fs::write(
            home_dir.join("agents").join("assistant").join("agent.toml"),
            "name = \"assistant\"\nmodule = \"builtin:chat\"\nversion = \"0.1.0\"\nauthor = \"test\"\ndescription = \"test\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(home_dir.join("workspaces").join("assistant")).unwrap();
        std::fs::write(
            home_dir
                .join("workspaces")
                .join("assistant")
                .join("SOUL.md"),
            "original workspace state",
        )
        .unwrap();
        std::fs::create_dir_all(home_dir.join("hands").join("renewal-monitor")).unwrap();
        std::fs::write(
            home_dir.join("hands").join("renewal-monitor").join("HAND.toml"),
            "id = \"renewal-monitor\"\nname = \"Renewal Monitor\"\ndescription = \"test\"\ncategory = \"productivity\"\nicon = \"\"\ntools = [\"file_read\"]\n\n[agent]\nname = \"Renewal Monitor\"\ndescription = \"test\"\nmodule = \"builtin:chat\"\nprovider = \"default\"\nmodel = \"default\"\nmax_tokens = 512\ntemperature = 0.1\nsystem_prompt = \"test\"\n",
        )
        .unwrap();
        std::fs::write(
            home_dir.join("proposal_jobs.json"),
            "[{\"job_id\":\"11111111-1111-1111-1111-111111111111\",\"approval_id\":\"22222222-2222-2222-2222-222222222222\",\"proposal\":{\"kind\":\"agent\",\"name\":\"demo\",\"description\":\"demo\",\"rationale\":\"test\",\"approval_required\":true,\"tags\":[],\"keywords\":[],\"suggested_tools\":[],\"agent_manifest_toml\":\"name = \\\"demo\\\"\\ndescription = \\\"demo\\\"\\nmodule = \\\"builtin:chat\\\"\\n\"},\"activate_after_create\":false,\"status\":\"failed\",\"created_at\":\"2026-03-16T00:00:00Z\",\"updated_at\":\"2026-03-16T00:00:00Z\",\"error\":\"test\"}]",
        )
        .unwrap();

        let backup = kernel.create_backup().unwrap();
        assert!(home_dir.join("backups").join(&backup.filename).exists());
        assert!(backup.components.iter().any(|c| c == "config"));
        assert!(backup.components.iter().any(|c| c == "workspaces"));
        assert!(backup.components.iter().any(|c| c == "hands"));
        assert!(backup.components.iter().any(|c| c == "proposal_jobs"));

        std::fs::write(home_dir.join("config.toml"), "log_level = \"error\"\n").unwrap();
        std::fs::write(
            home_dir
                .join("workspaces")
                .join("assistant")
                .join("SOUL.md"),
            "mutated workspace state",
        )
        .unwrap();
        std::fs::remove_file(
            home_dir
                .join("hands")
                .join("renewal-monitor")
                .join("HAND.toml"),
        )
        .unwrap();
        std::fs::write(home_dir.join("proposal_jobs.json"), "[]").unwrap();

        let restored = kernel
            .restore_backup(RestoreBackupRequest {
                filename: backup.filename.clone(),
            })
            .unwrap();
        assert!(restored.restart_required);
        assert!(restored.restored_files >= 3);
        assert!(restored.errors.is_empty());
        assert_eq!(
            std::fs::read_to_string(home_dir.join("config.toml")).unwrap(),
            "log_level = \"debug\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(
                home_dir
                    .join("workspaces")
                    .join("assistant")
                    .join("SOUL.md")
            )
            .unwrap(),
            "original workspace state"
        );
        assert!(home_dir
            .join("hands")
            .join("renewal-monitor")
            .join("HAND.toml")
            .exists());
        assert!(std::fs::read_to_string(home_dir.join("proposal_jobs.json"))
            .unwrap()
            .contains("\"job_id\""));
    }

    #[test]
    fn backup_and_restore_follow_custom_config_path() {
        let tmp = tempfile::tempdir().unwrap();
        let home_dir = tmp.path().join("openfang-home");
        let custom_config_path = tmp.path().join("openfang-custom.toml");
        std::fs::create_dir_all(&home_dir).unwrap();

        let config = KernelConfig {
            home_dir: home_dir.clone(),
            data_dir: home_dir.join("data"),
            ..KernelConfig::default()
        };
        std::fs::write(
            &custom_config_path,
            toml::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        let kernel = OpenFangKernel::boot(Some(&custom_config_path)).unwrap();
        assert_eq!(kernel.config_path(), custom_config_path.as_path());

        std::fs::write(&custom_config_path, "log_level = \"debug\"\n").unwrap();
        let backup = kernel.create_backup().unwrap();

        std::fs::write(&custom_config_path, "log_level = \"error\"\n").unwrap();
        let restored = kernel
            .restore_backup(RestoreBackupRequest {
                filename: backup.filename,
            })
            .unwrap();

        assert!(restored.errors.is_empty());
        assert_eq!(
            std::fs::read_to_string(&custom_config_path).unwrap(),
            "log_level = \"debug\"\n"
        );
    }
}
