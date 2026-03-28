#[derive(Debug, Clone, Default)]
pub struct SshHost {
    pub alias: String,
    pub description: Option<String>,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    /// 保留未识别的指令（如 ForwardAgent、StrictHostKeyChecking 等）
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
        match key.to_lowercase().as_str() {
            "hostname" => self.hostname = Some(value.to_string()),
            "user" => self.user = Some(value.to_string()),
            "port" => self.port = value.parse().ok(),
            "identityfile" => self.identity_file = Some(value.to_string()),
            "proxyjump" => self.proxy_jump = Some(value.to_string()),
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
