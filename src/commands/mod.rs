pub mod clone;
pub mod create;
pub mod delete;
pub mod edit;
pub mod export;
pub mod import;
pub mod ls;
pub mod open;
pub mod prune;

use std::path::PathBuf;

use anyhow::Result;
use inquire::{validator::Validation, Text};

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

// ─────────────────────────────────────────────────────────────────────────────
// 字段级 prompt 函数
// 规则：flag 有值 → 直接返回，跳过交互；flag 无值 → 弹交互，preset 作默认值
// ─────────────────────────────────────────────────────────────────────────────

/// 必填字段：flag 有值直接返回，无值则交互提示，preset 作默认值，空输入报错
pub fn prompt_required(label: &str, flag: Option<String>, preset: &str) -> Result<String> {
    if let Some(v) = flag {
        return Ok(v);
    }

    let default = if preset.is_empty() { "" } else { preset };
    let input = Text::new(label)
        .with_default(default)
        .with_validator(|s: &str| {
            if s.is_empty() {
                Ok(Validation::Invalid("This field is required.".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;
    Ok(input)
}

/// 可选字段：flag 有值直接返回，无值则交互提示，preset 作默认值，空输入返回 None
pub fn prompt_optional(
    label: &str,
    flag: Option<String>,
    preset: &str,
    help: Option<&str>,
) -> Result<Option<String>> {
    if let Some(v) = flag {
        return Ok(Some(v));
    }

    let default = if preset.is_empty() { "" } else { preset };
    let mut text = Text::new(label).with_default(default);
    if let Some(h) = help {
        text = text.with_help_message(h);
    }
    let input = text.prompt()?;
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

/// Port 字段：flag 有值直接返回，无值则交互，preset 作默认值，空输入返回 None
pub fn prompt_port(flag: Option<u16>, preset: Option<u16>) -> Result<Option<u16>> {
    if flag.is_some() {
        return Ok(flag);
    }

    let default_str = preset
        .map(|p| p.to_string())
        .unwrap_or_else(|| "22".to_string());
    let input = Text::new("Port:")
        .with_default(&default_str)
        .with_validator(|s: &str| {
            if s.is_empty() || s.parse::<u16>().is_ok() {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Port must be a number between 1 and 65535.".into(),
                ))
            }
        })
        .prompt()?;
    Ok(input.parse::<u16>().ok())
}

/// IdentityFile 字段：flag 有值直接处理，无值则交互，preset 作为默认值
pub fn prompt_identity(alias: &str, flag: Option<String>, preset: &str) -> Result<Option<String>> {
    if let Some(v) = flag {
        return resolve_identity_file(&v, alias);
    }
    let default = if preset.is_empty() { "" } else { preset };
    let input = Text::new("IdentityFile (path, filename, or paste public key):")
        .with_default(default)
        .with_help_message("filename → auto-prefix ~/.ssh/ | pubkey content → saved to ~/.ssh/<name>.pub")
        .prompt()?;
    resolve_identity_file(&input, alias)
}

/// Forward 规则收集：展示已有规则，允许追加/跳过
pub fn prompt_forwards(kind: &str, preset: &[String]) -> Result<Vec<String>> {
    let mut rules: Vec<String> = preset.to_vec();

    if !preset.is_empty() {
        println!("\nExisting {} rules:", kind);
        for rule in preset {
            println!("  {} [{}]: {}", kind, rules.len(), rule);
        }
        let again = Text::new(&format!("Add another {} rule? (press Enter to keep existing, or enter new rule):", kind))
            .with_help_message("format: local_port:dest_host:dest_port, leave blank to keep current")
            .prompt()?;
        if !again.trim().is_empty() && validate_forward_format(again.trim()) {
            rules.push(again.trim().to_string());
        }
    } else {
        println!("\nAdd {} rules (format: local_port:dest_host:dest_port)", kind);
        println!("Example: 8080:localhost:80  →  forwards local port 8080 to remote localhost:80");
        println!("Leave blank and press Enter to skip.\n");

        loop {
            let prompt = format!("{} [{}]:", kind, rules.len() + 1);
            let input = Text::new(&prompt)
                .with_help_message("format: local_port:dest_host:dest_port (e.g., 8080:localhost:80)")
                .prompt()?;

            if input.trim().is_empty() {
                break;
            }

            if validate_forward_format(input.trim()) {
                rules.push(input.trim().to_string());
            } else {
                println!("  Invalid format. Expected: local_port:dest_host:dest_port");
                println!("  Example: 8080:localhost:80");
            }
        }
    }

    if !rules.is_empty() {
        println!("  {} {} rule(s) configured\n", rules.len(), kind);
    }

    Ok(rules)
}

/// 验证 forward 格式: local_port:dest_host:dest_port
fn validate_forward_format(input: &str) -> bool {
    let parts: Vec<&str> = input.trim().split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    if parts[0].parse::<u16>().is_err() {
        return false;
    }
    if parts[1].is_empty() {
        return false;
    }
    if parts[2].parse::<u16>().is_err() {
        return false;
    }
    true
}
