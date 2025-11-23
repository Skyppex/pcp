use std::{
    io::{IsTerminal, Read},
    path::PathBuf,
};

use indicatif::MultiProgress;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    ThreadPoolBuilder,
};

use walkdir::WalkDir;

use crate::{
    cli::Cli,
    file_operations::{copy_files_par, delete_file, move_files_par},
    path_utils::get_path,
    progress::{cleanup, CompletionTracker},
};

pub fn run(cli: Cli) -> std::io::Result<()> {
    if cli.buf_size.to_bytes() == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Buffer size cannot be zero",
        ));
    }

    let mut stdin = std::io::stdin();
    let mut input = String::new();

    let has_stdin = !stdin.is_terminal() && stdin.read_to_string(&mut input)? != 0;

    if !has_stdin && cli.destinations.is_empty() {
        eprintln!("You must specify at least 1 destination path");
        std::process::exit(1);
    }

    ThreadPoolBuilder::new()
        .num_threads(cli.threads.map(|t| t.get()).unwrap_or_else(num_cpus::get))
        .build_global()
        .unwrap();

    if !has_stdin {
        let source = get_path(&cli.source.as_ref().ok_or(std::io::ErrorKind::Other)?)?;
        let destinations = cli
            .destinations
            .iter()
            .map(get_path)
            .collect::<Result<Vec<_>, _>>()?;

        if destinations.contains(&source) {
            eprintln!("Source and Destination paths are the same");
            std::process::exit(1);
        }

        let multi_progress = MultiProgress::new();
        multi_progress.set_move_cursor(true);
        handle_multiple_files(cli, source, destinations, &multi_progress)?;
    } else {
        let lines = input.lines();

        let multi_progress = MultiProgress::new();
        multi_progress.set_move_cursor(true);

        lines.par_bridge().try_for_each(|line| {
            if line.trim_start().starts_with('#') {
                return Ok(());
            }

            let (source, destinations) = parse_operation(line)?;
            handle_multiple_files(cli.clone(), source, destinations, &multi_progress)
        })?;
    }

    Ok(())
}

fn parse_operation(line: &str) -> std::io::Result<(PathBuf, Vec<PathBuf>)> {
    let mut split = line.split(':');
    let source = split.next().ok_or(std::io::ErrorKind::Other)?;
    let destinations = split.collect::<Vec<_>>();

    if destinations.is_empty() {
        eprintln!("You must specify at least 1 destination path");
        std::process::exit(1);
    }

    let source = get_path(source.trim())?;
    let destinations = destinations
        .iter()
        .map(|d| get_path(d.trim()))
        .collect::<std::io::Result<Vec<_>>>()?;

    Ok((source, destinations))
}

fn handle_multiple_files(
    cli: Cli,
    source: PathBuf,
    destinations: Vec<PathBuf>,
    multi_progress: &MultiProgress,
) -> std::io::Result<()> {
    let files = WalkDir::new(&source)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect::<Vec<_>>();

    for destination in &destinations {
        let mut tracker = CompletionTracker::open(destination, cli.use_progress)?;
        let completed = tracker.read();

        let files = files
            .clone()
            .into_iter()
            .filter(|e| !completed.contains(e.file_name()))
            .collect();

        if cli.move_files {
            if destinations.len() == 1 && std::fs::rename(&source, destination).is_ok() {
                println!("Renamed {} -> {}", source.display(), destination.display());
                return Ok(());
            }

            move_files_par(&cli, &source, destination, &tracker, &files, multi_progress)?;
        } else {
            copy_files_par(&cli, &source, destination, &tracker, &files, multi_progress)?;
        }

        if cli.purge {
            let dest_files = WalkDir::new(destination)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .collect::<Vec<_>>();

            dest_files.par_iter().for_each(|dest_file| {
                let dest_path = dest_file.path().strip_prefix(destination);

                if !files
                    .iter()
                    .any(|src_file| src_file.path().strip_prefix(&source) == dest_path)
                {
                    let (src_str, dest_str) = if cli.absolute_paths {
                        (source.to_str().unwrap(), dest_file.path().to_str().unwrap())
                    } else {
                        (
                            source
                                .strip_prefix(
                                    std::env::current_dir().expect("Error getting current dir"),
                                )
                                .unwrap_or(dest_file.path())
                                .to_str()
                                .unwrap(),
                            dest_file
                                .path()
                                .strip_prefix(
                                    std::env::current_dir().expect("Error getting current dir"),
                                )
                                .unwrap_or(dest_file.path())
                                .to_str()
                                .unwrap(),
                        )
                    };

                    eprintln!("Deleting: {}. Not found in source: {}", dest_str, src_str);

                    delete_file(dest_file.path())
                }
            });
        }

        tracker.remove()?;

        if cli.use_progress {
            cleanup(destination)?;
        }
    }

    Ok(())
}
