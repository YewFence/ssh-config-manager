#[derive(Debug, Clone, Default)]
pub struct SshHost {
    pub alias: String,
    pub description: Option<String>,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub preferred_authentications: Option<String>,
    pub forward_agent: Option<String>,
    pub local_forwards: Vec<String>,
    pub remote_forwards: Vec<String>,
    pub set_env: Vec<String>,
    pub send_env: Vec<String>,
    /// 保留未识别的指令（如 StrictHostKeyChecking 等）
    pub extra: Vec<(String, String)>,
}

impl SshHost {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            ..Default::default()
        }
    }

    pub fn apply_directive(&mut self, key: &str, value: &str) {
        match key.to_ascii_lowercase().as_str() {
            "hostname" => self.hostname = Some(value.to_string()),
            "user" => self.user = Some(value.to_string()),
            "port" => self.port = value.parse().ok(),
            "identityfile" => self.identity_file = Some(value.to_string()),
            "proxyjump" => self.proxy_jump = Some(value.to_string()),
            "preferredauthentications" => self.preferred_authentications = Some(value.to_string()),
            "forwardagent" => self.forward_agent = Some(value.to_string()),
            "localforward" => self.local_forwards.push(value.to_string()),
            "remoteforward" => self.remote_forwards.push(value.to_string()),
            "setenv" => self.set_env.push(value.to_string()),
            "sendenv" => self.send_env.push(value.to_string()),
            _ => self.extra.push((key.to_string(), value.to_string())),
        }
    }
}

#[derive(Debug, Default)]
pub struct SshConfig {
    pub hosts: Vec<SshHost>,
    pub header_comments: Vec<String>,
}

impl SshConfig {
    pub fn find(&self, alias: &str) -> Option<&SshHost> {
        self.hosts.iter().find(|h| h.alias == alias)
    }

    pub fn find_mut(&mut self, alias: &str) -> Option<&mut SshHost> {
        self.hosts.iter_mut().find(|h| h.alias == alias)
    }

    pub fn contains(&self, alias: &str) -> bool {
        self.find(alias).is_some()
    }
}
