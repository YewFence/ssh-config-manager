use anyhow::Result;

use crate::{
    commands::{host_builder::preferred_authentications_for, normalize_identity_file_path},
    config::{SshConfig, SshHost},
};

pub(super) const EDITABLE_FIELDS: [EditableField; 9] = [
    EditableField::Alias,
    EditableField::Description,
    EditableField::HostName,
    EditableField::User,
    EditableField::Port,
    EditableField::IdentityFile,
    EditableField::ProxyJump,
    EditableField::ForwardAgent,
    EditableField::PreferredAuthentications,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditableField {
    Alias,
    Description,
    HostName,
    User,
    Port,
    IdentityFile,
    ProxyJump,
    ForwardAgent,
    PreferredAuthentications,
}

impl EditableField {
    pub(super) fn label(self) -> &'static str {
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
        }
    }

    pub(super) fn example(self) -> &'static str {
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
        }
    }

    pub(super) fn index(self) -> usize {
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
        }
    }

    pub(super) fn edit_value(self, host: &SshHost) -> String {
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
        }
    }

    pub(super) fn apply(
        self,
        config: &mut SshConfig,
        host_index: usize,
        input: &str,
    ) -> Result<()> {
        match self {
            Self::Alias => {
                let alias = validate_alias(config, Some(host_index), input)?;
                let host = host_mut(config, host_index)?;
                host.alias = alias;
            }
            Self::Description => {
                host_mut(config, host_index)?.description =
                    optional_string(&input.trim().replace("\\n", "\n"));
            }
            Self::HostName => {
                host_mut(config, host_index)?.hostname = optional_string(input);
            }
            Self::User => {
                host_mut(config, host_index)?.user = optional_string(input);
            }
            Self::Port => {
                host_mut(config, host_index)?.port = parse_port(input)?;
            }
            Self::IdentityFile => {
                let identity_file = normalize_identity_file_path(input)?;
                let host = host_mut(config, host_index)?;
                host.identity_file = identity_file;
                host.preferred_authentications = preferred_authentications_for(
                    &host.identity_file,
                    host.preferred_authentications.as_deref(),
                );
            }
            Self::ProxyJump => {
                host_mut(config, host_index)?.proxy_jump = optional_string(input);
            }
            Self::ForwardAgent => {
                host_mut(config, host_index)?.forward_agent = parse_forward_agent(input)?;
            }
            Self::PreferredAuthentications => {
                host_mut(config, host_index)?.preferred_authentications = optional_string(input);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DetailRow {
    pub label: String,
    pub value: String,
    pub field: Option<EditableField>,
}

pub(super) fn detail_rows(host: &SshHost) -> Vec<DetailRow> {
    let mut rows = EDITABLE_FIELDS
        .iter()
        .copied()
        .map(|field| DetailRow {
            label: field.label().to_string(),
            value: display_value(field, host),
            field: Some(field),
        })
        .collect::<Vec<_>>();

    push_list_rows(&mut rows, "LocalForward", &host.local_forwards);
    push_list_rows(&mut rows, "RemoteForward", &host.remote_forwards);
    push_list_rows(&mut rows, "SetEnv", &host.set_env);
    push_list_rows(&mut rows, "SendEnv", &host.send_env);
    rows.push(DetailRow {
        label: "Extra directives".to_string(),
        value: host.extra.len().to_string(),
        field: None,
    });
    rows
}

pub(super) fn validate_alias(
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

fn display_value(field: EditableField, host: &SshHost) -> String {
    match field {
        EditableField::Alias => host.alias.clone(),
        EditableField::Description => host
            .description
            .as_deref()
            .map(escape_newlines)
            .unwrap_or_else(|| "-".to_string()),
        EditableField::HostName => optional_display(host.hostname.as_deref()),
        EditableField::User => optional_display(host.user.as_deref()),
        EditableField::Port => host
            .port
            .map(|port| port.to_string())
            .unwrap_or_else(|| "22".to_string()),
        EditableField::IdentityFile => optional_display(host.identity_file.as_deref()),
        EditableField::ProxyJump => optional_display(host.proxy_jump.as_deref()),
        EditableField::ForwardAgent => optional_display(host.forward_agent.as_deref()),
        EditableField::PreferredAuthentications => {
            optional_display(host.preferred_authentications.as_deref())
        }
    }
}

fn push_list_rows(rows: &mut Vec<DetailRow>, label: &str, values: &[String]) {
    if values.is_empty() {
        rows.push(DetailRow {
            label: label.to_string(),
            value: "-".to_string(),
            field: None,
        });
        return;
    }

    for (index, value) in values.iter().enumerate() {
        rows.push(DetailRow {
            label: if index == 0 {
                label.to_string()
            } else {
                String::new()
            },
            value: value.clone(),
            field: None,
        });
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

fn optional_display(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string()
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
    fn alias_validation_rejects_empty_and_duplicates() {
        let config = config_with_hosts(&["demo", "prod"]);

        assert!(validate_alias(&config, Some(0), "").is_err());
        assert!(validate_alias(&config, Some(0), "prod").is_err());
        assert_eq!(validate_alias(&config, Some(0), " demo ").unwrap(), "demo");
    }

    #[test]
    fn field_apply_validates_port_and_forward_agent() {
        let mut config = config_with_hosts(&["demo"]);

        assert!(EditableField::Port.apply(&mut config, 0, "0").is_err());
        EditableField::Port.apply(&mut config, 0, "2200").unwrap();
        assert_eq!(config.hosts[0].port, Some(2200));

        assert!(
            EditableField::ForwardAgent
                .apply(&mut config, 0, "maybe")
                .is_err()
        );
        EditableField::ForwardAgent
            .apply(&mut config, 0, "YES")
            .unwrap();
        assert_eq!(config.hosts[0].forward_agent.as_deref(), Some("yes"));
    }

    #[test]
    fn description_uses_escaped_newlines_in_editor() {
        let mut config = config_with_hosts(&["demo"]);

        EditableField::Description
            .apply(&mut config, 0, "line one\\nline two")
            .unwrap();

        let host = &config.hosts[0];
        assert_eq!(host.description.as_deref(), Some("line one\nline two"));
        assert_eq!(
            EditableField::Description.edit_value(host),
            "line one\\nline two"
        );
    }

    #[test]
    fn identity_file_updates_preferred_authentications() {
        let mut config = config_with_hosts(&["demo"]);
        config.hosts[0].preferred_authentications = Some("password".to_string());

        EditableField::IdentityFile
            .apply(&mut config, 0, "id_ed25519")
            .unwrap();

        assert_eq!(
            config.hosts[0].identity_file.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        assert_eq!(config.hosts[0].preferred_authentications, None);

        EditableField::IdentityFile
            .apply(&mut config, 0, "")
            .unwrap();

        assert_eq!(config.hosts[0].identity_file, None);
        assert_eq!(
            config.hosts[0].preferred_authentications.as_deref(),
            Some("password")
        );
    }

    #[test]
    fn detail_rows_keep_multi_value_directives_read_only() {
        let host = SshHost {
            alias: "demo".to_string(),
            local_forwards: vec!["8080:localhost:80".to_string()],
            send_env: vec!["LANG LC_*".to_string()],
            ..Default::default()
        };

        let rows = detail_rows(&host);

        assert_eq!(
            rows.iter().filter(|row| row.field.is_some()).count(),
            EDITABLE_FIELDS.len()
        );
        assert!(
            rows.iter()
                .any(|row| row.label == "SendEnv" && row.field.is_none())
        );
    }
}
