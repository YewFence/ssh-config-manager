use crate::commands::create::{CreateFlags, apply_flag_updates, prompt_host};
use crate::config;
use anyhow::Result;
use std::path::Path;

pub fn run(name: &str, flags: CreateFlags, config_path: &Path) -> Result<()> {
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
