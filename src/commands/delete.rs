use crate::config;
use anyhow::{Result, bail};
use inquire::Confirm;
use std::path::Path;

pub fn run(name: &str, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let Some(idx) = find_host_index(&config, name) else {
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

    remove_host_at(&mut config, idx);
    config::save_config(&config, config_path)?;
    println!("Host '{}' deleted.", name);
    Ok(())
}

fn find_host_index(config: &config::SshConfig, name: &str) -> Option<usize> {
    config.hosts.iter().position(|h| h.alias == name)
}

fn remove_host_at(config: &mut config::SshConfig, idx: usize) {
    config.hosts.remove(idx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SshConfig, SshHost};

    #[test]
    fn remove_host_at_deletes_only_selected_host() {
        let mut config = SshConfig {
            hosts: vec![
                SshHost {
                    alias: "first".to_string(),
                    hostname: Some("first.example.com".to_string()),
                    ..Default::default()
                },
                SshHost {
                    alias: "second".to_string(),
                    hostname: Some("second.example.com".to_string()),
                    ..Default::default()
                },
            ],
            header_comments: vec![],
        };

        let idx = find_host_index(&config, "first").unwrap();
        remove_host_at(&mut config, idx);

        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].alias, "second");
        assert_eq!(
            config.hosts[0].hostname.as_deref(),
            Some("second.example.com")
        );
    }
}
