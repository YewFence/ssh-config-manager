use crate::config::{self, SshHost};
use anyhow::Result;
use std::path::Path;

use super::{prompt_forwards, prompt_identity, prompt_optional, prompt_port, prompt_required};

pub struct CreateFlags {
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub description: Option<String>,
}

pub fn run(name: Option<String>, flags: CreateFlags, config_path: &Path) -> Result<()> {
    let mut config = config::load_config(config_path)?;

    let host = prompt_host(name, flags, None)?;

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

/// 构建 SshHost，create/edit 共用。
///
/// - `name`：CLI 位置参数（作为 alias 的 flag 等价值）
/// - `flags`：CLI flags，有值时跳过对应字段的交互
/// - `preset`：edit 时传入原始主机，用于预填默认值并保留 extra 中的非 forward 指令
pub fn prompt_host(
    name: Option<String>,
    flags: CreateFlags,
    preset: Option<&SshHost>,
) -> Result<SshHost> {
    let preset_alias = preset.map(|h| h.alias.as_str()).unwrap_or("");
    let name_flag = name.or_else(|| {
        if preset_alias.is_empty() {
            None
        } else {
            Some(preset_alias.to_string())
        }
    });

    let alias = prompt_required(
        "Host alias:",
        // edit 时 name 已经确定，直接当 flag 传入跳过提示
        name_flag,
        "",
    )?;

    let preset_desc = preset
        .and_then(|h| h.description.as_deref())
        .unwrap_or("");
    let description = prompt_optional("Description (leave blank to skip):", flags.description, preset_desc, None)?;

    let preset_hostname = preset
        .and_then(|h| h.hostname.as_deref())
        .unwrap_or("");
    let hostname = prompt_required("HostName (IP or domain):", flags.hostname, preset_hostname)?;

    let preset_user = preset.and_then(|h| h.user.as_deref()).unwrap_or("");
    let user = prompt_optional("User (leave blank to skip):", flags.user, preset_user, None)?;

    let preset_port = preset.and_then(|h| h.port);
    let port = prompt_port(flags.port, preset_port)?;

    let preset_identity = preset
        .and_then(|h| h.identity_file.as_deref())
        .unwrap_or("");
    let identity_file = prompt_identity(&alias, flags.identity_file, preset_identity)?;

    let preset_proxy = preset
        .and_then(|h| h.proxy_jump.as_deref())
        .unwrap_or("");
    let proxy_jump = prompt_optional(
        "ProxyJump (host alias, leave blank to skip):",
        flags.proxy_jump,
        preset_proxy,
        None,
    )?;

    let preset_local = forwards_from_extra(preset, "LocalForward");
    let preset_remote = forwards_from_extra(preset, "RemoteForward");
    let local_fwds = prompt_forwards("LocalForward", &preset_local)?;
    let remote_fwds = prompt_forwards("RemoteForward", &preset_remote)?;

    let extra = build_extra(&identity_file, &local_fwds, &remote_fwds, preset);

    Ok(SshHost {
        alias,
        description,
        hostname: Some(hostname),
        user,
        port,
        identity_file,
        proxy_jump,
        extra,
    })
}

/// 从 extra 中按 key 筛出已有的 forward 规则值
fn forwards_from_extra(preset: Option<&SshHost>, key: &str) -> Vec<String> {
    preset
        .map(|h| {
            h.extra
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// 构建 extra：PreferredAuthentications + forward 规则 + 保留其他 extra 项
fn build_extra(
    identity_file: &Option<String>,
    local_fwds: &[String],
    remote_fwds: &[String],
    preset: Option<&SshHost>,
) -> Vec<(String, String)> {
    let mut extra = Vec::new();

    if identity_file.is_none() {
        extra.push(("PreferredAuthentications".to_string(), "password".to_string()));
    }

    // 保留 preset 中非 forward 的 extra 项（如 ForwardAgent、StrictHostKeyChecking 等）
    if let Some(h) = preset {
        for (k, v) in &h.extra {
            if !k.eq_ignore_ascii_case("LocalForward")
                && !k.eq_ignore_ascii_case("RemoteForward")
                && !k.eq_ignore_ascii_case("PreferredAuthentications")
            {
                extra.push((k.clone(), v.clone()));
            }
        }
    }

    for rule in local_fwds {
        extra.push(("LocalForward".to_string(), rule.clone()));
    }
    for rule in remote_fwds {
        extra.push(("RemoteForward".to_string(), rule.clone()));
    }

    extra
}
