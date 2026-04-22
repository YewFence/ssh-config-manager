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
