use std::process::Command;

use anyhow::Result;

use crate::cli::OpenSubcommand;

pub fn run(subcommand: Option<OpenSubcommand>) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    if let Some(OpenSubcommand::Config) = subcommand {
        let config_file = home.join(".ssh").join("config");

        let opened = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/c", "start", "", &config_file.to_string_lossy()])
                .spawn()
        } else if cfg!(target_os = "macos") {
            Command::new("open").args(["-t", &config_file.to_string_lossy()]).spawn()
        } else if let Ok(visual) = std::env::var("VISUAL") {
            Command::new(&visual).arg(&config_file).spawn()
        } else if let Ok(editor) = std::env::var("EDITOR") {
            Command::new(&editor).arg(&config_file).spawn()
        } else {
            Command::new("xdg-open").arg(&config_file).spawn()
        };

        match opened {
            Ok(_) => println!("Opened {}", config_file.display()),
            Err(e) => return Err(anyhow::anyhow!("Failed to open editor: {}", e)),
        }
        return Ok(());
    }

    let ssh_dir = home.join(".ssh");

    let result = if cfg!(target_os = "windows") {
        Command::new("explorer").arg(&ssh_dir).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(&ssh_dir).spawn()
    } else {
        Command::new("xdg-open").arg(&ssh_dir).spawn()
    };

    match result {
        Ok(_) => {
            println!("Opened {}", ssh_dir.display());
        }
        Err(_) => {
            // 无图形环境，启动子 shell 进入 ~/.ssh 目录
            println!("Entering {} (exit to return)", ssh_dir.display());
            let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    "pwsh".to_string()
                } else {
                    "/bin/sh".to_string()
                }
            });
            Command::new(&shell)
                .current_dir(&ssh_dir)
                .status()?;
        }
    }

    Ok(())
}
