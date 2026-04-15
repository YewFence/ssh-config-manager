use crate::archive;
use crate::config;
use anyhow::{Context, Result};
use inquire::Confirm;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub fn run(archive_path: &Path, yes: bool, config_path: &Path) -> Result<()> {
    let ssh_dir = config_path
        .parent()
        .context("Cannot determine ~/.ssh directory from config path")?;
    fs::create_dir_all(ssh_dir).with_context(|| format!("Failed to create {}", ssh_dir.display()))?;

    let extraction_dir = tempdir()?;
    let imported = archive::extract_archive(archive_path, extraction_dir.path())?;

    println!("Archive:    {}", archive_path.display());
    println!("Created at: {}", imported.manifest.created_at);
    println!(
        "Contains:   1 config file and {} public key file(s)",
        imported.public_keys.len()
    );

    if !yes {
        let confirmed =
            Confirm::new("Import this backup and overwrite the matching SSH files?")
                .with_default(false)
                .prompt()?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    let backup_dir = backup_existing_files(ssh_dir, config_path, &imported.public_keys)?;

    let imported_config = fs::read_to_string(&imported.config_path)
        .with_context(|| format!("Failed to read {}", imported.config_path.display()))?;
    let parsed_config = config::parser::parse(&imported_config);
    config::save_config(&parsed_config, config_path)?;

    for public_key in &imported.public_keys {
        let destination = ssh_dir.join(&public_key.filename);
        fs::copy(&public_key.path, &destination).with_context(|| {
            format!(
                "Failed to restore {} to {}",
                public_key.path.display(),
                destination.display()
            )
        })?;
    }

    println!(
        "Imported 1 config file and {} public key file(s) into {}.",
        imported.public_keys.len(),
        ssh_dir.display()
    );
    if let Some(path) = backup_dir {
        println!("Backed up overwritten files to {}", path.display());
    } else {
        println!("No existing files needed backup.");
    }

    Ok(())
}

fn backup_existing_files(
    ssh_dir: &Path,
    config_path: &Path,
    public_keys: &[archive::ExtractedPublicKey],
) -> Result<Option<PathBuf>> {
    let existing_public_keys = public_keys
        .iter()
        .map(|public_key| ssh_dir.join(&public_key.filename))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    if !config_path.exists() && existing_public_keys.is_empty() {
        return Ok(None);
    }

    let backup_dir = next_backup_dir(ssh_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Failed to create {}", backup_dir.display()))?;

    if config_path.exists() {
        let backup_config_path = backup_dir.join(archive::CONFIG_ENTRY);
        fs::copy(config_path, &backup_config_path).with_context(|| {
            format!(
                "Failed to back up {} to {}",
                config_path.display(),
                backup_config_path.display()
            )
        })?;
    }

    if !existing_public_keys.is_empty() {
        let backup_keys_dir = backup_dir.join(archive::PUBLIC_KEYS_DIR);
        fs::create_dir_all(&backup_keys_dir)
            .with_context(|| format!("Failed to create {}", backup_keys_dir.display()))?;

        for source in existing_public_keys {
            let file_name = source
                .file_name()
                .context("Backup source is missing a file name")?;
            let destination = backup_keys_dir.join(file_name);
            fs::copy(&source, &destination).with_context(|| {
                format!(
                    "Failed to back up {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }

    Ok(Some(backup_dir))
}

fn next_backup_dir(ssh_dir: &Path) -> PathBuf {
    let base_name = archive::backup_dir_basename();
    let mut candidate = ssh_dir.join(&base_name);
    let mut suffix = 1;

    while candidate.exists() {
        candidate = ssh_dir.join(format!("{}-{}", base_name, suffix));
        suffix += 1;
    }

    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn import_backs_up_overwritten_files_and_preserves_unrelated_keys() -> Result<()> {
        let source = tempdir()?;
        let source_ssh = source.path().join(".ssh");
        fs::create_dir_all(&source_ssh)?;
        let source_config = source_ssh.join("config");
        fs::write(
            &source_config,
            "Host app\n    HostName new.example.com\n    IdentityFile ~/.ssh/app.pub\n",
        )?;
        fs::write(source_ssh.join("app.pub"), "ssh-ed25519 NEW app")?;

        let archive_path = source.path().join("backup.zip");
        archive::create_archive(&source_config, &archive_path)?;

        let destination = tempdir()?;
        let dest_ssh = destination.path().join(".ssh");
        fs::create_dir_all(&dest_ssh)?;
        let dest_config = dest_ssh.join("config");
        fs::write(
            &dest_config,
            "Host old\n    HostName old.example.com\n    IdentityFile ~/.ssh/app.pub\n",
        )?;
        fs::write(dest_ssh.join("app.pub"), "ssh-ed25519 OLD app")?;
        fs::write(dest_ssh.join("keep.pub"), "ssh-ed25519 KEEP keep")?;

        run(&archive_path, true, &dest_config)?;

        let imported_config = config::load_config(&dest_config)?;
        let host = imported_config
            .find("app")
            .context("Expected imported host 'app'")?;
        assert_eq!(host.hostname.as_deref(), Some("new.example.com"));
        assert_eq!(
            fs::read_to_string(dest_ssh.join("app.pub"))?,
            "ssh-ed25519 NEW app"
        );
        assert_eq!(
            fs::read_to_string(dest_ssh.join("keep.pub"))?,
            "ssh-ed25519 KEEP keep"
        );

        let backup_dirs = fs::read_dir(&dest_ssh)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("sshm-import-backup-"))
            })
            .collect::<Vec<_>>();

        assert_eq!(backup_dirs.len(), 1);
        let backup_dir = &backup_dirs[0];
        assert_eq!(
            fs::read_to_string(backup_dir.join(archive::CONFIG_ENTRY))?,
            "Host old\n    HostName old.example.com\n    IdentityFile ~/.ssh/app.pub\n"
        );
        assert_eq!(
            fs::read_to_string(
                backup_dir
                    .join(archive::PUBLIC_KEYS_DIR)
                    .join("app.pub")
            )?,
            "ssh-ed25519 OLD app"
        );

        Ok(())
    }
}
