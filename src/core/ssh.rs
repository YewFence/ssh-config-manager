use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub fn normalize_identity_file_path(input: &str) -> Result<Option<String>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if is_public_key(trimmed) {
        anyhow::bail!(
            "Pasted public keys need the interactive create/edit flow so sshm can ask for a filename."
        );
    }

    if !trimmed.contains('/') && !trimmed.contains('\\') {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        return Ok(Some(
            home.join(".ssh")
                .join(trimmed)
                .to_string_lossy()
                .into_owned(),
        ));
    }

    Ok(Some(trimmed.to_string()))
}

pub fn is_public_key(s: &str) -> bool {
    let prefixes = [
        "ssh-rsa ",
        "ssh-ed25519 ",
        "ssh-dss ",
        "ecdsa-sha2-nistp256 ",
        "ecdsa-sha2-nistp384 ",
        "ecdsa-sha2-nistp521 ",
        "sk-ssh-ed25519 ",
        "sk-ecdsa-sha2-nistp256 ",
    ];
    prefixes.iter().any(|p| s.starts_with(p))
}

pub fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        Ok(home.join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

pub fn sanitize_filename(hostname: &str) -> String {
    hostname
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn normalize_public_key_filename(input: &str, default_name: &str) -> Result<String> {
    let trimmed = input.trim();
    let name = if trimmed.is_empty() {
        default_name.trim()
    } else {
        trimmed
    };
    let name = name.strip_suffix(".pub").unwrap_or(name);

    if name.is_empty() {
        anyhow::bail!("Filename is required.");
    }
    if name == "." || name == ".." || name.starts_with('.') {
        anyhow::bail!("Filename cannot be hidden or parent-relative.");
    }
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("Filename cannot contain path separators.");
    }
    if !name
        .chars()
        .all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        anyhow::bail!("Filename can only contain letters, numbers, '.', '-', and '_'.");
    }

    Ok(name.to_string())
}

pub fn write_public_key_for_config(
    config_path: &Path,
    filename: &str,
    public_key: &str,
) -> Result<(String, PathBuf)> {
    let ssh_dir = config_path
        .parent()
        .context("Cannot determine SSH directory from config path")?;
    fs::create_dir_all(ssh_dir)
        .with_context(|| format!("Failed to create {}", ssh_dir.display()))?;

    let key_path = ssh_dir.join(format!("{}.pub", filename));
    if key_path.exists() {
        anyhow::bail!("Public key file already exists.");
    }

    fs::write(&key_path, public_key.trim())
        .with_context(|| format!("Failed to write {}", key_path.display()))?;
    Ok((format!("~/.ssh/{}.pub", filename), key_path))
}

pub fn validate_forward_format(input: &str) -> bool {
    let parts: Vec<&str> = input.trim().split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    if parts[0].parse::<u16>().is_err() {
        return false;
    }
    if parts[1].is_empty() {
        return false;
    }
    if parts[2].parse::<u16>().is_err() {
        return false;
    }
    true
}

pub fn validate_set_env_format(input: &str) -> bool {
    let trimmed = input.trim();
    let Some((key, _value)) = trimmed.split_once('=') else {
        return false;
    };
    !key.trim().is_empty()
}

pub fn validate_send_env_format(input: &str) -> bool {
    !input.trim().is_empty() && !input.contains('=')
}

pub fn preferred_authentications_for(
    identity_file: &Option<String>,
    current: Option<&str>,
) -> Option<String> {
    if identity_file.is_none() {
        return Some(current.unwrap_or("password").to_string());
    }

    match current {
        Some(value) if value.eq_ignore_ascii_case("password") => None,
        Some(value) => Some(value.to_string()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalize_identity_file_path_handles_empty_bare_name_and_path() {
        assert_eq!(normalize_identity_file_path("").unwrap(), None);
        assert_eq!(
            normalize_identity_file_path("id_ed25519").unwrap(),
            Some(
                dirs::home_dir()
                    .unwrap()
                    .join(".ssh")
                    .join("id_ed25519")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert_eq!(
            normalize_identity_file_path("/tmp/id_ed25519").unwrap(),
            Some("/tmp/id_ed25519".to_string())
        );
    }

    #[test]
    fn sanitize_filename_replaces_unsafe_characters() {
        assert_eq!(
            sanitize_filename("user@example.com:2222/dev"),
            "user_example_com_2222_dev"
        );
        assert_eq!(sanitize_filename("safe-host_01"), "safe-host_01");
    }

    #[test]
    fn normalize_public_key_filename_uses_default_and_strips_pub_suffix() {
        assert_eq!(
            normalize_public_key_filename("", "demo-host").unwrap(),
            "demo-host"
        );
        assert_eq!(
            normalize_public_key_filename("demo.pub", "default").unwrap(),
            "demo"
        );
        assert!(normalize_public_key_filename("../id", "default").is_err());
        assert!(normalize_public_key_filename(".hidden", "default").is_err());
    }

    #[test]
    fn write_public_key_for_config_writes_next_to_config() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(".ssh").join("config");
        let expected_key_path = temp.path().join(".ssh").join("demo.pub");

        let (identity_file, key_path) =
            write_public_key_for_config(&config_path, "demo", "ssh-ed25519 AAAA demo\n").unwrap();

        assert_eq!(identity_file, "~/.ssh/demo.pub");
        assert_eq!(key_path, expected_key_path);
        assert_eq!(
            std::fs::read_to_string(key_path.clone()).unwrap(),
            "ssh-ed25519 AAAA demo"
        );
    }

    #[test]
    fn expand_tilde_leaves_non_tilde_paths_unchanged() {
        assert_eq!(
            expand_tilde("/tmp/id_ed25519").unwrap(),
            PathBuf::from("/tmp/id_ed25519")
        );
        assert_eq!(
            expand_tilde("relative/key").unwrap(),
            PathBuf::from("relative/key")
        );
    }

    #[test]
    fn validators_accept_expected_formats() {
        assert!(validate_forward_format("8080:localhost:80"));
        assert!(!validate_forward_format("localhost:80"));
        assert!(!validate_forward_format("8080::80"));
        assert!(!validate_forward_format("8080:localhost:http"));

        assert!(validate_set_env_format("APP_ENV=prod"));
        assert!(!validate_set_env_format("APP_ENV"));
        assert!(!validate_set_env_format("=prod"));
        assert!(!validate_set_env_format("   =prod"));

        assert!(validate_send_env_format("LANG LC_*"));
        assert!(!validate_send_env_format("LANG=en_US.UTF-8"));
    }

    #[test]
    fn preferred_authentications_tracks_identity_file() {
        assert_eq!(
            preferred_authentications_for(&None, None),
            Some("password".to_string())
        );
        assert_eq!(
            preferred_authentications_for(&None, Some("publickey,password,keyboard-interactive")),
            Some("publickey,password,keyboard-interactive".to_string())
        );
        assert_eq!(
            preferred_authentications_for(&Some("~/.ssh/id_ed25519".to_string()), Some("password")),
            None
        );
        assert_eq!(
            preferred_authentications_for(
                &Some("~/.ssh/id_ed25519".to_string()),
                Some("publickey,password")
            ),
            Some("publickey,password".to_string())
        );
    }
}
