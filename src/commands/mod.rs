pub mod clone;
pub mod create;
pub mod delete;
pub mod edit;
pub mod export;
pub mod host_builder;
pub mod import;
pub mod ls;
pub mod open;
pub mod prune;

use std::fmt;
use std::path::PathBuf;

use anyhow::Result;
use inquire::{Select, Text, validator::Validation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvancedConfigChoice {
    ProxyJump,
    ForwardAgent,
    ForwardRules,
    EnvRules,
    Description,
    Finish,
}

impl fmt::Display for AdvancedConfigChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProxyJump => write!(f, "[p] ProxyJump"),
            Self::ForwardAgent => write!(f, "[a] ForwardAgent"),
            Self::ForwardRules => write!(f, "[f] Forward"),
            Self::EnvRules => write!(f, "[e] Env"),
            Self::Description => write!(f, "[d] Description"),
            Self::Finish => write!(f, "[q] Save and quit"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForwardRuleChoice {
    Local,
    Remote,
    Back,
}

impl fmt::Display for ForwardRuleChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "LocalForward"),
            Self::Remote => write!(f, "RemoteForward"),
            Self::Back => write!(f, "Back"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvRuleChoice {
    Set,
    Send,
    Back,
}

impl fmt::Display for EnvRuleChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set => write!(f, "SetEnv"),
            Self::Send => write!(f, "SendEnv"),
            Self::Back => write!(f, "Back"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectiveListAction {
    Add,
    ReplaceAll,
    ClearAll,
    Back,
}

impl fmt::Display for DirectiveListAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "Add entries"),
            Self::ReplaceAll => write!(f, "Replace all"),
            Self::ClearAll => write!(f, "Clear all"),
            Self::Back => write!(f, "Back"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToggleDirectiveChoice {
    Yes,
    No,
    Unset,
    Back,
}

impl fmt::Display for ToggleDirectiveChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yes => write!(f, "yes"),
            Self::No => write!(f, "no"),
            Self::Unset => write!(f, "unset"),
            Self::Back => write!(f, "back"),
        }
    }
}

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

        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
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

pub fn prompt_advanced_config_choice() -> Result<AdvancedConfigChoice> {
    let options = vec![
        AdvancedConfigChoice::ProxyJump,
        AdvancedConfigChoice::ForwardAgent,
        AdvancedConfigChoice::ForwardRules,
        AdvancedConfigChoice::EnvRules,
        AdvancedConfigChoice::Description,
        AdvancedConfigChoice::Finish,
    ];
    let start = options.len() - 1;

    Ok(Select::new("Advanced config:", options)
        .without_filtering()
        .with_starting_cursor(start)
        .prompt()?)
}

pub fn prompt_forward_rule_choice() -> Result<ForwardRuleChoice> {
    Ok(Select::new(
        "Forward config:",
        vec![
            ForwardRuleChoice::Local,
            ForwardRuleChoice::Remote,
            ForwardRuleChoice::Back,
        ],
    )
    .without_filtering()
    .with_starting_cursor(2)
    .prompt()?)
}

pub fn prompt_env_rule_choice() -> Result<EnvRuleChoice> {
    Ok(Select::new(
        "Env config:",
        vec![EnvRuleChoice::Set, EnvRuleChoice::Send, EnvRuleChoice::Back],
    )
    .without_filtering()
    .with_starting_cursor(2)
    .prompt()?)
}

pub fn prompt_yes_no_directive(label: &str, current: &mut Option<String>) -> Result<()> {
    let options = vec![
        ToggleDirectiveChoice::Yes,
        ToggleDirectiveChoice::No,
        ToggleDirectiveChoice::Unset,
        ToggleDirectiveChoice::Back,
    ];
    let starting_cursor = match current.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("yes") => 0,
        Some(value) if value.eq_ignore_ascii_case("no") => 1,
        None => 2,
        _ => 3,
    };

    let choice = Select::new(label, options)
        .without_filtering()
        .with_starting_cursor(starting_cursor)
        .prompt()?;

    match choice {
        ToggleDirectiveChoice::Yes => *current = Some("yes".to_string()),
        ToggleDirectiveChoice::No => *current = Some("no".to_string()),
        ToggleDirectiveChoice::Unset => *current = None,
        ToggleDirectiveChoice::Back => {}
    }

    Ok(())
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
        .with_help_message(
            "filename → auto-prefix ~/.ssh/ | pubkey content → saved to ~/.ssh/<name>.pub",
        )
        .prompt()?;
    resolve_identity_file(&input, alias)
}

