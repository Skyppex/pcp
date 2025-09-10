use std::path::PathBuf;

use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use walkdir::WalkDir;

use crate::{
    cli::Cli,
    file_operations::{copy_files_par, delete_file, move_files_par},
    path_utils::get_path,
};

pub fn run(cli: Cli) -> std::io::Result<()> {
    if cli.buf_size.to_bytes() == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Buffer size cannot be zero",
        ));
    }

    if cli.destinations.is_empty() {
        eprintln!("You must specify at least 1 destination path");
        std::process::exit(1);
        return Ok(());
    }

    ThreadPoolBuilder::new()
        .num_threads(cli.threads.map(|t| t.get()).unwrap_or_else(num_cpus::get))
        .build_global()
        .unwrap();

    let source = get_path(&cli.source)?;
    let destinations = cli
        .destinations
        .iter()
        .map(get_path)
        .collect::<Result<Vec<_>, _>>()?;

    if destinations.contains(&source) {
        eprintln!("Source and Destination paths are the same");
        std::process::exit(1);
        return Ok(());
    }

    handle_multiple_files(cli, source, destinations)?;

    Ok(())
}

fn handle_multiple_files(
    cli: Cli,
    source: PathBuf,
    destinations: Vec<PathBuf>,
) -> std::io::Result<()> {
    let files = WalkDir::new(&source)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect::<Vec<_>>();

    if cli.move_files {
        move_files_par(
            &cli,
            &source,
            destinations.iter().map(|d| d.as_path()).collect(),
            &files,
        )?;
    } else {
        copy_files_par(
            &cli,
            &source,
            destinations.iter().map(|d| d.as_path()).collect(),
            &files,
        )?;
    }

    if cli.purge {
        for destination in destinations {
            let dest_files = WalkDir::new(&destination)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .collect::<Vec<_>>();

            dest_files.par_iter().for_each(|dest_file| {
                let dest_path = dest_file.path().strip_prefix(&destination);

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
    }

    Ok(())
}
