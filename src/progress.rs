use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

const PROGRESS_DIR: &'static str = ".pcp";
const COMPLETED_FILE_NAME: &'static str = ".pcp-completed.pcp";

pub struct CompletionTracker {
    file: File,
    path: PathBuf,
    len: u64,
}

impl CompletionTracker {
    pub fn open(dest_dir: impl AsRef<Path>) -> std::io::Result<CompletionTracker> {
        let completed_file_path = dest_dir
            .as_ref()
            .join(PROGRESS_DIR)
            .join(COMPLETED_FILE_NAME);

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
            file,
            path: completed_file_path,
            len,
        })
    }

    pub fn add_completed(&mut self, completed: impl AsRef<Path>) -> std::io::Result<()> {
        let bytes = completed.as_ref().as_os_str().as_bytes();
        self.file.write_all(bytes)?;
        self.file.write_all(b"\n")
    }

    pub fn remove(self) -> std::io::Result<()> {
        std::fs::remove_file(self.path)
    }
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
