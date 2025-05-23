use filetime::{set_file_times, FileTime};
use indicatif::MultiProgress;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use walkdir::DirEntry;

use crate::cli::Cli;
use crate::progress::create_progress_bar;

pub fn copy_file(
    cli: &Cli,
    src: &Path,
    destination: &Path,
    multi_progress: &MultiProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut src_file = File::open(src)?;
    let metadata = src_file.metadata()?;
    let total_size = metadata.len();

    match cli.overwrite {
        crate::cli::OverwriteMode::Never => {
            if destination.exists() {
                return Ok(());
            }
        }
        crate::cli::OverwriteMode::SizeDiffers => {
            if destination.exists() {
                let dest_size = destination.metadata()?.len();

                if dest_size == total_size {
                    return Ok(());
                }
            }
        }
        crate::cli::OverwriteMode::Always => {
            // Proceed with writing the file
        }
    }

    let mut destination_file = File::create(destination)?;

    // Create a progress bar for the file
    let progress_bar = multi_progress.add(create_progress_bar(total_size).unwrap());

    let (src_str, dest_str) = if cli.absolute_paths {
        (src.to_str().unwrap(), destination.to_str().unwrap())
    } else {
        (
            src.strip_prefix(std::env::current_dir().expect("Error getting current dir"))
                .unwrap_or(src)
                .to_str()
                .unwrap(),
            destination
                .strip_prefix(std::env::current_dir().expect("Error getting current dir"))
                .unwrap_or(src)
                .to_str()
                .unwrap(),
        )
    };

    progress_bar.set_message(format!("{} -> {}", src_str, dest_str));

    let buf_size = cli.buf_size.to_bytes();

    let mut buffer = vec![0; buf_size];
    let mut bytes_copied = 0;

    while bytes_copied < total_size {
        let bytes_read = src_file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        destination_file.write_all(&buffer[..bytes_read])?;
        bytes_copied += bytes_read as u64;
        progress_bar.inc(bytes_read as u64);
    }

    progress_bar.finish();
    set_file_times(
        destination,
        FileTime::from_system_time(metadata.accessed()?),
        FileTime::from_system_time(metadata.modified()?),
    )?;

    Ok(())
}

pub fn copy_files_in_parallel(cli: &Cli, source: &Path, destination: &Path, files: &Vec<DirEntry>) {
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                entry.path(),
                relative_path,
                destination,
                cli,
                &multi_progress,
            )
        }
    });
}

pub fn move_files_in_parallel(cli: &Cli, source: &Path, destination: &Path, files: &Vec<DirEntry>) {
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                entry.path(),
                relative_path,
                destination,
                cli,
                &multi_progress,
            );

            delete_file(path);
        }
    });
}

fn create_dirs_and_copy_file(
    path: &Path,
    relative_path: &Path,
    destination: &Path,
    cli: &Cli,
    multi_progress: &MultiProgress,
) {
    let destination_path = destination.join(relative_path);

    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    if let Err(e) = copy_file(cli, path, &destination_path, multi_progress) {
        eprintln!("Error copying file: {:?}", e);
    }
}

pub fn delete_file(path: &Path) {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            eprintln!("Error deleting file: {:?}", e);
        }
    }
}
