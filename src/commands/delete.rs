use crate::config;
use anyhow::{Result, bail};
use inquire::Confirm;
use std::path::Path;

pub fn run(name: &str, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let pos = config.hosts.iter().position(|h| h.alias == name);
    let Some(idx) = pos else {
        bail!("Host '{}' not found in SSH config", name);
    };

    let host = &config.hosts[idx];
    let hostname_display = host.hostname.as_deref().unwrap_or("-");
    println!("Host:     {}", host.alias);
    println!("HostName: {}", hostname_display);

    let confirmed = Confirm::new(&format!("Delete host '{}'?", name))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        println!("Aborted.");
        return Ok(());
    }

    config.hosts.remove(idx);
    config::save_config(&config, config_path)?;
    println!("Host '{}' deleted.", name);
    Ok(())
}
