use anyhow::Result;

use super::{
    config::{SshConfig, SshHost},
    ssh::{
        normalize_identity_file_path, preferred_authentications_for, validate_forward_format,
        validate_send_env_format, validate_set_env_format,
    },
};

pub const EDITABLE_HOST_FIELDS: [HostField; 13] = [
    HostField::Alias,
    HostField::Description,
    HostField::HostName,
    HostField::User,
    HostField::Port,
    HostField::IdentityFile,
    HostField::ProxyJump,
    HostField::ForwardAgent,
    HostField::PreferredAuthentications,
    HostField::LocalForward,
    HostField::RemoteForward,
    HostField::SetEnv,
    HostField::SendEnv,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostField {
    Alias,
    Description,
    HostName,
    User,
    Port,
    IdentityFile,
    ProxyJump,
    ForwardAgent,
    PreferredAuthentications,
    LocalForward,
    RemoteForward,
    SetEnv,
    SendEnv,
}

impl HostField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Alias => "Alias",
            Self::Description => "Description",
            Self::HostName => "HostName",
            Self::User => "User",
            Self::Port => "Port",
            Self::IdentityFile => "IdentityFile",
            Self::ProxyJump => "ProxyJump",
            Self::ForwardAgent => "ForwardAgent",
            Self::PreferredAuthentications => "PreferredAuth",
            Self::LocalForward => "LocalForward",
            Self::RemoteForward => "RemoteForward",
            Self::SetEnv => "SetEnv",
            Self::SendEnv => "SendEnv",
        }
    }

    pub fn example(self) -> &'static str {
        match self {
            Self::Alias => "example: prod-api",
            Self::Description => "example: Production API\\nBehind bastion",
            Self::HostName => "example: 10.0.0.12 or example.com",
            Self::User => "example: ubuntu",
            Self::Port => "example: 22",
            Self::IdentityFile => "example: id_ed25519 or ~/.ssh/id_ed25519",
            Self::ProxyJump => "example: bastion",
            Self::ForwardAgent => "example: yes or no",
            Self::PreferredAuthentications => "example: publickey,password",
            Self::LocalForward => "example: 8080:localhost:80",
            Self::RemoteForward => "example: 9090:localhost:90",
            Self::SetEnv => "example: APP_ENV=prod",
            Self::SendEnv => "example: LANG LC_*",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Alias => 0,
            Self::Description => 1,
            Self::HostName => 2,
            Self::User => 3,
            Self::Port => 4,
            Self::IdentityFile => 5,
            Self::ProxyJump => 6,
            Self::ForwardAgent => 7,
            Self::PreferredAuthentications => 8,
            Self::LocalForward => 9,
            Self::RemoteForward => 10,
            Self::SetEnv => 11,
            Self::SendEnv => 12,
        }
    }

    pub fn is_multivalue(self) -> bool {
        matches!(
            self,
            Self::LocalForward | Self::RemoteForward | Self::SetEnv | Self::SendEnv
        )
    }

    pub fn edit_value(self, host: &SshHost) -> String {
        match self {
            Self::Alias => host.alias.clone(),
            Self::Description => host
                .description
                .as_deref()
                .map(escape_newlines)
                .unwrap_or_default(),
            Self::HostName => host.hostname.clone().unwrap_or_default(),
            Self::User => host.user.clone().unwrap_or_default(),
            Self::Port => host.port.map(|port| port.to_string()).unwrap_or_default(),
            Self::IdentityFile => host.identity_file.clone().unwrap_or_default(),
            Self::ProxyJump => host.proxy_jump.clone().unwrap_or_default(),
            Self::ForwardAgent => host.forward_agent.clone().unwrap_or_default(),
            Self::PreferredAuthentications => {
                host.preferred_authentications.clone().unwrap_or_default()
            }
            Self::LocalForward => host.local_forwards.join("\n"),
            Self::RemoteForward => host.remote_forwards.join("\n"),
            Self::SetEnv => host.set_env.join("\n"),
            Self::SendEnv => host.send_env.join("\n"),
        }
    }

    pub fn apply(self, config: &mut SshConfig, host_index: usize, input: &str) -> Result<()> {
        apply_host_field(config, host_index, self, input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldChange<T> {
    Keep,
    Clear,
    Set(T),
}

impl<T> Default for FieldChange<T> {
    fn default() -> Self {
        Self::Keep
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostPatch {
    pub hostname: FieldChange<String>,
    pub user: FieldChange<String>,
    pub port: FieldChange<u16>,
    pub identity_file: FieldChange<String>,
    pub proxy_jump: FieldChange<String>,
    pub description: FieldChange<String>,
}

pub fn optional_string_flag(flag: Option<String>) -> FieldChange<String> {
    match flag {
        Some(value) if value.trim().is_empty() => FieldChange::Clear,
        Some(value) => FieldChange::Set(value),
        None => FieldChange::Keep,
    }
}

pub fn optional_resolved_string(value: Option<String>) -> FieldChange<String> {
    match value {
        Some(value) => FieldChange::Set(value),
        None => FieldChange::Clear,
    }
}

pub fn add_host(config: &mut SshConfig, mut host: SshHost) -> Result<usize> {
    host.alias = validate_alias(config, None, &host.alias)?;
    config.hosts.push(host);
    Ok(config.hosts.len() - 1)
}

pub fn add_empty_host(config: &mut SshConfig, alias: &str) -> Result<usize> {
    add_host(config, SshHost::new(alias.to_string()))
}

pub fn clone_host(config: &mut SshConfig, source: &str, target: &str) -> Result<usize> {
    let source_host = config
        .find(source)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", source))?;
    let mut new_host = source_host.clone();
    new_host.alias = validate_alias(config, None, target)?;
    config.hosts.push(new_host);
    Ok(config.hosts.len() - 1)
}

pub fn replace_host(config: &mut SshConfig, current_alias: &str, mut host: SshHost) -> Result<()> {
    let index = find_host_index(config, current_alias)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found.", current_alias))?;
    host.alias = validate_alias(config, Some(index), &host.alias)?;
    config.hosts[index] = host;
    Ok(())
}

pub fn delete_host(config: &mut SshConfig, alias: &str) -> Result<SshHost> {
    let index = find_host_index(config, alias)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in SSH config", alias))?;
    Ok(config.hosts.remove(index))
}

pub fn find_host_index(config: &SshConfig, alias: &str) -> Option<usize> {
    config.hosts.iter().position(|host| host.alias == alias)
}

pub fn validate_alias(
    config: &SshConfig,
    current_index: Option<usize>,
    input: &str,
) -> Result<String> {
    let alias = input.trim();
    if alias.is_empty() {
        anyhow::bail!("Alias is required.");
    }

    let duplicate = config
        .hosts
        .iter()
        .enumerate()
        .any(|(index, host)| Some(index) != current_index && host.alias == alias);
    if duplicate {
        anyhow::bail!("Host '{}' already exists.", alias);
    }

    Ok(alias.to_string())
}

pub fn apply_host_patch(preset: &SshHost, alias: Option<String>, patch: HostPatch) -> SshHost {
    let identity_file = apply_string_change(patch.identity_file, preset.identity_file.clone());

    SshHost {
        alias: alias.unwrap_or_else(|| preset.alias.clone()),
        description: apply_string_change(patch.description, preset.description.clone()),
        hostname: apply_string_change(patch.hostname, preset.hostname.clone()),
        user: apply_string_change(patch.user, preset.user.clone()),
        port: apply_port_change(patch.port, preset.port),
        identity_file: identity_file.clone(),
        proxy_jump: apply_string_change(patch.proxy_jump, preset.proxy_jump.clone()),
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
    }
}

pub fn apply_host_field(
    config: &mut SshConfig,
    host_index: usize,
    field: HostField,
    input: &str,
) -> Result<()> {
    match field {
        HostField::Alias => {
            let alias = validate_alias(config, Some(host_index), input)?;
            let host = host_mut(config, host_index)?;
            host.alias = alias;
        }
        HostField::Description => {
            host_mut(config, host_index)?.description =
                optional_string(&input.trim().replace("\\n", "\n"));
        }
        HostField::HostName => {
            host_mut(config, host_index)?.hostname = optional_string(input);
        }
        HostField::User => {
            host_mut(config, host_index)?.user = optional_string(input);
        }
        HostField::Port => {
            host_mut(config, host_index)?.port = parse_port(input)?;
        }
        HostField::IdentityFile => {
            let identity_file = normalize_identity_file_path(input)?;
            let host = host_mut(config, host_index)?;
            host.identity_file = identity_file;
            host.preferred_authentications = preferred_authentications_for(
                &host.identity_file,
                host.preferred_authentications.as_deref(),
            );
        }
        HostField::ProxyJump => {
            host_mut(config, host_index)?.proxy_jump = optional_string(input);
        }
        HostField::ForwardAgent => {
            host_mut(config, host_index)?.forward_agent = parse_forward_agent(input)?;
        }
        HostField::PreferredAuthentications => {
            host_mut(config, host_index)?.preferred_authentications = optional_string(input);
        }
        HostField::LocalForward => {
            host_mut(config, host_index)?.local_forwards = parse_multivalue_lines(input, field)?;
        }
        HostField::RemoteForward => {
            host_mut(config, host_index)?.remote_forwards = parse_multivalue_lines(input, field)?;
        }
        HostField::SetEnv => {
            host_mut(config, host_index)?.set_env = parse_multivalue_lines(input, field)?;
        }
        HostField::SendEnv => {
            host_mut(config, host_index)?.send_env = parse_multivalue_lines(input, field)?;
        }
    }

    Ok(())
}

fn apply_string_change(change: FieldChange<String>, current: Option<String>) -> Option<String> {
    match change {
        FieldChange::Keep => current,
        FieldChange::Clear => None,
        FieldChange::Set(value) => Some(value),
    }
}

fn apply_port_change(change: FieldChange<u16>, current: Option<u16>) -> Option<u16> {
    match change {
        FieldChange::Keep => current,
        FieldChange::Clear => None,
        FieldChange::Set(value) => Some(value),
    }
}

fn parse_multivalue_lines(input: &str, field: HostField) -> Result<Vec<String>> {
    let mut values = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !validate_multivalue_line(field, trimmed) {
            anyhow::bail!("Line {}: {}", index + 1, multivalue_error(field));
        }

        values.push(trimmed.to_string());
    }

    Ok(values)
}

fn validate_multivalue_line(field: HostField, input: &str) -> bool {
    match field {
        HostField::LocalForward | HostField::RemoteForward => validate_forward_format(input),
        HostField::SetEnv => validate_set_env_format(input),
        HostField::SendEnv => validate_send_env_format(input),
        _ => !input.trim().is_empty(),
    }
}

fn multivalue_error(field: HostField) -> &'static str {
    match field {
        HostField::LocalForward | HostField::RemoteForward => {
            "expected local_port:dest_host:dest_port"
        }
        HostField::SetEnv => "expected KEY=value",
        HostField::SendEnv => "expected variable names or patterns like LANG LC_*",
        _ => "expected a non-empty value",
    }
}

fn optional_string(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_port(input: &str) -> Result<Option<u16>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let port = trimmed
        .parse::<u16>()
        .map_err(|_| anyhow::anyhow!("Port must be a number between 1 and 65535."))?;
    if port == 0 {
        anyhow::bail!("Port must be a number between 1 and 65535.");
    }

    Ok(Some(port))
}

fn parse_forward_agent(input: &str) -> Result<Option<String>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "yes" => Ok(Some("yes".to_string())),
        "no" => Ok(Some("no".to_string())),
        _ => anyhow::bail!("ForwardAgent must be yes, no, or blank."),
    }
}

