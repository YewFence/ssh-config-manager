use crate::archive;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(output: Option<PathBuf>, config_path: &Path) -> Result<()> {
    let output_path = match output {
        Some(path) => path,
        None => std::env::current_dir()?.join(archive::default_archive_name()),
    };

    let summary = archive::create_archive(config_path, &output_path)?;

    println!("Archive created at {}", summary.output_path.display());
    println!("Included 1 config file and {} public key file(s).", summary.public_keys.len());

    if !summary.public_keys.is_empty() {
        println!("Public keys:");
        for file_name in &summary.public_keys {
            println!("  {}", file_name);
        }
    }

    Ok(())
}
