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
    ///
    /// Reads ~/.ssh/config. No files are written or modified.
    Ls {
        /// Show full hostnames (default: masked)
        #[arg(long, short)]
        show: bool,
    },

    /// Clone an existing SSH host
    ///
    /// Reads and writes ~/.ssh/config only. No other files are accessed.
    #[command(alias = "cl")]
    Clone {
        /// Source host alias to clone from
        source: String,

        /// New host alias name (prompted if omitted)
        name: Option<String>,
    },

    /// Create a new SSH host
    ///
    /// Reads and writes ~/.ssh/config. If public key content is pasted as the identity file,
    /// it is also written to ~/.ssh/<name>.pub. No network requests are made.
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

        /// Host description (written as a comment in config)
        #[arg(long, short = 'd')]
        description: Option<String>,
    },

    /// Edit an existing SSH host
    ///
    /// Reads and writes ~/.ssh/config. If public key content is pasted as the identity file,
    /// it is also written to ~/.ssh/<name>.pub. No network requests are made.
    #[command(alias = "e")]
    Edit {
        /// Host alias name to edit
        name: String,

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

        /// Host description (written as a comment in config)
        #[arg(long, short = 'd')]
        description: Option<String>,
    },

    /// Delete an SSH host
    ///
    /// Reads and writes ~/.ssh/config only. Associated key files are not deleted.
    #[command(alias = "d")]
    Delete {
        /// Host alias name to delete
        name: String,
    },

    /// Scan for unused key files in ~/.ssh
    ///
    /// Reads ~/.ssh/config and scans the ~/.ssh/ directory listing.
    /// Read-only — no files are deleted or modified.
    Prune,

    /// Open ~/.ssh directory in system file manager
    ///
    /// Delegates to the system file manager (Explorer / Finder / xdg-open) or falls back to a
    /// subshell. sshm itself does not read any file contents.
    Open {
        #[command(subcommand)]
        subcommand: Option<OpenSubcommand>,
    },
}

#[derive(Subcommand)]
pub enum OpenSubcommand {
    /// Open ~/.ssh/config with default editor
    Config,
}
