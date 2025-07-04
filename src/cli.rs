use std::path::PathBuf;
use std::{num::NonZeroUsize, str::FromStr};

use clap::{Error, Parser, ValueEnum};

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Cli {
    /// The source directory to copy from
    ///
    /// > if this is a file, it will be read as though it contains a list of
    /// directories to be copied
    pub source: PathBuf,

    /// The destination directory to copy to
    pub destination: PathBuf,

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

    /// Set the buffer size for file operations
    #[arg(short, long, default_value = "8MiB")]
    pub buf_size: ByteSize,

    /// Display absolute paths
    #[arg(long)]
    pub absolute_paths: bool,

    #[clap(flatten)]
    pub verification: Verification,
}

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Verification {
    /// Verify file contents after copying with a hash
    #[arg(long)]
    pub verify: bool,

    /// Retry files which failed the hash check
    #[arg(long)]
    pub verify_retries: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum OverwriteMode {
    Never,
    SizeDiffers,
    Always,
}

#[derive(Debug, Clone, Parser, PartialEq)]
pub struct ByteSize {
    pub value: usize,
    pub unit: ByteUnit,
}

#[derive(Debug, Clone, Parser, PartialEq)]
pub enum ByteUnit {
    B,
    KB,
    KiB,
    MB,
    MiB,
    GB,
    GiB,
}

impl ByteSize {
    pub fn to_bytes(&self) -> usize {
        match self.unit {
            ByteUnit::B => self.value,
            ByteUnit::KB => self.value * 1000,
            ByteUnit::KiB => self.value * 1024,
            ByteUnit::MB => self.value * 1000 * 1000,
            ByteUnit::MiB => self.value * 1024 * 1024,
            ByteUnit::GB => self.value * 1000 * 1000 * 1000,
            ByteUnit::GiB => self.value * 1024 * 1024 * 1024,
        }
    }

    pub fn is_decimal_unit(&self) -> bool {
        match self.unit {
            ByteUnit::KB | ByteUnit::MB | ByteUnit::GB => true,
            ByteUnit::B | ByteUnit::KiB | ByteUnit::MiB | ByteUnit::GiB => false,
        }
    }

    pub fn is_binary_unit(&self) -> bool {
        !self.is_decimal_unit()
    }
}

impl FromStr for ByteSize {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut string_value = String::new();

        for c in s.chars() {
            if c.is_ascii_digit() {
                string_value.push(c);
            } else {
                break;
            }
        }

        Ok(ByteSize {
            value: string_value
                .parse()
                .map_err(|_| Error::new(clap::error::ErrorKind::ValueValidation))?,
            unit: s[string_value.len()..].parse()?,
        })
    }
}

impl FromStr for ByteUnit {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "B" | "b" => Ok(ByteUnit::B),
            "kb" | "kB" | "KB" => Ok(ByteUnit::KB),
            "kib" | "KiB" => Ok(ByteUnit::KiB),
            "mb" | "mB" | "MB" => Ok(ByteUnit::MB),
            "mib" | "MiB" => Ok(ByteUnit::MiB),
            "gb" | "gB" | "GB" => Ok(ByteUnit::GB),
            "gib" | "GiB" => Ok(ByteUnit::GiB),
            _ => Err(Error::new(clap::error::ErrorKind::ValueValidation)),
        }
    }
}
