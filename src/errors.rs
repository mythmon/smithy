use std::io;
use std::error::{Error};
use std::fmt::{self, Display, Formatter};
use std::path::StripPrefixError;

use walkdir::Error as WalkDirError;

#[derive(Debug)]
pub struct SmithyError {
    descr: String,
    cause: Option<Box<Error>>,
}

impl SmithyError {
    pub fn new<T: Into<String>>(descr: T, cause: Option<Box<Error>>) -> Self {
        SmithyError {
            descr: descr.into(),
            cause: cause,
        }
    }
}

impl Display for SmithyError {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        try!(write!(fmt, "{}", self.descr));
        Ok(())
    }
}

impl Error for SmithyError {
    fn description(&self) -> &str {
        &self.descr
    }
}

impl From<io::Error> for SmithyError {
    fn from(err: io::Error) -> SmithyError {
        SmithyError::new(format!("IO error: {}", err), Some(Box::new(err)))
    }
}

impl From<StripPrefixError> for SmithyError {
    fn from(err: StripPrefixError) -> SmithyError {
        SmithyError::new("Cannot process file outside of input path", Some(Box::new(err)))
    }
}

impl From<WalkDirError> for SmithyError {
    fn from(err: WalkDirError) -> SmithyError {
        SmithyError::new("Could not traverse input dir", Some(Box::new(err)))
    }
}
