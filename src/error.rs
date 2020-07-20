use parse_mediawiki_dump::Error as DumpParsingError;
use std::{fmt::Display, io::Error as IoError, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError {
        action: &'static str,
        path: PathBuf,
        cause: IoError,
    },
    NotTwoTabs {
        path: PathBuf,
        line_number: usize,
        line: String,
        tabs: usize,
    },
    DumpParsingError(DumpParsingError),
}

impl Error {
    pub fn io_error(
        action: &'static str,
        path: impl Into<PathBuf>,
        cause: IoError,
    ) -> Error {
        Error::IoError {
            action,
            path: path.into(),
            cause,
        }
    }
}

impl From<DumpParsingError> for Error {
    fn from(e: DumpParsingError) -> Self {
        Error::DumpParsingError(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotTwoTabs {
                path,
                line_number,
                line,
                tabs,
            } => write!(
                f,
                "line #{} in {} contained {} tabs, one expected: {}",
                line_number,
                path.display(),
                tabs,
                line
            ),
            Error::DumpParsingError(e) => {
                write!(f, "error while parsing XML: {}", e)
            }
            Error::IoError {
                action,
                path,
                cause,
            } => {
                write!(f, "failed to {} {}: {}", action, path.display(), cause)
            }
        }
    }
}
