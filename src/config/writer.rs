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
        for (k, v) in &host.extra {
            out.push_str(&format!("    {} {}\n", k, v));
        }
        out.push('\n');
    }

    out
}
