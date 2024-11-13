use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use walkdir::DirEntry;

use crate::progress::create_progress_bar;

pub fn copy_file(src: &str, destination: &str) -> io::Result<()> {
    let mut src_file = File::open(src)?;
    let mut destination_file = File::create(destination)?;

    let metadata = src_file.metadata()?;
    let total_size = metadata.len();

    // Create a progress bar for the file
    let progress_bar = create_progress_bar(total_size).unwrap();

    let mut buffer = [0; 8192];
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

    Ok(())
}

pub fn copy_files_in_parallel(source: &str, destination: &str, files: &Vec<DirEntry>) {
    files.par_iter().for_each(|entry| {
        let path = entry.path();
        if let Ok(relative_path) = path.strip_prefix(source) {
            let destination_path = Path::new(destination).join(relative_path);

            // Create parent directories if they don't exist
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            if let Err(e) = copy_file(path.to_str().unwrap(), destination_path.to_str().unwrap()) {
                eprintln!("Error copying file: {:?}", e);
            }
        }
    });
}