/// 通用的多值 directive 编辑器：支持追加、整体替换、清空。
pub fn prompt_directive_entries(
    kind: &str,
    preset: &[String],
    help: &'static str,
    validator: fn(&str) -> bool,
    validation_error: &'static str,
) -> Result<Vec<String>> {
    let mut rules: Vec<String> = preset.to_vec();

    loop {
        print_existing_entries(kind, &rules);

        let action = prompt_directive_list_action(!rules.is_empty())?;
        match action {
            DirectiveListAction::Add => {
                prompt_directive_input_loop(kind, &mut rules, help, validator, validation_error)?
            }
            DirectiveListAction::ReplaceAll => {
                rules.clear();
                prompt_directive_input_loop(kind, &mut rules, help, validator, validation_error)?;
            }
            DirectiveListAction::ClearAll => {
                rules.clear();
                println!("  Cleared all {} entries.\n", kind);
            }
            DirectiveListAction::Back => break,
        }
    }

    if !rules.is_empty() {
        println!("  {} {} rule(s) configured\n", rules.len(), kind);
    }

    Ok(rules)
}

/// Forward 规则收集：走通用的多值 directive 编辑器。
pub fn prompt_forwards(kind: &str, preset: &[String]) -> Result<Vec<String>> {
    prompt_directive_entries(
        kind,
        preset,
        "format: local_port:dest_host:dest_port (e.g., 8080:localhost:80)",
        validate_forward_format,
        "Expected: local_port:dest_host:dest_port",
    )
}

pub fn prompt_env_values(kind: &str, preset: &[String]) -> Result<Vec<String>> {
    let (help, validator, validation_error) = match kind {
        "SetEnv" => (
            "example: APP_ENV=prod or APP_ENV=prod REGION=hk",
            validate_set_env_format as fn(&str) -> bool,
            "Expected: KEY=value (one or more pairs)",
        ),
        "SendEnv" => (
            "example: LANG LC_*",
            validate_send_env_format as fn(&str) -> bool,
            "Expected: variable names or patterns like LANG LC_*",
        ),
        _ => (
            "leave blank to finish",
            validate_non_empty as fn(&str) -> bool,
            "",
        ),
    };

    prompt_directive_entries(kind, preset, help, validator, validation_error)
}

fn print_existing_entries(kind: &str, entries: &[String]) {
    if entries.is_empty() {
        println!("\nNo {} entries configured yet.\n", kind);
        return;
    }

    println!("\nCurrent {} entries:", kind);
    for (idx, entry) in entries.iter().enumerate() {
        println!("  {} [{}]: {}", kind, idx + 1, entry);
    }
    println!();
}

fn prompt_directive_list_action(has_entries: bool) -> Result<DirectiveListAction> {
    let mut options = vec![DirectiveListAction::Add];
    if has_entries {
        options.push(DirectiveListAction::ReplaceAll);
        options.push(DirectiveListAction::ClearAll);
    }
    options.push(DirectiveListAction::Back);

    let start = options.len() - 1;
    Ok(Select::new("Choose an action:", options)
        .without_filtering()
        .with_starting_cursor(start)
        .prompt()?)
}

fn prompt_directive_input_loop(
    kind: &str,
    rules: &mut Vec<String>,
    help: &'static str,
    validator: fn(&str) -> bool,
    validation_error: &'static str,
) -> Result<()> {
    loop {
        let prompt = format!("{} [{}]:", kind, rules.len() + 1);
        let input = Text::new(&prompt)
            .with_help_message(help)
            .with_validator(move |s: &str| {
                let trimmed = s.trim();
                if trimmed.is_empty() || validator(trimmed) {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(validation_error.into()))
                }
            })
            .prompt()?;

        let trimmed = input.trim();
        if trimmed.is_empty() {
            break;
        }

        rules.push(trimmed.to_string());
    }

    Ok(())
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

fn validate_set_env_format(input: &str) -> bool {
    input.contains('=')
}

fn validate_send_env_format(input: &str) -> bool {
    !input.trim().is_empty() && !input.contains('=')
}

fn validate_non_empty(input: &str) -> bool {
    !input.trim().is_empty()
}
