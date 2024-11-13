#![allow(dead_code, unreachable_code)]

mod cli;
mod file_operations;
mod progress;
mod utils;

use clap::Parser;
use cli::Cli;
use file_operations::copy_files_in_parallel;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let source = cli.source;
    let destination = cli.destination;

    // Collect all files to be copied
    let files: Vec<_> = WalkDir::new(&source)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();

    copy_files_in_parallel(&source, &destination, &files);

    Ok(())
}
