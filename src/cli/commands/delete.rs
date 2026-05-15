use crate::core::{config, hosts};
use anyhow::Result;
use inquire::Confirm;
use std::path::Path;

pub fn run(name: &str, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let host = config
        .find(name)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in SSH config", name))?;
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

    hosts::delete_host(&mut config, name)?;
    config::save_config(&config, config_path)?;
    println!("Host '{}' deleted.", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::core::{
        config::{SshConfig, SshHost},
        hosts,
    };

    #[test]
    fn core_delete_host_deletes_only_selected_host() {
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

        hosts::delete_host(&mut config, "first").unwrap();

        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.hosts[0].alias, "second");
        assert_eq!(
            config.hosts[0].hostname.as_deref(),
            Some("second.example.com")
        );
    }
}
