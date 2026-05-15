use crate::core::{config, hosts};
use anyhow::Result;
use inquire::{Text, validator::Validation};
use std::path::Path;

pub fn run(source: &str, name: Option<String>, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let new_alias = match name {
        Some(n) => n,
        None => Text::new("New host alias:")
            .with_validator(|s: &str| {
                Ok(match hosts::validate_alias(&config, None, s) {
                    Ok(_) => Validation::Valid,
                    Err(err) => Validation::Invalid(err.to_string().into()),
                })
            })
            .prompt()?,
    };

    let index = hosts::clone_host(&mut config, source, &new_alias)?;
    let new_alias = config.hosts[index].alias.clone();
    config::save_config(&config, config_path)?;
    println!("Host '{}' cloned from '{}'.", new_alias, source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{SshConfig, SshHost};

    #[test]
    fn run_clones_existing_host_with_new_alias() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        config::save_config(
            &SshConfig {
                hosts: vec![SshHost {
                    alias: "source".to_string(),
                    description: Some("Source host".to_string()),
                    hostname: Some("source.example.com".to_string()),
                    user: Some("ubuntu".to_string()),
                    port: Some(2222),
                    identity_file: Some("~/.ssh/id_ed25519".to_string()),
                    proxy_jump: Some("bastion".to_string()),
                    preferred_authentications: None,
                    forward_agent: Some("yes".to_string()),
                    local_forwards: vec!["8080:localhost:80".to_string()],
                    remote_forwards: vec!["9090:localhost:90".to_string()],
                    set_env: vec!["APP_ENV=prod".to_string()],
                    send_env: vec!["LANG".to_string()],
                    extra: vec![("StrictHostKeyChecking".to_string(), "no".to_string())],
                }],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();

        run("source", Some("copy".to_string()), &config_path).unwrap();

        let config = config::load_config(&config_path).unwrap();
        let source = config.find("source").unwrap();
        let copy = config.find("copy").unwrap();

        assert_eq!(config.hosts.len(), 2);
        assert_eq!(source.hostname, copy.hostname);
        assert_eq!(source.user, copy.user);
        assert_eq!(source.port, copy.port);
        assert_eq!(source.identity_file, copy.identity_file);
        assert_eq!(source.proxy_jump, copy.proxy_jump);
        assert_eq!(source.forward_agent, copy.forward_agent);
        assert_eq!(source.local_forwards, copy.local_forwards);
        assert_eq!(source.remote_forwards, copy.remote_forwards);
        assert_eq!(source.set_env, copy.set_env);
        assert_eq!(source.send_env, copy.send_env);
        assert_eq!(source.extra, copy.extra);
    }

    #[test]
    fn run_rejects_existing_target_alias() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config");
        config::save_config(
            &SshConfig {
                hosts: vec![
                    SshHost {
                        alias: "source".to_string(),
                        hostname: Some("source.example.com".to_string()),
                        ..Default::default()
                    },
                    SshHost {
                        alias: "copy".to_string(),
                        hostname: Some("copy.example.com".to_string()),
                        ..Default::default()
                    },
                ],
                header_comments: vec![],
            },
            &config_path,
        )
        .unwrap();

        let result = run("source", Some("copy".to_string()), &config_path);

        assert!(result.is_err());
        let config = config::load_config(&config_path).unwrap();
        assert_eq!(config.hosts.len(), 2);
    }
}
