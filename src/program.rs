use std::fs;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use walkdir::WalkDir;

use crate::{
    cli::Cli,
    file_operations::{copy_files_in_parallel, move_files_in_parallel},
    path_utils::get_path,
};

pub fn run(cli: Cli) -> std::io::Result<()> {
    let source = get_path(&cli.source)?;
    let destination = get_path(&cli.destination)?;

    let files = WalkDir::new(&source)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect::<Vec<_>>();

    if cli.move_files {
        move_files_in_parallel(cli.clone(), &source, &destination, &files);
    } else {
        copy_files_in_parallel(cli.clone(), &source, &destination, &files);
    }

    if cli.purge {
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
                fs::remove_file(dest_file.path()).expect("Error removing file");
            }
        });
    }

    Ok(())
}
