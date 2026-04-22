use crate::commands::create::{CreateFlags, prompt_host};
use crate::config;
use anyhow::Result;
use std::path::Path;

pub fn run(name: &str, flags: CreateFlags, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let original = config
        .find(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", name))?;

    let updated = prompt_host(Some(original.alias.clone()), flags, Some(&original))?;

    let host = config.find_mut(name).unwrap();
    *host = updated;

    config::save_config(&config, config_path)?;
    println!("Host '{}' updated.", name);
    Ok(())
}
