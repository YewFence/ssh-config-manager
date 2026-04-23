use anyhow::Result;
use std::path::PathBuf;

use sshm::cli::Cli;

fn main() -> Result<()> {
    let markdown = clap_markdown::help_markdown::<Cli>();
    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("CLI_HELP.md");
    std::fs::write(&out_path, markdown)?;
    println!("Generated {}", out_path.display());
    Ok(())
}
