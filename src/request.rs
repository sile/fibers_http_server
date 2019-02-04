use crate::{Error, ErrorKind, Result};
use httpcodec::{Header, HttpVersion, Request};
use std::fmt;
use url::Url;

/// HTTP request.
#[derive(Debug)]
pub struct Req<T> {
    inner: Request<T>,
    url: Url,
}
impl<T> Req<T> {
    /// Returns the method of the request.
    pub fn method(&self) -> &str {
        self.inner.method().as_str()
    }

    /// Returns the URL of the request.
    ///
    /// Note that the peer address of the client socket is used as the host and port of the URL.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns the HTTP version of the request.
    pub fn version(&self) -> HttpVersion {
        self.inner.http_version()
    }

    /// Returns the header of the response.
    pub fn header(&self) -> Header {
        self.inner.header()
    }

    /// Returns a reference to the body of the response.
    pub fn body(&self) -> &T {
        self.inner.body()
    }

    /// Takes ownership of the request, and returns its body.
    pub fn into_body(self) -> T {
        self.inner.into_body()
    }

    /// Splits the head part and the body part of the request.
    pub fn take_body(self) -> (Req<()>, T) {
        let (inner, body) = self.inner.take_body();
        let req = Req {
            inner,
            url: self.url,
        };
        (req, body)
    }

    pub(crate) fn map_body<U, F>(self, f: F) -> Req<U>
    where
        F: FnOnce(T) -> U,
    {
        let inner = self.inner.map_body(f);
        Req {
            inner,
            url: self.url,
        }
    }

    pub(crate) fn new(inner: Request<T>, base_url: &Url) -> Result<Self> {
        track_assert!(
            inner.request_target().as_str().starts_with('/'),
            ErrorKind::InvalidInput,
            "path={:?}",
            inner.request_target()
        );
        let url = track!(
            Url::options()
                .base_url(Some(base_url))
                .parse(inner.request_target().as_str())
                .map_err(Error::from),
            "path={:?}",
            inner.request_target()
        )?;
        Ok(Req { inner, url })
    }
}
impl<T: fmt::Display> fmt::Display for Req<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}
