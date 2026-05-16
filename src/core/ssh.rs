use std::path::PathBuf;

use anyhow::Result;

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
        return Ok(Some(format!("~/.ssh/{}", trimmed)));
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
    input.contains('=')
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
            Some("~/.ssh/id_ed25519".to_string())
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
