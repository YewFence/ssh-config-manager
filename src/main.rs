use anyhow::Result;
use clap::Parser;
use sshm::{
    cli::{Cli, Commands},
    commands, config,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = config::ssh_config_path()?;

    match cli.command {
        Commands::Clone { source, name } => commands::clone::run(&source, name, &config_path),
        Commands::Export { output } => commands::export::run(output, &config_path),
        Commands::Import { archive, yes } => commands::import::run(&archive, yes, &config_path),
        Commands::Ls { show } => commands::ls::run(&config_path, show),
        Commands::Create {
            name,
            hostname,
            user,
            port,
            identity_file,
            proxy_jump,
            description,
        } => commands::create::run(
            name,
            commands::host_builder::HostFlags {
                hostname,
                user,
                port,
                identity_file,
                proxy_jump,
                description,
            },
            &config_path,
        ),
        Commands::Edit {
            name,
            hostname,
            user,
            port,
            identity_file,
            proxy_jump,
            description,
        } => commands::edit::run(
            &name,
            commands::host_builder::HostFlags {
                hostname,
                user,
                port,
                identity_file,
                proxy_jump,
                description,
            },
            &config_path,
        ),
        Commands::Delete { name } => commands::delete::run(&name, &config_path),
        Commands::Prune => commands::prune::run(&config_path),
        Commands::Open { subcommand } => commands::open::run(subcommand),
    }
}
