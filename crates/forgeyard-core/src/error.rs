use std::fmt;
use std::io;
use std::process::ExitCode;

/// Process exit codes from SPEC.md §8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Exit {
    Ok = 0,
    Precondition = 1,
    Usage = 2,
    Conflict = 3,
    Busy = 4,
    Tokens = 5,
    Io = 10,
}

impl From<Exit> for ExitCode {
    fn from(e: Exit) -> Self {
        ExitCode::from(e as u8)
    }
}

impl From<Exit> for i32 {
    fn from(e: Exit) -> Self {
        e as i32
    }
}

#[derive(Debug)]
pub enum ForgeError {
    Usage(String),
    Precondition(String),
    Conflict(String),
    Busy(String),
    Tokens(String),
    Io(io::Error),
}

impl ForgeError {
    pub fn exit(&self) -> Exit {
        match self {
            ForgeError::Usage(_) => Exit::Usage,
            ForgeError::Precondition(_) => Exit::Precondition,
            ForgeError::Conflict(_) => Exit::Conflict,
            ForgeError::Busy(_) => Exit::Busy,
            ForgeError::Tokens(_) => Exit::Tokens,
            ForgeError::Io(_) => Exit::Io,
        }
    }
}

impl fmt::Display for ForgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForgeError::Usage(s)
            | ForgeError::Precondition(s)
            | ForgeError::Conflict(s)
            | ForgeError::Busy(s)
            | ForgeError::Tokens(s) => write!(f, "{s}"),
            ForgeError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ForgeError {}

impl From<io::Error> for ForgeError {
    fn from(e: io::Error) -> Self {
        ForgeError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, ForgeError>;
