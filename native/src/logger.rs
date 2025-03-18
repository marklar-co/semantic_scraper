use std::fs;
use std::io;

use log::{LevelFilter, SetLoggerError};
use simplelog::{Config, WriteLogger};

use crate::dummy;

/// Initialize the global logger.
///
/// Tries to open a 'real' file. If anything fails, falls back to stderr.
pub fn register() -> Result<(), SetLoggerError> {
    let level_filter = LevelFilter::Info;
    let config = Config::default();

    let result = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dummy::LOG_FILEPATH);

    match result {
        Ok(file) => WriteLogger::init(level_filter, config, file),

        Err(error) => {
            eprintln!(
                "Failed to open log file. Using stderr as fallback. {}",
                error
            );
            WriteLogger::init(level_filter, config, io::stderr())
        }
    }
}
