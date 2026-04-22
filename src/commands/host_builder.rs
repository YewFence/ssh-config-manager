use crate::config::SshHost;
use anyhow::Result;

use super::{
    AdvancedConfigChoice, EnvRuleChoice, ForwardRuleChoice, prompt_advanced_config_choice,
    prompt_env_rule_choice, prompt_env_values, prompt_forward_rule_choice, prompt_forwards,
    prompt_identity, prompt_optional, prompt_port, prompt_required, prompt_yes_no_directive,
    resolve_identity_file,
};

pub struct HostFlags {
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub description: Option<String>,
}

impl HostFlags {
    pub fn has_any(&self) -> bool {
        self.hostname.is_some()
            || self.user.is_some()
            || self.port.is_some()
            || self.identity_file.is_some()
            || self.proxy_jump.is_some()
            || self.description.is_some()
    }
}

/// 构建 SshHost，create/edit 共用。
///
/// - `name`：CLI 位置参数（作为 alias 的 flag 等价值）
/// - `flags`：CLI flags，有值时跳过对应字段的交互
/// - `preset`：edit 时传入原始主机，用于预填默认值并保留未识别指令
/// - `show_advanced_menu`：是否进入高级配置菜单
pub fn prompt_host(
    name: Option<String>,
    flags: HostFlags,
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
    let mut forward_agent = preset.and_then(|h| h.forward_agent.clone());
    let mut local_fwds = preset.map(|h| h.local_forwards.clone()).unwrap_or_default();
    let mut remote_fwds = preset
        .map(|h| h.remote_forwards.clone())
        .unwrap_or_default();
    let mut set_envs = preset.map(|h| h.set_env.clone()).unwrap_or_default();
    let mut send_envs = preset.map(|h| h.send_env.clone()).unwrap_or_default();

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

    Ok(SshHost {
        alias,
        description,
        hostname: Some(hostname),
        user,
        port,
        identity_file: identity_file.clone(),
        proxy_jump,
        preferred_authentications: preferred_authentications_for(
            &identity_file,
            preset.and_then(|h| h.preferred_authentications.as_deref()),
        ),
        forward_agent,
        local_forwards: local_fwds,
        remote_forwards: remote_fwds,
        set_env: set_envs,
        send_env: send_envs,
        extra: preset.map(|h| h.extra.clone()).unwrap_or_default(),
    })
}

pub fn apply_flag_updates(
    name: Option<String>,
    flags: HostFlags,
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

    Ok(SshHost {
        alias,
        description,
        hostname,
        user,
        port,
        identity_file: identity_file.clone(),
        proxy_jump,
        preferred_authentications: preferred_authentications_for(
            &identity_file,
            preset.preferred_authentications.as_deref(),
        ),
        forward_agent: preset.forward_agent.clone(),
        local_forwards: preset.local_forwards.clone(),
        remote_forwards: preset.remote_forwards.clone(),
        set_env: preset.set_env.clone(),
        send_env: preset.send_env.clone(),
        extra: preset.extra.clone(),
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

fn merge_optional_flag(flag: Option<String>, current: Option<String>) -> Option<String> {
    match flag {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(value),
        None => current,
    }
}

fn preferred_authentications_for(
    identity_file: &Option<String>,
    current: Option<&str>,
) -> Option<String> {
    if identity_file.is_none() {
        return Some(current.unwrap_or("password").to_string());
    }

    match current {
        Some(value) if value.eq_ignore_ascii_case("password") => None,
        Some(value) => Some(value.to_string()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn preferred_authentications_tracks_identity_file() {
        assert_eq!(
            preferred_authentications_for(&None, None),
            Some("password".to_string())
        );
        assert_eq!(
            preferred_authentications_for(&None, Some("publickey,password,keyboard-interactive")),
            Some("publickey,password,keyboard-interactive".to_string())
        );
        assert_eq!(
            preferred_authentications_for(&Some("~/.ssh/id_ed25519".to_string()), Some("password")),
            None
        );
        assert_eq!(
            preferred_authentications_for(
                &Some("~/.ssh/id_ed25519".to_string()),
                Some("publickey,password")
            ),
            Some("publickey,password".to_string())
        );
    }
}
