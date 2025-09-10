use filetime::{set_file_times, FileTime};
use indicatif::MultiProgress;
use rayon::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use walkdir::DirEntry;

use crate::cli::Cli;
use crate::progress::create_progress_bar;

pub fn copy_file(
    cli: &Cli,
    src: &Path,
    destination: &Path,
    multi_progress: &MultiProgress,
    retries: Arc<Mutex<Vec<PathBuf>>>,
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

    let mut dest_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(true)
        .open(destination)?;

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
    // let mut all_bytes = vec![];

    if cli.verification.verify {
        while bytes_copied < total_size {
            let bytes_read = src_file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];

            // all_bytes.extend(chunk);
            dest_file.write_all(chunk)?;
            bytes_copied += bytes_read as u64;
            progress_bar.inc(bytes_read as u64);
        }

        progress_bar.finish();

        // let mut dump1 = File::create("/home/skypex/dev/code/pcp/dump1")?;
        // dump1.write_all(&all_bytes)?;

        let mut src_bytes = vec![];
        src_file.seek(SeekFrom::Start(0))?;
        src_file.read_to_end(&mut src_bytes)?;
        let src_hash = blake3::hash(&src_bytes);
        dest_file.sync_all()?;
        let mut dest_bytes = vec![];
        dest_file.seek(SeekFrom::Start(0))?;
        dest_file.read_to_end(&mut dest_bytes)?;
        let dest_hash = blake3::hash(&dest_bytes);

        if src_hash != dest_hash {
            retries
                .lock()
                .expect("Failed to lock retries")
                .push(src.to_path_buf())
        } else {
            set_file_times(
                destination,
                FileTime::from_system_time(metadata.accessed()?),
                FileTime::from_system_time(metadata.modified()?),
            )?;
        }
    } else {
        while bytes_copied < total_size {
            let bytes_read = src_file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];
            dest_file.write_all(chunk)?;
            bytes_copied += bytes_read as u64;
            progress_bar.inc(bytes_read as u64);
        }

        progress_bar.finish();
        set_file_times(
            destination,
            FileTime::from_system_time(metadata.accessed()?),
            FileTime::from_system_time(metadata.modified()?),
        )?;
    }

    Ok(())
}

pub fn copy_file_threaded(
    cli: &Cli,
    src: &Path,
    destinations: Vec<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = {
        let src_file = File::open(src)?;
        src_file.metadata()?
    };

    let total_size = metadata.len();

    for destination in destinations {
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

        // Create a progress bar for the file
        let progress_bar = create_progress_bar(total_size).unwrap();

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

        let num_threads = cli
            .threads
            .unwrap_or_else(|| {
                num_cpus::get()
                    .try_into()
                    .expect("Number of CPUs is too large")
            })
            .get();

        let chunk_size = total_size.div_ceil(num_threads as u64);

        let buf_size = cli.buf_size.to_bytes();

        (0..num_threads).into_par_iter().for_each(|i| {
            let mut src_file = File::open(src).expect("Failed to open source file");
            let mut dest_file =
                File::create(destination).expect("Failed to create destination file");
            dest_file
                .set_len(total_size)
                .expect("Failed to set file length for destination file");

            let offset = i as u64 * chunk_size;

            src_file
                .seek(SeekFrom::Start(offset))
                .expect("Failed to seek");

            dest_file
                .seek(SeekFrom::Start(offset))
                .expect("Failed to seek");

            let mut buffer = vec![0; buf_size];
            let mut bytes_copied = 0;

            while bytes_copied < chunk_size {
                let bytes_read = src_file.read(&mut buffer).expect("Failed to read");

                if bytes_read == 0 {
                    break;
                }

                dest_file
                    .write_all(&buffer[..bytes_read])
                    .expect("Failed to write");

                bytes_copied += bytes_read as u64;
                progress_bar.inc(bytes_read as u64);
            }
        });

        progress_bar.finish();
        set_file_times(
            destination,
            FileTime::from_system_time(metadata.accessed()?),
            FileTime::from_system_time(metadata.modified()?),
        )?;
    }

    Ok(())
}

