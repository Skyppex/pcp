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
use crate::progress::CompletionTracker;
use crate::progress_bar::{create_progress_bar, create_verify_bar};

pub fn copy_file(
    cli: &Cli,
    src: &Path,
    destination: &Path,
    multi_progress: &MultiProgress,
    completed_tracker: &CompletionTracker,
    retries: Arc<Mutex<Vec<PathBuf>>>,
) -> std::io::Result<()> {
    let mut src_file = File::open(src)?;
    let metadata = src_file.metadata()?;
    let total_size = metadata.len();

    match (&cli.overwrite, &cli.use_progress) {
        (_, true) => {
            // Proceed with writing the file
        }
        (crate::cli::OverwriteMode::Never, _) => {
            if destination.exists() {
                return Ok(());
            }
        }
        (crate::cli::OverwriteMode::SizeDiffers, _) => {
            if destination.exists() {
                let dest_size = destination.metadata()?.len();

                if dest_size == total_size {
                    return Ok(());
                }
            }
        }
        (crate::cli::OverwriteMode::Always, _) => {
            // Proceed with writing the file
        }
    }

    let mut dest_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(!cli.use_progress)
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

    copy_chunks(
        destination,
        &mut src_file,
        &metadata,
        total_size,
        &mut dest_file,
        &progress_bar,
        buf_size,
        completed_tracker,
    )?;

    if !cli.verification.verify
        || verify(
            src,
            multi_progress,
            retries,
            &mut src_file,
            total_size,
            &mut dest_file,
            src_str,
            dest_str,
            buf_size,
        )?
    {
        completed_tracker.add_completed(destination)?;
    }

    Ok(())
}

fn copy_chunks(
    destination: &Path,
    src_file: &mut File,
    metadata: &fs::Metadata,
    total_size: u64,
    dest_file: &mut File,
    progress_bar: &indicatif::ProgressBar,
    buf_size: usize,
    completed_tracker: &CompletionTracker,
) -> std::io::Result<()> {
    let mut buffer = vec![0; buf_size];
    let mut bytes_copied = 0;

    let file_name = destination
        .file_name()
        .expect("Destination should have a file name");

    let progress = completed_tracker.add_progress_file(file_name, total_size)?;

    if let Some(progress) = progress {
        src_file.seek(SeekFrom::Start(progress.current))?;
        dest_file.seek(SeekFrom::Start(progress.current))?;
        progress_bar.set_position(progress.current);
        bytes_copied = progress.current;

        // TODO: handle this case more gracefully
        // this now just crashes the program and halts all ongoing copying
        assert_eq!(dest_file.stream_position().unwrap(), progress.current);
        assert_eq!(src_file.stream_position().unwrap(), progress.current);
        assert_eq!(metadata.len(), progress.total);
    }

    while bytes_copied < total_size {
        let bytes_read = src_file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        let chunk = &buffer[..bytes_read];

        dest_file.write_all(chunk)?;
        bytes_copied += bytes_read as u64;
        completed_tracker.write_progress(file_name, bytes_copied)?;
        progress_bar.set_position(bytes_copied);
    }

    progress_bar.finish();

    set_file_times(
        destination,
        FileTime::from_system_time(metadata.accessed()?),
        FileTime::from_system_time(metadata.modified()?),
    )?;

    completed_tracker.remove_progress_file(file_name)?;

    Ok(())
}

fn verify(
    src: &Path,
    multi_progress: &MultiProgress,
    retries: Arc<Mutex<Vec<PathBuf>>>,
    src_file: &mut File,
    total_size: u64,
    dest_file: &mut File,
    src_str: &str,
    dest_str: &str,
    buf_size: usize,
) -> std::io::Result<bool> {
    let verify_bar = multi_progress.add(create_verify_bar(total_size).unwrap());
    verify_bar.set_message(format!("{} -> {}", src_str, dest_str));

    let mut bytes_verified = 0;
    let mut src_hash_buf = vec![0; buf_size];
    let mut dest_hash_buf = vec![0; buf_size];

    src_file.seek(SeekFrom::Start(0))?;
    dest_file.seek(SeekFrom::Start(0))?;

    let mut different = false;

    while bytes_verified < total_size {
        let src_bytes_read = src_file.read(&mut src_hash_buf)?;

        if src_bytes_read == 0 {
            let dest_bytes_read = dest_file.read(&mut dest_hash_buf)?;

            if dest_bytes_read != 0 {
                // destination is longer than source
                different = true;
            }

            break;
        }

        let dest_bytes_read = dest_file.read(&mut dest_hash_buf)?;

        if src_bytes_read != dest_bytes_read {
            // destination is shorter than source
            different = true;
            break;
        }

        let src_chunk = &src_hash_buf[..src_bytes_read];
        let dest_chunk = &dest_hash_buf[..src_bytes_read];

        // efficient comparison since it usually uses memcmp under the hood
        if src_chunk != dest_chunk {
            different = true;
            break;
        }

        bytes_verified += src_bytes_read as u64;
        verify_bar.inc(src_bytes_read as u64);
    }

    if different {
        eprintln!("  Verification failed for {}", dest_str);

        retries
            .lock()
            .expect("Failed to lock retries")
            .push(src.to_path_buf())
    }

    verify_bar.finish();
    Ok(!different)
}

