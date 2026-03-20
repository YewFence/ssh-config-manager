use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sshm", about = "SSH config manager", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all SSH hosts
    Ls,

    /// Create a new SSH host
    #[command(alias = "c")]
    Create {
        /// Host alias name (prompted if omitted)
        name: Option<String>,

        /// HostName or IP address
        #[arg(long, short = 'H')]
        hostname: Option<String>,

        /// SSH user
        #[arg(long, short)]
        user: Option<String>,

        /// SSH port
        #[arg(long, short)]
        port: Option<u16>,

        /// Path to identity file
        #[arg(long, short = 'i')]
        identity_file: Option<String>,

        /// ProxyJump host
        #[arg(long, short = 'J')]
        proxy_jump: Option<String>,
    },

    /// Edit an existing SSH host
    #[command(alias = "e")]
    Edit {
        /// Host alias name to edit
        name: String,
    },
}
