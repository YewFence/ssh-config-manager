use super::types::{SshConfig, SshHost};

pub fn parse(content: &str) -> SshConfig {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;
    let mut header: Vec<String> = Vec::new();
    let mut past_first_host = false;
    // 收集紧邻下一个 Host 行前的注释（空行会重置）
    let mut pending_comments: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !past_first_host {
                header.push(line.to_string());
            } else {
                pending_comments.clear();
            }
            continue;
        }

        if trimmed.starts_with('#') {
            if !past_first_host {
                header.push(line.to_string());
            } else {
                pending_comments.push(trimmed[1..].trim().to_string());
            }
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
            current = Some(host);
        } else if let Some(ref mut h) = current {
            h.apply_directive(key, value);
        }
    }

    if let Some(h) = current {
        hosts.push(h);
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
