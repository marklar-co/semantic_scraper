use std::fs::{self, File};
use std::io;

use log::error;

pub enum LogFile {
    File(File),
    Stderr(io::Stderr),
}

impl LogFile {
    #[cfg(target_family = "unix")]
    const FILEPATH: &str = r"/tmp/nativeext.log";
    #[cfg(target_family = "windows")]
    const FILEPATH: &str = r"C:\Windows\Temp\nativeext.log";

    pub fn new() -> Self {
        let result = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(Self::FILEPATH);

        match result {
            Ok(file) => Self::File(file),
            Err(error) => {
                error!(
                    "Failed to open log file. Using stderr as fallback. {}",
                    error
                );
                Self::Stderr(io::stderr())
            }
        }
    }
}

impl io::Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.write(buf),
            Self::Stderr(stderr) => stderr.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(file) => file.flush(),
            Self::Stderr(stderr) => stderr.flush(),
        }
    }
}
