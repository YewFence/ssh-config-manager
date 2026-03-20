pub mod parser;
pub mod types;
pub mod writer;

pub use types::{SshConfig, SshHost};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn ssh_config_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home.join(".ssh").join("config"))
}

pub fn load_config(path: &Path) -> Result<SshConfig> {
    if !path.exists() {
        return Ok(SshConfig::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(parser::parse(&content))
}

pub fn save_config(config: &SshConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let content = writer::serialize(config);
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
