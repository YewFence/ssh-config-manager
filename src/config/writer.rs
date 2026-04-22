use super::types::SshConfig;

pub fn serialize(config: &SshConfig) -> String {
    let mut out = String::new();

    for line in &config.header_comments {
        out.push_str(line);
        out.push('\n');
    }

    if !config.header_comments.is_empty() && !config.hosts.is_empty() {
        out.push('\n');
    }

    for host in &config.hosts {
        if let Some(ref desc) = host.description {
            for line in desc.lines() {
                out.push_str(&format!("# {}\n", line));
            }
        }
        out.push_str(&format!("Host {}\n", host.alias));
        if let Some(ref v) = host.hostname {
            out.push_str(&format!("    HostName {}\n", v));
        }
        if let Some(ref v) = host.user {
            out.push_str(&format!("    User {}\n", v));
        }
        if let Some(v) = host.port {
            out.push_str(&format!("    Port {}\n", v));
        }
        if let Some(ref v) = host.identity_file {
            out.push_str(&format!("    IdentityFile {}\n", v));
        }
        if let Some(ref v) = host.proxy_jump {
            out.push_str(&format!("    ProxyJump {}\n", v));
        }
        if let Some(ref v) = host.preferred_authentications {
            out.push_str(&format!("    PreferredAuthentications {}\n", v));
        }
        if let Some(ref v) = host.forward_agent {
            out.push_str(&format!("    ForwardAgent {}\n", v));
        }
        for value in &host.local_forwards {
            out.push_str(&format!("    LocalForward {}\n", value));
        }
        for value in &host.remote_forwards {
            out.push_str(&format!("    RemoteForward {}\n", value));
        }
        for value in &host.set_env {
            out.push_str(&format!("    SetEnv {}\n", value));
        }
        for value in &host.send_env {
            out.push_str(&format!("    SendEnv {}\n", value));
        }
        for (k, v) in &host.extra {
            out.push_str(&format!("    {} {}\n", k, v));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::serialize;
    use crate::config::{SshConfig, SshHost};

    #[test]
    fn serialize_writes_structured_directives_before_extra() {
        let config = SshConfig {
            hosts: vec![SshHost {
                alias: "demo".to_string(),
                description: Some("desc".to_string()),
                hostname: Some("example.com".to_string()),
                user: Some("root".to_string()),
                port: Some(2222),
                identity_file: None,
                proxy_jump: Some("bastion".to_string()),
                preferred_authentications: Some("password".to_string()),
                forward_agent: Some("yes".to_string()),
                local_forwards: vec!["8080:localhost:80".to_string()],
                remote_forwards: vec!["9090:localhost:90".to_string()],
                set_env: vec!["APP_ENV=prod".to_string()],
                send_env: vec!["LANG LC_*".to_string()],
                extra: vec![("StrictHostKeyChecking".to_string(), "no".to_string())],
            }],
            header_comments: vec![],
        };

        assert_eq!(
            serialize(&config),
            "\
# desc
Host demo
    HostName example.com
    User root
    Port 2222
    ProxyJump bastion
    PreferredAuthentications password
    ForwardAgent yes
    LocalForward 8080:localhost:80
    RemoteForward 9090:localhost:90
    SetEnv APP_ENV=prod
    SendEnv LANG LC_*
    StrictHostKeyChecking no

"
        );
    }
}
