#![allow(dead_code, unreachable_code)]

mod cli;
mod file_operations;
mod io_utils;
mod path_utils;
mod program;
mod progress;
mod progress_bar;
mod utils;

use clap::Parser;
use cli::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    program::run(cli)?;

    Ok(())
}
