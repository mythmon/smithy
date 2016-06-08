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
        SmithyError {
            descr: "Encountered an IO error".to_string(),
            cause: Some(Box::new(err)),
        }
    }
}

impl From<StripPrefixError> for SmithyError {
    fn from(err: StripPrefixError) -> SmithyError {
        SmithyError {
            descr: "Cannot process file outside of input path".to_string(),
            cause: Some(Box::new(err)),
        }
    }
}

impl From<WalkDirError> for SmithyError {
    fn from(err: WalkDirError) -> SmithyError {
        SmithyError {
            descr: "Could not traverse input dir".to_string(),
            cause: Some(Box::new(err)),
        }
    }
}
