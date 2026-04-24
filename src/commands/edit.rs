use crate::config;
use anyhow::Result;
use std::path::Path;

use super::host_builder::{HostFlags, apply_flag_updates, prompt_host};

pub fn run(name: &str, flags: HostFlags, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let original = config
        .find(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", name))?;

    let updated = if flags.has_any() {
        apply_flag_updates(Some(original.alias.clone()), flags, &original)?
    } else {
        prompt_host(Some(original.alias.clone()), flags, Some(&original), true)?
    };

    let host = config.find_mut(name).unwrap();
    *host = updated;

    config::save_config(&config, config_path)?;
    println!("Host '{}' updated.", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SshConfig, SshHost};

    #[test]
    fn run_updates_flags_and_preserves_unmentioned_fields() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        let original = SshHost {
            alias: "demo".to_string(),
            description: Some("old description".to_string()),
            hostname: Some("old.example.com".to_string()),
            user: Some("ubuntu".to_string()),
            port: Some(22),
            identity_file: Some("~/.ssh/id_ed25519".to_string()),
            proxy_jump: Some("old-bastion".to_string()),
            preferred_authentications: None,
            forward_agent: Some("yes".to_string()),
            local_forwards: vec!["8080:localhost:80".to_string()],
            remote_forwards: vec![],
            set_env: vec!["APP_ENV=dev".to_string()],
            send_env: vec![],
            extra: vec![("StrictHostKeyChecking".to_string(), "no".to_string())],
        };
        config::save_config(
            &SshConfig {
                hosts: vec![original],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();

        run(
            "demo",
            HostFlags {
                hostname: Some("new.example.com".to_string()),
                user: Some(String::new()),
                port: Some(2200),
                identity_file: None,
                proxy_jump: Some(String::new()),
                description: Some("new description".to_string()),
            },
            &config_path,
        )
        .unwrap();

        let config = config::load_config(&config_path).unwrap();
        let host = config.find("demo").unwrap();

        assert_eq!(host.description.as_deref(), Some("new description"));
        assert_eq!(host.hostname.as_deref(), Some("new.example.com"));
        assert_eq!(host.user, None);
        assert_eq!(host.port, Some(2200));
        assert_eq!(host.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(host.proxy_jump, None);
        assert_eq!(host.forward_agent.as_deref(), Some("yes"));
        assert_eq!(host.local_forwards, vec!["8080:localhost:80"]);
        assert_eq!(host.set_env, vec!["APP_ENV=dev"]);
        assert_eq!(
            host.extra,
            vec![("StrictHostKeyChecking".to_string(), "no".to_string())]
        );
    }

    #[test]
    fn run_reports_missing_host() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");

        let result = run(
            "missing",
            HostFlags {
                hostname: Some("missing.example.com".to_string()),
                user: None,
                port: None,
                identity_file: None,
                proxy_jump: None,
                description: None,
            },
            &config_path,
        );

        assert!(result.is_err());
    }
}
