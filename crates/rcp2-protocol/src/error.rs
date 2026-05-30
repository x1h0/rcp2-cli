use std::fmt;

#[derive(Debug)]
pub enum Error {
    Parse(String),
    Transport(String),
    Timeout,
    State(String),
    Protocol(String),
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "parse error: {msg}"),
            Error::Transport(msg) => write!(f, "transport error: {msg}"),
            Error::Timeout => write!(f, "timeout"),
            Error::State(msg) => write!(f, "state error: {msg}"),
            Error::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Error::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<hidapi::HidError> for Error {
    fn from(err: hidapi::HidError) -> Self {
        Error::Transport(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
