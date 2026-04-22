use crate::archive;
use anyhow::{Context, Result};
use inquire::Confirm;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub fn run(archive_path: &Path, yes: bool, config_path: &Path) -> Result<()> {
    let ssh_dir = config_path
        .parent()
        .context("Cannot determine ~/.ssh directory from config path")?;
    let ssh_dir_existed = ssh_dir.exists();

    let extraction_dir = tempdir()?;
    let imported = archive::extract_archive(archive_path, extraction_dir.path())?;

    println!("Archive:    {}", archive_path.display());
    println!("Created at: {}", imported.manifest.created_at);
    println!(
        "Contains:   1 config file and {} public key file(s)",
        imported.public_keys.len()
    );
    println!(
        "Note: private keys are not included in this archive. Make sure the matching private keys already exist on this machine."
    );

    if !yes {
        let confirmed = Confirm::new("Import this backup and overwrite the matching SSH files?")
            .with_default(false)
            .prompt()?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    ensure_ssh_dir(ssh_dir, ssh_dir_existed)?;
    let backup_dir = backup_existing_files(ssh_dir, config_path, &imported.public_keys)?;
    restore_imported_files(&imported, ssh_dir, config_path, backup_dir.as_deref())?;

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

fn ensure_ssh_dir(ssh_dir: &Path, ssh_dir_existed: bool) -> Result<()> {
    fs::create_dir_all(ssh_dir)
        .with_context(|| format!("Failed to create {}", ssh_dir.display()))?;
    #[cfg(unix)]
    if !ssh_dir_existed {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(ssh_dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to set permissions on {}", ssh_dir.display()))?;
    }

    Ok(())
}

fn restore_imported_files(
    imported: &archive::ImportedArchive,
    ssh_dir: &Path,
    config_path: &Path,
    backup_dir: Option<&Path>,
) -> Result<()> {
    if let Err(error) = restore_imported_files_inner(imported, ssh_dir, config_path, backup_dir) {
        return match rollback_import(ssh_dir, config_path, &imported.public_keys, backup_dir) {
            Ok(()) => Err(error),
            Err(rollback_error) => {
                Err(error.context(format!("Rollback failed: {rollback_error:#}")))
            }
        };
    }

    Ok(())
}

fn restore_imported_files_inner(
    imported: &archive::ImportedArchive,
    ssh_dir: &Path,
    config_path: &Path,
    backup_dir: Option<&Path>,
) -> Result<()> {
    copy_restored_file(&imported.config_path, config_path, backup_dir, Some(0o600))?;

    for public_key in &imported.public_keys {
        let destination = ssh_dir.join(&public_key.filename);
        copy_restored_file(&public_key.path, &destination, backup_dir, None)?;
    }

    Ok(())
}

fn copy_restored_file(
    source: &Path,
    destination: &Path,
    backup_dir: Option<&Path>,
    unix_mode: Option<u32>,
) -> Result<()> {
    fs::copy(source, destination).with_context(|| {
        format!(
            "Failed to restore {} to {}{}",
            source.display(),
            destination.display(),
            recovery_hint(backup_dir)
        )
    })?;

    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(mode)).with_context(|| {
            format!(
                "Failed to set permissions on {}{}",
                destination.display(),
                recovery_hint(backup_dir)
            )
        })?;
    }

    Ok(())
}

fn rollback_import(
    ssh_dir: &Path,
    config_path: &Path,
    public_keys: &[archive::ExtractedPublicKey],
    backup_dir: Option<&Path>,
) -> Result<()> {
    let backup_config_path = backup_dir.map(|dir| dir.join(archive::CONFIG_ENTRY));
    restore_backup_or_remove(config_path, backup_config_path.as_deref(), Some(0o600))?;

    for public_key in public_keys {
        let destination = ssh_dir.join(&public_key.filename);
        let backup_public_key_path = backup_dir.map(|dir| {
            dir.join(archive::PUBLIC_KEYS_DIR)
                .join(&public_key.filename)
        });
        restore_backup_or_remove(&destination, backup_public_key_path.as_deref(), None)?;
    }

    Ok(())
}

fn restore_backup_or_remove(
    destination: &Path,
    backup_path: Option<&Path>,
    unix_mode: Option<u32>,
) -> Result<()> {
    if let Some(source) = backup_path.filter(|path| path.exists()) {
        fs::copy(source, destination).with_context(|| {
            format!(
                "Failed to restore backup {} to {}",
                source.display(),
                destination.display()
            )
        })?;

        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(destination, fs::Permissions::from_mode(mode)).with_context(
                || format!("Failed to set permissions on {}", destination.display()),
            )?;
        }
    } else if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("Failed to remove {}", destination.display()))?;
    }

    Ok(())
}

fn recovery_hint(backup_dir: Option<&Path>) -> String {
    backup_dir
        .map(|path| format!("; existing files were backed up to {}", path.display()))
        .unwrap_or_default()
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
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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

        assert_eq!(
            fs::read_to_string(&dest_config)?,
            "Host app\n    HostName new.example.com\n    IdentityFile ~/.ssh/app.pub\n"
        );
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
            fs::read_to_string(backup_dir.join(archive::PUBLIC_KEYS_DIR).join("app.pub"))?,
            "ssh-ed25519 OLD app"
        );

        Ok(())
    }

    #[test]
    fn import_preserves_raw_config_bytes() -> Result<()> {
        let source = tempdir()?;
        let source_ssh = source.path().join(".ssh");
        fs::create_dir_all(&source_ssh)?;

        let source_config = source_ssh.join("config");
        let raw_config = "\
# top comment
Include ~/.ssh/conf.d/*.conf

Host app
    HostName new.example.com
    User deploy
    LocalForward 127.0.0.1:5432 db.internal:5432

Match host bastion
    User ops
";
        fs::write(&source_config, raw_config)?;

        let archive_path = source.path().join("backup.zip");
        archive::create_archive(&source_config, &archive_path)?;

        let destination = tempdir()?;
        let dest_config = destination.path().join(".ssh").join("config");

        run(&archive_path, true, &dest_config)?;

        assert_eq!(fs::read_to_string(dest_config)?, raw_config);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn import_creates_ssh_dir_with_private_permissions() -> Result<()> {
        let source = tempdir()?;
        let source_ssh = source.path().join(".ssh");
        fs::create_dir_all(&source_ssh)?;

        let source_config = source_ssh.join("config");
        fs::write(&source_config, "Host app\n    HostName new.example.com\n")?;

        let archive_path = source.path().join("backup.zip");
        archive::create_archive(&source_config, &archive_path)?;

        let destination = tempdir()?;
        let dest_ssh = destination.path().join(".ssh");
        let dest_config = dest_ssh.join("config");

        run(&archive_path, true, &dest_config)?;

        let mode = fs::metadata(&dest_ssh)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn import_preserves_existing_ssh_dir_permissions() -> Result<()> {
        let source = tempdir()?;
        let source_ssh = source.path().join(".ssh");
        fs::create_dir_all(&source_ssh)?;

        let source_config = source_ssh.join("config");
        fs::write(&source_config, "Host app\n    HostName new.example.com\n")?;

        let archive_path = source.path().join("backup.zip");
        archive::create_archive(&source_config, &archive_path)?;

        let destination = tempdir()?;
        let dest_ssh = destination.path().join(".ssh");
        fs::create_dir_all(&dest_ssh)?;
        fs::set_permissions(&dest_ssh, fs::Permissions::from_mode(0o755))?;

        let dest_config = dest_ssh.join("config");
        run(&archive_path, true, &dest_config)?;

        let mode = fs::metadata(&dest_ssh)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
        Ok(())
    }
}
