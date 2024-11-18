use std::num::NonZeroUsize;

use clap::{Parser, ValueEnum};

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

    /// If and when to overwrite existing files
    #[arg(long, value_enum, default_value_t = OverwriteMode::Never)]
    pub overwrite: OverwriteMode,

    /// Move files instead of copying them
    #[arg(short = 'm', long = "move", default_value = "false")]
    pub move_files: bool,

    /// Limit the number of threads to use
    #[arg(short, long)]
    pub threads: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum OverwriteMode {
    Never,
    SizeDiffers,
    Always,
}
