use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Seek, Write};
use std::path::{Component, Path, PathBuf};
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub const FORMAT_VERSION: u32 = 1;
pub const MANIFEST_ENTRY: &str = "manifest.json";
pub const CONFIG_ENTRY: &str = "config";
pub const PUBLIC_KEYS_DIR: &str = "public-keys";
pub const ENCRYPTION_NONE: &str = "none";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    pub format_version: u32,
    pub created_at: String,
    pub sshm_version: String,
    pub encryption: String,
    pub public_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSummary {
    pub output_path: PathBuf,
    pub public_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPublicKey {
    pub filename: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedArchive {
    pub manifest: BackupManifest,
    pub config_path: PathBuf,
    pub public_keys: Vec<ExtractedPublicKey>,
}

pub fn default_archive_name() -> String {
    format!("sshm-backup-{}.zip", timestamp_slug())
}

pub fn backup_dir_basename() -> String {
    format!("sshm-import-backup-{}", timestamp_slug())
}

pub fn collect_public_key_files(ssh_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut public_keys = Vec::new();

    if !ssh_dir.exists() {
        return Ok(public_keys);
    }

    for entry in fs::read_dir(ssh_dir)
        .with_context(|| format!("Failed to read {}", ssh_dir.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = filename_only(&path)?;
        if is_valid_public_key_filename(&file_name) {
            public_keys.push(path);
        }
    }

    public_keys.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(public_keys)
}

pub fn create_archive(config_path: &Path, output_path: &Path) -> Result<ExportSummary> {
    if !config_path.exists() {
        bail!("SSH config not found at {}", config_path.display());
    }

    if output_path.exists() {
        bail!("Output archive already exists: {}", output_path.display());
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    let ssh_dir = config_path
        .parent()
        .context("Cannot determine ~/.ssh directory from config path")?;
    let public_key_paths = collect_public_key_files(ssh_dir)?;
    let public_keys = public_key_paths
        .iter()
        .map(|path| filename_only(path))
        .collect::<Result<Vec<_>>>()?;

    let manifest = BackupManifest {
        format_version: FORMAT_VERSION,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        sshm_version: env!("CARGO_PKG_VERSION").to_string(),
        encryption: ENCRYPTION_NONE.to_string(),
        public_keys: public_keys.clone(),
    };

    let archive_file = File::create(output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;
    let mut zip = ZipWriter::new(archive_file);

    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    write_zip_entry(&mut zip, MANIFEST_ENTRY, &manifest_json)?;

    let config_bytes = fs::read(config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    write_zip_entry(&mut zip, CONFIG_ENTRY, &config_bytes)?;

    for public_key_path in &public_key_paths {
        let entry_name = public_key_entry_name(&filename_only(public_key_path)?);
        let contents = fs::read(public_key_path)
            .with_context(|| format!("Failed to read {}", public_key_path.display()))?;
        write_zip_entry(&mut zip, &entry_name, &contents)?;
    }

    zip.finish()?;

    Ok(ExportSummary {
        output_path: output_path.to_path_buf(),
        public_keys,
    })
}

pub fn extract_archive(archive_path: &Path, destination: &Path) -> Result<ImportedArchive> {
    fs::create_dir_all(destination)
        .with_context(|| format!("Failed to create {}", destination.display()))?;

    let archive_file = File::open(archive_path)
        .with_context(|| format!("Failed to open {}", archive_path.display()))?;
    let mut zip = ZipArchive::new(archive_file)
        .with_context(|| format!("Failed to read zip archive {}", archive_path.display()))?;

    let mut seen_entries = HashSet::new();
    let mut extracted_public_keys = Vec::new();

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let entry_name = entry.name().to_string();

        validate_archive_entry_name(&entry_name)?;

        if !seen_entries.insert(entry_name.clone()) {
            bail!("Archive contains duplicate entry '{}'", entry_name);
        }

        if entry_name.ends_with('/') {
            bail!("Archive entry '{}' must be a file", entry_name);
        }

        let output_path = destination.join(&entry_name);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }

        let mut output_file = File::create(&output_path)
            .with_context(|| format!("Failed to create {}", output_path.display()))?;
        io::copy(&mut entry, &mut output_file)
            .with_context(|| format!("Failed to extract '{}'", entry_name))?;

        if let Some(filename) = public_key_name_from_entry(&entry_name) {
            extracted_public_keys.push(ExtractedPublicKey {
                filename: filename.to_string(),
                path: output_path,
            });
        }
    }

    if !seen_entries.contains(MANIFEST_ENTRY) {
        bail!("Archive is missing '{}'", MANIFEST_ENTRY);
    }
    if !seen_entries.contains(CONFIG_ENTRY) {
        bail!("Archive is missing '{}'", CONFIG_ENTRY);
    }

    let manifest_path = destination.join(MANIFEST_ENTRY);
    let manifest: BackupManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?,
    )
    .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

    validate_manifest(&manifest, &extracted_public_keys)?;
    extracted_public_keys.sort_by(|a, b| a.filename.cmp(&b.filename));

    Ok(ImportedArchive {
        manifest,
        config_path: destination.join(CONFIG_ENTRY),
        public_keys: extracted_public_keys,
    })
}

fn write_zip_entry<W>(zip: &mut ZipWriter<W>, entry_name: &str, contents: &[u8]) -> Result<()>
where
    W: Write + Seek,
{
    zip.start_file(entry_name, archive_file_options())
        .with_context(|| format!("Failed to add '{}' to archive", entry_name))?;
    zip.write_all(contents)
        .with_context(|| format!("Failed to write '{}' to archive", entry_name))?;
    Ok(())
}

fn archive_file_options() -> FileOptions {
    FileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn validate_archive_entry_name(entry_name: &str) -> Result<()> {
    if entry_name.is_empty() || entry_name.contains('\\') || entry_name.starts_with('/') {
        bail!("Archive contains unsafe entry '{}'", entry_name);
    }

    let path = Path::new(entry_name);
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("Archive contains unsafe entry '{}'", entry_name);
    }

    if entry_name == MANIFEST_ENTRY || entry_name == CONFIG_ENTRY {
        return Ok(());
    }

    if public_key_name_from_entry(entry_name).is_some() {
        return Ok(());
    }

    bail!("Unsupported archive entry '{}'", entry_name);
}

fn validate_manifest(manifest: &BackupManifest, extracted_public_keys: &[ExtractedPublicKey]) -> Result<()> {
    if manifest.format_version != FORMAT_VERSION {
        bail!(
            "Unsupported backup format version {}",
            manifest.format_version
        );
    }

    if manifest.encryption != ENCRYPTION_NONE {
        bail!(
            "Unsupported archive encryption mode '{}'",
            manifest.encryption
        );
    }

    let listed_public_keys = manifest
        .public_keys
        .iter()
        .map(|name| {
            if !is_valid_public_key_filename(name) {
                bail!("Manifest contains invalid public key filename '{}'", name);
            }
            Ok(name.clone())
        })
        .collect::<Result<HashSet<_>>>()?;

    if listed_public_keys.len() != manifest.public_keys.len() {
        bail!("Manifest contains duplicate public key filenames");
    }

    let extracted_names = extracted_public_keys
        .iter()
        .map(|key| key.filename.clone())
        .collect::<HashSet<_>>();

    if listed_public_keys != extracted_names {
        bail!("Manifest public_keys does not match archive contents");
    }

    Ok(())
}

fn public_key_entry_name(file_name: &str) -> String {
    format!("{}/{}", PUBLIC_KEYS_DIR, file_name)
}

fn public_key_name_from_entry(entry_name: &str) -> Option<&str> {
    let file_name = entry_name.strip_prefix(&format!("{}/", PUBLIC_KEYS_DIR))?;
    if is_valid_public_key_filename(file_name) {
        Some(file_name)
    } else {
        None
    }
}

fn is_valid_public_key_filename(file_name: &str) -> bool {
    !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && file_name.ends_with(".pub")
}

fn filename_only(path: &Path) -> Result<String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("Invalid UTF-8 filename: {}", path.display()))?;
    Ok(file_name.to_string())
}

fn timestamp_slug() -> String {
    Utc::now().format("%Y%m%d-%H%M%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_zip_entry_map(zip_path: &Path, entries: &[(&str, &[u8])]) -> Result<()> {
        let file = File::create(zip_path)?;
        let mut zip = ZipWriter::new(file);
        for (name, contents) in entries {
            write_zip_entry(&mut zip, name, contents)?;
        }
        zip.finish()?;
        Ok(())
    }

    #[test]
    fn collect_public_key_files_only_returns_top_level_pub_files() -> Result<()> {
        let temp = tempdir()?;
        let ssh_dir = temp.path();

        fs::write(ssh_dir.join("id_ed25519.pub"), "pub")?;
        fs::write(ssh_dir.join("id_ed25519"), "private")?;
        fs::write(ssh_dir.join("notes.txt"), "notes")?;
        fs::create_dir(ssh_dir.join("nested"))?;
        fs::write(ssh_dir.join("nested").join("nested.pub"), "nested")?;

        let public_keys = collect_public_key_files(ssh_dir)?;
        let file_names = public_keys
            .iter()
            .map(|path| filename_only(path))
            .collect::<Result<Vec<_>>>()?;

        assert_eq!(file_names, vec!["id_ed25519.pub"]);
        Ok(())
    }

    #[test]
    fn create_and_extract_archive_round_trip() -> Result<()> {
        let source = tempdir()?;
        let ssh_dir = source.path().join(".ssh");
        fs::create_dir_all(&ssh_dir)?;

        let config_path = ssh_dir.join("config");
        fs::write(
            &config_path,
            "Host app\n    HostName example.com\n    IdentityFile ~/.ssh/id_app.pub\n",
        )?;
        fs::write(ssh_dir.join("id_app.pub"), "ssh-ed25519 AAAA app")?;
        fs::write(ssh_dir.join("z.pub"), "ssh-ed25519 BBBB z")?;

        let archive_path = source.path().join("backup.zip");
        let export = create_archive(&config_path, &archive_path)?;
        assert_eq!(export.public_keys, vec!["id_app.pub", "z.pub"]);

        let extract_dir = source.path().join("extract");
        let imported = extract_archive(&archive_path, &extract_dir)?;

        assert_eq!(imported.manifest.format_version, FORMAT_VERSION);
        assert_eq!(imported.manifest.encryption, ENCRYPTION_NONE);
        assert_eq!(imported.manifest.public_keys, vec!["id_app.pub", "z.pub"]);
        assert_eq!(
            fs::read_to_string(imported.config_path)?,
            "Host app\n    HostName example.com\n    IdentityFile ~/.ssh/id_app.pub\n"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join(public_key_entry_name("id_app.pub")))?,
            "ssh-ed25519 AAAA app"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join(public_key_entry_name("z.pub")))?,
            "ssh-ed25519 BBBB z"
        );
        Ok(())
    }

    #[test]
    fn extract_archive_rejects_unknown_entries() -> Result<()> {
        let temp = tempdir()?;
        let archive_path = temp.path().join("invalid.zip");

        let manifest = BackupManifest {
            format_version: FORMAT_VERSION,
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            sshm_version: env!("CARGO_PKG_VERSION").to_string(),
            encryption: ENCRYPTION_NONE.to_string(),
            public_keys: Vec::new(),
        };
        let manifest_json = serde_json::to_vec(&manifest)?;

        write_zip_entry_map(
            &archive_path,
            &[
                (MANIFEST_ENTRY, &manifest_json),
                (CONFIG_ENTRY, b"Host app\n    HostName example.com\n"),
                ("known_hosts", b"bad"),
            ],
        )?;

        let extract_dir = temp.path().join("extract");
        let err = extract_archive(&archive_path, &extract_dir).unwrap_err();
        assert!(err.to_string().contains("Unsupported archive entry"));
        Ok(())
    }

    #[test]
    fn extract_archive_rejects_manifest_mismatch() -> Result<()> {
        let temp = tempdir()?;
        let archive_path = temp.path().join("invalid.zip");

        let manifest = BackupManifest {
            format_version: FORMAT_VERSION,
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            sshm_version: env!("CARGO_PKG_VERSION").to_string(),
            encryption: ENCRYPTION_NONE.to_string(),
            public_keys: vec!["missing.pub".to_string()],
        };
        let manifest_json = serde_json::to_vec(&manifest)?;

        write_zip_entry_map(
            &archive_path,
            &[
                (MANIFEST_ENTRY, &manifest_json),
                (CONFIG_ENTRY, b"Host app\n    HostName example.com\n"),
            ],
        )?;

        let extract_dir = temp.path().join("extract");
        let err = extract_archive(&archive_path, &extract_dir).unwrap_err();
        assert!(
            err.to_string()
                .contains("Manifest public_keys does not match archive contents")
        );
        Ok(())
    }
}
