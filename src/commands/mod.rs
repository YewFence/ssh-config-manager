pub mod create;
pub mod delete;
pub mod edit;
pub mod ls;
pub mod open;
pub mod prune;

use std::path::PathBuf;

use anyhow::Result;
use inquire::Text;

/// 识别 identity 输入类型并返回最终写入 config 的路径。
///
/// - 公钥内容 → 询问文件名，写入 ~/.ssh/<name>.pub，返回该路径
/// - 纯文件名（无斜杠）→ 打印提示，返回 ~/.ssh/<filename>
/// - 完整路径 → 原样返回
/// - 空 → None
pub fn resolve_identity_file(input: &str, alias: &str) -> Result<Option<String>> {
    if input.is_empty() {
        return Ok(None);
    }

    if is_public_key(input) {
        let default_name = sanitize_filename(alias);
        let name_input = Text::new("Filename for key (leave blank to use alias):")
            .with_default(&default_name)
            .with_help_message("will be saved as ~/.ssh/<name>.pub")
            .prompt()?;
        let key_name = if name_input.is_empty() {
            default_name
        } else {
            name_input
        };

        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        let ssh_dir = home.join(".ssh");
        std::fs::create_dir_all(&ssh_dir)?;
        let key_path = ssh_dir.join(format!("{}.pub", key_name));
        std::fs::write(&key_path, input)?;
        println!("Public key saved to {}", key_path.display());

        return Ok(Some(format!("~/.ssh/{}.pub", key_name)));
    }

    // 纯文件名：不含 / 或 \
    if !input.contains('/') && !input.contains('\\') {
        println!("Using ~/.ssh/{} as the identity file path.", input);
        return Ok(Some(format!("~/.ssh/{}", input)));
    }

    Ok(Some(input.to_string()))
}

fn is_public_key(s: &str) -> bool {
    let prefixes = [
        "ssh-rsa ",
        "ssh-ed25519 ",
        "ssh-dss ",
        "ecdsa-sha2-nistp256 ",
        "ecdsa-sha2-nistp384 ",
        "ecdsa-sha2-nistp521 ",
        "sk-ssh-ed25519 ",
        "sk-ecdsa-sha2-nistp256 ",
    ];
    prefixes.iter().any(|p| s.starts_with(p))
}

/// 展开 `~/` 前缀为实际 home 目录路径
pub fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        Ok(home.join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// hostname 转安全文件名（非字母数字替换为 _）
pub fn sanitize_filename(hostname: &str) -> String {
    hostname
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
