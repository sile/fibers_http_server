use bytecodec;
use std;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};
use trackable::error::{Failure, TrackableError};
use url;

/// This crate specific `Error` type.
#[derive(Debug, Clone, TrackableError)]
pub struct Error(TrackableError<ErrorKind>);
impl From<Failure> for Error {
    fn from(f: Failure) -> Self {
        ErrorKind::Other.takes_over(f).into()
    }
}
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<bytecodec::Error> for Error {
    fn from(f: bytecodec::Error) -> Self {
        let kind = match *f.kind() {
            bytecodec::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
            _ => ErrorKind::Other,
        };
        kind.takes_over(f).into()
    }
}
impl From<url::ParseError> for Error {
    fn from(f: url::ParseError) -> Self {
        ErrorKind::InvalidInput.cause(f).into()
    }
}

/// Possible error kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum ErrorKind {
    InvalidInput,
    Other,
}
impl TrackableErrorKind for ErrorKind {}
