use ini;
use std::{result, io, fmt};
use std::num::ParseIntError;

#[derive(Debug)]
pub enum Error {
    Custom(String),
    Ini(ini::Error),
    Io(io::Error),
    ParseInt(ParseIntError),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Custom(ref e) => write!(f, "{}", e),
            Error::Ini(ref e) => ini::Error::fmt(e, f),
            Error::Io(ref e) => io::Error::fmt(e, f),
            Error::ParseInt(ref e) => ParseIntError::fmt(e, f),
        }
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Custom(s.to_owned())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Custom(s)
    }
}

impl From<ini::Error> for Error {
    fn from(e: ini::Error) -> Self {
        Error::Ini(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::ParseInt(e)
    }
}
