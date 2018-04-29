use httpcodec::{Header, HttpVersion, Request};
use url::Url;

use {Error, ErrorKind, Result};

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
            ErrorKind::InvalidInput;
            inner.request_target()
        );
        let url = track!(
            Url::options()
                .base_url(Some(base_url))
                .parse(inner.request_target().as_str())
                .map_err(Error::from)
        )?;
        Ok(Req { inner, url })
    }
}
// TODO: impl Display
