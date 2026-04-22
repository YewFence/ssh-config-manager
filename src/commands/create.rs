use crate::config::{self, SshHost};
use anyhow::Result;
use std::path::Path;

use super::{
    AdvancedConfigChoice, EnvRuleChoice, ForwardRuleChoice, prompt_advanced_config_choice,
    prompt_env_rule_choice, prompt_env_values, prompt_forward_rule_choice, prompt_forwards,
    prompt_identity, prompt_optional, prompt_port, prompt_required, prompt_yes_no_directive,
    resolve_identity_file,
};

pub struct CreateFlags {
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub description: Option<String>,
}

impl CreateFlags {
    pub fn has_any(&self) -> bool {
        self.hostname.is_some()
            || self.user.is_some()
            || self.port.is_some()
            || self.identity_file.is_some()
            || self.proxy_jump.is_some()
            || self.description.is_some()
    }
}

pub fn run(name: Option<String>, flags: CreateFlags, config_path: &Path) -> Result<()> {
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

/// 构建 SshHost，create/edit 共用。
///
/// - `name`：CLI 位置参数（作为 alias 的 flag 等价值）
/// - `flags`：CLI flags，有值时跳过对应字段的交互
/// - `preset`：edit 时传入原始主机，用于预填默认值并保留 extra 中的非 forward 指令
/// - `show_advanced_menu`：是否进入高级配置菜单
pub fn prompt_host(
    name: Option<String>,
    flags: CreateFlags,
    preset: Option<&SshHost>,
    show_advanced_menu: bool,
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

    let preset_hostname = preset.and_then(|h| h.hostname.as_deref()).unwrap_or("");
    let hostname = prompt_required("HostName (IP or domain):", flags.hostname, preset_hostname)?;

    let preset_user = preset.and_then(|h| h.user.as_deref()).unwrap_or("");
    let user = prompt_optional("User (leave blank to skip):", flags.user, preset_user, None)?;

    let preset_port = preset.and_then(|h| h.port);
    let port = prompt_port(flags.port, preset_port)?;

    let preset_identity = preset
        .and_then(|h| h.identity_file.as_deref())
        .unwrap_or("");
    let identity_file = prompt_identity(&alias, flags.identity_file, preset_identity)?;

    let mut description = merge_optional_flag(
        flags.description,
        preset.and_then(|h| h.description.clone()),
    );
    let mut proxy_jump =
        merge_optional_flag(flags.proxy_jump, preset.and_then(|h| h.proxy_jump.clone()));
    let mut forward_agent = extra_value(preset, "ForwardAgent");
    let mut local_fwds = extra_values(preset, "LocalForward");
    let mut remote_fwds = extra_values(preset, "RemoteForward");
    let mut set_envs = extra_values(preset, "SetEnv");
    let mut send_envs = extra_values(preset, "SendEnv");

    if show_advanced_menu {
        prompt_advanced_sections(
            &mut description,
            &mut proxy_jump,
            &mut forward_agent,
            &mut local_fwds,
            &mut remote_fwds,
            &mut set_envs,
            &mut send_envs,
        )?;
    }

    let extra = build_extra(
        &identity_file,
        &forward_agent,
        &local_fwds,
        &remote_fwds,
        &set_envs,
        &send_envs,
        preset,
    );

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

pub fn apply_flag_updates(
    name: Option<String>,
    flags: CreateFlags,
    preset: &SshHost,
) -> Result<SshHost> {
    let alias = name.unwrap_or_else(|| preset.alias.clone());
    let hostname = flags.hostname.or_else(|| preset.hostname.clone());
    let user = merge_optional_flag(flags.user, preset.user.clone());
    let port = flags.port.or(preset.port);
    let identity_file = match flags.identity_file {
        Some(value) => resolve_identity_file(&value, &alias)?,
        None => preset.identity_file.clone(),
    };
    let proxy_jump = merge_optional_flag(flags.proxy_jump, preset.proxy_jump.clone());
    let description = merge_optional_flag(flags.description, preset.description.clone());
    let forward_agent = extra_value(Some(preset), "ForwardAgent");

    let local_fwds = extra_values(Some(preset), "LocalForward");
    let remote_fwds = extra_values(Some(preset), "RemoteForward");
    let set_envs = extra_values(Some(preset), "SetEnv");
    let send_envs = extra_values(Some(preset), "SendEnv");
    let extra = build_extra(
        &identity_file,
        &forward_agent,
        &local_fwds,
        &remote_fwds,
        &set_envs,
        &send_envs,
        Some(preset),
    );

    Ok(SshHost {
        alias,
        description,
        hostname,
        user,
        port,
        identity_file,
        proxy_jump,
        extra,
    })
}

fn prompt_advanced_sections(
    description: &mut Option<String>,
    proxy_jump: &mut Option<String>,
    forward_agent: &mut Option<String>,
    local_fwds: &mut Vec<String>,
    remote_fwds: &mut Vec<String>,
    set_envs: &mut Vec<String>,
    send_envs: &mut Vec<String>,
) -> Result<()> {
    loop {
        match prompt_advanced_config_choice()? {
            AdvancedConfigChoice::ProxyJump => {
                *proxy_jump = prompt_optional(
                    "ProxyJump (host alias, leave blank to skip):",
                    None,
                    proxy_jump.as_deref().unwrap_or(""),
                    None,
                )?;
            }
            AdvancedConfigChoice::ForwardAgent => {
                prompt_yes_no_directive("ForwardAgent:", forward_agent)?;
            }
            AdvancedConfigChoice::ForwardRules => {
                prompt_forward_sections(local_fwds, remote_fwds)?;
            }
            AdvancedConfigChoice::EnvRules => {
                prompt_env_sections(set_envs, send_envs)?;
            }
            AdvancedConfigChoice::Description => {
                *description = prompt_optional(
                    "Description (leave blank to skip):",
                    None,
                    description.as_deref().unwrap_or(""),
                    None,
                )?;
            }
            AdvancedConfigChoice::Finish => break,
        }
    }

    Ok(())
}

fn prompt_forward_sections(
    local_fwds: &mut Vec<String>,
    remote_fwds: &mut Vec<String>,
) -> Result<()> {
    loop {
        match prompt_forward_rule_choice()? {
            ForwardRuleChoice::Local => {
                *local_fwds = prompt_forwards("LocalForward", local_fwds)?;
            }
            ForwardRuleChoice::Remote => {
                *remote_fwds = prompt_forwards("RemoteForward", remote_fwds)?;
            }
            ForwardRuleChoice::Back => break,
        }
    }

    Ok(())
}

fn prompt_env_sections(set_envs: &mut Vec<String>, send_envs: &mut Vec<String>) -> Result<()> {
    loop {
        match prompt_env_rule_choice()? {
            EnvRuleChoice::Set => {
                *set_envs = prompt_env_values("SetEnv", set_envs)?;
            }
            EnvRuleChoice::Send => {
                *send_envs = prompt_env_values("SendEnv", send_envs)?;
            }
            EnvRuleChoice::Back => break,
        }
    }

    Ok(())
}

fn extra_value(preset: Option<&SshHost>, key: &str) -> Option<String> {
    extra_values(preset, key).into_iter().next()
}

/// 从 extra 中按 key 筛出已有的指令值
fn extra_values(preset: Option<&SshHost>, key: &str) -> Vec<String> {
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

fn merge_optional_flag(flag: Option<String>, current: Option<String>) -> Option<String> {
    match flag {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(value),
        None => current,
    }
}

/// 构建 extra：PreferredAuthentications + managed directives + 保留其他 extra 项
fn build_extra(
    identity_file: &Option<String>,
    forward_agent: &Option<String>,
    local_fwds: &[String],
    remote_fwds: &[String],
    set_envs: &[String],
    send_envs: &[String],
    preset: Option<&SshHost>,
) -> Vec<(String, String)> {
    let mut extra = Vec::new();

    if identity_file.is_none() {
        extra.push((
            "PreferredAuthentications".to_string(),
            "password".to_string(),
        ));
    }

    // 保留 preset 中非托管的 extra 项（如 StrictHostKeyChecking 等）
    if let Some(h) = preset {
        for (k, v) in &h.extra {
            if !is_managed_extra_key(k) {
                extra.push((k.clone(), v.clone()));
            }
        }
    }

    if let Some(value) = forward_agent {
        extra.push(("ForwardAgent".to_string(), value.clone()));
    }
    for rule in local_fwds {
        extra.push(("LocalForward".to_string(), rule.clone()));
    }
    for rule in remote_fwds {
        extra.push(("RemoteForward".to_string(), rule.clone()));
    }
    for value in set_envs {
        extra.push(("SetEnv".to_string(), value.clone()));
    }
    for value in send_envs {
        extra.push(("SendEnv".to_string(), value.clone()));
    }

    extra
}

fn is_managed_extra_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("ForwardAgent")
        || key.eq_ignore_ascii_case("LocalForward")
        || key.eq_ignore_ascii_case("RemoteForward")
        || key.eq_ignore_ascii_case("SetEnv")
        || key.eq_ignore_ascii_case("SendEnv")
        || key.eq_ignore_ascii_case("PreferredAuthentications")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_extra_replaces_managed_directives_and_keeps_unmanaged_ones() {
        let preset = SshHost {
            alias: "demo".to_string(),
            description: None,
            hostname: Some("example.com".to_string()),
            user: Some("root".to_string()),
            port: Some(22),
            identity_file: Some("~/.ssh/id_ed25519".to_string()),
            proxy_jump: None,
            extra: vec![
                ("ForwardAgent".to_string(), "yes".to_string()),
                (
                    "LocalForward".to_string(),
                    "1000:localhost:1000".to_string(),
                ),
                ("SetEnv".to_string(), "OLD_ENV=1".to_string()),
                (
                    "PreferredAuthentications".to_string(),
                    "password".to_string(),
                ),
            ],
        };

        let extra = build_extra(
            &None,
            &Some("no".to_string()),
            &["8080:localhost:80".to_string()],
            &["9090:localhost:90".to_string()],
            &["APP_ENV=prod".to_string()],
            &["LANG LC_*".to_string()],
            Some(&preset),
        );

        assert_eq!(
            extra,
            vec![
                (
                    "PreferredAuthentications".to_string(),
                    "password".to_string()
                ),
                ("ForwardAgent".to_string(), "no".to_string()),
                ("LocalForward".to_string(), "8080:localhost:80".to_string()),
                ("RemoteForward".to_string(), "9090:localhost:90".to_string()),
                ("SetEnv".to_string(), "APP_ENV=prod".to_string()),
                ("SendEnv".to_string(), "LANG LC_*".to_string()),
            ]
        );
    }

    #[test]
    fn merge_optional_flag_treats_blank_as_clear() {
        assert_eq!(
            merge_optional_flag(Some(String::new()), Some("root".to_string())),
            None
        );
        assert_eq!(
            merge_optional_flag(Some("ubuntu".to_string()), Some("root".to_string())),
            Some("ubuntu".to_string())
        );
        assert_eq!(
            merge_optional_flag(None, Some("root".to_string())),
            Some("root".to_string())
        );
    }
}
