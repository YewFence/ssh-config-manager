use crate::core::{
    config::SshHost,
    hosts::{EDITABLE_HOST_FIELDS, HostField},
};

pub(super) use crate::core::hosts::HostField as EditableField;

pub(super) const EDITABLE_FIELDS: [EditableField; 13] = EDITABLE_HOST_FIELDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DetailRow {
    pub label: String,
    pub value: String,
    pub field: Option<EditableField>,
}

pub(super) fn detail_rows(host: &SshHost) -> Vec<DetailRow> {
    let mut rows = EDITABLE_FIELDS[..HostField::LocalForward.index()]
        .iter()
        .copied()
        .map(|field| DetailRow {
            label: field.label().to_string(),
            value: display_value(field, host),
            field: Some(field),
        })
        .collect::<Vec<_>>();

    push_list_rows(&mut rows, HostField::LocalForward, &host.local_forwards);
    push_list_rows(&mut rows, HostField::RemoteForward, &host.remote_forwards);
    push_list_rows(&mut rows, HostField::SetEnv, &host.set_env);
    push_list_rows(&mut rows, HostField::SendEnv, &host.send_env);
    rows.push(DetailRow {
        label: "Extra directives".to_string(),
        value: host.extra.len().to_string(),
        field: None,
    });
    rows
}

fn display_value(field: EditableField, host: &SshHost) -> String {
    match field {
        HostField::Alias => host.alias.clone(),
        HostField::Description => host
            .description
            .as_deref()
            .map(escape_newlines)
            .unwrap_or_else(|| "-".to_string()),
        HostField::HostName => optional_display(host.hostname.as_deref()),
        HostField::User => optional_display(host.user.as_deref()),
        HostField::Port => host
            .port
            .map(|port| port.to_string())
            .unwrap_or_else(|| "22".to_string()),
        HostField::IdentityFile => optional_display(host.identity_file.as_deref()),
        HostField::ProxyJump => optional_display(host.proxy_jump.as_deref()),
        HostField::ForwardAgent => optional_display(host.forward_agent.as_deref()),
        HostField::PreferredAuthentications => {
            optional_display(host.preferred_authentications.as_deref())
        }
        HostField::LocalForward => {
            optional_display(host.local_forwards.first().map(String::as_str))
        }
        HostField::RemoteForward => {
            optional_display(host.remote_forwards.first().map(String::as_str))
        }
        HostField::SetEnv => optional_display(host.set_env.first().map(String::as_str)),
        HostField::SendEnv => optional_display(host.send_env.first().map(String::as_str)),
    }
}

fn push_list_rows(rows: &mut Vec<DetailRow>, field: EditableField, values: &[String]) {
    if values.is_empty() {
        rows.push(DetailRow {
            label: field.label().to_string(),
            value: "-".to_string(),
            field: Some(field),
        });
        return;
    }

    for (index, value) in values.iter().enumerate() {
        rows.push(DetailRow {
            label: if index == 0 {
                field.label().to_string()
            } else {
                String::new()
            },
            value: value.clone(),
            field: (index == 0).then_some(field),
        });
    }
}

fn optional_display(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string()
}

fn escape_newlines(input: &str) -> String {
    input.replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_rows_make_multi_value_directives_editable_as_groups() {
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
                .any(|row| row.label == "SendEnv" && row.field == Some(HostField::SendEnv))
        );
    }
}
