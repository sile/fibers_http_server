use std::fmt;
use bytecodec::{self, ByteCount, Encode, EncodeExt, Eos};
use bytecodec::bytes::Utf8Encoder;
use bytecodec::marker::Never;
use httpcodec::{BodyEncoder, Header, HeaderMut, HttpVersion, ReasonPhrase, Response,
                ResponseEncoder, StatusCode};

use header;
use status::Status;

/// HTTP response.
///
/// `T` is the type of the response body.
#[derive(Debug)]
pub struct Res<T>(pub(crate) Response<T>);
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

pub struct ResEncoder(Box<Encode<Item = Never> + Send + 'static>);
impl ResEncoder {
    pub fn new<E>(inner: E) -> Self
    where
        E: Encode<Item = Never> + Send + 'static,
    {
        ResEncoder(Box::new(inner))
    }

    pub fn error(status: Status) -> Self {
        let mut res = Res::new(status, status.reason_phrase());
        res.header_mut().add_field(header::Connection::Close);

        let mut encoder = ResponseEncoder::new(BodyEncoder::new(Utf8Encoder::new()));
        encoder.start_encoding(res.0).expect("Never fails");
        ResEncoder(Box::new(encoder.last()))
    }
}
impl fmt::Debug for ResEncoder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ResEncoder(_)")
    }
}
impl Encode for ResEncoder {
    type Item = Never;

    fn encode(&mut self, buf: &mut [u8], eos: Eos) -> bytecodec::Result<usize> {
        self.0.encode(buf, eos)
    }

    fn start_encoding(&mut self, _item: Self::Item) -> bytecodec::Result<()> {
        unreachable!()
    }

    fn is_idle(&self) -> bool {
        self.0.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        self.0.requiring_bytes()
    }
}
