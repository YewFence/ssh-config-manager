use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config;

const EXCLUDED_FILES: &[&str] = &[
    "config",
    "known_hosts",
    "known_hosts.old",
    "authorized_keys",
    "allowed_signers",
    "allowedSigners",
    "environment",
    "rc",
];

/// 收集 SSH config 中所有被引用的 identity_file 路径（含配对文件）
fn collect_referenced(config: &config::types::SshConfig) -> Result<HashSet<PathBuf>> {
    let mut referenced = HashSet::new();
    for host in &config.hosts {
        if let Some(ref id_file) = host.identity_file {
            let path = super::expand_tilde(id_file)?;
            referenced.insert(path.clone());

            // 同时标记配对文件（私钥 ↔ .pub）
            let s = path.to_string_lossy();
            if let Some(base) = s.strip_suffix(".pub") {
                referenced.insert(PathBuf::from(base));
            } else {
                referenced.insert(PathBuf::from(format!("{}.pub", s)));
            }
        }
    }
    Ok(referenced)
}

pub fn run(config_path: &Path) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let ssh_dir = home.join(".ssh");

    if !ssh_dir.exists() {
        println!("~/.ssh directory does not exist.");
        return Ok(());
    }

    let config = config::load_config(config_path)?;
    let referenced = collect_referenced(&config)?;

    let mut unreferenced: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(&ssh_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 跳过目录
        if path.is_dir() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // 跳过隐藏文件
        if file_name.starts_with('.') {
            continue;
        }

        // 跳过已知非密钥文件
        if EXCLUDED_FILES.contains(&file_name.as_str()) {
            continue;
        }

        if !referenced.contains(&path) {
            unreferenced.push(path);
        }
    }

    if unreferenced.is_empty() {
        println!("All key files in ~/.ssh/ are referenced by SSH config. Nothing to prune.");
        return Ok(());
    }

    unreferenced.sort();

    println!("Unreferenced files in ~/.ssh/:\n");
    for path in &unreferenced {
        println!("  {}", path.display());
    }
    println!(
        "\n{} file(s) found. These files are not referenced by any Host in SSH config.",
        unreferenced.len()
    );

    Ok(())
}
