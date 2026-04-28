use crate::config;
use anyhow::Result;
use std::path::Path;

use super::host_builder::{HostFlags, prompt_host};

pub fn run(name: Option<String>, flags: HostFlags, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;
    let show_advanced_menu = !flags.has_any();

    let host = prompt_host(name, flags, None, show_advanced_menu)?;

    if config.contains(&host.alias) {
        anyhow::bail!(
            "Host '{}' already exists. Use `sshm edit {}` to modify it.",
            host.alias,
            host.alias
        );
    }

    let alias = host.alias.clone();
    config.hosts.push(host);
    config::save_config(&config, config_path)?;
    println!("Host '{}' added.", alias);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_writes_host_from_flags_without_prompting() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(".ssh").join("config");

        run(
            Some("demo".to_string()),
            HostFlags {
                hostname: Some("demo.example.com".to_string()),
                user: Some("ubuntu".to_string()),
                port: Some(2222),
                identity_file: Some("id_ed25519".to_string()),
                proxy_jump: Some("bastion".to_string()),
                description: Some("Demo host".to_string()),
            },
            &config_path,
        )
        .unwrap();

        let config = config::load_config(&config_path).unwrap();
        let host = config.find("demo").unwrap();

        assert_eq!(host.description.as_deref(), Some("Demo host"));
        assert_eq!(host.hostname.as_deref(), Some("demo.example.com"));
        assert_eq!(host.user.as_deref(), Some("ubuntu"));
        assert_eq!(host.port, Some(2222));
        assert_eq!(host.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(host.proxy_jump.as_deref(), Some("bastion"));
    }

    #[test]
    fn run_rejects_duplicate_alias() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");

        run(
            Some("demo".to_string()),
            HostFlags {
                hostname: Some("first.example.com".to_string()),
                user: Some("ubuntu".to_string()),
                port: Some(22),
                identity_file: Some("id_ed25519".to_string()),
                proxy_jump: None,
                description: None,
            },
            &config_path,
        )
        .unwrap();

        let result = run(
            Some("demo".to_string()),
            HostFlags {
                hostname: Some("second.example.com".to_string()),
                user: Some("root".to_string()),
                port: Some(22),
                identity_file: Some("id_rsa".to_string()),
                proxy_jump: None,
                description: None,
            },
            &config_path,
        );

        assert!(result.is_err());
        let config = config::load_config(&config_path).unwrap();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(
            config.find("demo").unwrap().hostname.as_deref(),
            Some("first.example.com")
        );
    }
}
