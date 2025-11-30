#![allow(clippy::too_many_arguments)]
mod cli;
mod file_operations;
mod path_utils;
mod program;
mod progress;
mod progress_bar;

use clap::Parser;
use cli::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    program::run(cli)?;

    Ok(())
}
