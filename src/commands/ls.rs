use crate::config;
use anyhow::Result;
use comfy_table::{Attribute, Cell, Color, Table};
use std::path::Path;

/// 遮罩主机名：保留前 3 和后 3 个字符，中间替换为 `***`。
/// 长度不超过 6 时直接返回 `***`。
fn mask_host(s: &str) -> String {
    if s.len() <= 6 {
        return format!("{}***{}", &s[..s.len() / 2], &s[s.len() / 2..]);
    }
    format!("{}***{}", &s[..3], &s[s.len() - 3..])
}

/// 判断是否显示完整主机名：`--show` 标志或环境变量 `SSHM_SHOW=1|true|yes`。
fn should_show(flag: bool) -> bool {
    if flag {
        return true;
    }
    std::env::var("SSHM_SHOW")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

pub fn run(config_path: &Path, show: bool) -> Result<()> {
    let config = config::load_config(config_path)?;
    let show = should_show(show);

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

    let display = |v: Option<&str>| -> String {
        match v {
            Some(s) if show => s.to_string(),
            Some(s) => mask_host(s),
            None => "-".to_string(),
        }
    };

    for host in &config.hosts {
        table.add_row(vec![
            Cell::new(&host.alias).fg(Color::Cyan),
            Cell::new(display(host.hostname.as_deref())),
            Cell::new(host.user.as_deref().unwrap_or("-")),
            Cell::new(host.port.map(|p| p.to_string()).as_deref().unwrap_or("22")),
            Cell::new(host.identity_file.as_deref().unwrap_or("-")),
            Cell::new(display(host.proxy_jump.as_deref())),
        ]);
    }

    println!("{table}");
    Ok(())
}
