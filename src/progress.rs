use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};

use os_str_bytes::{OsStrBytes, OsStringBytes};

const PROGRESS_DIR: &str = ".pcp";
const COMPLETED_FILE_NAME: &str = ".pcp-completed.pcp";
const PROGRESS_EXT: &str = ".pcp";
const NEW_LINE_BUFFER: usize = 128;
const BLANK_SPACE_CHAR: &str = " "; // space

pub struct CompletionTracker {
    completed_file: Option<Mutex<File>>,
    dest: Option<PathBuf>,
    completed_path: Option<PathBuf>,
    progress_files: RwLock<HashMap<PathBuf, ProgressFile>>,
}

#[derive(Debug)]
struct ProgressFile {
    file: Mutex<File>,
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub current: u64,
    pub total: u64,
}

impl From<&ProgressFile> for Progress {
    fn from(value: &ProgressFile) -> Self {
        Progress {
            current: value.current,
            total: value.total,
        }
    }
}

impl CompletionTracker {
    pub fn open(dest_dir: impl AsRef<Path>, enabled: bool) -> std::io::Result<CompletionTracker> {
        if !enabled {
            return Ok(CompletionTracker {
                completed_file: None,
                dest: None,
                completed_path: None,
                progress_files: RwLock::new(HashMap::new()),
            });
        }

        let dest_dir = dest_dir.as_ref();
        let completed_file_path = dest_dir.join(PROGRESS_DIR).join(COMPLETED_FILE_NAME);

        if !completed_file_path.parent().unwrap().exists() {
            std::fs::create_dir_all(completed_file_path.parent().unwrap())?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&completed_file_path)?;

        Ok(CompletionTracker {
            completed_file: Some(Mutex::new(file)),
            dest: Some(dest_dir.to_path_buf()),
            completed_path: Some(completed_file_path),
            progress_files: RwLock::new(HashMap::new()),
        })
    }

    pub fn read(&mut self) -> HashSet<OsString> {
        let Some(file) = &mut self.completed_file else {
            return HashSet::new();
        };

        let mut buf = vec![];
        let Ok(bytes_read) = file
            .get_mut()
            .expect("failed to get mut file")
            .read_to_end(&mut buf)
        else {
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

    pub fn add_completed(&self, completed: impl AsRef<Path>) -> std::io::Result<()> {
        let (Some(file), Some(dest)) = (&self.completed_file, &self.dest) else {
            return Ok(());
        };

        let bytes = completed
            .as_ref()
            .strip_prefix(dest)
            .unwrap()
            .as_os_str()
            .to_io_bytes()
            .ok_or(std::io::ErrorKind::Other)?;

        let mut file = file.lock().expect("Failed to lock file");
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
        &self,
        file_name: impl AsRef<OsStr>,
        total_bytes: u64,
    ) -> std::io::Result<Option<Progress>> {
        let Some(dest) = &self.dest else {
            return Ok(None);
        };

        let file_path = get_progress_file_path(dest, file_name);

        if std::fs::exists(&file_path)? {
            let open = OpenOptions::new().read(true).write(true).open(&file_path);

            return match open {
                Ok(mut file) => {
                    let content = &mut String::new();
                    file.read_to_string(content)?;
                    let (current, total) = content.split_once('\n').unwrap();

                    let progress_file = ProgressFile {
                        file: Mutex::new(file),
                        current: current.trim_end().parse().unwrap(),
                        total: total.parse().unwrap(),
                    };

                    let progress = (&progress_file).into();

                    _ = self
                        .progress_files
                        .write()
                        .expect("Failed to obtain write lock")
                        .insert(file_path, progress_file);

                    Ok(Some(progress))
                }
                Err(e) => Err(e),
            };
        }

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

        let progress_file = ProgressFile {
            file: Mutex::new(file),
            current: 0,
            total: total_bytes,
        };

        let progress = (&progress_file).into();

        _ = self
            .progress_files
            .write()
            .expect("Failed to obtain write lock")
            .insert(file_path, progress_file);

        // yes, we have progress, but its just initialized. its also moved into the progress_files
        // map. we can't return it here without cloning and thats not possible on the mutex inside
        // the progress. would have to make it an Arc<Mutex<File>> then but it's not necessary.
        Ok(Some(progress))
    }

    pub fn write_progress(
        &self,
        file_name: impl AsRef<OsStr>,
        current_bytes: u64,
    ) -> std::io::Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };

        let file_path = get_progress_file_path(dest, file_name);

        let read = self.progress_files.read();
        let hash_map = read.expect("Failed to obtain read access");

        let progress = hash_map
            .get(&file_path)
            .ok_or(std::io::ErrorKind::NotFound)?;

        let mut file = progress.file.lock().expect("Failed to lock file");
        let data = current_bytes.to_string();
        let bytes = data.as_bytes();

        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(bytes)?;
        file.sync_data()?;

        Ok(())
    }

    pub fn remove_progress_file(&self, file_name: impl AsRef<OsStr>) -> std::io::Result<()> {
        let Some(dest) = &self.dest else {
            return Ok(());
        };

        let file_path = get_progress_file_path(dest, file_name);

        self.progress_files
            .write()
            .expect("Failed to obtain write lock")
            .remove(&file_path);

        std::fs::remove_file(file_path)
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

fn get_progress_file_name(file_name: impl AsRef<OsStr>) -> OsString {
    let mut progress_file_name = file_name.as_ref().to_os_string();
    progress_file_name.push(PROGRESS_EXT);
    progress_file_name
}

fn get_progress_file_path(dest: impl AsRef<Path>, file_name: impl AsRef<OsStr>) -> PathBuf {
    let progress_file_name = get_progress_file_name(file_name);
    dest.as_ref().join(PROGRESS_EXT).join(progress_file_name)
}
