use crate::config;
use anyhow::Result;
use comfy_table::{Attribute, Cell, Color, Table};
use std::path::Path;

pub fn run(config_path: &Path) -> Result<()> {
    let config = config::load_config(config_path)?;

    if config.hosts.is_empty() {
        println!("No hosts configured. Use `sshm create` to add one.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("NAME").add_attribute(Attribute::Bold),
        Cell::new("HOSTNAME").add_attribute(Attribute::Bold),
        Cell::new("USER").add_attribute(Attribute::Bold),
        Cell::new("PORT").add_attribute(Attribute::Bold),
        Cell::new("IDENTITY FILE").add_attribute(Attribute::Bold),
        Cell::new("PROXY JUMP").add_attribute(Attribute::Bold),
    ]);

    for host in &config.hosts {
        table.add_row(vec![
            Cell::new(&host.alias).fg(Color::Cyan),
            Cell::new(host.hostname.as_deref().unwrap_or("-")),
            Cell::new(host.user.as_deref().unwrap_or("-")),
            Cell::new(host.port.map(|p| p.to_string()).as_deref().unwrap_or("22")),
            Cell::new(host.identity_file.as_deref().unwrap_or("-")),
            Cell::new(host.proxy_jump.as_deref().unwrap_or("-")),
        ]);
    }

    println!("{table}");
    Ok(())
}
