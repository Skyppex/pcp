use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use os_str_bytes::{OsStrBytes, OsStringBytes};

const PROGRESS_DIR: &'static str = ".pcp";
const COMPLETED_FILE_NAME: &'static str = ".pcp-completed.pcp";

pub struct CompletionTracker {
    file: Option<File>,
    dest: Option<PathBuf>,
    path: Option<PathBuf>,
    len: u64,
}

impl CompletionTracker {
    pub fn open(dest_dir: impl AsRef<Path>, enabled: bool) -> std::io::Result<CompletionTracker> {
        if !enabled {
            return Ok(CompletionTracker {
                file: None,
                dest: None,
                path: None,
                len: 0,
            });
        }

        let dest_dir = dest_dir.as_ref();

        let completed_file_path = dest_dir.join(PROGRESS_DIR).join(COMPLETED_FILE_NAME);

        if !completed_file_path.parent().unwrap().exists() {
            std::fs::create_dir_all(completed_file_path.parent().unwrap())?;
        }

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(&completed_file_path)?;

        let len = file.metadata()?.len();

        Ok(CompletionTracker {
            file: Some(file),
            dest: Some(dest_dir.to_path_buf()),
            path: Some(completed_file_path),
            len,
        })
    }

    pub fn read(&mut self) -> HashSet<OsString> {
        let Some(file) = &mut self.file else {
            return HashSet::new();
        };

        let mut buf = vec![];
        let Ok(bytes_read) = file.read_to_end(&mut buf) else {
            return HashSet::new();
        };

        if bytes_read == 0 {
            return HashSet::new();
        }

        buf.split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| OsString::from_io_vec(line.to_vec()).unwrap())
            .collect::<HashSet<_>>()
    }

    pub fn add_completed(&mut self, completed: impl AsRef<Path>) -> std::io::Result<()> {
        let Some(file) = &mut self.file else {
            return Ok(());
        };

        let bytes = completed
            .as_ref()
            .strip_prefix(self.dest.as_ref().unwrap())
            .unwrap()
            .as_os_str()
            .to_io_bytes()
            .ok_or_else(|| std::io::ErrorKind::Other)?;

        file.write_all(bytes)?;
        file.write_all(b"\n")
    }

    pub fn remove(self) -> std::io::Result<()> {
        let Some(path) = self.path else {
            return Ok(());
        };

        std::fs::remove_file(path)
    }
}

pub fn cleanup(dest: impl AsRef<Path>) -> std::io::Result<()> {
    let progress_dir_path = dest.as_ref().join(PROGRESS_DIR);

    if let Err(e) = std::fs::remove_dir(progress_dir_path) {
        if e.kind() == std::io::ErrorKind::NotFound {
            // its fine if the directory doesn't exist
            return Ok(());
        }

        return Err(e);
    }

    Ok(())
}

// pub fn open_progress_file(
//     dest_dir: impl AsRef<Path>,
//     file: impl AsRef<Path>,
// ) -> std::io::Result<File> {
//     let progress_file_path = dest_dir.as_ref().join(PROGRESS_DIR).join(file);
//
//     OpenOptions::new()
//         .write(true)
//         .read(true)
//         .create(true)
//         .truncate(true)
//         .open(progress_file_path)
// }