pub fn copy_files_par(
    cli: &Cli,
    source: &Path,
    destination: &Path,
    completion_tracker: &CompletionTracker,
    files: &Vec<DirEntry>,
) -> std::io::Result<()> {
    let retries = Arc::new(Mutex::new(vec![]));
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().try_for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                entry.path(),
                relative_path,
                destination,
                cli,
                &multi_progress,
                completion_tracker,
                retries.clone(),
            )?;
        }

        Result::<_, std::io::Error>::Ok(())
    })?;

    if !cli.verification.verify {
        return Ok(());
    }

    let retries = retries.clone();
    let retries = retries.lock().expect("Failed to lock retries");

    match (retries.len(), cli.verification.verify_retries) {
        (0, _) => return Ok(()),
        (len, 0) if len >= 1 => {
            eprintln!("Verification failed for the following files:");

            for failed in retries.iter() {
                eprintln!("{}", failed.display());
            }

            exit(1);
        }
        _ => {}
    }

    retries.par_iter().try_for_each(|path| {
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            let mut cli = cli.clone();
            cli.overwrite = crate::cli::OverwriteMode::Always;

            create_dirs_and_copy_file(
                path,
                relative_path,
                destination,
                &cli,
                &multi_progress,
                completion_tracker,
                Arc::new(Mutex::new(vec![])),
            )?;
        }

        Result::<_, std::io::Error>::Ok(())
    })?;

    Ok(())
}

pub fn move_files_par(
    cli: &Cli,
    source: &Path,
    destination: &Path,
    completion_tracker: &CompletionTracker,
    try_rename: bool,
    files: &Vec<DirEntry>,
) -> std::io::Result<()> {
    let retries = Arc::new(Mutex::new(vec![]));
    let multi_progress = MultiProgress::new();
    multi_progress.set_move_cursor(true);

    files.par_iter().try_for_each(|entry| {
        let path = entry.path();
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            if try_rename {
                let dest = destination.join(relative_path);

                if std::fs::rename(path, &dest).is_ok() {
                    println!("Renamed {} -> {}", path.display(), dest.display());
                    return Ok(());
                }
            }

            create_dirs_and_copy_file(
                path,
                relative_path,
                destination,
                cli,
                &multi_progress,
                completion_tracker,
                retries.clone(),
            )?;

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

        Result::<_, std::io::Error>::Ok(())
    })?;

    let retries = retries.clone();
    let retries = retries.lock().expect("Failed to lock retries");

    match (retries.len(), cli.verification.verify_retries) {
        (0, _) => return Ok(()),
        (len, 0) if len >= 1 => {
            eprintln!("Verification failed for the following files:");

            for failed in retries.iter() {
                eprintln!("{}", failed.display());
            }

            exit(1);
        }
        _ => {}
    }

    retries.par_iter().try_for_each(|path| {
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            let mut cli = cli.clone();
            cli.overwrite = crate::cli::OverwriteMode::Always;
            let retries = Arc::new(Mutex::new(vec![]));

            create_dirs_and_copy_file(
                path,
                relative_path,
                destination,
                &cli,
                &multi_progress,
                completion_tracker,
                retries.clone(),
            )?;

            if !retries
                .lock()
                .expect("failed to lock retries")
                .contains(&path.to_path_buf())
            {
                delete_file(path);
            }
        }

        Result::<_, std::io::Error>::Ok(())
    })?;

    Ok(())
}

fn create_dirs_and_copy_file(
    path: &Path,
    relative_path: &Path,
    destination: &Path,
    cli: &Cli,
    multi_progress: &MultiProgress,
    completion_tracker: &CompletionTracker,
    retries: Arc<Mutex<Vec<PathBuf>>>,
) -> std::io::Result<()> {
    let destination_path = destination.join(relative_path);

    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    if let Err(e) = copy_file(
        cli,
        path,
        &destination_path,
        multi_progress,
        completion_tracker,
        retries.clone(),
    ) {
        eprintln!("Error copying file: {:?}", e);
    }

    Ok(())
}

pub fn delete_file(path: &Path) {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            eprintln!("Error deleting file: {:?}", e);
        }
    }
}
