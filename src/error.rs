use dump_parser::Error as DumpParsingError;
use std::{fmt::Display, io::Error as IoError, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError {
        action: &'static str,
        path: PathBuf,
        cause: IoError,
    },
    InvalidNameToCodeFormat {
        path: PathBuf,
        line_number: usize,
        line: String,
    },
    InvalidLanguageCode {
        path: PathBuf,
        line_number: usize,
        line: String,
    },
    DumpParsingError(DumpParsingError),
}

impl Error {
    pub fn from_io(
        cause: IoError,
        action: &'static str,
        path: impl Into<PathBuf>,
    ) -> Error {
        Error::IoError {
            action,
            path: path.into(),
            cause,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidNameToCodeFormat {
                path,
                line_number,
                line,
            } => write!(
                f,
                "line #{} in {} did not contain a language name, a tab, and a language code: {}",
                line_number,
                path.display(),
                line
            ),
            Error::InvalidLanguageCode {
                path,
                line_number,
                line,
            } => write!(
                f,
                "line #{} in {} contained an invalid language code: {}",
                line_number,
                path.display(),
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

impl From<DumpParsingError> for Error {
    fn from(e: DumpParsingError) -> Self {
        Error::DumpParsingError(e)
    }
}
