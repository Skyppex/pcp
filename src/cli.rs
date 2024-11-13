use clap::Parser;

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Cli {
    /// The source directory to copy from
    pub source: String,

    /// The destination directory to copy to
    pub destination: String,
}
