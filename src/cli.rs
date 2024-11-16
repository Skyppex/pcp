use std::num::NonZeroUsize;

use clap::Parser;

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Cli {
    /// The source directory to copy from
    pub source: String,

    /// The destination directory to copy to
    pub destination: String,

    /// Delete files in the destination directory
    /// that are not in the source directory
    #[arg(long, default_value = "false")]
    pub purge: bool,

    /// Overwrite files in the destination directory if they already exist even
    /// if they have the same size
    #[arg(long, default_value = "false")]
    pub overwrite: bool,

    /// Move files instead of copying them
    #[arg(short = 'm', long = "move", default_value = "false")]
    pub move_files: bool,

    /// Limit the number of threads to use
    #[arg(short, long)]
    pub threads: Option<NonZeroUsize>,
}
