use crate::config::{self, SshHost};
use anyhow::Result;
use inquire::{validator::Validation, Text};
use std::path::Path;

pub struct CreateFlags {
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
}

pub fn run(name: Option<String>, flags: CreateFlags, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let non_interactive = name.is_some() && flags.hostname.is_some();

    let host = if non_interactive {
        build_host_from_flags(name.unwrap(), flags)
    } else {
        prompt_host(name, flags)?
    };

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

fn build_host_from_flags(name: String, flags: CreateFlags) -> SshHost {
    let extra = if flags.identity_file.is_none() {
        vec![("PreferredAuthentications".to_string(), "password".to_string())]
    } else {
        vec![]
    };
    SshHost {
        alias: name,
        hostname: flags.hostname,
        user: flags.user,
        port: flags.port,
        identity_file: flags.identity_file,
        proxy_jump: flags.proxy_jump,
        extra,
    }
}

pub fn prompt_host(name: Option<String>, flags: CreateFlags) -> Result<SshHost> {
    let default_alias = name.as_deref().unwrap_or("").to_string();
    let alias = Text::new("Host alias:")
        .with_default(&default_alias)
        .with_validator(|s: &str| {
            if s.is_empty() {
                Ok(Validation::Invalid("Alias cannot be empty.".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;

    let default_hostname = flags.hostname.as_deref().unwrap_or("").to_string();
    let hostname = Text::new("HostName (IP or domain):")
        .with_default(&default_hostname)
        .with_validator(|s: &str| {
            if s.is_empty() {
                Ok(Validation::Invalid("HostName is required.".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;

    let default_user = flags.user.as_deref().unwrap_or("").to_string();
    let user_input = Text::new("User (leave blank to skip):")
        .with_default(&default_user)
        .prompt()?;
    let user = if user_input.is_empty() {
        None
    } else {
        Some(user_input)
    };

    let default_port = flags
        .port
        .map(|p| p.to_string())
        .unwrap_or_else(|| "22".to_string());
    let port_input = Text::new("Port:")
        .with_default(&default_port)
        .with_validator(|s: &str| {
            if s.is_empty() || s.parse::<u16>().is_ok() {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Port must be a number between 1 and 65535.".into(),
                ))
            }
        })
        .prompt()?;
    let port = port_input.parse::<u16>().ok();

    let default_identity = flags.identity_file.as_deref().unwrap_or("").to_string();
    let identity_input = Text::new("IdentityFile (path, filename, or paste public key):")
        .with_default(&default_identity)
        .with_help_message(
            "filename → auto-prefix ~/.ssh/ | pubkey content → saved to ~/.ssh/<name>.pub",
        )
        .prompt()?;
    let identity_file = super::resolve_identity_file(&identity_input, &alias)?;

    let default_proxy = flags.proxy_jump.as_deref().unwrap_or("").to_string();
    let proxy_input = Text::new("ProxyJump (host alias, leave blank to skip):")
        .with_default(&default_proxy)
        .prompt()?;
    let proxy_jump = if proxy_input.is_empty() {
        None
    } else {
        Some(proxy_input)
    };

    // 收集 forward 规则
    let local_forwards = collect_forwards("LocalForward")?;
    let remote_forwards = collect_forwards("RemoteForward")?;

    let mut extra = if identity_file.is_none() {
        vec![("PreferredAuthentications".to_string(), "password".to_string())]
    } else {
        vec![]
    };

    // 添加 forward 规则到 extra
    for rule in local_forwards {
        extra.push(("LocalForward".to_string(), rule));
    }
    for rule in remote_forwards {
        extra.push(("RemoteForward".to_string(), rule));
    }

    Ok(SshHost {
        alias,
        hostname: Some(hostname),
        user,
        port,
        identity_file,
        proxy_jump,
        extra,
    })
}

/// 循环收集 forward 规则，直到用户输入空行为止
/// 格式: local_port:dest_host:dest_port
fn collect_forwards(kind: &str) -> Result<Vec<String>> {
    let mut rules = Vec::new();

    println!("\nAdd {} rules (format: local_port:dest_host:dest_port)", kind);
    println!("Example: 8080:localhost:80  →  forwards local port 8080 to remote localhost:80");
    println!("Leave blank and press Enter to skip/continue.\n");

    loop {
        let prompt = format!("{} [{}]:", kind, rules.len() + 1);
        let input = Text::new(&prompt)
            .with_help_message("format: local_port:dest_host:dest_port (e.g., 8080:localhost:80)")
            .prompt()?;

        if input.trim().is_empty() {
            break;
        }

        // 简单验证格式
        if validate_forward_format(&input) {
            rules.push(input.trim().to_string());
        } else {
            println!("  Invalid format. Expected: local_port:dest_host:dest_port");
            println!("  Example: 8080:localhost:80");
        }
    }

    if !rules.is_empty() {
        println!("  Added {} {} rule(s)\n", rules.len(), kind);
    }

    Ok(rules)
}

/// 验证 forward 格式: local_port:dest_host:dest_port
fn validate_forward_format(input: &str) -> bool {
    let parts: Vec<&str> = input.trim().split(':').collect();
    if parts.len() != 3 {
        return false;
    }

    // 验证 local_port 是数字
    if parts[0].parse::<u16>().is_err() {
        return false;
    }

    // dest_host 不能为空
    if parts[1].is_empty() {
        return false;
    }

    // 验证 dest_port 是数字
    if parts[2].parse::<u16>().is_err() {
        return false;
    }

    true
}
