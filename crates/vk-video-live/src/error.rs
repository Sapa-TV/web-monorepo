use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::result::Result as StdResult;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    Http(String),
    Protocol(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(m) => write!(f, "http: {m}"),
            Error::Protocol(m) => write!(f, "protocol: {m}"),
        }
    }
}

impl StdError for Error {}

pub type Result<T, E = Error> = StdResult<T, E>;
