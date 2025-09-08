use blake3::Hasher;
use filetime::{set_file_times, FileTime};
use indicatif::MultiProgress;
use rayon::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
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
    let mut src_hasher = Hasher::new();
    let mut src_file = File::open(src)?;
    let metadata = src_file.metadata()?;
    let total_size = metadata.len();

    eprintln!("100");

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

    eprintln!("101");

    let mut destination_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(false)
        .open(destination)?;

    eprintln!("102");
    // Create a progress bar for the file
    let progress_bar = multi_progress.add(create_progress_bar(total_size).unwrap());

    eprintln!("103");
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

    eprintln!("104");
    progress_bar.set_message(format!("{} -> {}", src_str, dest_str));

    let buf_size = cli.buf_size.to_bytes();

    eprintln!("105");
    let mut buffer = vec![0; buf_size];
    let mut bytes_copied = 0;
    // let mut all_bytes = vec![];

    eprintln!("106");
    if cli.verification.verify {
        eprintln!("107");
        while bytes_copied < total_size {
            let bytes_read = src_file.read(&mut buffer)?;

            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];

            // all_bytes.extend(chunk);
            src_hasher.write_all(chunk)?;
            destination_file.write_all(chunk)?;
            bytes_copied += bytes_read as u64;
            progress_bar.inc(bytes_read as u64);
        }

        progress_bar.finish();

        eprintln!("108");
        // let mut dump1 = File::create("/home/skypex/dev/code/pcp/dump1")?;
        // dump1.write_all(&all_bytes)?;

        // NOTE: it looks like just checking the written bytes to the destination file's bytes
        // isn't good enough. if the source file changes then thats not reflected in the
        // src_hasher. maybe i just need to read the entire src again instead of this. but also i
        // might need both. have to think more about it.
        eprintln!("checking hashes");
        let src_hash = src_hasher.finalize();
        destination_file.sync_all()?;
        let mut dest_bytes = vec![];
        destination_file.seek(SeekFrom::Start(0))?;
        destination_file.read_to_end(&mut dest_bytes)?;
        let dest_hash = blake3::hash(&dest_bytes);
        dbg!(&src_hash, &dest_hash);

        if src_hash != dest_hash {
            eprintln!("hashes don't match");
            dbg!("adding {} to retry list", src);

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
            destination_file.write_all(chunk)?;
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

    eprintln!("\n\n0");
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

    eprintln!("1");

    let retries = retries.clone();
    eprintln!("2");
    let retries = retries.lock().expect("Failed to lock retries");
    eprintln!("3");

    if let (0, 0) = (retries.len(), cli.verification.verify_retries) {
        eprintln!("4");
        return;
    }

    retries.par_iter().for_each(|path| {
        eprintln!("5");
        let prefix = source.to_str().expect("Invalid path");

        eprintln!("6");
        if let Ok(relative_path) = path.strip_prefix(prefix) {
            eprintln!("7");
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
            eprintln!("8");
        }
        eprintln!("9");
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

            delete_file(path);
        } else {
            eprintln!("Error: Unable to get relative path");
        }
    });

    eprintln!("1");

    let retries = retries.clone();
    eprintln!("2");
    let retries = retries.lock().expect("Failed to lock retries");
    eprintln!("3");

    dbg!(&retries);

    if let (0, 0) = (retries.len(), cli.verification.verify_retries) {
        eprintln!("4");
        return;
    }

    retries.par_iter().for_each(|path| {
        let prefix = source.to_str().expect("Invalid path");

        if let Ok(relative_path) = path.strip_prefix(prefix) {
            create_dirs_and_copy_file(
                path,
                relative_path,
                destinations.clone(),
                cli,
                &multi_progress,
                Arc::new(Mutex::new(vec![])),
            )
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
