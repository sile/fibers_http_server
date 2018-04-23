use httpcodec::{BodyDecode, BodyEncode, Header, HttpVersion, ReasonPhrase, Request, Response,
                StatusCode};

use Error;
use status::Status;

pub trait HandleRequest: Sized {
    const METHOD: &'static str;
    const PATH: &'static str;

    type ReqBody;
    type ResBody;
    type ReqBodyDecoder: BodyDecode;
    type ResBodyEncoder: BodyEncode;

    #[allow(unused_variables)]
    fn handle_request_head(&self, req: &Req<()>) -> Option<Res<Self::ResBody>> {
        None
    }

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Reply<Self>;

    #[allow(unused_variables)]
    fn handle_decoding_error(&self, error: &Error) -> Option<Res<Self::ResBody>> {
        None
    }

    #[allow(unused_variables)]
    fn enable_async_decoding(&self, req: &Req<()>) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn enable_async_encoding(&self, res: &Res<Self::ResBody>) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct Reply<T>(T);
impl<T: HandleRequest> Reply<T> {
    // pub fn done(response: Response<T::ResBody>)
}

#[derive(Debug)]
pub struct Res<T>(Response<T>);
impl<T> Res<T> {
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

    // pub fn header_mut(&mut self)
    // pub fn body_mut(&mut self)
}
impl<T> From<Response<T>> for Res<T> {
    fn from(f: Response<T>) -> Self {
        Res(f)
    }
}

#[derive(Debug)]
pub struct Req<T>(Request<T>);
impl<T> Req<T> {
    pub fn method(&self) -> &str {
        self.0.method().as_str()
    }

    pub fn path(&self) -> &str {
        self.0.request_target().as_str()
    }

    pub fn http_version(&self) -> HttpVersion {
        self.0.http_version()
    }

    // TODO: wrap
    pub fn header(&self) -> Header {
        self.0.header()
    }

    pub fn body(&self) -> &T {
        self.0.body()
    }

    pub fn into_body(self) -> T {
        self.0.into_body()
    }
}
