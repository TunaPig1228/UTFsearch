use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("catalog is corrupt: {0}")]
    Corrupt(&'static str),
    #[error("unsupported catalog version {0}")]
    Version(u16),
    #[error("query rejected: {0}")]
    Query(&'static str),
    #[error("root is nested inside another root: {0}")]
    NestedRoot(String),
    #[error("root does not exist or is not a directory: {0}")]
    MissingRoot(PathBuf),
    #[error("path is outside every Root")]
    Jail,
    #[error("invalid cursor")]
    Cursor,
    #[error("{0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, Error>;