fn escape_newlines(input: &str) -> String {
    input.replace('\n', "\\n")
}

fn host_mut(config: &mut SshConfig, host_index: usize) -> Result<&mut SshHost> {
    config
        .hosts
        .get_mut(host_index)
        .ok_or_else(|| anyhow::anyhow!("No host selected."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_hosts(names: &[&str]) -> SshConfig {
        SshConfig {
            hosts: names
                .iter()
                .map(|name| SshHost {
                    alias: (*name).to_string(),
                    ..Default::default()
                })
                .collect(),
            header_comments: vec![],
        }
    }

    #[test]
    fn validate_alias_rejects_empty_and_duplicates() {
        let config = config_with_hosts(&["demo", "prod"]);

        assert!(validate_alias(&config, Some(0), "").is_err());
        assert!(validate_alias(&config, Some(0), "prod").is_err());
        assert_eq!(validate_alias(&config, Some(0), " demo ").unwrap(), "demo");
    }

    #[test]
    fn add_clone_replace_and_delete_hosts_use_shared_rules() {
        let mut config = config_with_hosts(&["source"]);
        config.hosts[0].hostname = Some("source.example.com".to_string());

        let added_index = add_empty_host(&mut config, " demo ").unwrap();
        assert_eq!(added_index, 1);
        assert_eq!(config.hosts[1].alias, "demo");
        assert!(add_empty_host(&mut config, "demo").is_err());

        let cloned_index = clone_host(&mut config, "source", "copy").unwrap();
        assert_eq!(cloned_index, 2);
        assert_eq!(
            config.find("copy").unwrap().hostname.as_deref(),
            Some("source.example.com")
        );

        replace_host(&mut config, "copy", SshHost::new("renamed".to_string())).unwrap();
        assert!(config.find("copy").is_none());
        assert!(config.find("renamed").is_some());

        let deleted = delete_host(&mut config, "renamed").unwrap();
        assert_eq!(deleted.alias, "renamed");
        assert!(config.find("renamed").is_none());
    }

    #[test]
    fn host_patch_preserves_unmentioned_fields_and_clears_blanks() {
        let preset = SshHost {
            alias: "demo".to_string(),
            hostname: Some("old.example.com".to_string()),
            user: Some("ubuntu".to_string()),
            port: Some(22),
            identity_file: Some("~/.ssh/id_ed25519".to_string()),
            proxy_jump: Some("bastion".to_string()),
            preferred_authentications: None,
            forward_agent: Some("yes".to_string()),
            local_forwards: vec!["8080:localhost:80".to_string()],
            ..Default::default()
        };
        let patch = HostPatch {
            hostname: FieldChange::Set("new.example.com".to_string()),
            user: FieldChange::Clear,
            port: FieldChange::Set(2200),
            proxy_jump: FieldChange::Clear,
            description: FieldChange::Set("new description".to_string()),
            ..Default::default()
        };

        let updated = apply_host_patch(&preset, None, patch);

        assert_eq!(updated.alias, "demo");
        assert_eq!(updated.description.as_deref(), Some("new description"));
        assert_eq!(updated.hostname.as_deref(), Some("new.example.com"));
        assert_eq!(updated.user, None);
        assert_eq!(updated.port, Some(2200));
        assert_eq!(updated.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(updated.proxy_jump, None);
        assert_eq!(updated.forward_agent.as_deref(), Some("yes"));
        assert_eq!(updated.local_forwards, vec!["8080:localhost:80"]);
    }

    #[test]
    fn field_apply_validates_port_and_forward_agent() {
        let mut config = config_with_hosts(&["demo"]);

        assert!(HostField::Port.apply(&mut config, 0, "0").is_err());
        HostField::Port.apply(&mut config, 0, "2200").unwrap();
        assert_eq!(config.hosts[0].port, Some(2200));

        assert!(
            HostField::ForwardAgent
                .apply(&mut config, 0, "maybe")
                .is_err()
        );
        HostField::ForwardAgent
            .apply(&mut config, 0, "YES")
            .unwrap();
        assert_eq!(config.hosts[0].forward_agent.as_deref(), Some("yes"));
    }

    #[test]
    fn description_uses_escaped_newlines_in_editor() {
        let mut config = config_with_hosts(&["demo"]);

        HostField::Description
            .apply(&mut config, 0, "line one\\nline two")
            .unwrap();

        let host = &config.hosts[0];
        assert_eq!(host.description.as_deref(), Some("line one\nline two"));
        assert_eq!(
            HostField::Description.edit_value(host),
            "line one\\nline two"
        );
    }

    #[test]
    fn identity_file_updates_preferred_authentications() {
        let mut config = config_with_hosts(&["demo"]);
        config.hosts[0].preferred_authentications = Some("password".to_string());

        HostField::IdentityFile
            .apply(&mut config, 0, "id_ed25519")
            .unwrap();

        assert_eq!(
            config.hosts[0].identity_file.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        assert_eq!(config.hosts[0].preferred_authentications, None);

        HostField::IdentityFile.apply(&mut config, 0, "").unwrap();

        assert_eq!(config.hosts[0].identity_file, None);
        assert_eq!(
            config.hosts[0].preferred_authentications.as_deref(),
            Some("password")
        );
    }

    #[test]
    fn multi_value_fields_parse_one_entry_per_line() {
        let mut config = config_with_hosts(&["demo"]);

        HostField::SendEnv
            .apply(&mut config, 0, "LANG LC_*\nTERM\n")
            .unwrap();

        assert_eq!(config.hosts[0].send_env, vec!["LANG LC_*", "TERM"]);
        assert!(HostField::SetEnv.apply(&mut config, 0, "APP_ENV").is_err());
    }
}
