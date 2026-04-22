use crate::config::{self, SshHost};
use anyhow::Result;
use inquire::{Text, validator::Validation};
use std::path::Path;

pub fn run(source: &str, name: Option<String>, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let source_host = config
        .find(source)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", source))?;
    let mut new_host: SshHost = source_host.clone();

    let new_alias = match name {
        Some(n) => n,
        None => Text::new("New host alias:")
            .with_validator(|s: &str| {
                if s.is_empty() {
                    Ok(Validation::Invalid("Alias cannot be empty.".into()))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?,
    };

    if config.contains(&new_alias) {
        anyhow::bail!(
            "Host '{}' already exists. Use `sshm edit {}` to modify it.",
            new_alias,
            new_alias
        );
    }

    new_host.alias = new_alias.clone();
    config.hosts.push(new_host);
    config::save_config(&config, config_path)?;
    println!("Host '{}' cloned from '{}'.", new_alias, source);
    Ok(())
}
