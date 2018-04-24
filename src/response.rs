use std::fmt;
use httpcodec::{Header, HeaderMut, HttpVersion, ReasonPhrase, Response, StatusCode};

use status::Status;

/// HTTP response.
///
/// `T` is the type of the response body.
#[derive(Debug)]
pub struct Res<T>(Response<T>);
impl<T> Res<T> {
    /// Makes a new `Res` instance.
    pub fn new(status: Status, body: T) -> Self {
        let inner = unsafe {
            Response::new(
                HttpVersion::V1_1,
                StatusCode::new_unchecked(status.code()),
                ReasonPhrase::new_unchecked(status.reason_phrase()),
                body,
            )
        };
        Res(inner)
    }

    /// Returns the HTTP version of the response.
    pub fn version(&self) -> HttpVersion {
        self.0.http_version()
    }

    /// Returns the status code of the response.
    pub fn status_code(&self) -> u16 {
        self.0.status_code().as_u16()
    }

    /// Returns the header of the response.
    pub fn header(&self) -> Header {
        self.0.header()
    }

    /// Returns the mutable header of the response.
    pub fn header_mut(&mut self) -> HeaderMut {
        self.0.header_mut()
    }

    /// Returns a reference to the body of the response.
    pub fn body(&self) -> &T {
        self.0.body()
    }

    /// Returns a mutable reference to the body of the response.
    pub fn body_mut(&mut self) -> &mut T {
        self.0.body_mut()
    }
}
impl<T: fmt::Display> fmt::Display for Res<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<T> From<Response<T>> for Res<T> {
    fn from(f: Response<T>) -> Self {
        Res(f)
    }
}
