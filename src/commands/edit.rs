use crate::commands::create::{prompt_host, CreateFlags};
use crate::config;
use anyhow::Result;
use std::path::Path;

pub fn run(name: &str, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let original = config
        .find(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", name))?;

    let flags = CreateFlags {
        hostname: original.hostname.clone(),
        user: original.user.clone(),
        port: original.port,
        identity_file: original.identity_file.clone(),
        proxy_jump: original.proxy_jump.clone(),
        description: original.description.clone(),
    };

    let mut updated = prompt_host(Some(original.alias.clone()), flags)?;
    updated.extra = original.extra.clone();

    let host = config.find_mut(name).unwrap();
    *host = updated;

    config::save_config(&config, config_path)?;
    println!("Host '{}' updated.", name);
    Ok(())
}