pub fn copy_files_par(cli: &Cli, source: &Path, destinations: Vec<&Path>, files: &Vec<DirEntry>) {
    let retries = Arc::new(Mutex::new(vec![]));
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                entry.path(),
                relative_path,
                destinations.clone(),
                cli,
                &multi_progress,
                retries.clone(),
            )
        }
    });

    if !cli.verification.verify {
        return;
    }

    let retries = retries.clone();
    let retries = retries.lock().expect("Failed to lock retries");

    match (retries.len(), cli.verification.verify_retries) {
        (0, _) => return,
        (len, 0) if len >= 1 => {
            eprintln!("Hash check failed for the following files:");

            for failed in retries.iter() {
                eprintln!("{}", failed.display());
            }

            exit(1);
        }
        _ => {}
    }

    retries.par_iter().for_each(|path| {
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            let mut cli = cli.clone();
            cli.overwrite = crate::cli::OverwriteMode::Always;

            create_dirs_and_copy_file(
                path,
                relative_path,
                destinations.clone(),
                &cli,
                &multi_progress,
                Arc::new(Mutex::new(vec![])),
            );
        }
    });
}

pub fn move_files_par(cli: &Cli, source: &Path, destinations: Vec<&Path>, files: &Vec<DirEntry>) {
    let retries = Arc::new(Mutex::new(vec![]));
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                entry.path(),
                relative_path,
                destinations.clone(),
                cli,
                &multi_progress,
                retries.clone(),
            );

            if !retries
                .lock()
                .expect("failed to lock retries")
                .contains(&path.to_path_buf())
            {
                delete_file(path);
            }
        } else {
            eprintln!("Error: Unable to get relative path");
        }
    });

    let retries = retries.clone();
    let retries = retries.lock().expect("Failed to lock retries");

    match (retries.len(), cli.verification.verify_retries) {
        (0, _) => return,
        (len, 0) if len >= 1 => {
            eprintln!("Hash check failed for the following files:");

            for failed in retries.iter() {
                eprintln!("{}", failed.display());
            }

            exit(1);
        }
        _ => {}
    }

    retries.par_iter().for_each(|path| {
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            let mut cli = cli.clone();
            cli.overwrite = crate::cli::OverwriteMode::Always;
            let retries = Arc::new(Mutex::new(vec![]));

            create_dirs_and_copy_file(
                path,
                relative_path,
                destinations.clone(),
                &cli,
                &multi_progress,
                retries.clone(),
            );

            if !retries
                .lock()
                .expect("failed to lock retries")
                .contains(&path.to_path_buf())
            {
                delete_file(path);
            }
        }
    });
}

pub fn copy_file_par(cli: &Cli, source: &Path, destinations: Vec<&Path>) {
    create_dir_and_copy_file_par(source, destinations, cli);
}

pub fn move_file_par(cli: &Cli, source: &Path, destinations: Vec<&Path>) {
    create_dir_and_copy_file_par(source, destinations, cli);
    delete_file(source);
}

fn create_dirs_and_copy_file(
    path: &Path,
    relative_path: &Path,
    destinations: Vec<&Path>,
    cli: &Cli,
    multi_progress: &MultiProgress,
    retries: Arc<Mutex<Vec<PathBuf>>>,
) {
    for destination in destinations {
        let destination_path = destination.join(relative_path);

        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        if let Err(e) = copy_file(
            cli,
            path,
            &destination_path,
            multi_progress,
            retries.clone(),
        ) {
            eprintln!("Error copying file: {:?}", e);
        }
    }
}

fn create_dir_and_copy_file_par(path: &Path, destinations: Vec<&Path>, cli: &Cli) {
    for destination in destinations.iter() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).unwrap();
        }
    }

    if let Err(e) = copy_file_threaded(cli, path, destinations) {
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
