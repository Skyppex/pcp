use std::{
    fs,
    io::{self, Read, Result, Write},
};

use crate::path_utils::convert_to_path;

#[derive(Debug)]
pub enum Reader {
    File(fs::File),
    Stdin(io::Stdin),
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Reader::File(file) => file.read(buf),
            Reader::Stdin(stdin) => stdin.read(buf),
        }
    }
}

impl From<fs::File> for Reader {
    fn from(file: fs::File) -> Self {
        Reader::File(file)
    }
}

impl From<io::Stdin> for Reader {
    fn from(stdin: io::Stdin) -> Self {
        Reader::Stdin(stdin)
    }
}

#[derive(Debug)]
pub enum Writer {
    File(fs::File),
    Stdout(io::Stdout),
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Writer::File(file) => file.write(buf),
            Writer::Stdout(stdout) => stdout.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Writer::File(file) => file.flush(),
            Writer::Stdout(stdout) => stdout.flush(),
        }
    }
}

impl From<fs::File> for Writer {
    fn from(file: fs::File) -> Self {
        Writer::File(file)
    }
}

impl From<io::Stdout> for Writer {
    fn from(stdout: io::Stdout) -> Self {
        Writer::Stdout(stdout)
    }
}

pub fn get_reader(source: Option<&str>) -> Result<Reader> {
    match convert_to_path(source)? {
        Some(src) => Ok(fs::File::open(&src)
            .unwrap_or_else(|_| panic!("Failed to open file {:?}", src))
            .into()),
        None => Ok(io::stdin().into()),
    }
}

pub fn get_writer(destination: Option<&str>) -> Result<Writer> {
    match convert_to_path(destination)? {
        Some(dest) => {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|_| panic!("Failed to create directories for {:?}", parent));
            }

            Ok(fs::File::create(&dest)
                .unwrap_or_else(|_| panic!("Failed to create file {:?}", dest))
                .into())
        }
        None => Ok(io::stdout().into()),
    }
}
