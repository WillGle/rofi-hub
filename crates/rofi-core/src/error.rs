use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to read manifest {path}: {source}")]
    ManifestRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse manifest {path}: {source}")]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("invalid provider manifest {path}: {reason}")]
    InvalidManifest { path: PathBuf, reason: String },

    #[error("unknown provider `{0}`")]
    UnknownProvider(String),

    #[error("rofi is not installed")]
    RofiMissing,

    #[error("required command `{0}` was not found in PATH")]
    CommandMissing(String),

    #[error("rofi exited with status {0}")]
    RofiStatus(i32),

    #[error("cannot start `{command}`: {source}")]
    Spawn {
        command: String,
        source: std::io::Error,
    },

    #[error("no provider manifests found")]
    NoProviders,
}
