use httpcodec::{Header, HttpVersion, Request};

/// HTTP request.
#[derive(Debug)]
pub struct Req<T>(Request<T>);
impl<T> Req<T> {
    /// Returns the method of the request.
    pub fn method(&self) -> &str {
        self.0.method().as_str()
    }

    /// Returns the target path of the request.
    ///
    /// Note that the string also contains the query and fragment parts of URL
    /// in addition to the path part.
    pub fn path(&self) -> &str {
        self.0.request_target().as_str()
    }

    /// Returns the HTTP version of the request.
    pub fn version(&self) -> HttpVersion {
        self.0.http_version()
    }

    /// Returns the header of the response.
    pub fn header(&self) -> Header {
        self.0.header()
    }

    /// Returns a reference to the body of the response.
    pub fn body(&self) -> &T {
        self.0.body()
    }

    /// Takes ownership of the request, and returns its body.
    pub fn into_body(self) -> T {
        self.0.into_body()
    }

    pub(crate) fn map_body<U, F>(self, f: F) -> Req<U>
    where
        F: FnOnce(T) -> U,
    {
        Req(self.0.map_body(f))
    }
}
