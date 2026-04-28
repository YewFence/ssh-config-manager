use super::types::{SshConfig, SshHost};

pub fn parse(content: &str) -> SshConfig {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;
    let mut header: Vec<String> = Vec::new();
    let mut past_first_host = false;
    // 收集紧邻下一个 Host 行前的注释（空行会重置）
    let mut pending_comments: Vec<String> = Vec::new();
    let mut pending_comment_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !past_first_host {
                header.append(&mut pending_comment_lines);
                header.push(line.to_string());
            } else {
                pending_comment_lines.clear();
            }
            pending_comments.clear();
            continue;
        }

        if trimmed.starts_with('#') {
            pending_comment_lines.push(line.to_string());
            pending_comments.push(trimmed[1..].trim().to_string());
            continue;
        }

        let (key, value) = split_kv(trimmed);

        if key.eq_ignore_ascii_case("host") {
            past_first_host = true;
            if let Some(h) = current.take() {
                hosts.push(h);
            }
            let mut host = SshHost::new(value.to_string());
            if !pending_comments.is_empty() {
                host.description = Some(pending_comments.join("\n"));
            }
            pending_comments.clear();
            pending_comment_lines.clear();
            current = Some(host);
        } else if let Some(ref mut h) = current {
            h.apply_directive(key, value);
            pending_comments.clear();
            pending_comment_lines.clear();
        }
    }

    if let Some(h) = current {
        hosts.push(h);
    }

    if !past_first_host {
        header.append(&mut pending_comment_lines);
        pending_comments.clear();
    }

    SshConfig {
        hosts,
        header_comments: header,
    }
}

fn split_kv(s: &str) -> (&str, &str) {
    let idx = s.find(char::is_whitespace).unwrap_or(s.len());
    let key = &s[..idx];
    let value = s[idx..].trim();
    (key, value)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parse_maps_known_advanced_directives_to_structured_fields() {
        let config = parse(
            "\
Host demo
    HostName example.com
    PreferredAuthentications password
    ForwardAgent yes
    LocalForward 8080:localhost:80
    RemoteForward 9090:localhost:90
    SetEnv APP_ENV=prod
    SendEnv LANG LC_*
    StrictHostKeyChecking no
",
        );

        let host = &config.hosts[0];
        assert_eq!(host.preferred_authentications.as_deref(), Some("password"));
        assert_eq!(host.forward_agent.as_deref(), Some("yes"));
        assert_eq!(host.local_forwards, vec!["8080:localhost:80"]);
        assert_eq!(host.remote_forwards, vec!["9090:localhost:90"]);
        assert_eq!(host.set_env, vec!["APP_ENV=prod"]);
        assert_eq!(host.send_env, vec!["LANG LC_*"]);
        assert_eq!(
            host.extra,
            vec![("StrictHostKeyChecking".to_string(), "no".to_string())]
        );
    }

    #[test]
    fn parse_keeps_header_comments_separate_from_host_description() {
        let config = parse(
            "\
# Managed by hand

# Demo host
# Uses bastion
Host demo
    HostName example.com
",
        );

        assert_eq!(
            config.header_comments,
            vec!["# Managed by hand".to_string(), "".to_string()]
        );
        assert_eq!(
            config.hosts[0].description.as_deref(),
            Some("Demo host\nUses bastion")
        );
    }

    #[test]
    fn parse_preserves_comment_only_config_as_header_comments() {
        let config = parse(
            "\
# Managed by hand
# Keep this file
",
        );

        assert!(config.hosts.is_empty());
        assert_eq!(
            config.header_comments,
            vec![
                "# Managed by hand".to_string(),
                "# Keep this file".to_string()
            ]
        );
    }

    #[test]
    fn parse_does_not_leak_in_host_comments_into_next_description() {
        let config = parse(
            "\
Host first
    HostName first.example.com
    # Applies to the first host directive below
    User root
Host second
    HostName second.example.com
",
        );

        assert_eq!(config.hosts.len(), 2);
        assert_eq!(config.hosts[1].description, None);
    }
}
