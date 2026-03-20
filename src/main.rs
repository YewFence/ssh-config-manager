mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::ssh_config_path()?;

    match cli.command {
        Commands::Ls => commands::ls::run(&config_path),
        Commands::Create {
            name,
            hostname,
            user,
            port,
            identity_file,
            proxy_jump,
        } => commands::create::run(
            name,
            commands::create::CreateFlags {
                hostname,
                user,
                port,
                identity_file,
                proxy_jump,
            },
            &config_path,
        ),
        Commands::Edit { name } => commands::edit::run(&name, &config_path),
    }
}
