use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use os_str_bytes::{OsStrBytes, OsStringBytes};

const PROGRESS_DIR: &'static str = ".pcp";
const COMPLETED_FILE_NAME: &'static str = ".pcp-completed.pcp";
const PROGRESS_EXT: &'static str = ".pcp";
const NEW_LINE_BUFFER: usize = 128;
const BLANK_SPACE_CHAR: &'static str = " "; // space

pub struct CompletionTracker {
    completed_file: Option<File>,
    dest: Option<PathBuf>,
    completed_path: Option<PathBuf>,
    progress_files: HashMap<PathBuf, Progress>,
}

struct Progress {
    file: File,
}

impl CompletionTracker {
    pub fn open(dest_dir: impl AsRef<Path>, enabled: bool) -> std::io::Result<CompletionTracker> {
        if !enabled {
            return Ok(CompletionTracker {
                completed_file: None,
                dest: None,
                completed_path: None,
                progress_files: HashMap::new(),
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

        Ok(CompletionTracker {
            completed_file: Some(file),
            dest: Some(dest_dir.to_path_buf()),
            completed_path: Some(completed_file_path),
            progress_files: HashMap::new(),
        })
    }

    pub fn read(&mut self) -> HashSet<OsString> {
        let Some(file) = &mut self.completed_file else {
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
        let Some(file) = &mut self.completed_file else {
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
        let Some(path) = self.completed_path else {
            return Ok(());
        };

        std::fs::remove_file(path)
    }

    pub fn add_progress_file(
        &mut self,
        file_name: impl AsRef<Path>,
        total_bytes: usize,
    ) -> std::io::Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };

        let file_path = dest.join(file_name.as_ref()).join(PROGRESS_EXT);

        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(false)
            .open(&file_path)?;

        file.write_all(
            format!(
                "{}{}\n{}",
                "0",
                BLANK_SPACE_CHAR.repeat(NEW_LINE_BUFFER - 1),
                total_bytes
            )
            .as_bytes(),
        )?;

        let progress = Progress { file };

        let _ = self.progress_files.insert(file_path, progress);

        Ok(())
    }

    pub fn write_progress(
        &mut self,
        file_name: impl AsRef<Path>,
        current_bytes: usize,
    ) -> std::io::Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };

        let file_path = dest.join(file_name).join(PROGRESS_EXT);
        let progress = self
            .progress_files
            .get_mut(&file_path)
            .ok_or_else(|| std::io::ErrorKind::NotFound)?;

        let file = &mut progress.file;
        let data = current_bytes.to_string();
        let bytes = data.as_bytes();

        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(bytes)?;
        file.sync_data()?;

        Ok(())
    }

    pub fn remove_progress_file(&mut self, file_name: impl AsRef<Path>) -> bool {
        self.progress_files.remove(file_name.as_ref()).is_some()
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
