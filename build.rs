use std::path::PathBuf;

// Include the cli module for build script
#[path = "src/cli.rs"]
mod cli;

fn main() {
    let markdown = clap_markdown::help_markdown::<cli::Cli>();

    // Write to a file in the project root
    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("CLI_HELP.md");
    std::fs::write(&out_path, markdown).expect("Failed to write CLI_HELP.md");

    println!(
        "cargo:warning=Generated CLI documentation at {}",
        out_path.display()
    );
}
